use console::style;
use crate::bindings::pumas;
use crate::materials::{MaterialsSet, Mdf, Material, Registry};
use crate::materials::definitions::CompositeData;
use crate::utils::convert::{Bremsstrahlung, PairProduction, Photonuclear, TransportMode};
use crate::utils::error::{self, Error};
use crate::utils::error::ErrorKind::{KeyboardInterrupt, ValueError};
use crate::utils::notify;
use crate::utils::numpy::{AnyArray, ArrayMethods, impl_dtype, NewArray};
use crate::utils::ptr::{Destroy, OwnedPtr};
use indexmap::IndexMap;
use ordered_float::OrderedFloat;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString, PyTuple};
use temp_dir::TempDir;
use ::std::borrow::Cow;
use ::std::ffi::{c_char, c_int, CStr, CString, c_uint};
use ::std::fs::File;
use ::std::hash::{DefaultHasher, Hash, Hasher};
use ::std::ptr::{NonNull, null, null_mut};
use ::std::sync::Arc;


#[pyclass(module="mulder")]
pub struct Physics {
    pub physics: Option<Arc<OwnedPtr<pumas::Physics>>>,
    pub context: Option<OwnedPtr<pumas::Context>>,
    settings: Settings,
    materials_version: Option<usize>,
    materials_indices: IndexMap<String, c_int>,
    composites_version: IndexMap<String, usize>,
}

struct Settings {
    cutoff: f64,
    elastic_ratio: f64,
    bremsstrahlung: Bremsstrahlung,
    pair_production: PairProduction,
    photonuclear: Photonuclear,
}

#[pyclass(module="mulder", frozen)]
pub struct CompiledMaterial {
    /// The material name.
    #[pyo3(get)]
    name: String,

    physics: Arc<OwnedPtr<pumas::Physics>>,
    index: c_int,
}

#[repr(C)]
#[derive(Debug)]
pub struct ElasticProperties {
    pub angle: f64,
    pub path: f64,
}

#[pymethods]
impl Physics {
    #[pyo3(signature=(**kwargs))]
    #[new]
    pub fn new<'py>(
        py: Python<'py>,
        kwargs: Option<&Bound<'py, PyDict>>,
    ) -> PyResult<Py<Self>> {
        let physics = None;
        let context = None;
        let settings = Settings::default();
        let materials_version = None;
        let materials_indices = IndexMap::new();
        let composites_version = IndexMap::new();

        let physics = Self {
            physics, context, settings, materials_version, materials_indices, composites_version,
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

    /// The Bremsstrahlung model for muon energy losses.
    #[getter]
    fn get_bremsstrahlung(&self) -> Bremsstrahlung {
        self.settings.bremsstrahlung
    }

    #[setter]
    fn set_bremsstrahlung(&mut self, value: Bremsstrahlung) {
        if value != self.settings.bremsstrahlung {
            self.settings.bremsstrahlung = value;
            self.destroy_physics();
        }
    }

    /// The cutoff between hard and soft energy losses.
    #[getter]
    fn get_cutoff(&self) -> f64 {
        self.settings.cutoff
    }

    #[setter]
    fn set_cutoff(&mut self, value: f64) {
        if value != self.settings.cutoff {
            self.settings.cutoff = value;
            self.destroy_physics();
        }
    }

    /// The hard to soft ratio for elastic collisions.
    #[getter]
    fn get_elastic_ratio(&self) -> f64 {
        self.settings.elastic_ratio
    }

    #[setter]
    fn set_elastic_ratio(&mut self, value: f64) {
        if value != self.settings.elastic_ratio {
            self.settings.elastic_ratio = value;
            self.destroy_physics();
        }
    }

    /// The e+e- pair-production model for muon energy losses.
    #[getter]
    fn get_pair_production(&self) -> PairProduction {
        self.settings.pair_production
    }

    #[setter]
    fn set_pair_production(&mut self, value: PairProduction) {
        if value != self.settings.pair_production {
            self.settings.pair_production = value;
            self.destroy_physics();
        }
    }

    /// The photonuclear model for muon energy losses.
    #[getter]
    fn get_photonuclear(&self) -> Photonuclear {
        self.settings.photonuclear
    }

    #[setter]
    fn set_photonuclear(&mut self, value: Photonuclear) {
        if value != self.settings.photonuclear {
            self.settings.photonuclear = value;
            self.destroy_physics();
        }
    }

    /// Compiles material definitions to physics tables.
    #[pyo3(signature=(*materials, notify=None))]
    fn compile(
        &mut self,
        py: Python,
        mut materials: Vec<String>,
        notify: Option<bool>,
    ) -> PyResult<PyObject> {
        if materials.is_empty() {
            let registry = &Registry::get(py)?.read().unwrap();
            for material in registry.materials().keys() {
                materials.push(material.clone());
            }
        }
        let set = MaterialsSet::from(materials.clone());
        self.update(py, &set, notify)?;

        let mut compiled_materials = Vec::new();
        for material in materials {
            let index = self.materials_indices[&material];
            let material = CompiledMaterial {
                name: material.to_string(),
                index: index,
                physics: Arc::clone(self.physics.as_ref().unwrap()),
            };
            compiled_materials.push(material);
        }
        let compiled_materials = match compiled_materials.len() {
            0 => py.None(),
            1 => Bound::new(py, compiled_materials.pop().unwrap())?.into_any().unbind(),
            _ => PyTuple::new(py, compiled_materials)?.into_any().unbind(),
        };
        Ok(compiled_materials)
    }
}

impl Drop for Physics {
    fn drop(&mut self) {
        self.destroy_physics();
    }
}

impl Physics {
    pub fn borrow_mut_context(&self) -> &mut pumas::Context {
        unsafe { &mut *self.context.as_ref().unwrap().0.as_ptr() }
    }

    #[inline]
    pub fn borrow_physics_ptr(&self) -> *const pumas::Physics {
        self.physics.as_ref().unwrap().0.as_ptr() as *const pumas::Physics
    }

    pub fn update<'py>(
        &mut self,
        py: Python,
        materials: &MaterialsSet,
        notify: Option<bool>,
    ) -> PyResult<()> {
        match self.materials_version {
            Some(version) => if version != materials.version() { self.destroy_physics() },
            None => self.destroy_physics(),
        }
        if self.physics.is_none() {
            // Load or create Pumas physics.
            let physics = if materials.is_cached(py)? {
                self.load_pumas(py, materials)
            } else {
                None
            };
            let physics = match physics {
                None => self.create_pumas(py, materials, notify)?,
                Some(physics) => physics,
            };
            self.physics = Some(Arc::new(OwnedPtr::new(physics)?));

            // Create the simulation context.
            let mut context: *mut pumas::Context = null_mut();
            error::clear();
            let rc = unsafe { pumas::context_create(&mut context, physics, 0) };
            Self::check_pumas(rc)?;
            unsafe {
                (*context).mode.decay = pumas::MODE_DISABLED;
            }
            self.context = Some(OwnedPtr::new(context)?);

            // Map materials indices and set composites.
            let registry = Registry::get(py)?.read().unwrap();
            for material in materials.borrow().iter() {
                let mut index: c_int = 0;
                unsafe {
                    let rc = pumas::physics_material_index(
                        physics,
                        CString::new(material.as_str())?.as_ptr(),
                        &mut index
                    );
                    Self::check_pumas(rc)?;
                }
                self.materials_indices.insert(material.to_owned(), index);

                if let Some(composite) = registry.materials()
                    .get(material.as_str())
                        .and_then(|m| m.as_composite()) {
                    update_composite(&composite.read(), physics, index)?;
                }
            }
        } else {
            // Update any composite.
            let registry = Registry::get(py)?.read().unwrap();
            let physics = self.physics.as_ref().unwrap().0.as_ptr();
            for material in materials.borrow().iter() {
                if let Some(composite) = registry.materials()
                    .get(material.as_str())
                        .and_then(|m| m.as_composite()) {
                    let index = self.materials_indices[material.as_str()];
                    update_composite(&composite.read(), physics, index)?;
                }
            }
        }
        Ok(())
    }

    fn check_pumas(rc: c_uint) -> PyResult<()> {
        if rc == pumas::SUCCESS {
            Ok(())
        } else {
            error::to_result(rc, Some("physics"))
        }
    }

    fn create_pumas(
        &self,
        py: Python,
        materials: &MaterialsSet,
        notify: Option<bool>,
    ) -> PyResult<*mut pumas::Physics> {
        let notify = notify.unwrap_or_else(|| notify::get());
        let tag = self.pumas_physics_tag();
        let dump_path = materials
            .cache_path(py, "pumas")?
            .with_tag(tag.clone())
            .with_makedirs()
            .into_path()?;
        let dedx_path = TempDir::new()?;
        let mdf_path = dedx_path.path().join("materials.xml");
        Mdf::new(py, &materials)?
            .dump(&mdf_path)?;

        let c_bremsstrahlung: CString = self.settings.bremsstrahlung.into();
        let c_pair_production: CString = self.settings.pair_production.into();
        let c_photonuclear: CString = self.settings.photonuclear.into();
        let mut settings = pumas::PhysicsSettings {
            cutoff: self.settings.cutoff,
            elastic_ratio: self.settings.elastic_ratio,
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
            let mut notifier = Notifier::new(materials.hash(py)?, tag.as_str());
            error::clear();
            let locale = libc::setlocale(libc::LC_NUMERIC, null());
            libc::setlocale(libc::LC_NUMERIC, CString::new("C")?.as_ptr());
            let rc = pumas::physics_create(
                &mut physics,
                pumas::MUON,
                mdf_path.as_ptr(),
                dedx_path.as_ptr(),
                &mut settings,
                if notify {
                    &mut notifier as *mut Notifier as *mut pumas::PhysicsNotifier
                } else {
                    null_mut()
                },
            );
            libc::setlocale(libc::LC_NUMERIC, locale);
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
        materials.cache_definitions(py)?;
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
        self.context = None;
        self.physics = None;
        self.materials_indices.clear();
        self.composites_version.clear();
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

    fn load_pumas(&self, py: Python, materials: &MaterialsSet) -> Option<*mut pumas::Physics> {
        let tag = self.pumas_physics_tag();
        let path = materials
            .cache_path(py, "pumas").ok()?
            .with_tag(tag)
            .into_path().ok()?;

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

        let check_cutoff = |expected: f64| -> bool {
            let cutoff = unsafe {
                pumas::physics_cutoff(physics)
            };
            cutoff == expected
        };

        let check_elastic_ratio = |expected: f64| -> bool {
            let ratio = unsafe {
                pumas::physics_elastic_ratio(physics)
            };
            ratio == expected
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
            check_cutoff(self.settings.cutoff) &&
            check_elastic_ratio(self.settings.elastic_ratio) &&
            check_process(pumas::BREMSSTRAHLUNG, self.settings.bremsstrahlung.as_pumas()) &&
            check_process(pumas::PAIR_PRODUCTION, self.settings.pair_production.as_pumas()) &&
            check_process(pumas::PHOTONUCLEAR, self.settings.photonuclear.as_pumas()) {
            Some(physics)
        } else {
            error::clear();
            unsafe { pumas::physics_destroy(&mut physics) };
            None
        }
    }

    fn pumas_physics_tag(&self)-> String {
        let mut state = DefaultHasher::new();
        self.settings.hash(&mut state);
        format!("{:016x}", state.finish())
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            cutoff: 5E-02,
            elastic_ratio: 5E-02,
            bremsstrahlung: Bremsstrahlung::default(),
            pair_production: PairProduction::default(),
            photonuclear: Photonuclear::default(),
        }
    }
}

impl Hash for Settings {
    fn hash<H>(&self, state: &mut H)
       where H: Hasher
    {
        // ensure that no attribute is ommitted.
        let Self { bremsstrahlung, cutoff, elastic_ratio, pair_production, photonuclear } = self;
        bremsstrahlung.hash(state);
        let cutoff: &OrderedFloat<f64> = unsafe { std::mem::transmute(cutoff) };
        cutoff.hash(state);
        let elastic_ratio: &OrderedFloat<f64> = unsafe { std::mem::transmute(elastic_ratio) };
        elastic_ratio.hash(state);
        pair_production.hash(state);
        photonuclear.hash(state);
    }
}

impl Destroy for NonNull<pumas::Context> {
    fn destroy(self) {
        unsafe { pumas::context_destroy(&mut self.as_ptr()) }
    }
}

impl Destroy for NonNull<pumas::Physics> {
    fn destroy(self) {
        unsafe { pumas::physics_destroy(&mut self.as_ptr()) }
    }
}

fn update_composite(
    data: &CompositeData,
    physics: *const pumas::Physics,
    index: c_int,
) -> PyResult<()> {
    let mut current_fractions = vec![0.0_f64; data.composition.len()];
    unsafe {
        let rc = pumas::physics_composite_properties(
            physics,
            index,
            null_mut(),
            null_mut(),
            current_fractions.as_mut_ptr(),
        );
        Physics::check_pumas(rc)?;
    }

    let fractions = data.composition.iter()
        .map(|c| c.weight as std::ffi::c_double)
        .collect::<Vec<_>>();

    if fractions.ne(&current_fractions) {
        unsafe {
            let rc = pumas::physics_composite_update(
                physics,
                index,
                fractions.as_ptr(),
            );
            Physics::check_pumas(rc)?;
        }
    }

    Ok(())
}

// ===============================================================================================
//
// Compiled materials interface.
//
// ===============================================================================================

enum Operation {
    Divide,
    Multiply,
}

macro_rules! compute_property {
    ($func:ident, $msg:literal, $op:expr, $slf:ident, $nrg:ident, $ntf:ident) => {
        {
            let py = $nrg.py();

            let physics = $slf.physics.as_ref().0.as_ptr();
            let registry = &Registry::get(py)?.read().unwrap();
            let density = match registry.material($slf.name.as_str()) {
                Material::Composite(composite) => {
                    update_composite(&composite.read(), physics, $slf.index)?;
                    composite.get_density(py)?
                }
                Material::Mixture(mixture) => mixture.density,
            };
            let factor = match $op {
                Operation::Divide => 1.0 / density,
                Operation::Multiply => density,
            };

            let mut array = NewArray::empty(py, $nrg.shape())?;
            let n = array.size();
            let values = array.as_slice_mut();
            let notifier = notify::Notifier::from_arg($ntf, n, $msg);
            for i in 0..n {
                let ei = $nrg.get_item(i)?;
                let mut vi = 0.0;
                unsafe {
                    pumas::$func(
                        physics, $slf.index, ei, &mut vi,
                    );
                }
                values[i] = vi * factor;
                notifier.tic();
            }

            Ok(array)
        }
    };

    ($func:ident, $msg:literal, $op:expr, $slf:ident, $mode:ident, $nrg:ident, $ntf:ident) => {
        {
            let py = $nrg.py();

            let mode = match $mode {
                Some(mode) => mode.to_pumas_mode(),
                None => pumas::MODE_CSDA,
            };
            let physics = $slf.physics.as_ref().0.as_ptr();
            let registry = &Registry::get(py)?.read().unwrap();
            let density = match registry.material($slf.name.as_str()) {
                Material::Composite(composite) => {
                    update_composite(&composite.read(), physics, $slf.index)?;
                    composite.get_density(py)?
                }
                Material::Mixture(mixture) => mixture.density,
            };
            let factor = match $op {
                Operation::Divide => 1.0 / density,
                Operation::Multiply => density,
            };

            let mut array = NewArray::empty(py, $nrg.shape())?;
            let n = array.size();
            let values = array.as_slice_mut();
            let notifier = notify::Notifier::from_arg($ntf, n, $msg);
            for i in 0..n {
                let ei = $nrg.get_item(i)?;
                let mut vi = 0.0;
                unsafe {
                    pumas::$func(
                        physics, mode, $slf.index, ei, &mut vi,
                    );
                }
                values[i] = vi * factor;
                notifier.tic();
            }

            Ok(array)
        }
    }
}

#[pymethods]
impl CompiledMaterial {
    fn __repr__(&self) -> String {
        format!(
            "CompiledMaterial('{:}', {})",
            self.name,
            self.index,
        )
    }

    /// The material definition.
    #[getter]
    fn get_definition(&self, py: Python) -> PyResult<Material> {
        let registry = &Registry::get(py)?.read().unwrap();
        let definition = registry.material(self.name.as_str());
        Ok(definition.clone())
    }

    /// Returns the macroscopic cross-section.
    #[pyo3(signature=(energy, /, *, notify=None))]
    fn cross_section<'py>(
        &self,
        energy: AnyArray<'py, f64>,
        notify: Option<notify::NotifyArg>,
    ) -> PyResult<NewArray<'py, f64>> {
        compute_property!(
            physics_property_cross_section,
            "computing cross-section(s)",
            Operation::Multiply,
            self,
            energy,
            notify
        )
    }

    /// Returns the muon elastic scattering properties.
    #[pyo3(signature=(energy, /, *, notify=None))]
    fn elastic_scattering<'py>(
        &self,
        energy: AnyArray<'py, f64>,
        notify: Option<notify::NotifyArg>,
    ) -> PyResult<NewArray<'py, ElasticProperties>> {
        let py = energy.py();

        let physics = self.physics.as_ref().0.as_ptr();
        let registry = &Registry::get(py)?.read().unwrap();
        let density = match registry.material(self.name.as_str()) {
            Material::Composite(composite) => {
                update_composite(&composite.read(), physics, self.index)?;
                composite.get_density(py)?
            }
            Material::Mixture(mixture) => mixture.density,
        };

        let mut array = NewArray::empty(py, energy.shape())?;
        let n = array.size();
        let values = array.as_slice_mut();
        let notifier = notify::Notifier::from_arg(notify, n, "computing elastic scattering(s)");
        for i in 0..n {
            let ei = energy.get_item(i)?;
            let mut angle = 0.0;
            unsafe {
                pumas::physics_property_elastic_cutoff_angle(
                    physics, self.index, ei, &mut angle,
                );
            }
            angle *= 180.0 / std::f64::consts::PI;
            let mut path = 0.0;
            unsafe {
                pumas::physics_property_elastic_path(
                    physics, self.index, ei, &mut path,
                );
            }
            path /= density;
            values[i] = ElasticProperties { angle, path };
            notifier.tic();
        }

        Ok(array)
    }

    /// Returns the muon CSDA range.
    #[pyo3(signature=(energy, /, *, mode=None, notify=None))]
    fn range<'py>(
        &self,
        energy: AnyArray<'py, f64>,
        mode: Option<TransportMode>,
        notify: Option<notify::NotifyArg>,
    ) -> PyResult<NewArray<'py, f64>> {
        compute_property!(
            physics_property_range,
            "computing range(s)",
            Operation::Divide,
            self,
            mode,
            energy,
            notify
        )
    }

    /// Returns the material stopping-power.
    #[pyo3(signature=(energy, /, *, mode=None, notify=None))]
    fn stopping_power<'py>(
        &self,
        energy: AnyArray<'py, f64>,
        mode: Option<TransportMode>,
        notify: Option<notify::NotifyArg>,
    ) -> PyResult<NewArray<'py, f64>> {
        compute_property!(
            physics_property_stopping_power,
            "computing stopping-power(s)",
            Operation::Multiply,
            self,
            mode,
            energy,
            notify
        )
    }

    /// Returns the transport mean free path.
    #[pyo3(signature=(energy, /, *, mode=None, notify=None))]
    fn transport_path<'py>(
        &self,
        energy: AnyArray<'py, f64>,
        mode: Option<TransportMode>,
        notify: Option<notify::NotifyArg>,
    ) -> PyResult<NewArray<'py, f64>> {
        compute_property!(
            physics_property_transport_path,
            "computing transport path(s)",
            Operation::Divide,
            self,
            mode,
            energy,
            notify
        )
    }
}

impl_dtype!(
    ElasticProperties,
    [
        ("angle", "f8"),
        ("path",  "f8"),
    ]
);

// ===============================================================================================
//
// Notifier for physics computations.
//
// ===============================================================================================

#[repr(C)]
struct Notifier {
    interface: pumas::PhysicsNotifier,
    bar: Option<notify::Notifier>,
    hash: String,
    section: usize,
}

impl Notifier {
    const SECTIONS: usize = 4;

    fn new(hash: u64, tag: &str) -> Self {
        let interface = pumas::PhysicsNotifier {
            configure: Some(pumas_physics_notifier_configure),
            notify: Some(pumas_physics_notifier_notify),
        };
        let hash = format!("{:016x}", hash);
        let hash: String = hash.chars().into_iter().take(7).collect();
        let tag: String = tag.chars().take(7).collect();
        let hash = format!("{}-{}", hash, tag);
        Self { interface, bar: None, hash, section: 0 }
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
                let msg = format!("{} Computing {} ({})", section, title, self.hash);
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
