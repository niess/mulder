use crate::utils::io::PathString;
use pyo3::prelude::*;
use pyo3::types::PyDict;

pub mod definitions;
pub mod registry;
pub mod set;
pub mod toml;
pub mod xml;

pub use definitions::{Component, Element, Material};
pub use set::{MaterialsSet, MaterialsSubscriber};
pub use xml::Mdf;
pub use registry::Registry;

use toml::ToToml;


/// Load material definitions.
#[pyfunction]
#[pyo3(signature=(path, /))]
pub fn load(py: Python, path: PathString) -> PyResult<()> {
    let registry = &mut Registry::get(py)?.write().unwrap();
    registry.load(py, path.0.as_str())
}

/// Dump material definitions.
#[pyfunction]
#[pyo3(signature=(path, *materials))]
pub fn dump(py: Python, path: PathString, mut materials: Vec<String>) -> PyResult<()> {
    if materials.is_empty() {
        let registry = &Registry::get(py)?.read().unwrap();
        for material in registry.materials.keys() {
            materials.push(material.clone());
        }
    }
    let materials = MaterialsSet::from(materials);
    std::fs::write(path.0.as_str(), materials.to_toml(py)?)?;
    Ok(())
}

/// Get the current definitions.
#[pyfunction]
#[pyo3(name="definitions", signature=(*, elements=true, materials=true))]
pub fn get_definitions<'py>(
    py: Python<'py>,
    elements: Option<bool>,
    materials: Option<bool>,
) -> PyResult<Bound<'py, PyDict>> {
    let elements = elements.unwrap_or(true);
    let materials = materials.unwrap_or(true);
    let definitions = PyDict::new(py);
    let registry = &Registry::get(py)?.read().unwrap();
    if elements {
        let elements = PyDict::new(py);
        for (k, v) in registry.elements.iter() {
            elements.set_item(k.clone(), v.clone())?;
        }
        definitions.set_item("elements", elements)?;
    }
    if materials {
        let materials = PyDict::new(py);
        for (k, v) in registry.materials.iter() {
            materials.set_item(k.clone(), v.clone())?;
        }
        definitions.set_item("materials", materials)?;
    }
    Ok(definitions)
}

// XXX Composite materials?
