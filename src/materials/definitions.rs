use crate::utils::error::Error;
use crate::utils::error::ErrorKind::{self, Exception, KeyError, TypeError, ValueError};
use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple, PyType};
use ordered_float::OrderedFloat;
use regex::Regex;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use super::registry::{MaterialsBroker, Registry};


type ElementContext<'a, 'py> = (&'a str, &'a Bound<'py, PyDict>);
type ErrorData = (ErrorKind, String);
type MaterialContext<'a, 'py> = (&'a str, &'a Bound<'py, PyDict>, &'a MaterialsBroker<'a, 'py>);

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

    /// The element Mean Excitation Energy, in GeV.
    #[pyo3(get)]
    pub I: f64,
}

#[derive(Clone, IntoPyObject, Hash, PartialEq)]
pub enum Material {
    Composite(Composite),
    Mixture(Mixture),
}

#[allow(non_snake_case)]
#[derive(Clone, Debug)]
#[pyclass(module="mulder.materials", name="Material", frozen)]
pub struct Mixture {
    /// The material density, in kg/m3.
    #[pyo3(get)]
    pub density: f64,

    /// The material Mean Excitation Energy, in GeV.
    #[pyo3(get)]
    pub I: Option<f64>,

    pub composition: Vec<Component>, // Beware: mass fractions.
    mass: f64,
}

#[derive(Clone, Debug, FromPyObject)]
pub struct Component {
    #[pyo3(item(0))]
    pub name: String,
    #[pyo3(item(1))]
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

#[derive(FromPyObject)]
enum MixtureComposition {
    #[pyo3(annotation="str")]
    Formula(String),

    #[pyo3(annotation="dict[str,float] | seq[(str,float)]")]
    Composition(Composition),
}

#[derive(FromPyObject)]
enum CompositeComposition {
    #[pyo3(annotation="seq[str]")]
    Names(Vec<String>),

    #[pyo3(annotation="dict[str,float] | seq[(str,float)]")]
    Composition(Composition),
}

#[pymethods]
impl Element {
    #[new]
    #[pyo3(signature=(symbol, /))]
    fn __new__(py: Python, symbol: &str) -> PyResult<Self> {
        let broker = MaterialsBroker::new(py)?;
        let element = broker.get_element(symbol)?;
        Ok(element)
    }

    fn __repr__(&self) -> String {
        format!("{{'Z': {}, 'A': {}, 'I': {}}}", self.Z, self.A, self.I)
    }

    /// Returns all currently defined elements.
    #[classmethod]
    fn all<'py>(cls: &Bound<'py, PyType>) -> PyResult<Bound<'py, PyDict>> {
        let py = cls.py();
        let registry = &Registry::get(py)?.read().unwrap();
        let elements = PyDict::new(py);
        for (k, v) in registry.elements().iter() {
            elements.set_item(k.clone(), v.clone())?;
        }
        Ok(elements)
    }

    /// Defines a new element.
    #[allow(non_snake_case)]
    #[pyo3(signature=(symbol, /, *, Z, A, I=None))]
    #[classmethod]
    fn define(
        cls: &Bound<'_, PyType>,
        symbol: String,
        Z: u32,
        A: f64,
        I: Option<f64>,
    ) -> PyResult<Self> {
        let py = cls.py();
        let I = I
            .or_else(|| {
                let binding = &*Registry::DEFAULT_ELEMENTS;
                let mut elements = binding.values();
                loop {
                    match elements.next() {
                        Some(element) => if Z == element.Z {
                            break Some(element.I)
                        }
                        None => break None,
                    }
                }
            })
            .ok_or_else(|| {
                Error::new(ValueError).what("element").why("could not infer I").to_err()
            })?;
        let element = Self { Z, A, I };
        Registry::get(py)?.write().unwrap().add_element(symbol, element.clone())?;
        Ok(element)
    }
}

#[pymethods]
impl Mixture {
    #[new]
    #[pyo3(signature=(name, /))]
    fn __new__(py: Python, name: &str) -> PyResult<Self> {
        let broker = MaterialsBroker::new(py)?;
        let mixture = broker.get_mixture(name)?;
        Ok(mixture)
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

    /// Returns all currently defined materials.
    #[classmethod]
    fn all<'py>(
        cls: &Bound<'py, PyType>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let py = cls.py();
        let registry = &Registry::get(py)?.read().unwrap();
        let mixtures = PyDict::new(py);
        for (k, v) in registry.materials().iter() {
            if let Some(v) = v.as_mixture() {
                mixtures.set_item(k.clone(), v.clone())?;
            }
        }
        Ok(mixtures)
    }

    /// Defines a new base material.
    #[allow(non_snake_case)]
    #[pyo3(signature=(name, /, *, composition, density, I=None))]
    #[classmethod]
    fn define(
        cls: &Bound<'_, PyType>,
        name: String,
        composition: MixtureComposition,
        density: f64,
        I: Option<f64>,
    ) -> PyResult<Self> {
        let py = cls.py();
        let broker = MaterialsBroker::new(py)?;
        let mixture = composition
            .into_mixture(density, I, &broker)
            .map_err(|(kind, why)| {
                let what = format!("'{}' material", name);
                Error::new(kind).what(&what).why(&why).to_err()
            })?;
        broker.registry.write().unwrap()
            .add_material(name.to_owned(), Material::Mixture(mixture.clone()))?;
        Ok(mixture)
    }

    /// The material mass composition.
    #[getter]
    fn get_composition<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        let composition = self.composition.iter().map(|c| (c.name.clone(), c.weight));
        PyTuple::new(py, composition)
    }
}

#[pymethods]
impl Composite {
    #[new]
    #[pyo3(signature=(name, /))]
    fn __new__(py: Python, name: &str) -> PyResult<Self> {
        let broker = MaterialsBroker::new(py)?;
        let composite = broker.get_composite(name)?;
        Ok(composite)
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
    pub(crate) fn get_density<'py>(&self, py: Python<'py>) -> PyResult<f64> {
        let registry = Registry::get(py)?.read().unwrap();
        let mut inverse_density = 0.0;
        let mut sum = 0.0;
        let data = self.read();
        for component in data.composition.iter() {
            let Component { name, weight } = component;
            let mixture = registry.mixture(name.as_str());
            inverse_density += *weight / mixture.density;
            sum += *weight;
        }
        Ok(sum / inverse_density)
    }

    /// Returns all currently defined composites.
    #[classmethod]
    fn all<'py>(
        cls: &Bound<'py, PyType>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let py = cls.py();
        let registry = &Registry::get(py)?.read().unwrap();
        let composites = PyDict::new(py);
        for (k, v) in registry.materials().iter() {
            if let Some(v) = v.as_composite() {
                composites.set_item(k.clone(), v.clone())?;
            }
        }
        Ok(composites)
    }

    /// Defines a new composite material.
    #[pyo3(signature=(name, /, *, composition))]
    #[classmethod]
    fn define(
        cls: &Bound<'_, PyType>,
        name: String,
        composition: CompositeComposition,
    ) -> PyResult<Self> {
        let py = cls.py();
        let broker = MaterialsBroker::new(py)?;
        let composite = composition
            .into_composite(&broker)
            .map_err(|(kind, why)| {
                let what = format!("'{}' composite", name);
                Error::new(kind).what(&what).why(&why).to_err()
            })?;
        broker.registry.write().unwrap()
            .add_material(name.to_owned(), Material::Composite(composite.clone()))?;
        Ok(composite)
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

const EPSILON: f64 = 1E-09;  // Prevent rounding errors.

impl PartialEq for Mixture {
    fn eq(&self, other: &Self) -> bool {
        if (self.density - other.density).abs() > EPSILON {
            return false
        }
        if (self.mass - other.mass).abs() > EPSILON {
            return false
        }
        match self.I {
            Some(i) => match other.I {
                Some(j) => if (i - j).abs() > EPSILON {
                    return false
                }
                None => return false,
            },
            None => match other.I {
                Some(_) => return false,
                None => (),
            },
        }
        self.composition.eq(&other.composition)
    }
}

impl PartialEq for Component {
    fn eq(&self, other: &Self) -> bool {
        if self.name.eq(&other.name) {
            (self.weight - other.weight).abs() <= EPSILON
        } else {
            false
        }
    }
}

impl Composite {
    pub fn new(
        mut composition: Vec<Component>,
        broker: &MaterialsBroker,
    ) -> Result<Self, ErrorData> {
        for Component { name, .. } in composition.iter() {
            let material = broker
                .get_material(name.as_str())
                .map_err(|err| (ValueError, err.to_string()))?;
            if material.is_composite() {
                let why = format!("{} is a composite material", name);
                return Err((TypeError, why))
            }
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
        broker: &MaterialsBroker,
    ) -> Result<Self, ErrorData> {
        let mut weights = IndexMap::<String, f64>::new();
        let mut sum = 0.0;
        for Component { name, weight: wi } in composition.iter() {
            let xi = match broker.get_element_opt(name.as_str())
                .map_err(to_error_data)? {
                Some(element) => {
                    let xi = wi / element.A;
                    weights
                        .entry(name.clone())
                        .and_modify(|x| *x += xi)
                        .or_insert(xi);
                    xi
                },
                None => {
                    let mixture: Option<Mixture> = broker.get_material_opt(name.as_str())
                        .map_err(to_error_data)?
                        .and_then(|material| material.into_mixture())
                        .or_else(|| Self::from_formula(0.0, name.as_str(), None, broker).ok());

                    match mixture {
                        Some(mixture) => {
                            let registry = &broker.registry.read().unwrap();
                            let xi = wi / mixture.mass;
                            for cj in mixture.composition {
                                let Component { name: symbol, weight: wj } = cj;
                                let element = registry.elements()
                                    .get(symbol.as_str()).unwrap();
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
                                "unknown element or material '{}'",
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
        Self::from_elements(density, &composition, I, broker)
    }

    pub fn from_elements(
        density: f64,
        composition: &[Component], // Beware: mole fractions.
        #[allow(non_snake_case)]
        I: Option<f64>,
        broker: &MaterialsBroker,
    ) -> Result<Self, ErrorData> {
        let n = composition.len();
        let mut mass = 0.0;
        let mut mass_composition = Vec::<Component>::with_capacity(n);
        for component in composition.iter() {
            let element = broker.get_element_opt(&component.name)
                .map_err(to_error_data)?
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
        broker: &MaterialsBroker,
    ) -> Result<Self, ErrorData> {
        let re = Regex::new(r"([A-Z][a-z]?)([0-9]*)").unwrap();
        let mut composition = Vec::<Component>::new();
        let mut sum = 0.0;
        for captures in re.captures_iter(formula) {
            let symbol = captures.get(1).unwrap().as_str();
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
        Mixture::from_elements(density, &composition, I, broker)
    }

    #[allow(non_snake_case)]
    fn new(density: f64, mass:f64, mut composition: Vec<Component>, I: Option<f64>) -> Self {
        composition.retain(|c| c.weight > 0.0);
        composition.sort_by(|a,b| a.name.cmp(&b.name));
        Self { density, mass, composition, I }
    }
}

impl<'a, 'py> TryFrom<ElementContext<'a, 'py>> for Element {
    type Error = PyErr;

    #[allow(non_snake_case)]
    fn try_from(value: ElementContext) -> PyResult<Self> {
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
                    I = Some(v);
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
        let (name, data, broker) = value;
        let py = data.py();
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

        let composition: CompositeComposition = composition.extract()
            .map_err(|err| to_err(TypeError, &err.value(py).to_string()))?;

        composition.into_composite(broker)
            .map_err(|(kind, why)| to_err(kind, &why))
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
        let (name, data, broker) = value;
        let py = data.py();
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

        let composition: MixtureComposition = composition.extract()
            .map_err(|err| to_err(TypeError, &err.value(py).to_string()))?;

        composition
            .into_mixture(density, mee, broker)
            .map_err(|(kind, why)| to_err(kind, &why))
    }
}

impl FromPyObject<'_> for Composition {
    fn extract_bound(obj: &Bound<'_, PyAny>) -> PyResult<Self> {
        #[derive(FromPyObject)]
        enum CompositionArg {
            #[pyo3(annotation="dict[str,float]")]
            Dict(IndexMap<String, f64>),

            #[pyo3(annotation="seq[(str,float)]")]
            Sequence(Vec<Component>),
        }

        let arg: CompositionArg = obj.extract()?;
        let composition = match arg {
            CompositionArg::Dict(d) => {
                let mut composition = Vec::<Component>::with_capacity(d.len());
                for (name, weight) in d.into_iter() {
                    composition.push(Component { name, weight })
                }
                composition
            },
            CompositionArg::Sequence(s) => s,
        };
        Ok(Self(composition))
    }
}

impl MixtureComposition {
    fn into_mixture(
        self,
        density: f64,
        mee: Option<f64>,
        broker: &MaterialsBroker,
    ) -> Result<Mixture, ErrorData> {
        match self {
            MixtureComposition::Formula(formula) => {
                Mixture::from_formula(density, formula.as_str(), mee, broker)
            },
            MixtureComposition::Composition(composition) => {
                Mixture::from_composition(density, &composition.0, mee, broker)
            },
        }
    }
}

impl CompositeComposition {
    fn into_composite(self, broker: &MaterialsBroker) -> Result<Composite, ErrorData> {
        let composition = match self {
            CompositeComposition::Names(composition) => if composition.len() > 0 {
                let weight = 1.0 / composition.len() as f64;
                composition.into_iter()
                    .map(|name| Component { name, weight })
                    .collect::<Vec<_>>()
            } else {
                Vec::<Component>::new()
            },
            CompositeComposition::Composition(composition) => composition.0,
        };
        Composite::new(composition, broker)
    }
}

impl Hash for Element {
    #[allow(non_snake_case)]
    fn hash<H>(&self, state: &mut H)
       where H: Hasher
    {
        let Self { Z, A, I } = self;  // ensure that no attribute is ommitted.
        Z.hash(state);
        toeps(A).hash(state);
        toeps(I).hash(state);
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
        toeps(density).hash(state);
        if let Some(I) = I.as_ref() {
            toeps(I).hash(state);
        }
        toeps(mass).hash(state);
        composition.hash(state);
    }
}

impl Hash for Component {
    fn hash<H>(&self, state: &mut H)
       where H: Hasher
    {
        let Self { name, weight } = self;  // ensure that no attribute is ommitted.
        name.hash(state);
        toeps(weight).hash(state);
    }
}

fn toeps(x: &f64) -> OrderedFloat<f64> {
    let x = (x / EPSILON).round() * EPSILON;
    OrderedFloat(x)
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

    pub fn into_composite(self) -> Option<Composite> {
        match self {
            Self::Composite(composite) => Some(composite),
            _ => None,
        }
    }

    pub fn into_mixture(self) -> Option<Mixture> {
        match self {
            Self::Mixture(mixture) => Some(mixture),
            _ => None,
        }
    }

    pub fn is_composite(&self) -> bool {
        match self {
            Self::Composite(_) => true,
            _ => false,
        }
    }
}

fn to_error_data(err: PyErr) -> ErrorData {
    (Exception, format!("{}", err))
}
