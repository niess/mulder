use crate::bindings::turtle;
use crate::utils::error::{self, Error};
use crate::utils::error::ErrorKind::IndexError;
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use std::ptr::null_mut;

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

        let mut stepper = null_mut();
        let rc = unsafe { turtle::stepper_create(&mut stepper) };
        error::to_result(rc, Some("geometry"))?;
        for layer in layers.iter() {
            let layer = layer.bind(py).borrow();
            unsafe { layer.insert(py, stepper)?; }
        }

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
}

impl Drop for Geometry {
    fn drop(&mut self) {
        unsafe {
            turtle::stepper_destroy(&mut self.stepper);
        }
    }
}
