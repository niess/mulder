use crate::simulation::materials::{Component, Material, MaterialsData};
use std::cmp::Ordering::{Less, Equal, Greater};


// ===============================================================================================
//
// Toml writer, for materials.
//
// ===============================================================================================

pub trait ToToml {
    fn to_toml(&self) -> String;
}

impl ToToml for MaterialsData {
    fn to_toml(&self) -> String {
        const EV: f64 = 1E-09;
        let mut lines = Vec::<String>::new();

        if let Some(elements) = self.raw_table() {
            let mut elements: Vec<_> = elements.iter().collect();
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
                    "\"{}\" = {{ Z = {}, A = {}, I = {} }}",
                    element.0,
                    element.1.Z,
                    element.1.A,
                    element.1.I * EV,
                ));
            }
            lines.push("".to_string());
        }

        let mut keys: Vec<_> = self.map.keys().collect();
        keys.sort();
        let n = keys.len();
        for (i, key) in keys.iter().enumerate() {
            lines.push(format!("[{}]", key));
            lines.push(self.map[key.as_str()].to_toml());
            if i < n - 1 {
                lines.push("".to_string());
            }
        }

        lines.join("\n")
    }
}

impl ToToml for Material {
    fn to_toml(&self) -> String {
        let mut lines = Vec::<String>::new();
        lines.push(format!("density = {}", self.density));
        if let Some(mee) = self.I {
            lines.push(format!("I = {}", mee));
        }
        let components: Vec<_> = self.composition.iter()
            .map(|component| component.to_toml())
            .collect();
        let composition = components.join(", ");
        lines.push(format!("composition = {{ {} }}", composition));
        lines.join("\n")
    }
}

impl<'a> ToToml for Component {
    fn to_toml(&self) -> String {
        format!("{} = {}", self.name, self.weight)
    }
}
