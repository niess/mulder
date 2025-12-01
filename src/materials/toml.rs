use pyo3::prelude::*;
use std::cmp::Ordering::{Less, Equal, Greater};
use super::definitions::{Component, Composite, Element, Mixture};
use super::registry::MaterialsBroker;
use super::set::{MaterialsSet, UnpackedMaterials};


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
        let broker = MaterialsBroker::new(py)?;
        let materials = self.borrow();
        let UnpackedMaterials { composites, elements, mixtures } = materials.unpack(&broker)?;
        let registry = &broker.registry.read().unwrap();

        let mut elements: Vec<_> = elements
            .iter()
            .map(|e| (e, registry.element(e)))
            .collect();
        elements.sort_by(|a, b| match a.1.Z.cmp(&b.1.Z) {
            Equal => match a.1.A.partial_cmp(&b.1.A).unwrap() {
                Equal => a.0.cmp(&b.0),
                Less => Less,
                Greater => Greater,
            },
            Less => Less,
            Greater => Greater,

        });

        let mut lines = Vec::<String>::new();
        lines.push("[elements]".to_string());

        for element in elements {
            lines.push(format!(
                "\"{}\" = {}",
                element.0,
                element.1.to_toml(py)?,
            ));
        }

        for name in mixtures {
            let mixture = registry.mixture(&name);
            lines.push(format!("\n[{}]", name));
            lines.push(mixture.to_toml(py)?);
        }

        for name in composites {
            let composite = registry.composite(&name);
            lines.push(format!("\n[{}]", name));
            lines.push(composite.to_toml(py)?);
        }

        Ok(lines.join("\n"))
    }
}

impl ToToml for Element {
    fn to_toml(&self, _py: Python) -> PyResult<String> {
        Ok(format!(
            "{{ Z = {}, A = {}, I = {} }}",
            self.Z,
            self.A,
            self.I,
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
