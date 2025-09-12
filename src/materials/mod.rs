use crate::utils::io::PathString;
use pyo3::prelude::*;

pub mod definitions;
pub mod registry;
pub mod set;

pub use definitions::{Element, Material};
use registry::Registry;


/// Load material definitions.
#[pyfunction]
#[pyo3(signature=(path, /))]
pub fn load(py: Python, path: PathString) -> PyResult<()> {
    let registry = &mut Registry::get(py)?.write().unwrap();
    registry.load(py, path.0.as_str())
}

// XXX list materials (as a dict?)
