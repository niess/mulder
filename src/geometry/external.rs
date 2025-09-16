#![allow(non_snake_case)]

use crate::materials::{Component, Element, Material, MaterialsSet, MaterialsSubscriber, Registry};
use crate::utils::coordinates::LocalFrame;
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::{TypeError, ValueError};
use crate::utils::extract::Size;
use crate::utils::io::PathString;
use crate::utils::numpy::{AnyArray, ArrayMethods, Dtype, impl_dtype, NewArray};
use crate::utils::ptr::{Destroy, null_pointer_err, OwnedPtr};
use libloading::Library;
use paste::paste;
use pyo3::prelude::*;
use pyo3::types::PyTuple;
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
    subscribers: Vec<MaterialsSubscriber>,

    /// The geometry media.
    #[pyo3(get)]
    pub media: Py<PyTuple>, // sequence of Py<Medium>.

    /// The geometry reference frame.
    #[pyo3(get, set)]
    pub frame: LocalFrame,
}

#[derive(Debug)]
#[pyclass(module="mulder")]
pub struct Medium {
    /// The medium constitutive material.
    #[pyo3(get)]
    pub material: String,

    /// The medium bulk density.
    #[pyo3(get, set)]
    pub density: Option<f64>,

    /// A brief description of this medium.
    #[pyo3(get, set)]
    pub description: Option<String>,

    pub geometry: Option<Py<ExternalGeometry>>,
}

pub struct ExternalTracer<'a> {
    #[allow(dead_code)]
    definition: &'a ExternalGeometry, // for lifetime guarantee.
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
    destroy: Option<
        extern "C" fn(*mut CTracer)
    >,
    locate: Option<
        extern "C" fn(*mut CTracer, position: CVec3) -> usize
    >,
    reset: Option<
        extern "C" fn(*mut CTracer, position: CVec3, direction: CVec3)
    >,
    trace: Option<
        extern "C" fn(*mut CTracer, max_length: c_double) -> c_double
    >,
    move_: Option<
        extern "C" fn(*mut CTracer, length: c_double)
    >,
    turn: Option<
        extern "C" fn(*mut CTracer, direction: CVec3)
    >,
    medium: Option<
        extern "C" fn(*mut CTracer) -> usize
    >,
    position: Option<
        extern "C" fn(*mut CTracer) -> CVec3
    >,
}

#[repr(C)]
struct CVec3 {
    x: c_double,
    y: c_double,
    z: c_double,
}

#[repr(C)]
#[derive(Debug)]
struct Intersection {
    pub before: i32,
    pub after: i32,
    pub position: [f64; 3],
    pub distance: f64,
}

macro_rules! null_pointer_fmt {
    ($($arg:tt)*) => {
        {
            let what = format!($($arg)*);
            Error::new(ValueError).what(&what).why("null pointer").to_err()
        }
    }
}

unsafe impl Send for ExternalGeometry {}
unsafe impl Sync for ExternalGeometry {}

#[inline]
fn type_error(what: &str, why: &str) -> PyErr {
    Error::new(TypeError).what(what).why(&why).to_err()
}

#[pymethods]
impl ExternalGeometry {
    #[new]
    #[pyo3(signature=(path, /, *, frame=None))]
    pub unsafe fn new(
        py: Python,
        path: PathString,
        frame: Option<LocalFrame>,
    ) -> PyResult<Py<Self>> {
        // Fetch geometry description from entry point.
        type Initialise = unsafe fn() -> CInterface;
        const INITIALISE: &[u8] = b"mulder_initialise\0";

        let library = Library::new(path.as_str())
            .map_err(|err| type_error(
                "geometry",
                &format!("{}: {}", path.as_str(), err)
            ))?;
        let initialise = library.get::<Initialise>(INITIALISE)
            .map_err(|err| type_error(
                "geometry",
                &format!("{}: {}", path.as_str(), err)
            ))?;
        let interface = unsafe { initialise() };
        let geometry = interface.definition()?;

        // Register any materials.
        let size = geometry.materials_len()?;
        let mut registry = Registry::get(py)?.write().unwrap();
        for i in 0..size {
            geometry.material(i)?.register(&mut registry)?;
        }
        drop(registry);

        // Build geometry media.
        let size = geometry.media_len()?;
        let mut media = Vec::with_capacity(size);
        for i in 0..size {
            let medium = Medium::try_from(geometry.medium(i)?)?;
            media.push(medium);
        }
        let media = PyTuple::new(py, media)?.unbind();

        // Bundle the geometry definition.
        let subscribers = Vec::new();
        let frame = frame.unwrap_or_else(|| LocalFrame::default());
        let external_geometry = Self {
            lib: library,
            interface,
            subscribers,
            geometry,
            media,
            frame,
        };
        let external_geometry = Py::new(py, external_geometry)?;
        for medium in external_geometry.bind(py).borrow().media.bind(py).iter() {
            let medium: &Bound<Medium> = medium.downcast().unwrap();
            medium.borrow_mut().geometry = Some(external_geometry.clone_ref(py));
        }

        Ok(external_geometry)
    }

    #[pyo3(name="locate", signature=(*, position))]
    fn py_locate<'py>(&self, position: AnyArray<'py, f64>) -> PyResult<NewArray<'py, i32>> {
        let py = position.py();

        let size = Size::try_from_vec3(&position)
            .map_err(|why| type_error("position", &why))?;

        let tracer = self.tracer()?;
        let mut array = NewArray::empty(py, size.shape())?;
        let n = array.size();
        let media = array.as_slice_mut();
        for i in 0..n {
            let ri = [
                position.get_item(3 * i + 0)?,
                position.get_item(3 * i + 1)?,
                position.get_item(3 * i + 2)?,
            ];
            media[i] = tracer.locate(ri) as i32;
        }

        Ok(array)
    }

    #[pyo3(name="scan", signature=(*, position, direction))]
    #[allow(unused)] // XXX remove.
    fn py_scan<'py>(
        &self,
        position: AnyArray<'py, f64>,
        direction: AnyArray<'py, f64>,
    ) -> PyResult<NewArray<'py, f64>> {
        // XXX implement this.
        // XXX Use dict as return type?
        unimplemented!()
    }

    #[pyo3(name="trace", signature=(*, position, direction))]
    fn py_trace<'py>(
        &self,
        position: AnyArray<'py, f64>,
        direction: AnyArray<'py, f64>,
    ) -> PyResult<NewArray<'py, Intersection>> {
        let py = position.py();

        let sizes = [
            Size::try_from_vec3(&position)
                .map_err(|why| type_error("position", &why))?,
            Size::try_from_vec3(&direction)
                .map_err(|why| type_error("direction", &why))?,
        ];
        let common_size = Size::join(&sizes)
            .map_err(|why| type_error("coordinates", why))?;

        let tracer = self.tracer()?;
        let mut array = NewArray::empty(py, common_size.shape())?;
        let n = array.size();
        let intersections = array.as_slice_mut();
        for i in 0..n {
            let ri = [
                position.get_item(3 * i + 0)?,
                position.get_item(3 * i + 1)?,
                position.get_item(3 * i + 2)?,
            ];
            let ui = [
                direction.get_item(3 * i + 0)?,
                direction.get_item(3 * i + 1)?,
                direction.get_item(3 * i + 2)?,
            ];
            tracer.reset(ri, ui);
            let before = tracer.medium() as i32;
            let distance = tracer.trace(f64::INFINITY);
            tracer.move_(distance);
            let after = tracer.medium() as i32;
            let position = tracer.position();
            intersections[i] = Intersection {
                before, after, position, distance
            }
        }

        Ok(array)
    }
}

impl ExternalGeometry {
    pub fn subscribe(&mut self, py: Python, set: &MaterialsSet) {
        for medium in self.media.bind(py).iter() {
            set.add(medium.downcast::<Medium>().unwrap().borrow().material.as_str());
        }
        self.subscribers.push(set.subscribe());
        self.subscribers.retain(|s| s.is_alive());
    }

    pub fn unsubscribe(&mut self, py: Python, set: &MaterialsSet) {
        for medium in self.media.bind(py).iter() {
            set.remove(medium.downcast::<Medium>().unwrap().borrow().material.as_str());
        }
        self.subscribers.retain(|s| s.is_alive() && !s.is_subscribed(set));
    }

    pub fn tracer<'a>(&'a self) -> PyResult<ExternalTracer<'a>> {
        let tracer = self.interface.tracer(&self.geometry)?;
        if tracer.is_none_locate() {
            return Err(null_pointer_fmt!("ExternalTracer::locate"))
        }
        if tracer.is_none_reset() {
            return Err(null_pointer_fmt!("ExternalTracer::reset"))
        }
        if tracer.is_none_trace() {
            return Err(null_pointer_fmt!("ExternalTracer::trace"))
        }
        if tracer.is_none_move_() {
            return Err(null_pointer_fmt!("ExternalTracer::move"))
        }
        if tracer.is_none_turn() {
            return Err(null_pointer_fmt!("ExternalTracer::turn"))
        }
        if tracer.is_none_medium() {
            return Err(null_pointer_fmt!("ExternalTracer::medium"))
        }
        if tracer.is_none_position() {
            return Err(null_pointer_fmt!("ExternalTracer::position"))
        }
        Ok(ExternalTracer { definition: self, ptr: tracer })
    }
}

impl<'a> ExternalTracer<'a> {
    #[inline]
    pub fn locate(&self, position: [f64; 3]) -> usize {
        let func = unsafe { self.ptr.0.as_ref() }.locate.unwrap();
        func(self.ptr.0.as_ptr(), position.into())
    }

    #[inline]
    pub fn reset(&self, position: [f64; 3], direction: [f64; 3]) {
        let func = unsafe { self.ptr.0.as_ref() }.reset.unwrap();
        func(self.ptr.0.as_ptr(), position.into(), direction.into())
    }

    #[inline]
    pub fn trace(&self, max_length: f64) -> f64 {
        let func = unsafe { self.ptr.0.as_ref() }.trace.unwrap();
        func(self.ptr.0.as_ptr(), max_length)
    }

    #[inline]
    pub fn move_(&self, length: f64) {
        let func = unsafe { self.ptr.0.as_ref() }.move_.unwrap();
        func(self.ptr.0.as_ptr(), length)
    }

    #[inline]
    pub fn turn(&self, direction: [f64; 3]) {
        let func = unsafe { self.ptr.0.as_ref() }.turn.unwrap();
        func(self.ptr.0.as_ptr(), direction.into())
    }

    #[inline]
    pub fn medium(&self) -> usize {
        let func = unsafe { self.ptr.0.as_ref() }.medium.unwrap();
        func(self.ptr.0.as_ptr())
    }

    #[inline]
    pub fn position(&self) -> [f64; 3] {
        let func = unsafe { self.ptr.0.as_ref() }.position.unwrap();
        func(self.ptr.0.as_ptr()).into()
    }
}

impl From<[f64; 3]> for CVec3 {
    fn from(value: [f64; 3]) -> Self {
        Self { x: value[0], y: value[1], z: value[2] }
    }
}

impl From<CVec3> for [f64; 3] {
    fn from(value: CVec3) -> Self {
        [ value.x, value.y, value.z ]
    }
}

impl_dtype!(
    Intersection,
    [
        ("before",    "i4"),
        ("after",     "i4"),
        ("position",  "3f8"),
        ("distance",  "f8")
    ]
);

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

    #[setter]
    fn set_material(&mut self, py: Python, value: &str) {
        if value != self.material {
            if let Some(geometry) = self.geometry.as_ref() {
                let mut geometry = geometry.bind(py).borrow_mut();
                geometry.subscribers.retain(|subscriber|
                    subscriber.replace(self.material.as_str(), value)
                )
            }
            self.material = value.to_owned();
        }
    }
}

impl OwnedPtr<CMaterial> {
    fn register(
        self,
        registry: &mut Registry,
    ) -> PyResult<()> {
        let name = self.name()?;
        if self.is_none_density() &&
            self.is_none_element() &&
            self.is_none_elements_len() &&
            self.is_none_I()
        {
            return Ok(())
        }

        let n = self.elements_len()
            .ok_or_else(|| null_pointer_fmt!("elements_len for {} material", name))?;

        let mut composition = Vec::with_capacity(n);
        for i in 0..n {
            let element = self.element(i)?
                .ok_or_else(|| null_pointer_fmt!("element#{} for {} material", i, name))?;
            let symbol = element.symbol()?;
            if element.is_some_Z() || element.is_some_A() || element.is_some_I() {
                let Z = element.Z()
                    .ok_or_else(|| null_pointer_fmt!("Z for {} element", symbol))? as u32;
                let A = element.A()
                    .ok_or_else(|| null_pointer_fmt!("A for {} element", symbol))?;
                let mut I = element.I()
                    .ok_or_else(|| null_pointer_fmt!("I for {} element", symbol))?;
                I *= 1E+09; // to eV.
                let element = Element { Z, A, I };
                registry.add_element(symbol.clone(), element)?;
            }
            let weight = element.weight()?;
            composition.push(Component { name: symbol, weight });

        }
        let density = self.density()
            .ok_or_else(|| null_pointer_fmt!("density for {} material", name))?;
        let I = self.I();
        let material = Material::from_elements(density, &composition, I, registry)
            .map_err(|(kind, why)| {
                let what = format!("{} material", name);
                Error::new(kind).what(&what).why(&why).to_err()
            })?;
        registry.add_material(name, material)?;
        Ok(())
    }
}

impl TryFrom<OwnedPtr<CMedium>> for Medium {
    type Error = PyErr;

    fn try_from(value: OwnedPtr<CMedium>) -> PyResult<Self> {
        let material = value.material()?;
        let density = value.density();
        let description = value.description()?;
        let geometry = None;
        Ok(Self { material, density, description, geometry })
    }
}

// ===============================================================================================
// C structs wrappers.
// ===============================================================================================

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
            fn [< is_some_ $name >] (&self) -> bool {
                unsafe { self.0.as_ref().$name }
                    .is_some()
            }
        }
    }
}

macro_rules! impl_is_none {
    ($name:tt) => {
        paste! {
            fn [< is_none_ $name >](&self) -> bool {
                unsafe { self.0.as_ref().$name }
                    .is_none()
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
    }
}

macro_rules! impl_get_opt_item {
    ($name:tt, $output:ty) => {
        fn $name(&self, index: usize) -> PyResult<Option<OwnedPtr<$output>>> {
            unsafe { self.0.as_ref().$name }
                .map(|func| OwnedPtr::new(func(self.0.as_ptr(), index)))
                .transpose()
        }
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
    impl_get_opt_item!(element, CElement);
    impl_get_opt_attr!(elements_len, usize);
    impl_get_opt_attr!(I, c_double);

    impl_is_none!(density);
    impl_is_none!(element);
    impl_is_none!(elements_len);
    impl_is_none!(I);
}

impl OwnedPtr<CElement> {
    impl_get_str_attr!(symbol);
    impl_get_attr!(weight, c_double);
    impl_get_opt_attr!(Z, c_int);
    impl_get_opt_attr!(A, c_double);
    impl_get_opt_attr!(I, c_double);

    impl_is_some!(Z);
    impl_is_some!(A);
    impl_is_some!(I);
}

impl OwnedPtr<CMedium> {
    impl_get_str_attr!(material);
    impl_get_opt_attr!(density, c_double);
    impl_get_opt_str!(description);
}

impl OwnedPtr<CTracer> {
    impl_is_none!(locate);
    impl_is_none!(reset);
    impl_is_none!(trace);
    impl_is_none!(move_);
    impl_is_none!(turn);
    impl_is_none!(medium);
    impl_is_none!(position);
}
