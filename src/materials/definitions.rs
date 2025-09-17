use crate::utils::error::Error;
use crate::utils::error::ErrorKind::{self, KeyError, TypeError, ValueError};
use crate::utils::traits::TypeName;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use ordered_float::OrderedFloat;
use regex::Regex;
use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
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

#[derive(Clone, IntoPyObject, Hash, PartialEq)]
pub enum Material {
    Composite(Composite),
    Mixture(Mixture),
}

#[allow(non_snake_case)]
#[derive(Clone, Debug)]
#[pyclass(module="mulder.materials", frozen)]
pub struct Mixture {
    /// The mixture density, in kg/m3.
    #[pyo3(get)]
    pub density: f64,

    /// The mixture Mean Excitation Energy, in GeV.
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

#[derive(Clone, Debug)]
#[pyclass(module="mulder.Composite", frozen, mapping)]
pub struct Composite (Arc<RwLock<CompositeData>>);

#[derive(Debug)]
pub struct CompositeData {
    pub composition: Vec<Component>,
}

struct Composition(Vec<Component>);  // for parsing.

#[pymethods]
impl Element {
    #[new]
    #[pyo3(signature=(symbol, /, **kwargs))]
    fn __new__(py: Python, symbol: &str, kwargs: Option<&Bound<PyDict>>) -> PyResult<Self> {
        let registry = &Registry::get(py)?;

        if let Some(kwargs) = kwargs {
            let element: Element = (symbol, kwargs).try_into()?;
            registry.write().unwrap().add_element(symbol.to_owned(), element)?;
        }

        let element = registry.read().unwrap()
            .get_element(symbol)?
            .clone();
        Ok(element)
    }

    fn __repr__(&self) -> String {
        format!("{{'Z': {}, 'A': {}, 'I': {:.10}}}", self.Z, self.A, self.I * 1E-09)
    }

    /// The element Mean Excitation Energy, in GeV.
    #[allow(non_snake_case)]
    #[getter]
    fn get_I(&self) -> f64 {
        self.I * 1E-09
    }
}

#[pymethods]
impl Mixture {
    #[new]
    #[pyo3(signature=(name, /, **kwargs))]
    fn __new__(py: Python, name: &str, kwargs: Option<&Bound<PyDict>>) -> PyResult<Self> {
        let registry = &Registry::get(py)?;

        if let Some(kwargs) = kwargs {
            let mixture: Mixture = (name, kwargs, &*registry.read().unwrap()).try_into()?;
            registry.write().unwrap().add_material(name.to_owned(), Material::Mixture(mixture))?;
        }

        let material = registry.read().unwrap()
            .get_mixture(name)?
            .clone();
        Ok(material)
    }

    fn __repr__(&self) -> String {
        let composition = self.composition.iter()
            .map(|c| format!("'{}': {}", c.name, c.weight))
            .collect::<Vec<_>>()
            .join(", ");
        let mut attributes = vec![
            format!("'density': {}", self.density),
        ];
        if let Some(mee) = self.I {
            attributes.push(format!("'I': {}", mee));
        }
        attributes.push(format!("'composition': {{{}}}", composition));
        format!("{{{}}}", attributes.join(", "))
    }

    /// The mixture mass composition.
    #[getter]
    fn get_composition<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        let composition = self.composition.iter().map(|c| (c.name.clone(), c.weight));
        PyTuple::new(py, composition)
    }
}

#[pymethods]
impl Composite {
    #[new]
    #[pyo3(signature=(name, /, **kwargs))]
    fn __new__(py: Python, name: &str, kwargs: Option<&Bound<PyDict>>) -> PyResult<Self> {
        let registry = &Registry::get(py)?;

        if let Some(kwargs) = kwargs {
            let composite: Composite = (name, kwargs, &*registry.read().unwrap()).try_into()?;
            registry.write().unwrap().add_material(
                name.to_owned(), Material::Composite(composite)
            )?;
        }

        let material = registry.read().unwrap()
            .get_composite(name)?
            .clone();
        Ok(material)
    }

    fn __repr__(&self) -> String {
        let data = self.read();
        let composition = data.composition.iter()
            .map(|c| format!("'{}'", c.name))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{{composition: {{{}}}}}", composition)
    }

    fn __getitem__(&self, key: String) -> PyResult<f64> {
        let data = self.read();
        for Component { name, weight } in data.composition.iter() {
            if key.eq(name) {
                return Ok(*weight)
            }
        }
        let err = Error::new(KeyError).why(&key).to_err();
        Err(err)
    }

    fn __setitem__(&self, key: String, value: f64) -> PyResult<()> {
        let mut data = self.write();
        for Component { name, weight } in data.composition.iter_mut() {
            if key.eq(name) {
                *weight = value;
                return Ok(())
            }
        }
        let err = Error::new(KeyError).why(&key).to_err();
        Err(err)
    }

    /// The composite content.
    #[getter]
    fn get_composition<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        let data = self.read();
        let composition = data.composition.iter().map(|c| c.name.clone());
        PyTuple::new(py, composition)
    }

    /// The composite density, in kg/m3.
    #[getter]
    fn get_density<'py>(&self, py: Python<'py>) -> PyResult<f64> {
        let registry = Registry::get(py)?.read().unwrap();
        let mut density = 0.0;
        let mut sum = 0.0;
        let data = self.read();
        for component in data.composition.iter() {
            let Component { name, weight } = component;
            let mixture = registry.get_mixture(name.as_str()).unwrap();
            density += *weight * mixture.density; // XXX Check this.
            sum += *weight;
        }
        Ok(density / sum)
    }
}

impl PartialEq for Composite {
    fn eq(&self, rhs: &Self) -> bool {
        if Arc::ptr_eq(&self.0, &rhs.0) {
            return true
        }
        let lhs = self.read();
        let rhs = rhs.read();
        if lhs.composition.len() != rhs.composition.len() {
            false
        } else {
            let mut lhs = lhs.composition.iter().map(|c| c.name.as_str()).collect::<Vec<_>>();
            lhs.sort();
            let mut rhs = rhs.composition.iter().map(|c| c.name.as_str()).collect::<Vec<_>>();
            rhs.sort();
            lhs.eq(&rhs)
        }
    }
}

impl PartialEq for Mixture {
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

impl Composite {
    pub fn new(mut composition: Vec<Component>, registry: &Registry) -> Result<Self, ErrorData> {
        for Component { name, .. } in composition.iter() {
            let _ = registry.get_mixture(name.as_str())
                .map_err(|err| (ValueError, err.to_string()))?;
        }
        composition.sort_by(|a,b| a.name.cmp(&b.name));
        let data = CompositeData { composition };
        Ok(Self(Arc::new(RwLock::new(data))))
    }

    #[inline]
    pub fn read(&self) -> RwLockReadGuard<'_, CompositeData> {
        self.0.read().unwrap()
    }

    #[inline]
    pub fn write(&self) -> RwLockWriteGuard<'_, CompositeData> {
        self.0.write().unwrap()
    }
}

impl Mixture {
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
                    let mixture: Option<Cow<Mixture>> = registry.materials.get(name.as_str())
                        .and_then(|material| material.as_mixture())
                        .map(|mixture| Cow::Borrowed(mixture))
                        .or_else(||
                            Self::from_formula(0.0, name.as_str(), None, registry)
                                .ok()
                                .map(|material| Cow::Owned(material))
                        );

                    match mixture {
                        Some(mixture) => {
                            let xi = wi / mixture.mass;
                            for cj in mixture.composition.iter() {
                                let Component { name: symbol, weight: wj } = cj;
                                let (symbol, element) = registry.elements
                                    .get_key_value(symbol.as_str()).unwrap();
                                let xj = wj / element.A * mixture.mass;
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
                                "unknown element, molecule or mixture '{}'",
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
        Mixture::from_elements(density, &composition, I, registry)
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

impl<'a, 'py> TryFrom<MaterialContext<'a, 'py>> for Composite {
    type Error = PyErr;

    fn try_from(value: MaterialContext) -> PyResult<Self> {
        let (name, data, registry) = value;
        let to_err = |kind: ErrorKind, why: &str| -> PyErr {
            let what = format!("'{}' composite", name);
            Error::new(kind)
                .what(&what)
                .why(why)
                .to_err()
        };

        let mut composition: Option<Bound<PyAny>> = None;
        for (k, v) in data.iter() {
            let k: String = k.extract()
                .map_err(|_| to_err(TypeError, "key is not a string"))?;
            match k.as_str() {
                "composition" => composition = Some(v),
                _ => {
                    return Err(to_err(KeyError, &format!("invalid property '{}'", k)));
                }
            }
        }
        let composition = composition
            .ok_or_else(|| to_err(KeyError, "missing 'composition'"))?;
        let composition: Bound<PyDict> = composition.extract() // XXX Try from a sequence?
            .map_err(|_| {
                let why = format!(
                    "expected a 'dict' for 'composition', found a '{:?}'",
                    composition.as_any().type_name(),
                );
                to_err(TypeError, &why)
            })?;
        let composition = Composition::try_from(&composition)
            .map_err(|(kind, why)| to_err(kind, &why))?;
        Composite::new(composition.0, registry)
            .map_err(|(kind, why)| to_err(kind, &why))
    }
}

impl<'py> TryFrom<&Bound<'py, PyDict>> for Composition {
    type Error = ErrorData;

    fn try_from(value: &Bound<'py, PyDict>) -> Result<Self, Self::Error> {
        let mut composition = Vec::<Component>::new();
        for (k, v) in value {
            let name: String = k.extract()
                .map_err(|_| (TypeError, "key is not a string".to_owned()))?;
            let weight: f64 = v.extract()
                .map_err(|_| {
                    let why = format!("weight for '{}' is not a float", k);
                    (TypeError, why)
                })?;
            if weight > 0.0 {
                composition.push(Component { name, weight })
            }
        }
        Ok(Self(composition))
    }
}

impl<'a, 'py> TryFrom<MaterialContext<'a, 'py>> for Material {
    type Error = PyErr;

    fn try_from(value: MaterialContext) -> PyResult<Self> {
        let (_, data, _) = &value;
        let material = if data.contains("density")? {
            let mixture = Mixture::try_from(value)?;
            Material::Mixture(mixture)
        } else {
            let composite = Composite::try_from(value)?;
            Material::Composite(composite)
        };
        Ok(material)
    }
}

impl<'a, 'py> TryFrom<MaterialContext<'a, 'py>> for Mixture {
    type Error = PyErr;

    fn try_from(value: MaterialContext) -> PyResult<Self> {
        let (name, data, registry) = value;
        let to_err = |kind: ErrorKind, why: &str| -> PyErr {
            let what = format!("'{}' mixture", name);
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
        let mixture = match formula {
            Some(formula) => Mixture::from_formula(density, formula.as_str(), mee, registry)
                .map_err(|(kind, why)| to_err(kind, &why))?,
            None => {
                let composition: Bound<PyDict> = composition.extract() // XXX Try from a sequence?
                    .map_err(|_| {
                        let why = format!(
                            "expected a 'dict' or a 'string' for 'composition', found a '{:?}'",
                            composition.as_any().type_name(),
                        );
                        to_err(TypeError, &why)
                    })?;
                let composition = Composition::try_from(&composition)
                    .map_err(|(kind, why)| to_err(kind, &why))?;
                Mixture::from_composition(density, &composition.0, mee, registry)
                    .map_err(|(kind, why)| to_err(kind, &why))?
            },
        };
        Ok(mixture)
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

impl Hash for Composite {
    fn hash<H>(&self, state: &mut H)
       where H: Hasher
    {
        let data = self.read();
        let components = data.composition.iter().map(|c| c.name.as_str()).collect::<Vec<_>>();
        components.hash(state)
    }
}

impl Hash for Mixture {
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

impl Material {
    pub fn as_composite(&self) -> Option<&Composite> {
        match self {
            Self::Composite(composite) => Some(composite),
            _ => None,
        }
    }

    pub fn as_mixture(&self) -> Option<&Mixture> {
        match self {
            Self::Mixture(mixture) => Some(mixture),
            _ => None,
        }
    }
}
