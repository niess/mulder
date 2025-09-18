use crate::materials::definitions::{Component, Composite, Element, Mixture};
use crate::materials::registry::Registry;
use crate::materials::set::{MaterialsSet, UnpackedMaterials};
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
        let materials = materials.borrow();
        let UnpackedMaterials { composites, elements, mixtures } = materials.unpack(registry)?;

        let mut lines = Vec::<String>::new();
        lines.push("<pumas>".to_string());

        for symbol in elements {
            let element = registry.get_element(symbol).unwrap();
            let element = element.to_xml(Some(symbol));
            lines.push(element);
        }

        for name in mixtures {
            if let Some(mixture) = registry.get_material(name.as_str()).unwrap().as_mixture() {
                let mixture = mixture.to_xml(Some(name.as_str()));
                lines.push(mixture);
            }
        }
        for name in composites {
            if let Some(composite) = registry.get_material(name).unwrap().as_composite() {
                let composite = composite.to_xml(Some(name));
                lines.push(composite);
            }
        }

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
