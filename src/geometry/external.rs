#![allow(non_snake_case)]

use crate::simulation::materials::{
    AtomicElement, Component, ElementsTable, Material, Materials, MaterialsData
};
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::{TypeError, ValueError};
use crate::utils::io::PathString;
use libloading::Library;
use paste::paste;
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use std::collections::HashMap;
use std::ffi::{c_char, c_double, c_int, CStr};
use std::ptr::NonNull;


// ===============================================================================================
// External geometry, dynamicaly loaded.
// ===============================================================================================

#[pyclass(module="mulder")]
pub struct ExternalGeometry {
    #[allow(dead_code)]
    lib: Library, // for keeping the library alive.
    interface: CInterface,
    geometry: OwnedPtr<CGeometry>,

    #[pyo3(get)]
    materials: Option<Py<Materials>>,

    #[pyo3(get)]
    media: Py<PyTuple>, // sequence of Py<Medium>.
}

#[pyclass(module="mulder")]
pub struct Medium {
    #[pyo3(get, set)]
    material: String,

    #[pyo3(get, set)]
    density: Option<f64>,

    #[pyo3(get, set)]
    description: Option<String>,
}

pub struct ExternalTracer<'a> {
    definition: &'a ExternalGeometry,
    ptr: OwnedPtr<CTracer>,
}

#[repr(C)]
struct CInterface {
    definition: Option<
        extern "C" fn() -> *mut CGeometry
    >,
    tracer: Option<
        extern "C" fn(*const CGeometry) -> *mut CTracer
    >,
}

#[repr(C)]
struct CGeometry {
    destroy: Option<
        extern "C" fn(*mut CGeometry)
    >,
    material: Option<
        extern "C" fn(*const CGeometry, index: usize) -> *mut CMaterial
    >,
    materials_len: Option<
        extern "C" fn(*const CGeometry) -> usize
    >,
    medium: Option<
        extern "C" fn(*const CGeometry, index: usize) -> *mut CMedium
    >,
    media_len: Option<
        extern "C" fn(*const CGeometry) -> usize
    >,
}

#[repr(C)]
struct CMedium {
    destroy: Option<
        extern "C" fn(*mut CMedium)
    >,
    material: Option<extern "C" fn(
        *const CMedium) -> *const c_char
    >,
    density: Option<extern "C" fn(
        *const CMedium) -> c_double
    >,
    description: Option<extern "C" fn(
        *const CMedium) -> *const c_char
    >,
}

#[repr(C)]
struct CMaterial {
    destroy: Option<
        extern "C" fn(*mut CMaterial)
    >,
    name: Option<extern "C" fn(
        *const CMaterial) -> *const c_char
    >,
    density: Option<extern "C" fn(
        *const CMaterial) -> c_double
    >,
    element: Option<
        extern "C" fn(*const CMaterial, usize) -> *mut CElement
    >,
    elements_len: Option<
        extern "C" fn(*const CMaterial) -> usize
    >,
    I: Option<extern "C" fn(
        *const CMaterial) -> c_double
    >,
}

#[repr(C)]
struct CElement {
    destroy: Option<
        extern "C" fn(*mut CElement)
    >,
    symbol: Option<extern "C" fn(
        *const CElement) -> *const c_char
    >,
    weight: Option<extern "C" fn(
        *const CElement) -> c_double
    >,
    A: Option<extern "C" fn(
        *const CElement) -> c_double
    >,
    I: Option<extern "C" fn(
        *const CElement) -> c_double
    >,
    Z: Option<extern "C" fn(
        *const CElement) -> c_int
    >,
}

#[repr(C)]
struct CTracer {
    geometry: *const CGeometry,

    destroy: Option<
        extern "C" fn(*mut CTracer)
    >,
    trace: Option<
        extern "C" fn(
            *mut CTracer,
            position: *const c_double,
            direction: *const c_double,
            medium: *mut usize,
            step: *mut c_double,
        )
    >,
}

fn null_pointer_err() -> PyErr {
    Error::new(ValueError).what("pointer").why("null").to_err()
}

macro_rules! null_pointer_fmt {
    ($($arg:tt)*) => {
        {
            let what = format!($($arg)*);
            Error::new(ValueError).what(&what).why("null pointer").to_err()
        }
    }
}

#[pymethods]
impl ExternalGeometry {
    #[new]
    #[pyo3(signature=(path, /))]
    pub unsafe fn new(py: Python, path: PathString) -> PyResult<Self> {
        // Fetch geometry description from entry point.
        type Initialise = unsafe fn() -> CInterface;
        const INITIALISE: &[u8] = b"mulder_initialise\0";

        let library = Library::new(path.as_str())
            .map_err(|err| {
                let why = format!("{}: {}", path.as_str(), err);
                Error::new(TypeError).what("geometry").why(&why).to_err()
            })?;
        let initialise = library.get::<Initialise>(INITIALISE)
            .map_err(|err| {
                let why = format!("{}: {}", path.as_str(), err);
                Error::new(TypeError).what("geometry").why(&why).to_err()
            })?;
        let interface = unsafe { initialise() };
        let geometry = interface.definition()?;

        // Build material definitions.
        let size = geometry.materials_len()?;
        let mut table = ElementsTable::empty();
        let mut materials = HashMap::<String, Material>::new();
        for i in 0..size {
            if let Some((name, material)) = geometry.material(i)?.to_material(&mut table)? {
                materials.insert(name, material);
            }
        }
        let materials = if materials.len() > 0 {
            let materials = MaterialsData::new(materials).with_table(table);
            let tag = "external".to_owned();
            Some(Py::new(py, Materials::new(py, tag, materials)?)?)
        } else {
            None
        };

        // Build geometry media.
        let size = geometry.media_len()?;
        let mut media = Vec::with_capacity(size);
        for i in 0..size {
            let medium = Medium::try_from(geometry.medium(i)?)?;
            media.push(medium);
        }
        let media = PyTuple::new(py, media)?.unbind();

        // Bundle the geometry definition.
        let external_geometry = Self {
            lib: library,
            interface,
            geometry,
            media,
            materials,
        };
        Ok(external_geometry)
    }
}

unsafe impl Send for ExternalGeometry {}
unsafe impl Sync for ExternalGeometry {}

#[pymethods]
impl Medium {
    fn __repr__(&self) -> String {
        let mut tokens = vec![format!("material='{}'", self.material)];
        if let Some(density) = self.density {
            tokens.push(format!("density={}", density));
        }
        if let Some(description) = &self.description {
            tokens.push(format!("description='{}'", description));
        }
        let tokens = tokens.join(", ");
        format!("Medium({})", tokens)
    }
}

impl OwnedPtr<CMaterial> {
    fn to_material(
        self,
        table: &mut ElementsTable,
    ) -> PyResult<Option<(String, Material)>> {
        let name = self.name()?;
        if self.is_none_density() &&
            self.is_none_element() &&
            self.is_none_elements_len() &&
            self.is_none_I()
        {
            return Ok(None)
        }

        let n = self.elements_len()
            .ok_or_else(|| null_pointer_fmt!("elements_len for {} material", name))?;

        let mut composition = Vec::with_capacity(n);
        for i in 0..n {
            let element = self.element(i)?
                .ok_or_else(|| null_pointer_fmt!("element#{} for {} material", i, name))?;
            let mut symbol = element.symbol()?;
            if element.is_some_Z() || element.is_some_A() || element.is_some_I() {
                let Z = element.Z()
                    .ok_or_else(|| null_pointer_fmt!("Z for {} element", symbol))? as u32;
                let A = element.A()
                    .ok_or_else(|| null_pointer_fmt!("A for {} element", symbol))?;
                let I = element.I()
                    .ok_or_else(|| null_pointer_fmt!("I for {} element", symbol))?;
                let element = AtomicElement { Z, A, I };
                if let Some(other) = table.get(&name) {
                    if other != &element {
                        symbol = format!("{name}:{symbol}");
                    }
                }
                table.insert(symbol.clone(), element);
            }
            let weight = element.weight()?;
            composition.push(Component { name: symbol, weight });

        }
        let density = self.density()
            .ok_or_else(|| null_pointer_fmt!("density for {} material", name))?;
        let I = self.I();
        let material = Material::from_elements(density, &composition, I, table)
            .map_err(|(kind, why)| {
                let what = format!("{} material", name);
                Error::new(kind).what(&what).why(&why).to_err()
            })?;
        Ok(Some((name, material)))
    }
}

impl TryFrom<OwnedPtr<CMedium>> for Medium {
    type Error = PyErr;

    fn try_from(value: OwnedPtr<CMedium>) -> PyResult<Self> {
        let material = value.material()?;
        let density = value.density();
        let description = value.description()?;
        Ok(Self { material, density, description })
    }
}


// ===============================================================================================
// C structs wrappers.
// ===============================================================================================

struct OwnedPtr<T> (NonNull<T>) where NonNull<T>: Destroy;

impl<T> OwnedPtr<T>
where
    NonNull<T>: Destroy
{
    fn new(ptr: *mut T) -> PyResult<Self> {
        NonNull::new(ptr)
            .map(|ptr| Self(ptr))
            .ok_or_else(|| null_pointer_err())
    }
}

impl<T> Drop for OwnedPtr<T>
where
    NonNull<T>: Destroy
{
    fn drop(&mut self) {
        self.0.destroy();
    }
}

trait Destroy {
    fn destroy(self);
}

macro_rules! impl_destroy {
    ($($type:ty),+) => {
        $(
            impl Destroy for NonNull<$type> {
                fn destroy(mut self) {
                    if let Some(destroy) = unsafe { self.as_mut().destroy } {
                        destroy(self.as_ptr());
                    }
                }
            }
        )+
    }
}

impl_destroy! { CGeometry, CElement, CMaterial, CMedium, CTracer }

impl CInterface {
    fn definition(&self) -> PyResult<OwnedPtr<CGeometry>> {
        match self.definition {
            Some(func) => OwnedPtr::new(func()),
            None => Err(null_pointer_fmt!("CInterface::definition")),
        }
    }

    fn tracer(&self, definition: &OwnedPtr::<CGeometry>) -> PyResult<OwnedPtr<CTracer>> {
        match self.tracer {
            Some(func) => OwnedPtr::new(func(definition.0.as_ptr())),
            None => Err(null_pointer_fmt!("CInterface::tracer")),
        }
    }
}

macro_rules! impl_get_attr {
    ($name:tt, $output:ty) => {
        fn $name(&self) -> PyResult<$output> {
            match unsafe { self.0.as_ref().$name } {
                Some(func) => Ok(func(self.0.as_ptr())),
                None => Err(null_pointer_err()),
            }
        }
    }
}

macro_rules! impl_get_str_attr {
    ($name:tt) => {
        fn $name(&self) -> PyResult<String> {
            match unsafe { self.0.as_ref().$name } {
                Some(func) => {
                    let cstr = unsafe { CStr::from_ptr(func(self.0.as_ptr())) };
                    Ok(cstr.to_str()?.to_owned())
                },
                None => Err(null_pointer_err()),
            }
        }
    }
}

macro_rules! impl_get_item {
    ($name:tt, $output:ty) => {
        fn $name(&self, index: usize) -> PyResult<OwnedPtr<$output>> {
            match unsafe { self.0.as_ref().$name } {
                Some(func) => OwnedPtr::new(func(self.0.as_ptr(), index)),
                None => Err(null_pointer_err()),
            }
        }
    }
}

macro_rules! impl_is_some {
    ($name:tt) => {
        paste! {
            #[allow(unused)]
            fn [< is_none_ $name >](&self) -> bool {
                unsafe { self.0.as_ref().$name }
                    .is_none()
            }

            #[allow(unused)]
            fn [< is_some_ $name >] (&self) -> bool {
                unsafe { self.0.as_ref().$name }
                    .is_some()
            }
        }
    }
}

macro_rules! impl_get_opt_attr {
    ($name:tt, $output:ty) => {
        fn $name(&self) -> Option<$output> {
            unsafe { self.0.as_ref().$name }
                .map(|func| func(self.0.as_ptr()))
        }

        impl_is_some!($name);
    }
}

macro_rules! impl_get_opt_str {
    ($name:tt) => {
        fn $name(&self) -> PyResult<Option<String>> {
            unsafe { self.0.as_ref().$name }
                .map(|func| {
                    let cstr = unsafe { CStr::from_ptr(func(self.0.as_ptr())) };
                    Ok(cstr.to_str()?.to_owned())
                })
                .transpose()
        }

        impl_is_some!($name);
    }
}

macro_rules! impl_get_opt_item {
    ($name:tt, $output:ty) => {
        fn $name(&self, index: usize) -> PyResult<Option<OwnedPtr<$output>>> {
            unsafe { self.0.as_ref().$name }
                .map(|func| OwnedPtr::new(func(self.0.as_ptr(), index)))
                .transpose()
        }

        impl_is_some!($name);
    }
}

impl OwnedPtr<CGeometry> {
    impl_get_attr!(materials_len, usize);
    impl_get_attr!(media_len, usize);
    impl_get_item!(material, CMaterial);
    impl_get_item!(medium, CMedium);
}

impl OwnedPtr<CMaterial> {
    impl_get_str_attr!(name);
    impl_get_opt_attr!(density, c_double);
    impl_get_opt_attr!(elements_len, usize);
    impl_get_opt_item!(element, CElement);
    impl_get_opt_attr!(I, c_double);
}

impl OwnedPtr<CElement> {
    impl_get_str_attr!(symbol);
    impl_get_attr!(weight, c_double);
    impl_get_opt_attr!(Z, c_int);
    impl_get_opt_attr!(A, c_double);
    impl_get_opt_attr!(I, c_double);
}

impl OwnedPtr<CMedium> {
    impl_get_str_attr!(material);
    impl_get_opt_attr!(density, c_double);
    impl_get_opt_str!(description);
}
