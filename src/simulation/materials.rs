use crate::utils::cache;
use crate::utils::convert::ToToml;
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::{self, KeyError, ValueError};
use crate::utils::io::{ConfigFormat, PathString, Toml};
use pyo3::prelude::*;
use pyo3::sync::GILOnceCell;
use regex::Regex;
use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::sync::{Arc, OnceLock};


// ===============================================================================================
//
// Materials interface.
//
// ===============================================================================================

pub const DEFAULT_MATERIALS: &'static str = "default";

pub fn initialise() -> PyResult<()> {
    // Initialise atomic elements data.
    ElementsTable::initialise();

    Ok(())
}

#[pyclass(frozen, module="mulder")]
#[derive(Clone)]
pub struct Materials {
    pub cache_key: String,
    pub data: Arc<MaterialsData>,
}

#[derive(FromPyObject)]
pub enum MaterialsArg<'py> {
    Materials(Bound<'py, Materials>),
    Path(PathString),
}

#[pymethods]
impl Materials {
    #[new]
    #[pyo3(signature=(path=None, /))]
    pub fn py_new(py: Python, path: Option<PathString>) -> PyResult<Self> {
        let path = match path.as_ref() {
            Some(path) => Cow::Borrowed(Path::new(&path.0)),
            None => {
                let data = Arc::clone(MaterialsData::default(py)?);
                return Ok(Self{ cache_key: DEFAULT_MATERIALS.to_string(), data })
            },
        };

        match path.extension().and_then(OsStr::to_str) {
            Some("toml") => {
                let cache_key = path
                    .file_stem()
                    .and_then(OsStr::to_str)
                    .ok_or_else(|| {
                        let stem = path.file_stem()
                            .and_then(|stem| Some(stem.to_string_lossy()))
                            .unwrap_or(path.to_string_lossy());
                        let why = format!("invalid file name '{}'", stem);
                        Error::new(ValueError)
                            .what("materials")
                            .why(&why)
                            .to_err()
                    })?;

                let data = MaterialsData::from_file(py, &path)?
                    .with_default(py)?;
                Self::new(cache_key.to_string(), data)
            },
            _ => {
                let why = format!("invalid file format '{}'", path.display());
                let err = Error::new(ValueError)
                    .what("materials")
                    .why(&why);
                Err(err.to_err())
            },
        }
    }

    fn __getitem__<'py>(&self, py: Python<'py>, name: &str) -> PyResult<Bound<'py, PyAny>> {
        let material = self.data.map.get(name)
            .ok_or_else(|| {
                let why = format!("unknown material '{}'", name);
                Error::new(KeyError)
                    .what("material")
                    .why(&why)
                    .to_err()
            })?;
        Ok(material.into_pyobject(py)?)
    }
}

impl Materials {
    pub fn default(py: Python) -> PyResult<Self> {
        Self::py_new(py, None)
    }

    pub fn from_arg<'py>(
        py: Python<'py>,
        arg: Option<MaterialsArg>
    ) -> PyResult<Self> {
        match arg {
            Some(arg) => match arg {
                MaterialsArg::Materials(materials) => Ok(materials.borrow().clone()),
                MaterialsArg::Path(materials) => Materials::py_new(py, Some(materials)),
            },
            None => Self::default(py),
        }
    }

    pub fn is_cached(&self, py: Python) -> PyResult<bool> {
        let materials_cache = cache::path()?
            .join("materials");
        let path = materials_cache
            .join(format!("{}.toml", self.cache_key));
        let cached = if path.try_exists().unwrap_or(false) {
            let cached = MaterialsData::from_file(py, &path)?;
            println!("XXX data = {:?}", cached.map == self.data.map); // XXX HERE I AM.
            println!("XXX table = {:?}", cached.table == self.data.table);
            cached == *self.data
        } else {
            false
        };
        Ok(cached)
    }

    pub fn new(cache_key: String, data: MaterialsData) -> PyResult<Self> {
        let data = Arc::new(data);
        Ok(Self { data, cache_key })
    }

    pub fn update_cache(&self) -> PyResult<()> {
        let materials_cache = cache::path()?
            .join("materials");
        match std::fs::read_dir(&materials_cache) {
            Ok(content) => {
                // Remove any cached tables.
                for entry in content {
                    if let Ok(entry) = entry {
                        if let Some(filename) = entry.file_name().to_str() {
                            if filename.starts_with(&self.cache_key) &&
                               filename.ends_with(".pumas") {
                                std::fs::remove_file(&entry.path())?;
                            }
                        }
                    }
                }
            },
            Err(_) => std::fs::create_dir_all(&materials_cache)?,
        }

        let path = materials_cache
            .join(format!("{}.toml", self.cache_key));
        std::fs::write(path, self.data.to_toml())?;
        Ok(())
    }
}


// ===============================================================================================
//
// Atomic elements.
//
// ===============================================================================================

#[allow(non_snake_case)]
#[derive(Debug, PartialEq)]
pub struct AtomicElement {
    pub Z: u32,
    pub A: f64,
    pub I: f64,
}

#[derive(Debug, PartialEq)]
pub struct ElementsTable {
    data: HashMap<String, AtomicElement>,
    default: Option<&'static Self>,
}

pub static ELEMENTS: OnceLock<ElementsTable> = OnceLock::new();

impl ElementsTable {
    pub fn contains_key(&self, symbol: &str) -> bool {
        self.data.contains_key(symbol) ||
        self.default.map(|default| default.contains_key(symbol))
            .unwrap_or(false)
    }

    pub fn get(&self, symbol: &str) -> Option<&AtomicElement> {
        self.data.get(symbol)
            .or_else(|| self.default.and_then(|default| default.get(symbol)))
    }

    pub fn get_key_value(&self, symbol: &str) -> Option<(&String, &AtomicElement)> {
        self.data.get_key_value(symbol)
            .or_else(|| self.default.and_then(|default| default.get_key_value(symbol)))
    }

    fn initialise() {
        // Data from https://pdg.lbl.gov/2024/AtomicNuclearProperties/index.html.
        let data = HashMap::from([
            ("H" .to_owned(), AtomicElement { Z: 1,   A: 1.008,   I: 19.2   }),
            ("D" .to_owned(), AtomicElement { Z: 1,   A: 2.0141,  I: 19.2   }),
            ("He".to_owned(), AtomicElement { Z: 2,   A: 4.0026,  I: 41.8   }),
            ("Li".to_owned(), AtomicElement { Z: 3,   A: 6.94,    I: 40.0   }),
            ("Be".to_owned(), AtomicElement { Z: 4,   A: 9.01218, I: 63.7   }),
            ("B" .to_owned(), AtomicElement { Z: 5,   A: 10.81,   I: 76.0   }),
            ("C" .to_owned(), AtomicElement { Z: 6,   A: 12.0107, I: 78.0   }),
            ("N" .to_owned(), AtomicElement { Z: 7,   A: 14.007,  I: 82.0   }),
            ("O" .to_owned(), AtomicElement { Z: 8,   A: 15.999,  I: 95.0   }),
            ("F" .to_owned(), AtomicElement { Z: 9,   A: 18.9984, I: 115.0  }),
            ("Ne".to_owned(), AtomicElement { Z: 10,  A: 20.1797, I: 137.0  }),
            ("Rk".to_owned(), AtomicElement { Z: 11,  A: 22.0,    I: 136.4  }), // Fictitious Rockium.
            ("Na".to_owned(), AtomicElement { Z: 11,  A: 22.9898, I: 149.0  }),
            ("Mg".to_owned(), AtomicElement { Z: 12,  A: 24.305,  I: 156.0  }),
            ("Al".to_owned(), AtomicElement { Z: 13,  A: 26.9815, I: 166.0  }),
            ("Si".to_owned(), AtomicElement { Z: 14,  A: 28.0855, I: 173.0  }),
            ("P" .to_owned(), AtomicElement { Z: 15,  A: 30.9738, I: 173.0  }),
            ("S" .to_owned(), AtomicElement { Z: 16,  A: 32.065,  I: 180.0  }),
            ("Cl".to_owned(), AtomicElement { Z: 17,  A: 35.453,  I: 174.0  }),
            ("Ar".to_owned(), AtomicElement { Z: 18,  A: 39.948,  I: 188.0  }),
            ("K" .to_owned(), AtomicElement { Z: 19,  A: 39.0983, I: 190.0  }),
            ("Ca".to_owned(), AtomicElement { Z: 20,  A: 40.078,  I: 191.0  }),
            ("Sc".to_owned(), AtomicElement { Z: 21,  A: 44.9559, I: 216.0  }),
            ("Ti".to_owned(), AtomicElement { Z: 22,  A: 47.867,  I: 233.0  }),
            ("V" .to_owned(), AtomicElement { Z: 23,  A: 50.9415, I: 245.0  }),
            ("Cr".to_owned(), AtomicElement { Z: 24,  A: 51.9961, I: 257.0  }),
            ("Mn".to_owned(), AtomicElement { Z: 25,  A: 54.938,  I: 272.0  }),
            ("Fe".to_owned(), AtomicElement { Z: 26,  A: 55.845,  I: 286.0  }),
            ("Co".to_owned(), AtomicElement { Z: 27,  A: 58.9332, I: 297.0  }),
            ("Ni".to_owned(), AtomicElement { Z: 28,  A: 58.6934, I: 311.0  }),
            ("Cu".to_owned(), AtomicElement { Z: 29,  A: 63.546,  I: 322.0  }),
            ("Zn".to_owned(), AtomicElement { Z: 30,  A: 65.38,   I: 330.0  }),
            ("Ga".to_owned(), AtomicElement { Z: 31,  A: 69.723,  I: 334.0  }),
            ("Ge".to_owned(), AtomicElement { Z: 32,  A: 72.63,   I: 350.0  }),
            ("As".to_owned(), AtomicElement { Z: 33,  A: 74.9216, I: 347.0  }),
            ("Se".to_owned(), AtomicElement { Z: 34,  A: 78.971,  I: 348.0  }),
            ("Br".to_owned(), AtomicElement { Z: 35,  A: 79.904,  I: 357.0  }),
            ("Kr".to_owned(), AtomicElement { Z: 36,  A: 83.798,  I: 352.0  }),
            ("Rb".to_owned(), AtomicElement { Z: 37,  A: 85.4678, I: 363.0  }),
            ("Sr".to_owned(), AtomicElement { Z: 38,  A: 87.62,   I: 366.0  }),
            ("Y" .to_owned(), AtomicElement { Z: 39,  A: 88.9058, I: 379.0  }),
            ("Zr".to_owned(), AtomicElement { Z: 40,  A: 91.224,  I: 393.0  }),
            ("Nb".to_owned(), AtomicElement { Z: 41,  A: 92.9064, I: 417.0  }),
            ("Mo".to_owned(), AtomicElement { Z: 42,  A: 95.95,   I: 424.0  }),
            ("Tc".to_owned(), AtomicElement { Z: 43,  A: 97.9072, I: 428.0  }),
            ("Ru".to_owned(), AtomicElement { Z: 44,  A: 101.07,  I: 441.0  }),
            ("Rh".to_owned(), AtomicElement { Z: 45,  A: 102.906, I: 449.0  }),
            ("Pd".to_owned(), AtomicElement { Z: 46,  A: 106.42,  I: 470.0  }),
            ("Ag".to_owned(), AtomicElement { Z: 47,  A: 107.868, I: 470.0  }),
            ("Cd".to_owned(), AtomicElement { Z: 48,  A: 112.414, I: 469.0  }),
            ("In".to_owned(), AtomicElement { Z: 49,  A: 114.818, I: 488.0  }),
            ("Sn".to_owned(), AtomicElement { Z: 50,  A: 118.71,  I: 488.0  }),
            ("Sb".to_owned(), AtomicElement { Z: 51,  A: 121.76,  I: 487.0  }),
            ("Te".to_owned(), AtomicElement { Z: 52,  A: 127.6,   I: 485.0  }),
            ("I" .to_owned(), AtomicElement { Z: 53,  A: 126.904, I: 491.0  }),
            ("Xe".to_owned(), AtomicElement { Z: 54,  A: 131.293, I: 482.0  }),
            ("Cs".to_owned(), AtomicElement { Z: 55,  A: 132.905, I: 488.0  }),
            ("Ba".to_owned(), AtomicElement { Z: 56,  A: 137.327, I: 491.0  }),
            ("La".to_owned(), AtomicElement { Z: 57,  A: 138.905, I: 501.0  }),
            ("Ce".to_owned(), AtomicElement { Z: 58,  A: 140.116, I: 523.0  }),
            ("Pr".to_owned(), AtomicElement { Z: 59,  A: 140.908, I: 535.0  }),
            ("Nd".to_owned(), AtomicElement { Z: 60,  A: 144.242, I: 546.0  }),
            ("Pm".to_owned(), AtomicElement { Z: 61,  A: 144.913, I: 560.0  }),
            ("Sm".to_owned(), AtomicElement { Z: 62,  A: 150.36,  I: 574.0  }),
            ("Eu".to_owned(), AtomicElement { Z: 63,  A: 151.964, I: 580.0  }),
            ("Gd".to_owned(), AtomicElement { Z: 64,  A: 157.25,  I: 591.0  }),
            ("Tb".to_owned(), AtomicElement { Z: 65,  A: 158.925, I: 614.0  }),
            ("Dy".to_owned(), AtomicElement { Z: 66,  A: 162.5,   I: 628.0  }),
            ("Ho".to_owned(), AtomicElement { Z: 67,  A: 164.93,  I: 650.0  }),
            ("Er".to_owned(), AtomicElement { Z: 68,  A: 167.259, I: 658.0  }),
            ("Tm".to_owned(), AtomicElement { Z: 69,  A: 168.934, I: 674.0  }),
            ("Yb".to_owned(), AtomicElement { Z: 70,  A: 173.054, I: 684.0  }),
            ("Lu".to_owned(), AtomicElement { Z: 71,  A: 174.967, I: 694.0  }),
            ("Hf".to_owned(), AtomicElement { Z: 72,  A: 178.49,  I: 705.0  }),
            ("Ta".to_owned(), AtomicElement { Z: 73,  A: 180.948, I: 718.0  }),
            ("W" .to_owned(), AtomicElement { Z: 74,  A: 183.84,  I: 727.0  }),
            ("Re".to_owned(), AtomicElement { Z: 75,  A: 186.207, I: 736.0  }),
            ("Os".to_owned(), AtomicElement { Z: 76,  A: 190.23,  I: 746.0  }),
            ("Ir".to_owned(), AtomicElement { Z: 77,  A: 192.217, I: 757.0  }),
            ("Pt".to_owned(), AtomicElement { Z: 78,  A: 195.084, I: 790.0  }),
            ("Au".to_owned(), AtomicElement { Z: 79,  A: 196.967, I: 790.0  }),
            ("Hg".to_owned(), AtomicElement { Z: 80,  A: 200.592, I: 800.0  }),
            ("Tl".to_owned(), AtomicElement { Z: 81,  A: 204.38,  I: 810.0  }),
            ("Pb".to_owned(), AtomicElement { Z: 82,  A: 207.2,   I: 823.0  }),
            ("Bi".to_owned(), AtomicElement { Z: 83,  A: 208.98,  I: 823.0  }),
            ("Po".to_owned(), AtomicElement { Z: 84,  A: 208.982, I: 830.0  }),
            ("At".to_owned(), AtomicElement { Z: 85,  A: 209.987, I: 825.0  }),
            ("Rn".to_owned(), AtomicElement { Z: 86,  A: 222.018, I: 794.0  }),
            ("Fr".to_owned(), AtomicElement { Z: 87,  A: 223.02,  I: 827.0  }),
            ("Ra".to_owned(), AtomicElement { Z: 88,  A: 226.025, I: 826.0  }),
            ("Ac".to_owned(), AtomicElement { Z: 89,  A: 227.028, I: 841.0  }),
            ("Th".to_owned(), AtomicElement { Z: 90,  A: 232.038, I: 847.0  }),
            ("Pa".to_owned(), AtomicElement { Z: 91,  A: 231.036, I: 878.0  }),
            ("U" .to_owned(), AtomicElement { Z: 92,  A: 238.029, I: 890.0  }),
            ("Np".to_owned(), AtomicElement { Z: 93,  A: 237.048, I: 902.0  }),
            ("Pu".to_owned(), AtomicElement { Z: 94,  A: 244.064, I: 921.0  }),
            ("Am".to_owned(), AtomicElement { Z: 95,  A: 243.061, I: 934.0  }),
            ("Cm".to_owned(), AtomicElement { Z: 96,  A: 247.07,  I: 939.0  }),
            ("Bk".to_owned(), AtomicElement { Z: 97,  A: 247.07,  I: 952.0  }),
            ("Cf".to_owned(), AtomicElement { Z: 98,  A: 251.08,  I: 966.0  }),
            ("Es".to_owned(), AtomicElement { Z: 99,  A: 252.083, I: 980.0  }),
            ("Fm".to_owned(), AtomicElement { Z: 100, A: 257.095, I: 994.0  }),
            ("Md".to_owned(), AtomicElement { Z: 101, A: 258.098, I: 1007.0 }),
            ("No".to_owned(), AtomicElement { Z: 102, A: 259.101, I: 1020.0 }),
            ("Lr".to_owned(), AtomicElement { Z: 103, A: 262.11,  I: 1034.0 }),
            ("Rf".to_owned(), AtomicElement { Z: 104, A: 267.122, I: 1047.0 }),
            ("Db".to_owned(), AtomicElement { Z: 105, A: 268.126, I: 1061.0 }),
            ("Sg".to_owned(), AtomicElement { Z: 106, A: 269.129, I: 1074.0 }),
            ("Bh".to_owned(), AtomicElement { Z: 107, A: 270.133, I: 1087.0 }),
            ("Hs".to_owned(), AtomicElement { Z: 108, A: 269.134, I: 1102.0 }),
            ("Mt".to_owned(), AtomicElement { Z: 109, A: 278.156, I: 1115.0 }),
            ("Ds".to_owned(), AtomicElement { Z: 110, A: 281.164, I: 1129.0 }),
            ("Rg".to_owned(), AtomicElement { Z: 111, A: 282.169, I: 1143.0 }),
            ("Cn".to_owned(), AtomicElement { Z: 112, A: 285.177, I: 1156.0 }),
            ("Nh".to_owned(), AtomicElement { Z: 113, A: 286.182, I: 1171.0 }),
            ("Fl".to_owned(), AtomicElement { Z: 114, A: 289.19,  I: 1185.0 }),
            ("Mc".to_owned(), AtomicElement { Z: 115, A: 289.194, I: 1199.0 }),
            ("Lv".to_owned(), AtomicElement { Z: 116, A: 293.204, I: 1213.0 }),
            ("Ts".to_owned(), AtomicElement { Z: 117, A: 294.211, I: 1227.0 }),
            ("Og".to_owned(), AtomicElement { Z: 118, A: 294.214, I: 1242.0 }),
        ]);
        let _unused = ELEMENTS
            .set(Self { data, default: None });
    }

    pub fn insert(&mut self, key: String, value: AtomicElement) {
        self.data.insert(key, value);
    }

    pub fn empty() -> Self {
        let data = HashMap::new();
        Self::new(data)
    }

    pub fn new(data: HashMap<String, AtomicElement>) -> Self {
        let default = ELEMENTS.get();
        Self { data, default }
    }

    pub fn raw(&self) -> &HashMap<String, AtomicElement> {
        &self.data
    }
}


// ===============================================================================================
//
// Material data.
//
// ===============================================================================================

#[allow(non_snake_case)]
#[derive(Clone, Debug)]
pub struct Material {
    pub density: f64,
    mass: f64,
    pub composition: Vec<Component>, // Beware: mass fractions.
    pub I: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct Component {
    pub name: String,
    pub weight: f64,
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

type ErrorData = (ErrorKind, String);

impl Material {
    #[allow(non_snake_case)]
    pub fn new(density: f64, mass:f64, mut composition: Vec<Component>, I: Option<f64>) -> Self {
        composition.sort_by(|a,b| a.name.cmp(&b.name));
        Self { density, mass, composition, I }
    }

    pub fn from_composition(
        density: f64,
        composition: &[Component], // Beware: mass fractions.
        others: &MaterialsData,
        #[allow(non_snake_case)]
        I: Option<f64>,
    ) -> Result<Self, ErrorData> {
        let table = others.table();
        let mut weights = HashMap::<&'static str, f64>::new();
        let mut sum = 0.0;
        for Component { name, weight: wi } in composition.iter() {
            let xi = match table.get(name.as_str()) {
                Some(element) => {
                    let xi = wi / element.A;
                    weights
                        .entry(name.as_ref())
                        .and_modify(|x| *x += xi)
                        .or_insert(xi);
                    xi
                },
                None => {
                    let material: Option<Cow<Material>> = others.map.get(name.as_str())
                        .map(|material| Cow::Borrowed(material))
                        .or_else(||
                            Self::from_formula(0.0, name.as_str(), None, table)
                                .ok()
                                .map(|material| Cow::Owned(material))
                        );

                    match material {
                        Some(material) => {
                            let xi = wi / material.mass;
                            for cj in material.composition.iter() {
                                let Component { name: symbol, weight: wj } = cj;
                                let (symbol, element) = table
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
                                "unknown element, material or molecule '{}'",
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
        Self::from_elements(density, &composition, I, table)
    }

    pub fn from_elements(
        density: f64,
        composition: &[Component], // Beware: mole fractions.
        #[allow(non_snake_case)]
        I: Option<f64>,
        table: &ElementsTable,
    ) -> Result<Self, ErrorData> {
        let n = composition.len();
        let mut mass = 0.0;
        let mut mass_composition = Vec::<Component>::with_capacity(n);
        for component in composition.iter() {
            let element = table.get(&component.name)
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
        table: &ElementsTable,
    ) -> Result<Self, ErrorData> {
        let re = Regex::new(r"([A-Z][a-z]?)([0-9]*)").unwrap();
        let mut composition = Vec::<Component>::new();
        let mut sum = 0.0;
        for captures in re.captures_iter(formula) {
            let symbol = captures.get(1).unwrap().as_str();
            if !table.contains_key(symbol) {
                let why = format!("unknown element '{}'", symbol);
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
        Material::from_elements(density, &composition, I, table)
    }
}

impl Component {
    pub fn new(name: String, weight: f64) -> Self {
        Self { name, weight }
    }
}


// ===============================================================================================
//
// Data relative to a set of materials.
//
// ===============================================================================================

#[derive(Debug, PartialEq)]
pub struct MaterialsData {
    pub map: HashMap<String, Material>,
    table: Option<ElementsTable>,
}

static DEFAULT_MATERIALS_DATA: GILOnceCell<Arc<MaterialsData>> = GILOnceCell::new();

impl MaterialsData {
    pub fn empty() -> Self {
        let map = HashMap::new();
        let table = None;
        Self { map, table }
    }

    pub fn default(py: Python) -> PyResult<&Arc<Self>> {
        DEFAULT_MATERIALS_DATA.get_or_try_init(py, || -> PyResult<_> {
            let path = Path::new(crate::PREFIX.get(py).unwrap())
                .join(format!("data/materials/{}.toml", DEFAULT_MATERIALS));
            let data = Self::from_file(py, path)?;
            Ok(Arc::new(data))
        })
    }

    pub fn from_file<P: AsRef<Path>>(py: Python, path: P) -> PyResult<Self> {
        Toml::load_dict(py, path.as_ref())?
            .try_into()
    }

    pub fn new(map: HashMap<String, Material>) -> Self {
        let table = None;
        Self { map, table }
    }

    pub fn table(&self) -> &ElementsTable {
        self.table.as_ref().unwrap_or_else(|| ELEMENTS.get().unwrap())
    }

    pub fn raw_table(&self) -> Option<&HashMap<String, AtomicElement>> {
        self.table.as_ref().map(|table| table.raw())
    }

    pub fn with_default(mut self, py: Python) -> PyResult<Self> {
        let default_data = Self::default(py)?;
        for material in ["Air", "Rock", "Water"] {
            self.map.entry(material.to_string()).or_insert_with(|| {
                default_data.map[material].clone()
            });
        }
        Ok(self)
    }

    pub fn with_table(mut self, table: ElementsTable) -> Self {
        self.table = Some(table);
        self
    }
}
