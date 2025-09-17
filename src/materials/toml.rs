use pyo3::prelude::*;
use std::cmp::Ordering::{Less, Equal, Greater};
use super::definitions::{Component, Composite, Element, Mixture};
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
            let definition = registry
                .get_material(material)?
                .as_mixture();
            if let Some(definition) = definition {
                for Component { name, .. } in definition.composition.iter() {
                    elements.push(name)
                }
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

        let mut keys: Vec<_> = materials.iter().collect();
        keys.sort();

        for key in keys.iter() {
            if let Some(mixture) = registry.get_material(key)?.as_mixture() {
                lines.push(format!("\n[{}]", key));
                lines.push(mixture.to_toml(py)?);
            }
        }

        for key in keys {
            if let Some(composite) = registry.get_material(key)?.as_composite() {
                lines.push(format!("\n[{}]", key));
                lines.push(composite.to_toml(py)?);
            }
        }

        Ok(lines.join("\n"))
    }
}

impl ToToml for Element {
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

impl ToToml for Composite {
    #[inline]
    fn to_toml(&self, py: Python) -> PyResult<String> {
        let data = self.read();
        data.composition.to_toml(py)
    }
}

impl ToToml for Mixture {
    fn to_toml(&self, py: Python) -> PyResult<String> {
        let mut lines = Vec::<String>::new();
        lines.push(format!("density = {}", self.density));
        if let Some(mee) = self.I {
            lines.push(format!("I = {}", mee));
        }
        lines.push(self.composition.to_toml(py)?);
        Ok(lines.join("\n"))
    }
}

impl ToToml for Vec<Component> {
    fn to_toml(&self, py: Python) -> PyResult<String> {
        let mut lines = Vec::<String>::new();
        let components = self.iter()
            .map(|component| component.to_toml(py))
            .collect::<PyResult<Vec<_>>>()?;
        let composition = components.join(", ");
        lines.push(format!("composition = {{ {} }}", composition));
        Ok(lines.join("\n"))
    }
}

impl ToToml for Component {
    fn to_toml(&self, _py: Python) -> PyResult<String> {
        Ok(format!("{} = {}", self.name, self.weight))
    }
}
