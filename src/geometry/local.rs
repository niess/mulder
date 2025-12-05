#![allow(non_snake_case)]

use crate::materials::{MaterialsSet, MaterialsSubscriber};
use crate::module::{CGeometry, CMedium, CModule, CTracer, CVec3, Module};
use crate::simulation::coordinates::{CoordinatesExtractor, LocalFrame, Maybe, PositionExtractor};
use crate::utils::error::{self, Error};
use crate::utils::error::ErrorKind::{TypeError, ValueError};
use crate::utils::io::PathString;
use crate::utils::notify::{Notifier, NotifyArg};
use crate::utils::numpy::NewArray;
use crate::utils::ptr::OwnedPtr;
use crate::utils::traits::EnsureFile;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use std::ffi::OsStr;
use std::path::Path;
use super::intersections::{LocalIntersection, IntersectionsArray};


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

#[derive(FromPyObject)]
pub enum LocalArg<'py> {
    Path(PathString),
    Any(Bound<'py, PyAny>),
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

#[pymethods]
impl LocalGeometry {
    #[new]
    #[pyo3(signature=(data, /, *, frame=None))]
    pub fn new<'py>(
        py: Python<'py>,
        data: LocalArg<'py>,
        frame: Option<LocalFrame>,
    ) -> PyResult<Py<Self>> {
        match data {
            LocalArg::Path(path) => {
                let _ = Path::new(&path.0).ensure_file("data")?;
                match Path::new(&path.0).extension().and_then(OsStr::to_str) {
                    Some("json") | Some("toml") | Some("yml") | Some("yaml") => {
                        let geometry = py.import("calzone")?
                            .getattr("Geometry")?
                            .call1((&path.0,))?
                            .call_method1("export", ("mulder",))?
                            .downcast_into_exact::<Self>()?;
                        if let Some(frame) = frame {
                            geometry.borrow_mut().frame = frame;
                        }
                        Ok(geometry.unbind())
                    },
                    Some("dll") | Some("dylib") | Some("mod") | Some("pyd") | Some("so")  => {
                        let module = unsafe { Module::new(py, path)? }
                            .bind(py)
                            .borrow();
                        module.geometry(py, frame)
                    },
                    ext => {
                        let why = format!("{}: invalid extension ({:?})", &path.0, ext);
                        let err = Error::new(TypeError).what("data").why(&why).to_err();
                        return Err(err)
                    },
                }
            },
            LocalArg::Any(obj) => {
                let typename = obj
                    .get_type()
                    .fully_qualified_name()?;
                if typename.to_string_lossy() == "calzone.Geometry" {
                    let geometry = obj
                        .call_method1("export", ("mulder",))?
                        .downcast_into_exact::<Self>()?;
                    if let Some(frame) = frame {
                        geometry.borrow_mut().frame = frame;
                    }
                    Ok(geometry.unbind())
                } else {
                    let why = format!(
                        "cannot create LocalGeometry from a '{}' instance",
                        typename,
                    );
                    Err(Error::new(TypeError).what("data").why(&why).to_err())
                }
            },
        }
    }

    /// Locates point(s) within the local geometry.
    #[pyo3(
        name="locate",
        signature=(position=None, /, *, notify=None, frame=None, **kwargs),
        text_signature="(self, position=None, /, *, notify=None, **kwargs)",
    )]
    fn py_locate<'py>(
        &mut self,
        py: Python<'py>,
        position: Option<&Bound<PyAny>>,
        notify: Option<NotifyArg>,
        frame: Option<LocalFrame>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<NewArray<'py, i32>> {
        let frame = Maybe::always(frame.as_ref(), &self.frame);
        let position = PositionExtractor::new(py, position, kwargs, frame, None)?;
        let transformer = position.transformer(&self.frame);

        let notifier = Notifier::from_arg(notify, position.size(), "locating position(s)");
        let tracer = self.tracer()?;

        let mut array = NewArray::empty(py, position.shape())?;
        let n = array.size();
        let media = array.as_slice_mut();
        for i in 0..n {
            const WHY: &str = "while locating position(s)";
            if (i % 100) == 0 { error::check_ctrlc(WHY)? }

            let ri = position
                .extract(i)?
                .into_local(&self.frame, transformer.as_ref());
            media[i] = tracer.locate(ri) as i32;
            notifier.tic();
        }

        Ok(array)
    }

    // XXX sum & grammage mode?
    /// Performs a detailed tracing of the local geometry.
    #[pyo3(
        signature=(coordinates=None, /, *, notify=None, frame=None, **kwargs),
        text_signature="(self, coordinates=None, /, *, notify=None, **kwargs)",
    )]
    fn scan<'py>(
        &self,
        py: Python<'py>,
        coordinates: Option<&Bound<PyAny>>,
        notify: Option<NotifyArg>,
        frame: Option<LocalFrame>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<NewArray<'py, f64>> {
        let frame = Maybe::always(frame.as_ref(), &self.frame);
        let coordinates = CoordinatesExtractor::new(
            py, coordinates, kwargs, frame, None
        )?;
        let transformer = coordinates.transformer(&self.frame);
        let (size, shape, n) = {
            let size = coordinates.size();
            let mut shape = coordinates.shape();
            let n = self.media.bind(py).len();
            shape.push(n);
            (size, shape, n)
        };

        let tracer = self.tracer()?;
        let notifier = Notifier::from_arg(notify, size, "scanning geometry");

        let mut array = NewArray::<f64>::zeros(py, shape)?;
        let distances = array.as_slice_mut();
        for i in 0..size {
            const WHY: &str = "while scanning geometry";
            if (i % 100) == 0 { error::check_ctrlc(WHY)? }

            let (ri, ui) = coordinates
                .extract(i)?
                .into_local(&self.frame, transformer.as_ref());
            tracer.reset(ri, ui);
            let mut medium = tracer.medium();
            let mut inside = medium < n;
            loop {
                if medium < n {
                    let distance = tracer.trace(f64::INFINITY);
                    distances[i * n + medium] += distance;
                    tracer.move_(distance);
                    medium = tracer.medium();
                } else if !inside {
                    let distance = tracer.trace(f64::INFINITY);
                    if distance > 0.0 {
                        tracer.move_(distance);
                        medium = tracer.medium();
                        inside = true;
                    } else {
                        break
                    }
                } else {
                    break
                }
            }
            notifier.tic();
        }

        Ok(array)
    }

    /// Performs a tracing step of the local geometry.
    #[pyo3(
        name="trace",
        signature=(coordinates=None, /, *, notify=None, frame=None, **kwargs),
        text_signature="(self, coordinates=None, /, *, notify=None, **kwargs)",
    )]
    fn py_trace<'py>(
        &self,
        py: Python<'py>,
        coordinates: Option<&Bound<PyAny>>,
        notify: Option<NotifyArg>,
        frame: Option<LocalFrame>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<IntersectionsArray<'py>> {
        let frame = Maybe::always(frame.as_ref(), &self.frame);
        let coordinates = CoordinatesExtractor::new(
            py, coordinates, kwargs, frame, None
        )?;
        let size = coordinates.size();
        let shape = coordinates.shape();
        let transformer = coordinates.transformer(&self.frame);

        let tracer = self.tracer()?;
        let notifier = Notifier::from_arg(notify, size, "tracing geometry");

        let mut array = if coordinates.is_geographic() {
            IntersectionsArray::Geographic(NewArray::empty(py, shape)?)
        } else {
            IntersectionsArray::Local(NewArray::empty(py, shape)?)
        };
        let mut intersections = array.as_slice_mut();

        for i in 0..size {
            const WHY: &str = "while tracing geometry";
            if (i % 100) == 0 { error::check_ctrlc(WHY)? }

            let (ri, ui) = coordinates
                .extract(i)?
                .into_local(&self.frame, transformer.as_ref());
            tracer.reset(ri, ui);
            let before = tracer.medium() as i32;
            let distance = tracer.trace(f64::INFINITY);
            tracer.move_(distance);
            let after = tracer.medium() as i32;
            let position = tracer.position();
            let intersection = LocalIntersection {
                before, after, position, distance
            };
            intersections.set_local(i, intersection, Some(&self.frame), transformer.as_ref());
            notifier.tic();
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
