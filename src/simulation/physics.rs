use console::style;
use crate::bindings::pumas;
use crate::simulation::materials::{Materials, MaterialsData};
use crate::utils::cache;
use crate::utils::convert::{Bremsstrahlung, Mdf, PairProduction, Photonuclear};
use crate::utils::error::{self, Error};
use crate::utils::error::ErrorKind::KeyboardInterrupt;
use indicatif::{ProgressBar, ProgressStyle};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString, PyTuple};
use temp_dir::TempDir;
use ::std::borrow::Cow;
use ::std::ffi::{c_char, c_int, CStr, CString, c_uint};
use ::std::fs::File;
use ::std::os::fd::IntoRawFd;
use ::std::ptr::{null, null_mut};


#[pyclass(module="danton")]
pub struct Physics {
    /// The Bremsstrahlung model for tau energy losses.
    #[pyo3(get)]
    bremsstrahlung: Bremsstrahlung,
    /// The e+e- pair-production model for tau energy losses.
    #[pyo3(get)]
    pair_production: PairProduction,
    /// The photonuclear model for tau energy losses.
    #[pyo3(get)]
    photonuclear: Photonuclear,

    physics: *mut pumas::Physics,
    materials_instance: Option<usize>,
    pub modified: bool,
}

unsafe impl Send for Physics {}

#[pymethods]
impl Physics {
    #[pyo3(signature=(*, **kwargs))]
    #[new]
    pub fn new<'py>(
        py: Python<'py>,
        kwargs: Option<&Bound<'py, PyDict>>,
    ) -> PyResult<Py<Self>> {
        let bremsstrahlung = Bremsstrahlung::default();
        let pair_production = PairProduction::default();
        let photonuclear = Photonuclear::default();
        let physics = null_mut();
        let materials_instance = None;
        let modified = false;

        let physics = Self {
            bremsstrahlung, pair_production, photonuclear, physics,
            materials_instance, modified,
        };
        let physics = Bound::new(py, physics)?;

        if let Some(kwargs) = kwargs {
            for (key, value) in kwargs.iter() {
                let key: Bound<PyString> = key.extract()?;
                physics.setattr(key, value)?
            }
        }

        Ok(physics.unbind())
    }

    #[setter]
    fn set_bremsstrahlung(&mut self, value: Bremsstrahlung) {
        if value != self.bremsstrahlung {
            self.bremsstrahlung = value;
            self.modified = true;
        }
    }

    #[setter]
    fn set_pair_production(&mut self, value: PairProduction) {
        if value != self.pair_production {
            self.pair_production = value;
            self.modified = true;
        }
    }

    #[setter]
    fn set_photonuclear(&mut self, value: Photonuclear) {
        if value != self.photonuclear {
            self.photonuclear = value;
            self.modified = true;
        }
    }
}

impl Drop for Physics {
    fn drop(&mut self) {
        self.destroy_physics();
    }
}

impl Physics {
    pub fn apply(
        &mut self,
        py: Python,
        materials: &Materials,
    ) -> PyResult<bool> {
        let modified = if self.modified || (self.materials_instance != Some(materials.instance)) {
            self.destroy_physics();
            self.create_physics(py, &materials.tag)?;
            self.materials_instance = Some(materials.instance);
            self.modified = false;
            true
        } else {
            false
        };

        // XXX Map materials indices?
        // XXX Forward physics object?

        Ok(modified)
    }

    fn check_pumas(rc: c_uint) -> PyResult<()> {
        if rc == pumas::SUCCESS {
            Ok(())
        } else {
            error::to_result(rc, Some("physics"))
        }
    }

    fn create_physics(&mut self, py: Python, materials: &str) -> PyResult<()> {
        // Load or create Pumas physics.
        let pumas = match self.load_pumas(materials) {
            None => self.create_pumas(py, materials)?,
            Some(pumas) => pumas,
        };
        self.physics = pumas;
        Ok(())
    }

    fn create_pumas(&self, py: Python, materials: &str) -> PyResult<*mut pumas::Physics> {
        let tag = self.pumas_physics_tag();
        let dump_path = cache::path()?
            .join("materials");
        let description = dump_path.join(format!("{}.toml", materials));
        let description = MaterialsData::from_file(py, &description)?;
        std::fs::create_dir_all(&dump_path)?;
        let dump_path = dump_path
            .join(format!("{}-{}.pumas", materials, tag));
        let dedx_path = TempDir::new()?;
        let mdf_path = dedx_path.path().join("materials.xml");
        Mdf::new(py, &description)
            .dump(&mdf_path)?;

        let c_bremsstrahlung: CString = self.bremsstrahlung.into();
        let c_pair_production: CString = self.pair_production.into();
        let c_photonuclear: CString = self.photonuclear.into();
        let mut settings = pumas::PhysicsSettings {
            cutoff: 0.0,
            elastic_ratio: 0.0,
            bremsstrahlung: c_bremsstrahlung.as_c_str().as_ptr(),
            pair_production: c_pair_production.as_c_str().as_ptr(),
            photonuclear: c_photonuclear.as_c_str().as_ptr(),
            n_energies: 0,
            energy: null_mut(),
            update: 0,
            dry: 0,
        };

        let physics = unsafe {
            let mut physics: *mut pumas::Physics = null_mut();
            let mdf_path = CString::new(mdf_path.to_string_lossy().as_ref())?;
            let dedx_path = CString::new(dedx_path.path().to_string_lossy().as_ref())?;
            let mut notifier = Notifier::new(materials);
            error::clear();
            let rc = pumas::physics_create(
                &mut physics,
                pumas::MUON,
                mdf_path.as_c_str().as_ptr(),
                dedx_path.as_c_str().as_ptr(),
                &mut settings,
                &mut notifier as *mut Notifier as *mut pumas::PhysicsNotifier,
            );
            if rc == pumas::INTERRUPT {
                // Ctrl-C has been catched.
                error::clear();
                let err = Error::new(KeyboardInterrupt)
                    .why("while computing materials tables");
                return Err(err.to_err())
            } else {
                Self::check_pumas(rc)?;
                physics
            }
        };

        // Cache physics data for subsequent usage.
        if let Ok(file) = File::create(dump_path) {
            unsafe {
                let stream = libc::fdopen(
                    file.into_raw_fd(),
                    CStr::from_bytes_with_nul_unchecked(b"wb\0").as_ptr(),
                );
                let rc = pumas::physics_dump(physics, stream);
                libc::fclose(stream);
                Self::check_pumas(rc)?;
            }
        };

        Ok(physics)
    }

    fn destroy_physics(&mut self) {
        unsafe {
            pumas::physics_destroy(&mut self.physics);
        }
    }

    fn load_pumas(&self, materials: &str) -> Option<*mut pumas::Physics> {
        let tag = self.pumas_physics_tag();
        let path = cache::path().ok()?
            .join(format!("materials/{}-{}.pumas", materials, tag));
        let file = File::open(path).ok()?;
        let mut physics = null_mut();
        let rc = unsafe {
            let stream = libc::fdopen(
                file.into_raw_fd(),
                CStr::from_bytes_with_nul_unchecked(b"rb\0").as_ptr(),
            );
            let rc = pumas::physics_load(&mut physics, stream);
            libc::fclose(stream);
            rc
        };
        error::clear();
        if rc != pumas::SUCCESS {
            return None;
        }

        let check_particle = || -> bool {
            let mut particle: c_uint = pumas::TAU;
            let rc = unsafe {
                pumas::physics_particle(physics, &mut particle, null_mut(), null_mut())
            };
            (rc == pumas::SUCCESS) && (particle == pumas::MUON)
        };

        let check_process = |process: c_uint, expected: &str| -> bool {
            unsafe {
                let mut name: *const c_char = null();
                let rc = pumas::physics_dcs(
                    physics,
                    process,
                    &mut name,
                    null_mut(),
                );
                if rc == pumas::SUCCESS {
                    CStr::from_ptr(name)
                        .to_str()
                        .ok()
                        .map(|name| name == expected)
                        .unwrap_or(false)
                } else {
                    false
                }
            }
        };
        if  check_particle() &&
            check_process(pumas::BREMSSTRAHLUNG, self.bremsstrahlung.as_pumas()) &&
            check_process(pumas::PAIR_PRODUCTION, self.pair_production.as_pumas()) &&
            check_process(pumas::PHOTONUCLEAR, self.photonuclear.as_pumas()) {
            Some(physics)
        } else {
            error::clear();
            unsafe { pumas::physics_destroy(&mut physics) };
            None
        }
    }

    fn pumas_physics_tag(&self)-> String {
        let bremsstrahlung: &str = self.bremsstrahlung.into();
        let pair_production: &str = self.pair_production.into();
        let photonuclear: &str = self.photonuclear.into();
        format!(
            "{}-{}-{}",
            bremsstrahlung,
            pair_production,
            photonuclear,
        )
    }
}


// ===============================================================================================
//
// Notifier for physics computations.
//
// ===============================================================================================

#[repr(C)]
struct Notifier {
    interface: pumas::PhysicsNotifier,
    bar: Option<ProgressBar>,
    client: String,
    section: usize,
}

impl Notifier {
    const SECTIONS: usize = 4;

    fn new(client: &str) -> Self {
        let interface = pumas::PhysicsNotifier {
            configure: Some(pumas_physics_notifier_configure),
            notify: Some(pumas_physics_notifier_notify),
        };
        Self { interface, bar: None, client: client.to_string(), section: 0 }
    }

    fn configure(&mut self, title: Option<&str>, steps: c_int) {
        self.bar = match self.bar {
            None => {
                self.section += 1;
                let title = title.unwrap();
                let title = if title.starts_with("multiple") {
                    Cow::Borrowed(title)
                } else {
                    Cow::Owned(format!("{}s", title))
                };
                let bar = ProgressBar::new(steps as u64);
                let bar_style = ProgressStyle::with_template(
                    "{msg} [{wide_bar:.dim}] {percent}%, {elapsed})"
                )
                    .unwrap()
                    .progress_chars("=> ");
                bar.set_style(bar_style);
                let section = style(format!("[{}/{}]", self.section, Self::SECTIONS)).dim();
                bar.set_message(format!("({} {} Computing {}", self.client, section, title));
                bar.set_position(0);
                Some(bar)
            },
            Some(_) => None,
        }
    }

    fn notify(&self) {
        self.bar.as_ref().unwrap().inc(1)
    }
}

impl Drop for Notifier {
    fn drop(&mut self) {
        if let Some(bar) = self.bar.as_ref() {
            bar.finish_and_clear()
        }
    }
}

#[no_mangle]
extern "C" fn pumas_physics_notifier_configure(
    slf: *mut pumas::PhysicsNotifier,
    title: *const c_char,
    steps: c_int
) -> c_uint {
    if error::ctrlc_catched() {
        pumas::INTERRUPT
    } else {
        let notifier = unsafe { &mut *(slf as *mut Notifier) };
        let title = if title.is_null() {
            None
        } else {
            let title = unsafe { CStr::from_ptr(title) };
            Some(title.to_str().unwrap())
        };
        notifier.configure(title, steps);
        pumas::SUCCESS
    }
}

#[no_mangle]
extern "C" fn pumas_physics_notifier_notify(slf: *mut pumas::PhysicsNotifier) -> c_uint {
    if error::ctrlc_catched() {
        pumas::INTERRUPT
    } else {
        let notifier = unsafe { &*(slf as *mut Notifier) };
        notifier.notify();
        pumas::SUCCESS
    }
}


// ===============================================================================================
//
// Pre-computation interface.
//
// ===============================================================================================

/// Compute materials tables.
#[pyfunction]
#[pyo3(signature=(*args, **kwargs))]
pub fn compute(
    py: Python,
    args: &Bound<PyTuple>,
    kwargs: Option<&Bound<PyDict>>,
) -> PyResult<()> {
    let mut physics = Physics::new(py, kwargs)?
        .bind(py)
        .borrow_mut();
    if args.is_empty() {
        let materials = Materials::new(py, None)?;
        physics
            .create_physics(py, materials.tag.as_str())?;
    } else {
        for arg in args.iter() {
            let arg: String = arg.extract()?;
            let materials = Materials::new(py, Some(arg.as_str()))?;
            physics
                .create_physics(py, materials.tag.as_str())?;
        }
    }
    Ok(())
}
