use crate::bindings::turtle;
use crate::utils::coordinates::{self, Direction, Position};
use crate::utils::error::{self, Error};
use crate::utils::error::ErrorKind::{IndexError, TypeError};
use crate::utils::numpy::NewArray;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use std::ptr::{null, null_mut};

pub mod atmosphere;
pub mod grid;
pub mod layer;

use atmosphere::{Atmosphere, AtmosphereLike};
use layer::{DataArg, DataLike, Layer};


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
enum LayersArg<'py> {
    Data(DataLike<'py>),
    Layer(Py<Layer>),
    Layers(Vec<Py<Layer>>),
}

#[pymethods]
impl Geometry {
    #[pyo3(signature=(layers, /, *, atmosphere=None))]
    #[new]
    fn new(py: Python, layers: LayersArg, atmosphere: Option<AtmosphereArg>) -> PyResult<Self> {
        let layers = match layers {
            LayersArg::Data(data) => {
                let layer = Layer::new(py, DataArg::One(data), None, None)?;
                let layer = Py::new(py, layer)?;
                vec![layer]
            },
            LayersArg::Layer(layer) => vec![layer],
            LayersArg::Layers(layers) => layers,
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
        let position = coordinates::select_position(position, kwargs)?
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
            layer[i] = index[0];
        }
        Ok(array)
    }

    #[pyo3(signature=(coordinates=None, /, **kwargs))]
    fn trace<'py>(
        &self,
        py: Python<'py>,
        coordinates: Option<&Bound<PyAny>>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<NewArray<'py, f64>> {
        let coordinates = coordinates::select_coordinates(coordinates, kwargs)?
            .ok_or_else(|| Error::new(TypeError)
                .what("coordinates")
                .why("expected one argument, found zero")
                .to_err()
            )?;
        let position = Position::extract_bound(coordinates)?;
        let direction = Direction::extract_bound(coordinates)?;
        let (size, shape) = position.common(&direction)?;

        let mut array = NewArray::empty(py, shape)?;
        let distance = array.as_slice_mut();
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
            if (start_layer >= 1) && (start_layer as usize <= self.layers.len() + 1) {
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
            }
            distance[i] = di;
        }
        Ok(array)
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
