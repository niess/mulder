use crate::materials::definitions::{Component, Composite, Element, Mixture};
use crate::materials::registry::Registry;
use crate::materials::set::MaterialsSet;
use pyo3::prelude::*;
use ::std::path::Path;


// ===============================================================================================
//
// Materials Description File (MDF) for Pumas.
//
// ===============================================================================================

pub struct Mdf (String);

impl Mdf {
    pub fn new(py: Python, materials: &MaterialsSet) -> PyResult<Self> {
        let registry = &Registry::get(py)?.read().unwrap();
        let mut lines = Vec::<String>::new();
        lines.push("<pumas>".to_string());

        let mut elements = Vec::<&str>::new();
        for material in materials.borrow().iter() {
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

        for symbol in elements {
            let element = registry.get_element(symbol).unwrap();
            let element = element.to_xml(Some(symbol));
            lines.push(element);
        }

        let borrow = materials.borrow();
        let mut keys = borrow.iter().collect::<Vec<_>>();
        keys.sort();
        for key in keys.iter() {
            if let Some(mixture) = registry.get_material(key).unwrap().as_mixture() {
                let mixture = mixture.to_xml(Some(key));
                lines.push(mixture);
            }
        }
        for key in keys.iter() {
            if let Some(composite) = registry.get_material(key).unwrap().as_composite() {
                let composite = composite.to_xml(Some(key));
                lines.push(composite);
            }
        }
        drop(borrow);

        lines.push("</pumas>".to_string());
        let mdf = lines.join("\n");
        let mdf = Self (mdf);

        Ok(mdf)
    }

    pub fn dump<P: AsRef<Path>>(&self, destination: P) -> PyResult<()> {
        std::fs::write(destination, self.0.as_str())?;
        Ok(())
    }
}


// ===============================================================================================
//
// Xml conversions.
//
// ===============================================================================================

trait ToXml {
    fn to_xml(&self, key: Option<&str>) -> String;
}

impl ToXml for Composite {
    fn to_xml(&self, key: Option<&str>) -> String {
        let data = self.read();
        let mut lines = Vec::<String>::new();
        lines.push(format!("<composite name=\"{}\">", key.unwrap()));
        lines.push(data.composition.to_xml(None));
        lines.push("</composite>".to_string());
        lines.join("\n")
    }
}

impl ToXml for Element {
    fn to_xml(&self, key: Option<&str>) -> String {
        format!(
            "<element name=\"{}\" Z=\"{}\" A=\"{}\" I=\"{}\" />",
            key.unwrap(),
            self.Z,
            self.A, // g/mol
            self.I, // eV.
        )
    }
}

impl ToXml for Mixture {
    fn to_xml(&self, key: Option<&str>) -> String {
        let mut lines = Vec::<String>::new();
        let header = match self.I {
            Some(mee) => format!(
                "<material name=\"{}\" density=\"{}\" I=\"{}\">",
                key.unwrap(),
                self.density * 1E-03, // g/cm3.
                mee * 1E+09, // eV
            ),
            None => format!(
                "<material name=\"{}\" density=\"{}\">",
                key.unwrap(),
                self.density * 1E-03, // g/cm3.
            ),
        };
        lines.push(header);
        lines.push(self.composition.to_xml(None));
        lines.push("</material>".to_string());
        lines.join("\n")
    }
}

impl ToXml for Vec<Component> {
    fn to_xml(&self, _key: Option<&str>) -> String {
        let mut lines = Vec::<String>::new();
        let mut composition: Vec<_> = self.iter().collect();
        composition.sort_by(|a, b| a.name.cmp(&b.name));
        for Component { name, weight } in composition.drain(..) {
            let line = format!(
                "    <component name=\"{}\" fraction=\"{}\" />",
                name,
                weight,
            );
            lines.push(line);
        }
        lines.join("\n")
    }
}
