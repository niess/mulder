use crate::utils::error::Error;
use crate::utils::error::ErrorKind::{self, KeyError, TypeError, ValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use ordered_float::OrderedFloat;
use regex::Regex;
use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use super::registry::Registry;


type ElementContext<'a, 'py> = (&'a str, &'a Bound<'py, PyDict>);
type ErrorData = (ErrorKind, String);
type MaterialContext<'a, 'py> = (&'a str, &'a Bound<'py, PyDict>, &'a Registry);

#[allow(non_snake_case)]
#[derive(Clone, Debug, PartialEq)]
#[pyclass(module="mulder.materials", frozen)]
pub struct Element {
    /// The element atomic number.
    #[pyo3(get)]
    pub Z: u32,

    /// The element mass number, in g/mol.
    #[pyo3(get)]
    pub A: f64,

    pub I: f64,  // Beware: eV.
}

#[allow(non_snake_case)]
#[derive(Clone, Debug)]
#[pyclass(module="mulder.materials", frozen)]
pub struct Material {
    /// The material density, in kg/m3.
    #[pyo3(get)]
    pub density: f64,

    /// The material Mean Excitation Energy, in GeV.
    #[pyo3(get)]
    pub I: Option<f64>,

    pub composition: Vec<Component>, // Beware: mass fractions.
    mass: f64,
}

#[derive(Clone, Debug)]
pub struct Component {
    pub name: String,
    pub weight: f64,
}

#[pymethods]
impl Element {
    #[new]
    #[pyo3(signature=(symbol, /, **kwargs))]
    fn __new__(py: Python, symbol: &str, kwargs: Option<&Bound<PyDict>>) -> PyResult<Self> {
        let registry = &Registry::get(py)?;

        if let Some(kwargs) = kwargs { // XXX explicit 'define' function?
            let element: Element = (symbol, kwargs).try_into()?;
            registry.write().unwrap().add_element(symbol.to_owned(), element)?;
        }

        let element = registry.read().unwrap()
            .get_element(symbol)?
            .clone();
        Ok(element)
    }

    /// The element Mean Excitation Energy, in GeV.
    #[allow(non_snake_case)]
    #[getter]
    fn get_I(&self) -> f64 {
        self.I * 1E-09
    }
}

#[pymethods]
impl Material {
    #[new]
    #[pyo3(signature=(name, /, **kwargs))]
    fn __new__(py: Python, name: &str, kwargs: Option<&Bound<PyDict>>) -> PyResult<Self> {
        let registry = &Registry::get(py)?;

        if let Some(kwargs) = kwargs {
            let material: Material = (name, kwargs, &*registry.read().unwrap()).try_into()?;
            registry.write().unwrap().add_material(name.to_owned(), material)?;
        }

        let material = registry.read().unwrap()
            .get_material(name)?
            .clone();
        Ok(material)
    }

    /// The material mass composition.
    #[getter]
    fn get_composition<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        let composition = self.composition.iter().map(|c| (c.name.clone(), c.weight));
        PyTuple::new(py, composition)
    }
}

impl PartialEq for Material {
    fn eq(&self, other: &Self) -> bool {
        if self.density != other.density {
            return false;
        }
        if (self.mass - other.mass).abs() > 1E-09 { // Prevent rounding errors.
            return false;
        }
        if self.I != other.I {
            return false;
        }
        self.composition.eq(&other.composition)
    }
}

impl PartialEq for Component {
    fn eq(&self, other: &Self) -> bool {
        if self.name.eq(&other.name) {
            (self.weight - other.weight).abs() <= 1E-09 // Prevent rounding errors.
        } else {
            false
        }
    }
}

impl Material {
    pub fn from_composition(
        density: f64,
        composition: &[Component], // Beware: mass fractions.
        #[allow(non_snake_case)]
        I: Option<f64>,
        registry: &Registry,
    ) -> Result<Self, ErrorData> {
        let mut weights = HashMap::<&str, f64>::new();
        let mut sum = 0.0;
        for Component { name, weight: wi } in composition.iter() {
            let xi = match registry.elements.get(name.as_str()) {
                Some(element) => {
                    let xi = wi / element.A;
                    weights
                        .entry(name.as_ref())
                        .and_modify(|x| *x += xi)
                        .or_insert(xi);
                    xi
                },
                None => {
                    let material: Option<Cow<Material>> = registry.materials.get(name.as_str())
                        .map(|material| Cow::Borrowed(material))
                        .or_else(||
                            Self::from_formula(0.0, name.as_str(), None, registry)
                                .ok()
                                .map(|material| Cow::Owned(material))
                        );

                    match material {
                        Some(material) => {
                            let xi = wi / material.mass;
                            for cj in material.composition.iter() {
                                let Component { name: symbol, weight: wj } = cj;
                                let (symbol, element) = registry.elements
                                    .get_key_value(symbol.as_str()).unwrap();
                                let xj = wj / element.A * material.mass;
                                let xij = xi * xj;
                                weights
                                    .entry(symbol)
                                    .and_modify(|x| *x += xij)
                                    .or_insert(xij);
                            }
                            xi
                        },
                        None => {
                            let why = format!(
                                "undefined element, material or molecule '{}'",
                                name.as_str(),
                            );
                            return Err((KeyError, why))
                        },
                    }
                },
            };
            sum += xi;
        }
        let n = composition.len();
        let mut composition = Vec::<Component>::with_capacity(n);
        for (symbol, weight) in weights.iter() {
            composition.push(Component { name: symbol.to_string(), weight: weight / sum })
        }
        Self::from_elements(density, &composition, I, registry)
    }
    pub fn from_elements(
        density: f64,
        composition: &[Component], // Beware: mole fractions.
        #[allow(non_snake_case)]
        I: Option<f64>,
        registry: &Registry,
    ) -> Result<Self, ErrorData> {
        let n = composition.len();
        let mut mass = 0.0;
        let mut mass_composition = Vec::<Component>::with_capacity(n);
        for component in composition.iter() {
            let element = registry.elements.get(&component.name)
                .ok_or_else(|| {
                    let why = format!("unkown element '{}'", &component.name);
                    (KeyError, why)
                })?;
            let weight = component.weight * element.A;
            mass_composition.push(Component { name: component.name.clone(), weight });
            mass += weight;
        }
        for i in 0..n {
            mass_composition[i].weight /= mass;
        }

        let material = Self::new(
            density,
            mass,
            mass_composition,
            I,
        );
        Ok(material)
    }

    pub fn from_formula(
        density: f64,
        formula: &str,
        #[allow(non_snake_case)]
        I: Option<f64>,
        registry: &Registry,
    ) -> Result<Self, ErrorData> {
        let re = Regex::new(r"([A-Z][a-z]?)([0-9]*)").unwrap();
        let mut composition = Vec::<Component>::new();
        let mut sum = 0.0;
        for captures in re.captures_iter(formula) {
            let symbol = captures.get(1).unwrap().as_str();
            if !registry.elements.contains_key(symbol) {
                let why = format!("undefined element '{}'", symbol);
                return Err((KeyError, why))
            }
            let weight = captures.get(2).unwrap().as_str();
            let weight: f64 = if weight.len() == 0 {
                1.0
            } else {
                weight.parse::<f64>()
                    .map_err(|_| {
                        let why = format!(
                            "could not parse weight ('{}') for '{}'",
                            weight,
                            symbol,
                        );
                        (ValueError, why)
                    })?
            };
            composition.push(Component { name: symbol.to_string(), weight });
            sum += weight;
        }
        if (sum - 1.0).abs() > 1E-09 {
            for component in composition.iter_mut() {
                component.weight /= sum;
            }
        }
        Material::from_elements(density, &composition, I, registry)
    }

    #[allow(non_snake_case)]
    fn new(density: f64, mass:f64, mut composition: Vec<Component>, I: Option<f64>) -> Self {
        composition.sort_by(|a,b| a.name.cmp(&b.name));
        Self { density, mass, composition, I }
    }
}

impl<'a, 'py> TryFrom<ElementContext<'a, 'py>> for Element {
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

impl<'a, 'py> TryFrom<MaterialContext<'a, 'py>> for Material {
    type Error = PyErr;

    fn try_from(value: MaterialContext) -> PyResult<Self> {
        let (name, data, registry) = value;
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
            .ok_or_else(|| to_err(KeyError, "missing 'composition'"))?; // XXX Default to name?

        let formula: Option<String> = composition.extract().ok();
        let material = match formula {
            Some(formula) => Material::from_formula(density, formula.as_str(), mee, registry)
                .map_err(|(kind, why)| to_err(kind, &why))?,
            None => {
                let composition: Bound<PyDict> = composition.extract() // XXX Try from a sequence?
                    .map_err(|_| {
                        let tp = composition.get_type();
                        let why = format!(
                            "expected a 'dict' or a 'string' for 'composition', found a '{:?}'",
                            tp,
                        ); // XXX tp prints out as <class 'T'>, instead of 'T'.
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
                        components.push(Component { name, weight })
                    }
                }
                Material::from_composition(density, &components, mee, registry)
                    .map_err(|(kind, why)| to_err(kind, &why))?
            },
        };
        Ok(material)
    }
}

impl Hash for Element {
    #[allow(non_snake_case)]
    fn hash<H>(&self, state: &mut H)
       where H: Hasher
    {
        let Self { Z, A, I } = self;  // ensure that no attribute is ommitted.
        Z.hash(state);
        let A: &OrderedFloat<f64> = unsafe { std::mem::transmute(A) };
        A.hash(state);
        let I: &OrderedFloat<f64> = unsafe { std::mem::transmute(I) };
        I.hash(state);
    }
}

impl Hash for Material {
    #[allow(non_snake_case)]
    fn hash<H>(&self, state: &mut H)
       where H: Hasher
    {
        let Self { density, I, mass, composition } = self;  // ensure that no attribute is ommitted.
        let density: &OrderedFloat<f64> = unsafe { std::mem::transmute(density) };
        density.hash(state);
        if let Some(I) = I.as_ref() {
            let I: &OrderedFloat<f64> = unsafe { std::mem::transmute(I) };
            I.hash(state);
        }
        let mass: &OrderedFloat<f64> = unsafe { std::mem::transmute(mass) };
        mass.hash(state);
        composition.hash(state);
    }
}

impl Hash for Component {
    fn hash<H>(&self, state: &mut H)
       where H: Hasher
    {
        let Self { name, weight } = self;  // ensure that no attribute is ommitted.
        name.hash(state);
        let weight: &OrderedFloat<f64> = unsafe { std::mem::transmute(weight) };
        weight.hash(state);
    }
}
