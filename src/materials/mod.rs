use crate::utils::io::PathString;
use crate::utils::traits::EnsureFile;
use pyo3::prelude::*;
use std::path::Path;

pub mod definitions;
pub mod registry;
pub mod set;
pub mod toml;
pub mod xml;

pub use definitions::{Component, Composite, Element, Material, Mixture};
pub use set::{MaterialsSet, MaterialsSubscriber};
pub use xml::Mdf;
pub use registry::{MaterialsBroker, Registry};

use toml::ToToml;


/// Load material definitions.
#[pyfunction]
#[pyo3(signature=(path, /))]
pub fn load(py: Python, path: PathString) -> PyResult<()> {
    let _ = Path::new(&path.0).ensure_file("path")?;
    let broker = MaterialsBroker::new(py)?;
    broker.load(py, path.0.as_str())
}

/// Dump material definitions.
#[pyfunction]
#[pyo3(signature=(path, *materials))]
pub fn dump(py: Python, path: PathString, mut materials: Vec<String>) -> PyResult<()> {
    if materials.is_empty() {
        let registry = &Registry::get(py)?.read().unwrap();
        for material in registry.materials().keys() {
            materials.push(material.clone());
        }
    }
    let materials = MaterialsSet::from(materials);
    std::fs::write(path.0.as_str(), materials.to_toml(py)?)?;
    Ok(())
}
