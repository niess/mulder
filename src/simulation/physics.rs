use console::style;
use crate::bindings::pumas;
use crate::simulation::materials::{Materials, MaterialsArg, MaterialsData};
use crate::utils::cache;
use crate::utils::convert::{Bremsstrahlung, Mdf, PairProduction, Photonuclear};
use crate::utils::error::{self, Error};
use crate::utils::error::ErrorKind::{KeyboardInterrupt, ValueError};
use crate::utils::io::PathString;
use crate::utils::notify;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString, PyTuple};
use temp_dir::TempDir;
use ::std::borrow::Cow;
use ::std::collections::HashMap;
use ::std::ffi::{c_char, c_int, CStr, CString, c_uint};
use ::std::fs::File;
use ::std::ptr::{null, null_mut};


#[pyclass(module="mulder")]
pub struct Physics {
    /// The Bremsstrahlung model for muon energy losses.
    #[pyo3(get)]
    bremsstrahlung: Bremsstrahlung,
    /// The e+e- pair-production model for muon energy losses.
    #[pyo3(get)]
    pair_production: PairProduction,
    /// The photonuclear model for muon energy losses.
    #[pyo3(get)]
    photonuclear: Photonuclear,
    #[pyo3(get)]
    /// The materials definition.
    materials: Py<Materials>,

    pub physics: *mut pumas::Physics,
    pub context: *mut pumas::Context,
    materials_indices: HashMap<String, c_int>,
}

unsafe impl Send for Physics {}
unsafe impl Sync for Physics {}

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
        let materials = Py::new(py, Materials::empty())?;
        let physics = null_mut();
        let context = null_mut();
        let materials_indices = HashMap::new();

        let physics = Self {
            bremsstrahlung, pair_production, photonuclear, materials, physics, context,
            materials_indices,
        };
        let physics = Bound::new(py, physics)?;

        let mut set_materials = true;
        if let Some(kwargs) = kwargs {
            for (key, value) in kwargs.iter() {
                let key: Bound<PyString> = key.extract()?;
                if key == "materials" {
                    set_materials = false;
                }
                physics.setattr(key, value)?
            }
        }
        if set_materials {
            let materials = Materials::new(py, None)?;
            let materials = Bound::new(py, materials)?;
            let materials = MaterialsArg::Materials(materials);
            physics
                .borrow_mut()
                .set_materials(py, materials)?;
        }

        Ok(physics.unbind())
    }

    #[setter]
    fn set_bremsstrahlung(&mut self, value: Bremsstrahlung) {
        if value != self.bremsstrahlung {
            self.bremsstrahlung = value;
            self.destroy_physics();
        }
    }

    #[setter]
    fn set_materials(&mut self, py: Python, value: MaterialsArg) -> PyResult<()> {
        let value = Materials::from_arg(py, value)?;
        if !self.materials.is(&value) {
            self.materials = value;
            self.destroy_physics();
        }
        Ok(())
    }

    #[setter]
    fn set_pair_production(&mut self, value: PairProduction) {
        if value != self.pair_production {
            self.pair_production = value;
            self.destroy_physics();
        }
    }

    #[setter]
    fn set_photonuclear(&mut self, value: Photonuclear) {
        if value != self.photonuclear {
            self.photonuclear = value;
            self.destroy_physics();
        }
    }

    #[pyo3(signature=(materials=None, /))]
    pub fn compile<'py>(&mut self, py: Python, materials: Option<MaterialsArg>) -> PyResult<()> {
        if let Some(materials) = materials {
            self.set_materials(py, materials)?;
        }
        if self.physics == null_mut() {
            // Load or create Pumas physics.
            let materials = self.materials.bind(py).borrow();
            let pumas = match self.load_pumas(materials.tag.as_str()) {
                None => self.create_pumas(py, materials.tag.as_str())?,
                Some(pumas) => pumas,
            };
            self.physics = pumas;

            // Create the simulation context.
            let mut context: *mut pumas::Context = null_mut();
            error::clear();
            let rc = unsafe { pumas::context_create(&mut context, self.physics, 0) };
            Self::check_pumas(rc)?;
            unsafe {
                (*context).mode.decay = pumas::MODE_DISABLED;
            }
            self.context = context;

            // Map materials indices.
            for material in materials.data.0.keys() {
                let mut index: c_int = 0;
                unsafe {
                    let rc = pumas::physics_material_index(
                        self.physics,
                        CString::new(material.as_str())?.as_ptr(),
                        &mut index
                    );
                    Self::check_pumas(rc)?;
                }
                self.materials_indices.insert(material.clone(), index);
            }
        }
        Ok(())
    }
}

impl Drop for Physics {
    fn drop(&mut self) {
        self.destroy_physics();
    }
}

impl Physics {
    fn check_pumas(rc: c_uint) -> PyResult<()> {
        if rc == pumas::SUCCESS {
            Ok(())
        } else {
            error::to_result(rc, Some("physics"))
        }
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
        if File::create(&dump_path).is_ok() {
            let dump_path = CString::new(dump_path.as_os_str().to_string_lossy().as_ref())?;
            unsafe {
                let stream = libc::fopen(
                    dump_path.as_c_str().as_ptr(),
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
            pumas::context_destroy(&mut self.context);
            pumas::physics_destroy(&mut self.physics);
        }
        self.materials_indices.clear();
    }

    pub fn material_index(&self, name: &str) -> PyResult<c_int> {
        self.materials_indices.get(name)
            .ok_or_else(|| {
                let why = format!("undefined material '{}'", name);
                Error::new(ValueError)
                    .what("material")
                    .why(&why)
                    .to_err()
            })
            .copied()
    }

    fn load_pumas(&self, materials: &str) -> Option<*mut pumas::Physics> {
        let tag = self.pumas_physics_tag();
        let path = cache::path().ok()?
            .join(format!("materials/{}-{}.pumas", materials, tag));
        File::open(&path).ok()?;
        let mut physics = null_mut();
        let rc = unsafe {
            let path = CString::new(path.as_os_str().to_string_lossy().as_ref()).unwrap();
            let stream = libc::fopen(
                path.as_c_str().as_ptr(),
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
    bar: Option<notify::Notifier>,
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
                let section = style(format!("[{}/{}]", self.section, Self::SECTIONS)).dim();
                let msg = format!("{} Computing {} [materials={}]", section, title, self.client);
                let bar = notify::Notifier::new(steps as usize, msg);
                Some(bar)
            },
            Some(_) => None,
        }
    }

    fn notify(&self) {
        if let Some(bar) = &self.bar {
            bar.tic()
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

/// Compile materials tables.
#[pyfunction]
#[pyo3(signature=(*args, **kwargs))]
pub fn compile(
    py: Python,
    args: &Bound<PyTuple>,
    kwargs: Option<&Bound<PyDict>>,
) -> PyResult<()> {
    let mut physics = Physics::new(py, kwargs)?
        .bind(py)
        .borrow_mut();
    if args.is_empty() {
        physics.compile(py, None)?;
    } else {
        for arg in args.iter() {
            let arg: String = arg.extract()?;
            let materials = MaterialsArg::Path(PathString(arg));
            physics.compile(py, Some(materials))?;
        }
    }
    Ok(())
}
