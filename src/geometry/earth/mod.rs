use crate::bindings::turtle;
use crate::materials::set::{MaterialsSet, MaterialsSubscriber};
use crate::simulation::coordinates::{
    CoordinatesExtractor, GeographicCoordinates, HorizontalCoordinates, LocalFrame,
    PositionExtractor,
};
use crate::utils::error;
use crate::utils::notify::{Notifier, NotifyArg};
use crate::utils::numpy::NewArray;
use crate::utils::ptr::{Destroy, OwnedPtr};
use crate::utils::traits::MinMax;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use std::ffi::c_int;
use std::ptr::{NonNull, null, null_mut};
use super::intersections::{GeographicIntersection, IntersectionsArray};

pub mod grid;
pub mod layer;

use layer::DataLike;

pub use grid::Grid;
pub use layer::Layer;


// XXX Allow for any geoid?

/// A stratified Earth geometry.
#[pyclass(module="mulder")]
pub struct EarthGeometry {
    /// The Earth geometry limits along the z-coordinate.
    #[pyo3(get)]
    pub zlim: (f64, f64),

    /// The Earth geometry layers.
    #[pyo3(get)]
    pub layers: Py<PyTuple>,  // sequence of Py<Layer>.

    pub subscribers: Vec<MaterialsSubscriber>,
}

unsafe impl Send for EarthGeometry {}
unsafe impl Sync for EarthGeometry {}

#[derive(FromPyObject)]
enum LayerLike<'py> {
    Layer(Py<Layer>),
    OneData(DataLike<'py>),
    ManyData(Vec<DataLike<'py>>),
}

pub struct EarthGeometryStepper {
    ptr: OwnedPtr<turtle::Stepper>,
    pub layers: usize,
}

#[pymethods]
impl EarthGeometry {
    #[pyo3(signature=(*layers))]
    #[new]
    pub fn new(layers: &Bound<PyTuple>) -> PyResult<Py<Self>> {
        let py = layers.py();
        let (layers, zlim) = {
            let mut zlim = (Self::ZMIN, -f64::INFINITY);
            let mut v = Vec::with_capacity(layers.len());
            for layer in layers.iter() {
                let layer: LayerLike = layer.extract()?;
                let layer = match layer {
                    LayerLike::Layer(layer) => layer,
                    LayerLike::OneData(data) => {
                        let data = vec![data.into_data(py)?];
                        let layer = Layer::new(py, data, None, None, None)?;
                        Py::new(py, layer)?
                    },
                    LayerLike::ManyData(data) => {
                        let data: PyResult<Vec<_>> = data.into_iter()
                            .map(|data| data.into_data(py))
                            .collect();
                        let layer = Layer::new(py, data?, None, None, None)?;
                        Py::new(py, layer)?
                    },
                };
                let lz = layer.bind(py).borrow().zlim;
                if lz.min() < zlim.min() { *zlim.mut_min() = lz.min(); }
                if lz.max() > zlim.max() { *zlim.mut_max() = lz.max(); }
                v.push(layer)
            }
            (v, zlim)
        };
        let layers = PyTuple::new(py, layers)?.unbind();

        let subscribers = Vec::new();

        let geometry = Self { layers, zlim, subscribers };
        let geometry = Py::new(py, geometry)?;
        for layer in geometry.bind(py).borrow().layers.bind(py).iter() {
            let mut layer = layer.downcast::<Layer>().unwrap().borrow_mut();
            layer.geometry = Some(geometry.clone_ref(py));
        }

        Ok(geometry)
    }

    /// Locates point(s) within the Earth geometry.
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
        let position = PositionExtractor::new(py, position, kwargs, frame.as_ref().into(), None)?;

        let mut stepper = self.stepper(py)?;
        let notifier = Notifier::from_arg(notify, position.size(), "locating position(s)");

        let mut array = NewArray::empty(py, position.shape())?;
        let layer = array.as_slice_mut();
        for i in 0..position.size() {
            const WHY: &str = "while locating position(s)";
            if (i % 100) == 0 { error::check_ctrlc(WHY)? }

            stepper.reset();

            let geographic = position.extract(i)?
                .into_geographic();
            layer[i] = stepper.locate(geographic)?;
            notifier.tic();
        }
        Ok(array)
    }

    /// Performs a detailed tracing of the Earth geometry.
    #[pyo3(
        signature=(coordinates=None, /, *, notify=None, frame=None, **kwargs),
        text_signature="(self, coordinates=None, /, *, notify=None, **kwargs)",
    )]
    fn scan<'py>(
        &mut self,
        py: Python<'py>,
        coordinates: Option<&Bound<PyAny>>,
        notify: Option<NotifyArg>,
        frame: Option<LocalFrame>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<NewArray<'py, f64>> {
        let coordinates = CoordinatesExtractor::new(
            py, coordinates, kwargs, frame.as_ref().into(), None
        )?;
        let (size, shape, n) = {
            let size = coordinates.size();
            let mut shape = coordinates.shape();
            let n = self.layers.bind(py).len();
            shape.push(n);
            (size, shape, n)
        };

        let mut stepper = self.stepper(py)?;
        let notifier = Notifier::from_arg(notify, size, "scanning geometry");

        let mut array = NewArray::<f64>::zeros(py, shape)?;
        let distances = array.as_slice_mut();
        for i in 0..size {
            const WHY: &str = "while scanning geometry";
            if (i % 100) == 0 { error::check_ctrlc(WHY)? }

            stepper.reset();

            // Get the starting point.
            let (geographic, horizontal) = coordinates
                .extract(i)?
                .into_geographic();
            let mut r = geographic.to_ecef();
            let mut index = [ -2; 2 ];
            error::to_result(
                unsafe {
                    turtle::stepper_step(
                        stepper.as_ptr(),
                        r.as_mut_ptr(),
                        null(),
                        null_mut(),
                        null_mut(),
                        null_mut(),
                        null_mut(),
                        null_mut(),
                        index.as_mut_ptr(),
                    )
                },
                None::<&str>,
            )?;

            // Iterate until the particle exits.
            let u = horizontal.to_ecef(&geographic);
            while (index[0] >= 1) && (index[0] as usize <= n + 1) {
                let current = index[0];
                let mut di = 0.0;
                while index[0] == current {
                    let mut step: f64 = 0.0;
                    error::to_result(
                        unsafe {
                            turtle::stepper_step(
                                stepper.as_ptr(),
                                r.as_mut_ptr(),
                                u.as_ptr(),
                                null_mut(),
                                null_mut(),
                                null_mut(),
                                null_mut(),
                                &mut step,
                                index.as_mut_ptr(),
                            )
                        },
                        None::<&str>,
                    )?;
                    di += step;
                }

                let current = current as usize;
                if current <= n {
                    distances[i * n + current - 1]+= di;
                }

                // Push the particle through the boundary.
                const EPS: f64 = f32::EPSILON as f64;
                for i in 0..3 {
                    r[i] += EPS * u[i];
                }
            }

            notifier.tic();
        }

        Ok(array)
    }

    /// Performs a tracing step of the Earth geometry.
    #[pyo3(
        name="trace",
        signature=(coordinates=None, /, *, notify=None, frame=None, **kwargs),
        text_signature="(self, coordinates=None, /, *, notify=None, **kwargs)",
    )]
    fn py_trace<'py>(
        &mut self,
        py: Python<'py>,
        coordinates: Option<&Bound<PyAny>>,
        notify: Option<NotifyArg>,
        frame: Option<LocalFrame>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<IntersectionsArray<'py>> {
        let coordinates = CoordinatesExtractor::new(
            py, coordinates, kwargs, frame.as_ref().into(), None
        )?;
        let size = coordinates.size();
        let shape = coordinates.shape();

        let mut stepper = self.stepper(py)?;
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

            stepper.reset();
            let (position, direction) = coordinates
                .extract(i)?
                .into_geographic();
            let intersection = stepper.trace(position, direction)?.0;
            intersections.set_geographic(i, intersection, coordinates.frame());
            notifier.tic();
        }
        Ok(array)
    }
}

#[inline]
fn layer_index(stepper_index: c_int) -> c_int {
    if stepper_index >= 1 {
        stepper_index - 1
    } else {
        stepper_index
    }
}

impl EarthGeometry {
    // Height of the bottom layer, in m.
    pub const ZMIN: f64 = -11E+03;

    // Height of the atmosphere layer, in m.
    pub const ZMAX: f64 = 120E+03;

    pub fn subscribe(&mut self, py: Python, set: &MaterialsSet) {
        for layer in self.layers.bind(py).iter() {
            let layer = layer.downcast::<Layer>().unwrap().borrow();
            set.add(layer.material.as_str());
        }
        self.subscribers.push(set.subscribe());
        self.subscribers.retain(|s| s.is_alive());
    }

    pub fn unsubscribe(&mut self, py: Python, set: &MaterialsSet) {
        for layer in self.layers.bind(py).iter() {
            let layer = layer.downcast::<Layer>().unwrap().borrow();
            set.remove(layer.material.as_str());
        }
        self.subscribers.retain(|s| s.is_alive() && !s.is_subscribed(set));
    }

    pub fn stepper(&self, py: Python) -> PyResult<EarthGeometryStepper> {
        let layers = self.layers.bind(py);
        const WHAT: Option<&str> = Some("geometry");
        let mut ptr = null_mut();
        error::to_result(unsafe { turtle::stepper_create(&mut ptr) }, WHAT)?;
        error::to_result(unsafe { turtle::stepper_add_flat(ptr, self.zlim.min()) }, WHAT)?;
        for layer in layers.iter() {
            let layer = layer.downcast::<Layer>().unwrap().borrow();
            unsafe { layer.insert(py, ptr)?; }
        }
        error::to_result(unsafe { turtle::stepper_add_layer(ptr) }, WHAT)?;
        error::to_result(unsafe { turtle::stepper_add_flat(ptr, Self::ZMAX) }, WHAT)?;
        let ptr = OwnedPtr::new(ptr)?;
        let layers = layers.len();
        let stepper = EarthGeometryStepper { ptr, layers };
        Ok(stepper)
    }
}

impl EarthGeometryStepper {
    pub fn step(
        &mut self,
        position: &mut [f64; 3],
        geographic: &mut GeographicCoordinates
    ) -> (f64, usize) {
        let mut step = 0.0;
        let mut index = [ -1; 2 ];
        unsafe {
            turtle::stepper_step(
                self.as_ptr(),
                position.as_mut_ptr(),
                null(),
                &mut geographic.latitude,
                &mut geographic.longitude,
                &mut geographic.altitude,
                null_mut(),
                &mut step,
                index.as_mut_ptr(),
            );
        }
        (step, index[0] as usize)
    }

    #[inline]
    pub fn reset(&mut self) {
        unsafe {
            turtle::stepper_reset(self.as_ptr());
        }
    }

    #[inline]
    fn as_ptr(&self) -> *mut turtle::Stepper {
        self.ptr.0.as_ptr()
    }

    pub fn locate(&mut self, position: GeographicCoordinates) -> PyResult<i32> {
        let mut r = position.to_ecef();
        let mut index = [ -2; 2 ];
        error::to_result(
            unsafe {
                turtle::stepper_step(
                    self.as_ptr(),
                    r.as_mut_ptr(),
                    null(),
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    index.as_mut_ptr(),
                )
            },
            None::<&str>,
        )?;
        Ok(layer_index(index[0]))
    }

    pub fn trace(
        &mut self,
        position: GeographicCoordinates,
        direction: HorizontalCoordinates
    ) -> PyResult<(GeographicIntersection, i32)> {
        let mut r = position.to_ecef();
        let mut index = [ -2; 2 ];
        error::to_result(
            unsafe {
                turtle::stepper_step(
                    self.as_ptr(),
                    r.as_mut_ptr(),
                    null(),
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    index.as_mut_ptr(),
                )
            },
            None::<&str>,
        )?;
        let start_layer = index[0];
        let mut di = 0.0;
        let position = if (start_layer >= 1) &&
                          (start_layer as usize <= self.layers + 1) {

            // Iterate until a boundary is hit.
            let u = direction.to_ecef(&position);
            let mut step = 0.0_f64;
            while index[0] == start_layer {
                error::to_result(
                    unsafe {
                        turtle::stepper_step(
                            self.as_ptr(),
                            r.as_mut_ptr(),
                            u.as_ptr(),
                            null_mut(),
                            null_mut(),
                            null_mut(),
                            null_mut(),
                            &mut step,
                            index.as_mut_ptr(),
                        )
                    },
                    None::<&str>,
                )?;
                di += step;
            }

            // Push the particle through the boundary.
            const EPS: f64 = f32::EPSILON as f64;
            di += EPS;
            for i in 0..3 {
                r[i] += EPS * u[i];
            }
            GeographicCoordinates::from_ecef(&r)
        } else {
            position
        };
        Ok((
            GeographicIntersection {
                before: layer_index(start_layer),
                after: layer_index(index[0]),
                latitude: position.latitude,
                longitude: position.longitude,
                altitude: position.altitude,
                distance: di,
            },
            index[1],
        ))
    }
}

impl Destroy for NonNull<turtle::Stepper> {
    fn destroy(self) {
        unsafe { turtle::stepper_destroy(&mut self.as_ptr()); }
    }
}
