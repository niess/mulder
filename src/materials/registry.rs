use crate::utils::error::Error;
use crate::utils::error::ErrorKind::{ValueError, TypeError};
use crate::utils::io::{ConfigFormat, Toml};
use crate::utils::traits::TypeName;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::sync::GILOnceCell;
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;
use super::definitions::{Composite, Element, Material, Mixture};


#[derive(Default)]
pub struct Registry {
    pub elements: HashMap<String, Element>,
    pub materials: HashMap<String, Material>,
}

static REGISTRY: GILOnceCell<RwLock<Registry>> = GILOnceCell::new();

impl Registry {
    #[inline]
    pub fn get(py: Python) -> PyResult<&'static RwLock<Self>> {
        REGISTRY.get_or_try_init(py, || Ok(RwLock::new(Self::new(py)?)))
    }

    pub fn add_element(&mut self, symbol: String, definition: Element) -> PyResult<()> {
        match self.elements.get(&symbol) {
            Some(value) => if value.ne(&definition) {
                let why = format!("'{}' already exists with a different definition", symbol);
                let err = Error::new(ValueError).what("element").why(&why).to_err();
                return Err(err)
            },
            None => {
                self.elements.insert(symbol, definition);
            },
        }
        Ok(())
    }

    pub fn add_material(&mut self, name: String, definition: Material) -> PyResult<()> {
        match self.materials.get(&name) {
            Some(value) => if value.ne(&definition) {
                let why = format!("'{}' already exists with a different definition", name);
                let err = Error::new(ValueError).what("material").why(&why).to_err();
                return Err(err)
            } else if definition.is_composite() {
                self.materials.insert(name, definition);
            },
            None => {
                self.materials.insert(name, definition);
            },
        }
        Ok(())
    }

    pub fn load<P: AsRef<Path>>(&mut self, py: Python, path: P) -> PyResult<()> {
        let to_err = |expected: &str, found: &Bound<PyAny>| {
            let why = format!("expected a '{}', found a '{}'", expected, found.type_name());
            Error::new(TypeError)
                .what("materials")
                .why(&why)
                .to_err()
        };

        let toml = Toml::load_dict(py, path.as_ref())?;

        if let Some(elements) = toml.get_item("elements")? {
            let elements = elements.downcast::<PyDict>()
                .map_err(|_| to_err("dict", &elements))?;
            for (k, v) in elements.iter() {
                let k: String = k.extract()
                    .map_err(|_| to_err("string", &k))?;
                let v: Bound<PyDict> = v.extract()
                    .map_err(|_| to_err("dict", &v))?;
                let element: Element = (k.as_str(), &v).try_into()?;
                self.add_element(k, element)?;
            }
        }

        for (k, v) in toml.iter() {
            let k: String = k.extract()
                .map_err(|_| to_err("string", &k))?;
            if k == "elements" { continue }
            let v: Bound<PyDict> = v.extract()
                .map_err(|_| to_err("dict", &v))?;
            let material: Material = (k.as_str(), &v, self as &Self).try_into()?;
            self.add_material(k, material)?;
        }

        Ok(())
    }
}

impl Registry {
    const DEFAULT_MATERIALS: &'static str = "default";

    fn new(py: Python) -> PyResult<Self> {
        let elements = HashMap::from([
            ("H" .to_owned(), Element { Z: 1,   A: 1.008,   I: 19.2   }),
            ("D" .to_owned(), Element { Z: 1,   A: 2.0141,  I: 19.2   }),
            ("He".to_owned(), Element { Z: 2,   A: 4.0026,  I: 41.8   }),
            ("Li".to_owned(), Element { Z: 3,   A: 6.94,    I: 40.0   }),
            ("Be".to_owned(), Element { Z: 4,   A: 9.01218, I: 63.7   }),
            ("B" .to_owned(), Element { Z: 5,   A: 10.81,   I: 76.0   }),
            ("C" .to_owned(), Element { Z: 6,   A: 12.0107, I: 78.0   }),
            ("N" .to_owned(), Element { Z: 7,   A: 14.007,  I: 82.0   }),
            ("O" .to_owned(), Element { Z: 8,   A: 15.999,  I: 95.0   }),
            ("F" .to_owned(), Element { Z: 9,   A: 18.9984, I: 115.0  }),
            ("Ne".to_owned(), Element { Z: 10,  A: 20.1797, I: 137.0  }),
            ("Rk".to_owned(), Element { Z: 11,  A: 22.0,    I: 136.4  }), // Fictitious Rockium.
            ("Na".to_owned(), Element { Z: 11,  A: 22.9898, I: 149.0  }),
            ("Mg".to_owned(), Element { Z: 12,  A: 24.305,  I: 156.0  }),
            ("Al".to_owned(), Element { Z: 13,  A: 26.9815, I: 166.0  }),
            ("Si".to_owned(), Element { Z: 14,  A: 28.0855, I: 173.0  }),
            ("P" .to_owned(), Element { Z: 15,  A: 30.9738, I: 173.0  }),
            ("S" .to_owned(), Element { Z: 16,  A: 32.065,  I: 180.0  }),
            ("Cl".to_owned(), Element { Z: 17,  A: 35.453,  I: 174.0  }),
            ("Ar".to_owned(), Element { Z: 18,  A: 39.948,  I: 188.0  }),
            ("K" .to_owned(), Element { Z: 19,  A: 39.0983, I: 190.0  }),
            ("Ca".to_owned(), Element { Z: 20,  A: 40.078,  I: 191.0  }),
            ("Sc".to_owned(), Element { Z: 21,  A: 44.9559, I: 216.0  }),
            ("Ti".to_owned(), Element { Z: 22,  A: 47.867,  I: 233.0  }),
            ("V" .to_owned(), Element { Z: 23,  A: 50.9415, I: 245.0  }),
            ("Cr".to_owned(), Element { Z: 24,  A: 51.9961, I: 257.0  }),
            ("Mn".to_owned(), Element { Z: 25,  A: 54.938,  I: 272.0  }),
            ("Fe".to_owned(), Element { Z: 26,  A: 55.845,  I: 286.0  }),
            ("Co".to_owned(), Element { Z: 27,  A: 58.9332, I: 297.0  }),
            ("Ni".to_owned(), Element { Z: 28,  A: 58.6934, I: 311.0  }),
            ("Cu".to_owned(), Element { Z: 29,  A: 63.546,  I: 322.0  }),
            ("Zn".to_owned(), Element { Z: 30,  A: 65.38,   I: 330.0  }),
            ("Ga".to_owned(), Element { Z: 31,  A: 69.723,  I: 334.0  }),
            ("Ge".to_owned(), Element { Z: 32,  A: 72.63,   I: 350.0  }),
            ("As".to_owned(), Element { Z: 33,  A: 74.9216, I: 347.0  }),
            ("Se".to_owned(), Element { Z: 34,  A: 78.971,  I: 348.0  }),
            ("Br".to_owned(), Element { Z: 35,  A: 79.904,  I: 357.0  }),
            ("Kr".to_owned(), Element { Z: 36,  A: 83.798,  I: 352.0  }),
            ("Rb".to_owned(), Element { Z: 37,  A: 85.4678, I: 363.0  }),
            ("Sr".to_owned(), Element { Z: 38,  A: 87.62,   I: 366.0  }),
            ("Y" .to_owned(), Element { Z: 39,  A: 88.9058, I: 379.0  }),
            ("Zr".to_owned(), Element { Z: 40,  A: 91.224,  I: 393.0  }),
            ("Nb".to_owned(), Element { Z: 41,  A: 92.9064, I: 417.0  }),
            ("Mo".to_owned(), Element { Z: 42,  A: 95.95,   I: 424.0  }),
            ("Tc".to_owned(), Element { Z: 43,  A: 97.9072, I: 428.0  }),
            ("Ru".to_owned(), Element { Z: 44,  A: 101.07,  I: 441.0  }),
            ("Rh".to_owned(), Element { Z: 45,  A: 102.906, I: 449.0  }),
            ("Pd".to_owned(), Element { Z: 46,  A: 106.42,  I: 470.0  }),
            ("Ag".to_owned(), Element { Z: 47,  A: 107.868, I: 470.0  }),
            ("Cd".to_owned(), Element { Z: 48,  A: 112.414, I: 469.0  }),
            ("In".to_owned(), Element { Z: 49,  A: 114.818, I: 488.0  }),
            ("Sn".to_owned(), Element { Z: 50,  A: 118.71,  I: 488.0  }),
            ("Sb".to_owned(), Element { Z: 51,  A: 121.76,  I: 487.0  }),
            ("Te".to_owned(), Element { Z: 52,  A: 127.6,   I: 485.0  }),
            ("I" .to_owned(), Element { Z: 53,  A: 126.904, I: 491.0  }),
            ("Xe".to_owned(), Element { Z: 54,  A: 131.293, I: 482.0  }),
            ("Cs".to_owned(), Element { Z: 55,  A: 132.905, I: 488.0  }),
            ("Ba".to_owned(), Element { Z: 56,  A: 137.327, I: 491.0  }),
            ("La".to_owned(), Element { Z: 57,  A: 138.905, I: 501.0  }),
            ("Ce".to_owned(), Element { Z: 58,  A: 140.116, I: 523.0  }),
            ("Pr".to_owned(), Element { Z: 59,  A: 140.908, I: 535.0  }),
            ("Nd".to_owned(), Element { Z: 60,  A: 144.242, I: 546.0  }),
            ("Pm".to_owned(), Element { Z: 61,  A: 144.913, I: 560.0  }),
            ("Sm".to_owned(), Element { Z: 62,  A: 150.36,  I: 574.0  }),
            ("Eu".to_owned(), Element { Z: 63,  A: 151.964, I: 580.0  }),
            ("Gd".to_owned(), Element { Z: 64,  A: 157.25,  I: 591.0  }),
            ("Tb".to_owned(), Element { Z: 65,  A: 158.925, I: 614.0  }),
            ("Dy".to_owned(), Element { Z: 66,  A: 162.5,   I: 628.0  }),
            ("Ho".to_owned(), Element { Z: 67,  A: 164.93,  I: 650.0  }),
            ("Er".to_owned(), Element { Z: 68,  A: 167.259, I: 658.0  }),
            ("Tm".to_owned(), Element { Z: 69,  A: 168.934, I: 674.0  }),
            ("Yb".to_owned(), Element { Z: 70,  A: 173.054, I: 684.0  }),
            ("Lu".to_owned(), Element { Z: 71,  A: 174.967, I: 694.0  }),
            ("Hf".to_owned(), Element { Z: 72,  A: 178.49,  I: 705.0  }),
            ("Ta".to_owned(), Element { Z: 73,  A: 180.948, I: 718.0  }),
            ("W" .to_owned(), Element { Z: 74,  A: 183.84,  I: 727.0  }),
            ("Re".to_owned(), Element { Z: 75,  A: 186.207, I: 736.0  }),
            ("Os".to_owned(), Element { Z: 76,  A: 190.23,  I: 746.0  }),
            ("Ir".to_owned(), Element { Z: 77,  A: 192.217, I: 757.0  }),
            ("Pt".to_owned(), Element { Z: 78,  A: 195.084, I: 790.0  }),
            ("Au".to_owned(), Element { Z: 79,  A: 196.967, I: 790.0  }),
            ("Hg".to_owned(), Element { Z: 80,  A: 200.592, I: 800.0  }),
            ("Tl".to_owned(), Element { Z: 81,  A: 204.38,  I: 810.0  }),
            ("Pb".to_owned(), Element { Z: 82,  A: 207.2,   I: 823.0  }),
            ("Bi".to_owned(), Element { Z: 83,  A: 208.98,  I: 823.0  }),
            ("Po".to_owned(), Element { Z: 84,  A: 208.982, I: 830.0  }),
            ("At".to_owned(), Element { Z: 85,  A: 209.987, I: 825.0  }),
            ("Rn".to_owned(), Element { Z: 86,  A: 222.018, I: 794.0  }),
            ("Fr".to_owned(), Element { Z: 87,  A: 223.02,  I: 827.0  }),
            ("Ra".to_owned(), Element { Z: 88,  A: 226.025, I: 826.0  }),
            ("Ac".to_owned(), Element { Z: 89,  A: 227.028, I: 841.0  }),
            ("Th".to_owned(), Element { Z: 90,  A: 232.038, I: 847.0  }),
            ("Pa".to_owned(), Element { Z: 91,  A: 231.036, I: 878.0  }),
            ("U" .to_owned(), Element { Z: 92,  A: 238.029, I: 890.0  }),
            ("Np".to_owned(), Element { Z: 93,  A: 237.048, I: 902.0  }),
            ("Pu".to_owned(), Element { Z: 94,  A: 244.064, I: 921.0  }),
            ("Am".to_owned(), Element { Z: 95,  A: 243.061, I: 934.0  }),
            ("Cm".to_owned(), Element { Z: 96,  A: 247.07,  I: 939.0  }),
            ("Bk".to_owned(), Element { Z: 97,  A: 247.07,  I: 952.0  }),
            ("Cf".to_owned(), Element { Z: 98,  A: 251.08,  I: 966.0  }),
            ("Es".to_owned(), Element { Z: 99,  A: 252.083, I: 980.0  }),
            ("Fm".to_owned(), Element { Z: 100, A: 257.095, I: 994.0  }),
            ("Md".to_owned(), Element { Z: 101, A: 258.098, I: 1007.0 }),
            ("No".to_owned(), Element { Z: 102, A: 259.101, I: 1020.0 }),
            ("Lr".to_owned(), Element { Z: 103, A: 262.11,  I: 1034.0 }),
            ("Rf".to_owned(), Element { Z: 104, A: 267.122, I: 1047.0 }),
            ("Db".to_owned(), Element { Z: 105, A: 268.126, I: 1061.0 }),
            ("Sg".to_owned(), Element { Z: 106, A: 269.129, I: 1074.0 }),
            ("Bh".to_owned(), Element { Z: 107, A: 270.133, I: 1087.0 }),
            ("Hs".to_owned(), Element { Z: 108, A: 269.134, I: 1102.0 }),
            ("Mt".to_owned(), Element { Z: 109, A: 278.156, I: 1115.0 }),
            ("Ds".to_owned(), Element { Z: 110, A: 281.164, I: 1129.0 }),
            ("Rg".to_owned(), Element { Z: 111, A: 282.169, I: 1143.0 }),
            ("Cn".to_owned(), Element { Z: 112, A: 285.177, I: 1156.0 }),
            ("Nh".to_owned(), Element { Z: 113, A: 286.182, I: 1171.0 }),
            ("Fl".to_owned(), Element { Z: 114, A: 289.19,  I: 1185.0 }),
            ("Mc".to_owned(), Element { Z: 115, A: 289.194, I: 1199.0 }),
            ("Lv".to_owned(), Element { Z: 116, A: 293.204, I: 1213.0 }),
            ("Ts".to_owned(), Element { Z: 117, A: 294.211, I: 1227.0 }),
            ("Og".to_owned(), Element { Z: 118, A: 294.214, I: 1242.0 }),
        ]);

        let materials = HashMap::new();
        let mut registry = Self { elements, materials };

        let path = Path::new(crate::PREFIX.get(py).unwrap())
            .join(format!("data/materials/{}.toml", Self::DEFAULT_MATERIALS));
        registry.load(py, &path)?;
        Ok(registry)
    }

    pub fn get_composite<'a>(&'a self, name: &str) -> PyResult<&'a Composite> {
        self.materials.get(name)
            .and_then(|m| m.as_composite())
            .ok_or_else(|| {
                let why = format!("undefined composite '{}'", name);
                Error::new(ValueError).why(&why).to_err()
            })
    }

    pub fn get_element<'a>(&'a self, symbol: &str) -> PyResult<&'a Element> {
        self.elements.get(symbol)
            .ok_or_else(|| {
                let why = format!("undefined element '{}'", symbol);
                Error::new(ValueError).why(&why).to_err()
            })
    }

    pub fn get_material<'a>(&'a self, name: &str) -> PyResult<&'a Material> {
        self.materials.get(name)
            .ok_or_else(|| {
                let why = format!("undefined material '{}'", name);
                Error::new(ValueError).why(&why).to_err()
            })
    }

    pub fn get_mixture<'a>(&'a self, name: &str) -> PyResult<&'a Mixture> {
        self.materials.get(name)
            .and_then(|m| m.as_mixture())
            .ok_or_else(|| {
                let why = format!("undefined mixture '{}'", name);
                Error::new(ValueError).why(&why).to_err()
            })
    }
}
