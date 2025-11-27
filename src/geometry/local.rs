#![allow(non_snake_case)]

use crate::materials::{MaterialsSet, MaterialsSubscriber, Registry};
use crate::module::{CGeometry, CMedium, CModule, CTracer, CVec3, Module};
use crate::simulation::coordinates::LocalFrame;
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::{TypeError, ValueError};
use crate::utils::extract::Size;
use crate::utils::io::PathString;
use crate::utils::numpy::{AnyArray, ArrayMethods, Dtype, impl_dtype, NewArray};
use crate::utils::ptr::OwnedPtr;
use pyo3::prelude::*;
use pyo3::exceptions::PyNotImplementedError;
use pyo3::types::PyTuple;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::Path;


// ===============================================================================================
// External geometry, dynamicaly loaded.
// ===============================================================================================

#[pyclass(module="mulder")]
pub struct LocalGeometry {
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

    /// An optional description.
    #[pyo3(get, set)]
    pub description: Option<String>,

    pub geometry: Option<Py<LocalGeometry>>,
}

pub struct LocalTracer<'a> {
    #[allow(dead_code)]
    definition: &'a LocalGeometry, // for lifetime guarantee.
    ptr: OwnedPtr<CTracer>,
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

unsafe impl Send for LocalGeometry {}
unsafe impl Sync for LocalGeometry {}

#[inline]
fn type_error(what: &str, why: &str) -> PyErr {
    Error::new(TypeError).what(what).why(&why).to_err()
}

#[pymethods]
impl LocalGeometry {
    #[new]
    #[pyo3(signature=(path, /, *, frame=None))]
    pub fn new(
        py: Python,
        path: PathString,
        frame: Option<LocalFrame>,
    ) -> PyResult<Py<Self>> {
        match Path::new(&path.0).extension().and_then(OsStr::to_str) {
            Some("json") | Some("toml") | Some("yml") | Some("yaml") => {
                unimplemented!() // XXX impl from calzone.
            },
            Some("dll") | Some("dylib") | Some("mod") | Some("so")  => {
                let module = unsafe { Module::new(py, path)? }
                    .bind(py)
                    .borrow();
                module.geometry(py, frame)
            },
            _ => Err(PyNotImplementedError::new_err("")),
        }
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

impl LocalGeometry {
    pub fn from_module(
        py: Python<'_>,
        module: &CModule,
        frame: Option<LocalFrame>,
    ) -> PyResult<Py<Self>> {
        let geometry = module.geometry()?;
        let registry = &mut Registry::get(py)?.write().unwrap();

        // Build geometry media.
        let size = geometry.media_len()?;
        let mut media = Vec::with_capacity(size);
        let mut materials = HashSet::new();
        for i in 0..size {
            let medium = Medium::try_from(geometry.medium(i)?)?;
            materials.insert(medium.material.clone());
            media.push(medium);
        }
        for name in materials {
            if let Some(material) = module.material(&name, registry)? {
                registry.add_material(name, material)?;
            }
        }
        let media = PyTuple::new(py, media)?.unbind();

        // Bundle the geometry definition.
        let subscribers = Vec::new();
        let frame = frame.unwrap_or_else(|| LocalFrame::default());
        let local_geometry = Self {
            subscribers,
            geometry,
            media,
            frame,
        };
        let local_geometry = Py::new(py, local_geometry)?;
        for medium in local_geometry.bind(py).borrow().media.bind(py).iter() {
            let medium: &Bound<Medium> = medium.downcast().unwrap();
            medium.borrow_mut().geometry = Some(local_geometry.clone_ref(py));
        }

        Ok(local_geometry)
    }

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

    pub fn tracer<'a>(&'a self) -> PyResult<LocalTracer<'a>> {
        let tracer = self.geometry.tracer()?;
        if tracer.is_none_locate() {
            return Err(null_pointer_fmt!("LocalTracer::locate"))
        }
        if tracer.is_none_reset() {
            return Err(null_pointer_fmt!("LocalTracer::reset"))
        }
        if tracer.is_none_trace() {
            return Err(null_pointer_fmt!("LocalTracer::trace"))
        }
        if tracer.is_none_move_() {
            return Err(null_pointer_fmt!("LocalTracer::move"))
        }
        if tracer.is_none_turn() {
            return Err(null_pointer_fmt!("LocalTracer::turn"))
        }
        if tracer.is_none_medium() {
            return Err(null_pointer_fmt!("LocalTracer::medium"))
        }
        if tracer.is_none_position() {
            return Err(null_pointer_fmt!("LocalTracer::position"))
        }
        Ok(LocalTracer { definition: self, ptr: tracer })
    }
}

impl<'a> LocalTracer<'a> {
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
