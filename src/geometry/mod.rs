use crate::bindings::turtle;
use crate::utils::coordinates::GeographicCoordinates;
use crate::utils::error::{self, Error};
use crate::utils::error::ErrorKind::{IndexError, TypeError};
use crate::utils::extract::{self, Direction, Position};
use crate::utils::numpy::{Dtype, NewArray};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use pyo3::sync::GILOnceCell;
use std::ffi::c_int;
use std::ptr::{null, null_mut};

pub mod atmosphere;
pub mod grid;
pub mod layer;

use atmosphere::{Atmosphere, AtmosphereLike};
use layer::{DataLike, Layer};


#[pyclass(module="mulder")]
pub struct Geometry {
    /// The geometry atmosphere.
    #[pyo3(get)]
    pub atmosphere: Py<Atmosphere>,

    pub layers: Vec<Py<Layer>>,
    pub stepper: *mut turtle::Stepper,
}

unsafe impl Send for Geometry {}
unsafe impl Sync for Geometry {}

#[derive(FromPyObject)]
enum AtmosphereArg<'py> {
    Model(AtmosphereLike<'py>),
    Object(Py<Atmosphere>),
}

#[derive(FromPyObject)]
enum LayerLike<'py> {
    Layer(Py<Layer>),
    OneData(DataLike<'py>),
    ManyData(Vec<DataLike<'py>>),
}

#[repr(C)]
struct Intersection {
    before: i32,
    after: i32,
    latitude: f64,
    longitude: f64,
    altitude: f64,
    distance: f64,
}

#[pymethods]
impl Geometry {
    #[pyo3(signature=(*layers, atmosphere=None))]
    #[new]
    fn new(layers: &Bound<PyTuple>, atmosphere: Option<AtmosphereArg>) -> PyResult<Self> {
        let py = layers.py();
        let layers = {
            let mut v = Vec::with_capacity(layers.len());
            for layer in layers.iter() {
                let layer: LayerLike = layer.extract()?;
                let layer = match layer {
                    LayerLike::Layer(layer) => layer,
                    LayerLike::OneData(data) => {
                        let data = vec![data.into_data(py)?];
                        let layer = Layer::new(data, None, None)?;
                        Py::new(py, layer)?
                    },
                    LayerLike::ManyData(data) => {
                        let data: PyResult<Vec<_>> = data.into_iter()
                            .map(|data| data.into_data(py))
                            .collect();
                        let layer = Layer::new(data?, None, None)?;
                        Py::new(py, layer)?
                    },
                };
                v.push(layer)
            }
            v
        };

        let atmosphere = match atmosphere {
            Some(atmosphere) => match atmosphere {
                AtmosphereArg::Model(model) => {
                    let atmosphere = Atmosphere::new(Some(model))?;
                    Py::new(py, atmosphere)?
                },
                AtmosphereArg::Object(atmosphere) => atmosphere,
            },
            None => {
                let atmosphere = Atmosphere::new(None)?;
                Py::new(py, atmosphere)?
            },
        };

        const WHAT: Option<&str> = Some("geometry");
        let mut stepper = null_mut();
        error::to_result(unsafe { turtle::stepper_create(&mut stepper) }, WHAT)?;
        error::to_result(unsafe { turtle::stepper_add_flat(stepper, Self::ZMIN) }, WHAT)?;
        for layer in layers.iter() {
            let layer = layer.bind(py).borrow();
            unsafe { layer.insert(py, stepper)?; }
        }
        error::to_result(unsafe { turtle::stepper_add_layer(stepper) }, WHAT)?;
        error::to_result(unsafe { turtle::stepper_add_flat(stepper, Self::ZMAX) }, WHAT)?;

        Ok(Self { layers, atmosphere, stepper })
    }

    /// The geometry layers.
    #[getter]
    fn get_layers<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        let elements = self.layers
            .iter()
            .map(|data| data.clone_ref(py));
        PyTuple::new(py, elements)
    }

    fn __getitem__(&self, py: Python, index: usize) -> PyResult<Py<Layer>> {
        self.layers
            .get(index)
            .map(|layer| layer.clone_ref(py))
            .ok_or_else(|| {
                let why = format!(
                    "expected a value in [0, {}], found '{}'",
                    self.layers.len() - 1,
                    index,
                );
                Error::new(IndexError)
                    .what("layer index")
                    .why(&why)
                    .to_err()
            })
    }

    #[pyo3(signature=(position=None, /, **kwargs))]
    fn locate<'py>(
        &self,
        py: Python<'py>,
        position: Option<&Bound<PyAny>>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<NewArray<'py, i32>> {
        let position = extract::select_position(position, kwargs)?
            .ok_or_else(|| Error::new(TypeError)
                .what("position")
                .why("expected one argument, found zero")
                .to_err()
            )?;
        let position = Position::extract_bound(position)?;

        let mut array = NewArray::empty(py, position.shape())?;
        let layer = array.as_slice_mut();
        for i in 0..position.size() {
            let geographic = position.get(i)?;
            let mut r = geographic.to_ecef();
            let mut index = [ -2; 2 ];
            error::to_result(
                unsafe {
                    turtle::stepper_step(
                        self.stepper,
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
            layer[i] = layer_index(index[0]);
        }
        Ok(array)
    }

    #[pyo3(signature=(coordinates=None, /, *, **kwargs))]
    fn scan<'py>(
        &self,
        py: Python<'py>,
        coordinates: Option<&Bound<PyAny>>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<NewArray<'py, f64>> {
        let coordinates = extract::select_coordinates(coordinates, kwargs)?
            .ok_or_else(|| Error::new(TypeError)
                .what("coordinates")
                .why("expected one argument, found zero")
                .to_err()
            )?;
        let position = Position::extract_bound(coordinates)?;
        let direction = Direction::extract_bound(coordinates)?;
        let (size, mut shape) = position.common(&direction)?;
        let (shape, n) = {
            let n = self.layers.len();
            shape.push(n);
            (shape, n)
        };

        let mut array = NewArray::<f64>::zeros(py, shape)?;
        let distances = array.as_slice_mut();
        for i in 0..size {
            // Get the starting point.
            let geographic = position.get(i)?;
            let mut r = geographic.to_ecef();
            let mut index = [ -2; 2 ];
            error::to_result(
                unsafe {
                    turtle::stepper_step(
                        self.stepper,
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
            let horizontal = direction.get(i)?;
            let u = horizontal.to_ecef(&geographic);
            while (index[0] >= 1) && (index[0] as usize <= n + 1) {
                let current = index[0];
                let mut di = 0.0;
                while index[0] == current {
                    let mut step: f64 = 0.0;
                    error::to_result(
                        unsafe {
                            turtle::stepper_step(
                                self.stepper,
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
        }

        Ok(array)
    }

    #[pyo3(signature=(coordinates=None, /, **kwargs))]
    fn trace<'py>(
        &self,
        py: Python<'py>,
        coordinates: Option<&Bound<PyAny>>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<NewArray<'py, Intersection>> {
        let coordinates = extract::select_coordinates(coordinates, kwargs)?
            .ok_or_else(|| Error::new(TypeError)
                .what("coordinates")
                .why("expected one argument, found zero")
                .to_err()
            )?;
        let position = Position::extract_bound(coordinates)?;
        let direction = Direction::extract_bound(coordinates)?;
        let (size, shape) = position.common(&direction)?;

        let mut array = NewArray::empty(py, shape)?;
        let intersections = array.as_slice_mut();
        for i in 0..size {
            let geographic = position.get(i)?;
            let mut r = geographic.to_ecef();
            let mut index = [ -2; 2 ];
            error::to_result(
                unsafe {
                    turtle::stepper_step(
                        self.stepper,
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
                              (start_layer as usize <= self.layers.len() + 1) {

                // Iterate until a boundary is hit.
                let horizontal = direction.get(i)?;
                let u = horizontal.to_ecef(&geographic);
                let mut step = 0.0_f64;
                while index[0] == start_layer {
                    error::to_result(
                        unsafe {
                            turtle::stepper_step(
                                self.stepper,
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
                geographic.clone()
            };
            intersections[i] = Intersection {
                before: layer_index(start_layer),
                after: layer_index(index[0]),
                latitude: position.latitude,
                longitude: position.longitude,
                altitude: position.altitude,
                distance: di,
            };
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

impl Geometry {
    // Height of the bottom layer, in m.
    const ZMIN: f64 = -11E+03;

    // Top most height, in m.
    const ZMAX: f64 = 120E+03;
}

impl Drop for Geometry {
    fn drop(&mut self) {
        unsafe {
            turtle::stepper_destroy(&mut self.stepper);
        }
    }
}

static INTERSECTION_DTYPE: GILOnceCell<PyObject> = GILOnceCell::new();

impl Dtype for Intersection {
    fn dtype<'py>(py: Python<'py>) -> PyResult<&'py Bound<'py, PyAny>> {
        let ob = INTERSECTION_DTYPE.get_or_try_init(py, || -> PyResult<_> {
            let ob = PyModule::import(py, "numpy")?
                .getattr("dtype")?
                .call1(([
                        ("before",    "i4"),
                        ("after",     "i4"),
                        ("latitude",  "f8"),
                        ("longitude", "f8"),
                        ("altitude",  "f8"),
                        ("distance",  "f8")
                    ],
                    true,
                ))?
                .unbind();
            Ok(ob)
        })?
        .bind(py);
        Ok(ob)
    }
}
