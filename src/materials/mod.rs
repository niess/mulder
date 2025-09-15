use crate::utils::io::PathString;
use pyo3::prelude::*;

pub mod definitions;
pub mod registry;
pub mod set;
pub mod toml;
pub mod xml;

pub use definitions::{Component, Element, Material};
pub use set::{MaterialsSet, MaterialsSubscriber};
pub use xml::Mdf;
pub use registry::Registry;


/// Load material definitions.
#[pyfunction]
#[pyo3(signature=(path, /))]
pub fn load(py: Python, path: PathString) -> PyResult<()> {
    let registry = &mut Registry::get(py)?.write().unwrap();
    registry.load(py, path.0.as_str())
}

// XXX list materials (as a dict?)
