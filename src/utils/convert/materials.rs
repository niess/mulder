use crate::simulation::materials::{
    AtomicElement, Component, ElementsTable, Material, MaterialsData
};
use crate::utils::error::{Error, ErrorKind};
use crate::utils::error::ErrorKind::{KeyError, TypeError, ValueError};
use crate::utils::namespace::Namespace;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use std::collections::HashMap;


// ===============================================================================================
//
// Materials data extraction.
//
// ===============================================================================================

impl<'py> TryFrom<Bound<'py, PyDict>> for MaterialsData {
    type Error = PyErr;

    fn try_from(value: Bound<'py, PyDict>) -> PyResult<Self> {
        let to_err = |expected: &str, found: &Bound<PyAny>| {
            let tp = found.get_type();
            let why = format!("expected a '{}', found a '{:?}'", expected, tp);
            Error::new(TypeError)
                .what("materials")
                .why(&why)
                .to_err()
        };

        let mut materials = Self::new();
        materials = match value.get_item("elements")? {
            Some(elements) => {
                let mut table = HashMap::new();
                let elements = elements.downcast::<PyDict>()
                    .map_err(|_| to_err("dict", &elements))?;
                for (k, v) in elements.iter() {
                    let k: String = k.extract()
                        .map_err(|_| to_err("string", &k))?;
                    let v: Bound<PyDict> = v.extract()
                        .map_err(|_| to_err("dict", &v))?;
                    let element: AtomicElement = (k.as_str(), &v).try_into()?;
                    table.insert(k, element);
                }
                let table = ElementsTable::new(table);
                materials.with_table(table)
            },
            None => materials,
        };

        for (k, v) in value.iter() {
            let k: String = k.extract()
                .map_err(|_| to_err("string", &k))?;
            if k == "elements" { continue }
            let v: Bound<PyDict> = v.extract()
                .map_err(|_| to_err("dict", &v))?;
            let material: Material = (k.as_str(), &v, &materials).try_into()?;
            materials.map.insert(k, material);
        }

        Ok(materials)
    }
}

// ===============================================================================================
//
// Elements extraction.
//
// ===============================================================================================

type ElementContext<'a, 'py> = (&'a str, &'a Bound<'py, PyDict>);

impl<'a, 'py> TryFrom<ElementContext<'a, 'py>> for AtomicElement {
    type Error = PyErr;

    #[allow(non_snake_case)]
    fn try_from(value: ElementContext) -> PyResult<Self> {
        const EV: f64 = 1E-09;
        let (name, data) = value;
        let to_err = |kind: ErrorKind, why: &str| -> PyErr {
            let what = format!("'{}' element", name);
            Error::new(kind)
                .what(&what)
                .why(why)
                .to_err()
        };

        let mut Z: Option<u32> = None;
        let mut A: Option<f64> = None;
        let mut I: Option<f64> = None;

        for (k, v) in data.iter() {
            let k: String = k.extract()
                .map_err(|_| to_err(TypeError, "key is not a string"))?;
            match k.as_str() {
                "Z" => {
                    let v: u32 = v.extract()
                        .map_err(|_| to_err(ValueError, "'Z' is not an unsigned integer"))?;
                    Z = Some(v);
                },
                "A" => {
                    let v: f64 = v.extract()
                        .map_err(|_| to_err(ValueError, "'A' is not a float"))?;
                    A = Some(v);
                },
                "I" => {
                    let v: f64 = v.extract()
                        .map_err(|_| to_err(ValueError, "'I' is not a float"))?;
                    I = Some(v / EV);
                },
                _ => {
                    return Err(to_err(KeyError, &format!("invalid property '{}'", k)));
                },
            }
        }
        let Z = Z
            .ok_or_else(|| to_err(KeyError, "missing 'Z'"))?;
        let A = A
            .ok_or_else(|| to_err(KeyError, "missing 'A'"))?;
        let I = I
            .ok_or_else(|| to_err(KeyError, "missing 'I'"))?;

        Ok(Self { Z, A, I })
    }
}


// ===============================================================================================
//
// Material extraction.
//
// ===============================================================================================

type MaterialContext<'a, 'py> = (&'a str, &'a Bound<'py, PyDict>, &'a MaterialsData);

impl<'a, 'py> TryFrom<MaterialContext<'a, 'py>> for Material {
    type Error = PyErr;

    fn try_from(value: MaterialContext) -> PyResult<Self> {
        let (name, data, others) = value;
        let table = others.table();
        let to_err = |kind: ErrorKind, why: &str| -> PyErr {
            let what = format!("'{}' material", name);
            Error::new(kind)
                .what(&what)
                .why(why)
                .to_err()
        };

        let mut density: Option<f64> = None;
        let mut mee: Option<f64> = None;
        let mut composition: Option<Bound<PyAny>> = None;
        for (k, v) in data.iter() {
            let k: String = k.extract()
                .map_err(|_| to_err(TypeError, "key is not a string"))?;
            match k.as_str() {
                "density" => {
                    let v: f64 = v.extract()
                        .map_err(|_| to_err(ValueError, "'density' is not a float"))?;
                    density = Some(v);
                },
                "composition" => composition = Some(v),
                "I" => {
                    let v: f64 = v.extract()
                        .map_err(|_| to_err(ValueError, "'I' is not a float"))?;
                    mee = Some(v);
                },
                _ => {
                    return Err(to_err(KeyError, &format!("invalid property '{}'", k)));
                },
            }
        }
        let density = density
            .ok_or_else(|| to_err(KeyError, "missing 'density'"))?;
        let composition = composition
            .ok_or_else(|| to_err(KeyError, "missing 'composition'"))?;

        let formula: Option<String> = composition.extract().ok();
        let material = match formula {
            Some(formula) => Material::from_formula(density, formula.as_str(), mee, table)
                .map_err(|(kind, why)| to_err(kind, &why))?,
            None => {
                let composition: Bound<PyDict> = composition.extract()
                    .map_err(|_| {
                        let tp = composition.get_type();
                        let why = format!(
                            "expected a 'dict' or a 'string' for 'composition', found a '{:?}'",
                            tp,
                        );
                        to_err(TypeError, &why)
                    })?;
                let mut components = Vec::<Component>::new();
                for (k, v) in composition {
                    let name: String = k.extract()
                        .map_err(|_| to_err(TypeError, "key is not a string"))?;
                    let weight: f64 = v.extract()
                        .map_err(|_| {
                            let why = format!("weight for '{}' is not a float", k);
                            to_err(TypeError, &why)
                        })?;
                    if weight > 0.0 {
                        components.push(Component::new(name, weight))
                    }
                }
                Material::from_composition(density, &components, others, mee)
                    .map_err(|(kind, why)| to_err(kind, &why))?
            },
        };
        Ok(material)
    }
}


// ===============================================================================================
//
// Conversion to a Namespace.
//
// ===============================================================================================

impl<'a, 'py> IntoPyObject<'py> for &'a Material {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let composition = PyTuple::new(py, &self.composition)?;
        let namespace = match self.I {
            Some(mee) => {
                Namespace::new(py, [
                    ("density", self.density.into_pyobject(py)?.into_any()),
                    ("I", mee.into_pyobject(py)?.into_any()),
                    ("composition", composition.into_any()),
                ])?
            },
            None => {
                Namespace::new(py, [
                    ("density", self.density.into_pyobject(py)?.into_any()),
                    ("composition", composition.into_any()),
                ])?
            },
        };
        Ok(namespace.into_pyobject(py)?)
    }
}

impl<'a, 'py> IntoPyObject<'py> for &'a Component {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let namespace = Namespace::new(py, [
            ("name", self.name.as_str().into_pyobject(py)?.into_any()),
            ("weight", self.weight.into_pyobject(py)?.into_any()),
        ]).unwrap();
        Ok(namespace.into_pyobject(py)?)
    }
}
