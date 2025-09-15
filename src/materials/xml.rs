use crate::materials::definitions::{Element, Component, Material};
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
            let definition = registry.get_material(material)?;
            for Component { name, .. } in definition.composition.iter() {
                elements.push(name)
            }
        }
        elements.sort();
        elements.dedup();

        for symbol in elements {
            let element = registry.get_element(symbol).unwrap();
            let element = element.to_xml(symbol);
            lines.push(element);
        }

        let borrow = materials.borrow();
        let mut keys = borrow.iter().collect::<Vec<_>>();
        keys.sort();
        for key in keys.drain(..) {
            let material = registry.get_material(key).unwrap();
            let material = material.to_xml(key);
            lines.push(material);
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
    fn to_xml(&self, key: &str) -> String;
}

impl ToXml for Element {
    fn to_xml(&self, key: &str) -> String {
        format!(
            "<element name=\"{}\" Z=\"{}\" A=\"{}\" I=\"{}\" />",
            key,
            self.Z,
            self.A, // g/mol
            self.I, // eV.
        )
    }
}

impl ToXml for Material {
    fn to_xml(&self, key: &str) -> String {
        let mut lines = Vec::<String>::new();
        let header = match self.I {
            Some(mee) => format!(
                "<material name=\"{}\" density=\"{}\" I=\"{}\">",
                key,
                self.density * 1E-03, // g/cm3.
                mee * 1E+09, // eV
            ),
            None => format!(
                "<material name=\"{}\" density=\"{}\">",
                key,
                self.density * 1E-03, // g/cm3.
            ),
        };
        lines.push(header);
        let mut composition: Vec<_> = self.composition.iter().collect();
        composition.sort_by(|a, b| a.name.cmp(&b.name));
        for Component { name, weight } in composition.drain(..) {
            let line = format!(
                "    <component name=\"{}\" fraction=\"{}\" />",
                name,
                weight,
            );
            lines.push(line);
        }
        lines.push("</material>".to_string());
        lines.join("\n")
    }
}
