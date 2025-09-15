use pyo3::prelude::*;
use std::cmp::Ordering::{Less, Equal, Greater};
use super::definitions::{Component, Element, Material};
use super::registry::Registry;
use super::set::MaterialsSet;


// ===============================================================================================
//
// Toml writer, for materials.
//
// ===============================================================================================

pub trait ToToml {
    fn to_toml(&self, py: Python) -> PyResult<String>;
}

impl ToToml for MaterialsSet {
    fn to_toml(&self, py: Python) -> PyResult<String> {
        let registry = &Registry::get(py)?.read().unwrap();
        let materials = self.borrow();
        let mut lines = Vec::<String>::new();

        let mut elements = Vec::<&str>::new();
        for material in materials.iter() {
            let definition = registry.get_material(material)?;
            for Component { name, .. } in definition.composition.iter() {
                elements.push(name)
            }
        }
        elements.sort();
        elements.dedup();

        let mut elements = elements
            .iter()
            .map(|e| -> PyResult<_> { Ok((e, registry.get_element(e)?)) })
            .collect::<PyResult<Vec<_>>>()?;
        elements.sort_by(|a, b| match a.1.Z.cmp(&b.1.Z) {
            Equal => match a.1.A.partial_cmp(&b.1.A).unwrap() {
                Equal => a.0.cmp(&b.0),
                Less => Less,
                Greater => Greater,
            },
            Less => Less,
            Greater => Greater,

        });
        lines.push("[elements]".to_string());
        for element in elements {
            lines.push(format!(
                "\"{}\" = {}",
                element.0,
                element.1.to_toml(py)?,
            ));
        }
        lines.push("".to_string());

        let mut keys: Vec<_> = materials.iter().collect();
        keys.sort();
        let n = keys.len();
        for (i, key) in keys.iter().enumerate() {
            lines.push(format!("[{}]", key));
            lines.push(registry.get_material(key)?.to_toml(py)?);
            if i < n - 1 {
                lines.push("".to_string());
            }
        }

        Ok(lines.join("\n"))
    }
}

impl<'a> ToToml for Element {
    fn to_toml(&self, _py: Python) -> PyResult<String> {
        const EV: f64 = 1E-09;
        Ok(format!(
            "{{ Z = {}, A = {}, I = {} }}",
            self.Z,
            self.A,
            self.I * EV,
        ))
    }
}

impl ToToml for Material {
    fn to_toml(&self, py: Python) -> PyResult<String> {
        let mut lines = Vec::<String>::new();
        lines.push(format!("density = {}", self.density));
        if let Some(mee) = self.I {
            lines.push(format!("I = {}", mee));
        }
        let components = self.composition.iter()
            .map(|component| component.to_toml(py))
            .collect::<PyResult<Vec<_>>>()?;
        let composition = components.join(", ");
        lines.push(format!("composition = {{ {} }}", composition));
        Ok(lines.join("\n"))
    }
}

impl<'a> ToToml for Component {
    fn to_toml(&self, _py: Python) -> PyResult<String> {
        Ok(format!("{} = {}", self.name, self.weight))
    }
}
