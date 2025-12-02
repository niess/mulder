use crate::materials::Component;
use crate::module::{calzone, Module, modules};
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::{ValueError, TypeError};
use crate::utils::io::{ConfigFormat, Toml};
use crate::utils::traits::TypeName;
use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::sync::GILOnceCell;
use std::path::Path;
use std::sync::{LazyLock, RwLock};
use super::definitions::{Composite, Element, Material, Mixture};


#[derive(Default)]
pub struct Registry {
    elements: IndexMap<String, Element>,
    materials: IndexMap<String, Material>,
}

pub struct MaterialsBroker<'a, 'py:'a> {
    pub registry: &'a RwLock<Registry>,
    modules: Vec<Bound<'py, Module>>,
    companions: Vec<Bound<'py, Module>>,
}

struct MixtureData {
    density: f64,
    composition: Composition,
}

enum Composition {
    Formula(&'static str),
    Mass(Vec<Component>)
}

static REGISTRY: GILOnceCell<RwLock<Registry>> = GILOnceCell::new();

impl Registry {
    #[inline]
    pub fn get(py: Python) -> PyResult<&'static RwLock<Self>> {
        REGISTRY.get_or_try_init(py, || Ok(RwLock::new(Self::default())))
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
                // Update fractions.
                self.materials.insert(name, definition);
            },
            None => {
                self.materials.insert(name, definition);
            },
        }
        Ok(())
    }

    #[inline]
    pub fn composite<'a>(&'a self, name: &str) -> &'a Composite {
        self.materials.get(name).and_then(|material| material.as_composite()).unwrap()
    }

    #[inline]
    pub fn element<'a>(&'a self, symbol: &str) -> &'a Element {
        self.elements.get(symbol).unwrap()
    }

    #[inline]
    pub fn elements(&self) -> &IndexMap<String, Element> {
        &self.elements
    }

    #[inline]
    pub fn material<'a>(&'a self, name: &str) -> &'a Material {
        self.materials.get(name).unwrap()
    }

    #[inline]
    pub fn materials(&self) -> &IndexMap<String, Material> {
        &self.materials
    }

    #[inline]
    pub fn mixture<'a>(&'a self, name: &str) -> &'a Mixture {
        self.materials.get(name).and_then(|material| material.as_mixture()).unwrap()
    }
}

impl Registry {
    pub const DEFAULT_ELEMENTS: LazyLock<IndexMap<String, Element>> = LazyLock::new(|| {
        IndexMap::from([
            ("H" .to_owned(), Element { Z: 1,   A: 1.008,   I: 19.2E-09   }),
            ("D" .to_owned(), Element { Z: 1,   A: 2.0141,  I: 19.2E-09   }),
            ("He".to_owned(), Element { Z: 2,   A: 4.0026,  I: 41.8E-09   }),
            ("Li".to_owned(), Element { Z: 3,   A: 6.94,    I: 40.0E-09   }),
            ("Be".to_owned(), Element { Z: 4,   A: 9.01218, I: 63.7E-09   }),
            ("B" .to_owned(), Element { Z: 5,   A: 10.81,   I: 76.0E-09   }),
            ("C" .to_owned(), Element { Z: 6,   A: 12.0107, I: 78.0E-09   }),
            ("N" .to_owned(), Element { Z: 7,   A: 14.007,  I: 82.0E-09   }),
            ("O" .to_owned(), Element { Z: 8,   A: 15.999,  I: 95.0E-09   }),
            ("F" .to_owned(), Element { Z: 9,   A: 18.9984, I: 115.0E-09  }),
            ("Ne".to_owned(), Element { Z: 10,  A: 20.1797, I: 137.0E-09  }),
            ("Rk".to_owned(), Element { Z: 11,  A: 22.0,    I: 136.4E-09  }), // Fictitious Rockium.
            ("Na".to_owned(), Element { Z: 11,  A: 22.9898, I: 149.0E-09  }),
            ("Mg".to_owned(), Element { Z: 12,  A: 24.305,  I: 156.0E-09  }),
            ("Al".to_owned(), Element { Z: 13,  A: 26.9815, I: 166.0E-09  }),
            ("Si".to_owned(), Element { Z: 14,  A: 28.0855, I: 173.0E-09  }),
            ("P" .to_owned(), Element { Z: 15,  A: 30.9738, I: 173.0E-09  }),
            ("S" .to_owned(), Element { Z: 16,  A: 32.065,  I: 180.0E-09  }),
            ("Cl".to_owned(), Element { Z: 17,  A: 35.453,  I: 174.0E-09  }),
            ("Ar".to_owned(), Element { Z: 18,  A: 39.948,  I: 188.0E-09  }),
            ("K" .to_owned(), Element { Z: 19,  A: 39.0983, I: 190.0E-09  }),
            ("Ca".to_owned(), Element { Z: 20,  A: 40.078,  I: 191.0E-09  }),
            ("Sc".to_owned(), Element { Z: 21,  A: 44.9559, I: 216.0E-09  }),
            ("Ti".to_owned(), Element { Z: 22,  A: 47.867,  I: 233.0E-09  }),
            ("V" .to_owned(), Element { Z: 23,  A: 50.9415, I: 245.0E-09  }),
            ("Cr".to_owned(), Element { Z: 24,  A: 51.9961, I: 257.0E-09  }),
            ("Mn".to_owned(), Element { Z: 25,  A: 54.938,  I: 272.0E-09  }),
            ("Fe".to_owned(), Element { Z: 26,  A: 55.845,  I: 286.0E-09  }),
            ("Co".to_owned(), Element { Z: 27,  A: 58.9332, I: 297.0E-09  }),
            ("Ni".to_owned(), Element { Z: 28,  A: 58.6934, I: 311.0E-09  }),
            ("Cu".to_owned(), Element { Z: 29,  A: 63.546,  I: 322.0E-09  }),
            ("Zn".to_owned(), Element { Z: 30,  A: 65.38,   I: 330.0E-09  }),
            ("Ga".to_owned(), Element { Z: 31,  A: 69.723,  I: 334.0E-09  }),
            ("Ge".to_owned(), Element { Z: 32,  A: 72.63,   I: 350.0E-09  }),
            ("As".to_owned(), Element { Z: 33,  A: 74.9216, I: 347.0E-09  }),
            ("Se".to_owned(), Element { Z: 34,  A: 78.971,  I: 348.0E-09  }),
            ("Br".to_owned(), Element { Z: 35,  A: 79.904,  I: 357.0E-09  }),
            ("Kr".to_owned(), Element { Z: 36,  A: 83.798,  I: 352.0E-09  }),
            ("Rb".to_owned(), Element { Z: 37,  A: 85.4678, I: 363.0E-09  }),
            ("Sr".to_owned(), Element { Z: 38,  A: 87.62,   I: 366.0E-09  }),
            ("Y" .to_owned(), Element { Z: 39,  A: 88.9058, I: 379.0E-09  }),
            ("Zr".to_owned(), Element { Z: 40,  A: 91.224,  I: 393.0E-09  }),
            ("Nb".to_owned(), Element { Z: 41,  A: 92.9064, I: 417.0E-09  }),
            ("Mo".to_owned(), Element { Z: 42,  A: 95.95,   I: 424.0E-09  }),
            ("Tc".to_owned(), Element { Z: 43,  A: 97.9072, I: 428.0E-09  }),
            ("Ru".to_owned(), Element { Z: 44,  A: 101.07,  I: 441.0E-09  }),
            ("Rh".to_owned(), Element { Z: 45,  A: 102.906, I: 449.0E-09  }),
            ("Pd".to_owned(), Element { Z: 46,  A: 106.42,  I: 470.0E-09  }),
            ("Ag".to_owned(), Element { Z: 47,  A: 107.868, I: 470.0E-09  }),
            ("Cd".to_owned(), Element { Z: 48,  A: 112.414, I: 469.0E-09  }),
            ("In".to_owned(), Element { Z: 49,  A: 114.818, I: 488.0E-09  }),
            ("Sn".to_owned(), Element { Z: 50,  A: 118.71,  I: 488.0E-09  }),
            ("Sb".to_owned(), Element { Z: 51,  A: 121.76,  I: 487.0E-09  }),
            ("Te".to_owned(), Element { Z: 52,  A: 127.6,   I: 485.0E-09  }),
            ("I" .to_owned(), Element { Z: 53,  A: 126.904, I: 491.0E-09  }),
            ("Xe".to_owned(), Element { Z: 54,  A: 131.293, I: 482.0E-09  }),
            ("Cs".to_owned(), Element { Z: 55,  A: 132.905, I: 488.0E-09  }),
            ("Ba".to_owned(), Element { Z: 56,  A: 137.327, I: 491.0E-09  }),
            ("La".to_owned(), Element { Z: 57,  A: 138.905, I: 501.0E-09  }),
            ("Ce".to_owned(), Element { Z: 58,  A: 140.116, I: 523.0E-09  }),
            ("Pr".to_owned(), Element { Z: 59,  A: 140.908, I: 535.0E-09  }),
            ("Nd".to_owned(), Element { Z: 60,  A: 144.242, I: 546.0E-09  }),
            ("Pm".to_owned(), Element { Z: 61,  A: 144.913, I: 560.0E-09  }),
            ("Sm".to_owned(), Element { Z: 62,  A: 150.36,  I: 574.0E-09  }),
            ("Eu".to_owned(), Element { Z: 63,  A: 151.964, I: 580.0E-09  }),
            ("Gd".to_owned(), Element { Z: 64,  A: 157.25,  I: 591.0E-09  }),
            ("Tb".to_owned(), Element { Z: 65,  A: 158.925, I: 614.0E-09  }),
            ("Dy".to_owned(), Element { Z: 66,  A: 162.5,   I: 628.0E-09  }),
            ("Ho".to_owned(), Element { Z: 67,  A: 164.93,  I: 650.0E-09  }),
            ("Er".to_owned(), Element { Z: 68,  A: 167.259, I: 658.0E-09  }),
            ("Tm".to_owned(), Element { Z: 69,  A: 168.934, I: 674.0E-09  }),
            ("Yb".to_owned(), Element { Z: 70,  A: 173.054, I: 684.0E-09  }),
            ("Lu".to_owned(), Element { Z: 71,  A: 174.967, I: 694.0E-09  }),
            ("Hf".to_owned(), Element { Z: 72,  A: 178.49,  I: 705.0E-09  }),
            ("Ta".to_owned(), Element { Z: 73,  A: 180.948, I: 718.0E-09  }),
            ("W" .to_owned(), Element { Z: 74,  A: 183.84,  I: 727.0E-09  }),
            ("Re".to_owned(), Element { Z: 75,  A: 186.207, I: 736.0E-09  }),
            ("Os".to_owned(), Element { Z: 76,  A: 190.23,  I: 746.0E-09  }),
            ("Ir".to_owned(), Element { Z: 77,  A: 192.217, I: 757.0E-09  }),
            ("Pt".to_owned(), Element { Z: 78,  A: 195.084, I: 790.0E-09  }),
            ("Au".to_owned(), Element { Z: 79,  A: 196.967, I: 790.0E-09  }),
            ("Hg".to_owned(), Element { Z: 80,  A: 200.592, I: 800.0E-09  }),
            ("Tl".to_owned(), Element { Z: 81,  A: 204.38,  I: 810.0E-09  }),
            ("Pb".to_owned(), Element { Z: 82,  A: 207.2,   I: 823.0E-09  }),
            ("Bi".to_owned(), Element { Z: 83,  A: 208.98,  I: 823.0E-09  }),
            ("Po".to_owned(), Element { Z: 84,  A: 208.982, I: 830.0E-09  }),
            ("At".to_owned(), Element { Z: 85,  A: 209.987, I: 825.0E-09  }),
            ("Rn".to_owned(), Element { Z: 86,  A: 222.018, I: 794.0E-09  }),
            ("Fr".to_owned(), Element { Z: 87,  A: 223.02,  I: 827.0E-09  }),
            ("Ra".to_owned(), Element { Z: 88,  A: 226.025, I: 826.0E-09  }),
            ("Ac".to_owned(), Element { Z: 89,  A: 227.028, I: 841.0E-09  }),
            ("Th".to_owned(), Element { Z: 90,  A: 232.038, I: 847.0E-09  }),
            ("Pa".to_owned(), Element { Z: 91,  A: 231.036, I: 878.0E-09  }),
            ("U" .to_owned(), Element { Z: 92,  A: 238.029, I: 890.0E-09  }),
            ("Np".to_owned(), Element { Z: 93,  A: 237.048, I: 902.0E-09  }),
            ("Pu".to_owned(), Element { Z: 94,  A: 244.064, I: 921.0E-09  }),
            ("Am".to_owned(), Element { Z: 95,  A: 243.061, I: 934.0E-09  }),
            ("Cm".to_owned(), Element { Z: 96,  A: 247.07,  I: 939.0E-09  }),
            ("Bk".to_owned(), Element { Z: 97,  A: 247.07,  I: 952.0E-09  }),
            ("Cf".to_owned(), Element { Z: 98,  A: 251.08,  I: 966.0E-09  }),
            ("Es".to_owned(), Element { Z: 99,  A: 252.083, I: 980.0E-09  }),
            ("Fm".to_owned(), Element { Z: 100, A: 257.095, I: 994.0E-09  }),
            ("Md".to_owned(), Element { Z: 101, A: 258.098, I: 1007.0E-09 }),
            ("No".to_owned(), Element { Z: 102, A: 259.101, I: 1020.0E-09 }),
            ("Lr".to_owned(), Element { Z: 103, A: 262.11,  I: 1034.0E-09 }),
            ("Rf".to_owned(), Element { Z: 104, A: 267.122, I: 1047.0E-09 }),
            ("Db".to_owned(), Element { Z: 105, A: 268.126, I: 1061.0E-09 }),
            ("Sg".to_owned(), Element { Z: 106, A: 269.129, I: 1074.0E-09 }),
            ("Bh".to_owned(), Element { Z: 107, A: 270.133, I: 1087.0E-09 }),
            ("Hs".to_owned(), Element { Z: 108, A: 269.134, I: 1102.0E-09 }),
            ("Mt".to_owned(), Element { Z: 109, A: 278.156, I: 1115.0E-09 }),
            ("Ds".to_owned(), Element { Z: 110, A: 281.164, I: 1129.0E-09 }),
            ("Rg".to_owned(), Element { Z: 111, A: 282.169, I: 1143.0E-09 }),
            ("Cn".to_owned(), Element { Z: 112, A: 285.177, I: 1156.0E-09 }),
            ("Nh".to_owned(), Element { Z: 113, A: 286.182, I: 1171.0E-09 }),
            ("Fl".to_owned(), Element { Z: 114, A: 289.19,  I: 1185.0E-09 }),
            ("Mc".to_owned(), Element { Z: 115, A: 289.194, I: 1199.0E-09 }),
            ("Lv".to_owned(), Element { Z: 116, A: 293.204, I: 1213.0E-09 }),
            ("Ts".to_owned(), Element { Z: 117, A: 294.211, I: 1227.0E-09 }),
            ("Og".to_owned(), Element { Z: 118, A: 294.214, I: 1242.0E-09 }),
        ])
    });

    const DEFAULT_MATERIALS: LazyLock<IndexMap<String, MixtureData>> = LazyLock::new(|| {
        IndexMap::from([
            ("Air".to_owned(), MixtureData {
                density: 1.205,
                composition: Composition::Mass(vec![
                    Component { name: "C" .to_owned(), weight: 0.000124 },
                    Component { name: "N" .to_owned(), weight: 0.755267 },
                    Component { name: "O" .to_owned(), weight: 0.231781 },
                    Component { name: "Ar".to_owned(), weight: 0.012827 },
                ]),
            }),
            ("Rock".to_owned(), MixtureData {
                density: 2.65E+03,
                composition: Composition::Formula("Rk")
            }),
            ("Water".to_owned(), MixtureData {
                density: 1.02E+03,
                composition: Composition::Formula("H2O")
            }),
        ])
    });
}

impl <'a, 'py: 'a> MaterialsBroker<'a, 'py> {
    pub fn new(py: Python<'py>) -> PyResult<Self> {
        let registry = Registry::get(py)?;
        let cz = calzone(py)?
            .map(|module| module.bind(py).clone());
        let modules = modules(py)?
            .read()
            .unwrap()
            .values()
            .map(|module| module.bind(py).clone())
            .filter(|m| cz.as_ref().map(|cz| *m.borrow() != *cz.borrow()).unwrap_or(true))
            .collect();
        let companions = match cz {
            Some(cz) => vec![cz],
            None => Vec::new(),
        };
        Ok(Self { registry, modules, companions })
    }

    pub fn get_composite(&self, name: &str) -> PyResult<Composite> {
        self.get_material_opt(name)?
            .and_then(|material| material.into_composite())
            .ok_or_else(|| {
                let why = format!("undefined composite '{}'", name);
                Error::new(ValueError).why(&why).to_err()
            })
    }

    pub fn get_element(&self, symbol: &str) -> PyResult<Element> {
        self.get_element_opt(symbol)?
            .ok_or_else(|| {
                let why = format!("undefined element '{}'", symbol);
                Error::new(ValueError).why(&why).to_err()
            })
    }

    pub fn get_element_opt(&self, symbol: &str) -> PyResult<Option<Element>> {
        if let Some(element) = self.registry.read().unwrap().elements.get(symbol) {
            return Ok(Some(element.clone()))
        }
        for module in self.modules.iter() {
            if let Some(element) = module.borrow().interface.element(symbol)? {
                self.registry.write().unwrap()
                    .add_element(symbol.to_owned(), element.clone())?;
                return Ok(Some(element))
            }
        }
        for module in self.companions.iter() {
            if let Some(element) = module.borrow().interface.element(symbol)? {
                self.registry.write().unwrap()
                    .add_element(symbol.to_owned(), element.clone())?;
                return Ok(Some(element))
            }
        }
        if let Some(element) = (&*Registry::DEFAULT_ELEMENTS).get(symbol) {
            self.registry.write().unwrap()
                .add_element(symbol.to_owned(), element.clone())?;
            return Ok(Some(element.clone()))
        }
        Ok(None)
    }

    pub fn get_material(&self, name: &str) -> PyResult<Material> {
        self.get_material_opt(name)?
            .ok_or_else(|| {
                let why = format!("undefined material '{}'", name);
                Error::new(ValueError).why(&why).to_err()
            })
    }

    pub fn get_material_opt(&self, name: &str) -> PyResult<Option<Material>> {
        if let Some(material) = self.registry.read().unwrap().materials.get(name) {
            return Ok(Some(material.clone()))
        }
        for module in self.modules.iter() {
            let material = module.borrow().interface.material(name, self)?;
            if let Some(material) = material {
                self.registry.write().unwrap()
                    .add_material(name.to_owned(), material.clone())?;
                return Ok(Some(material))
            }
        }
        for module in self.companions.iter() {
            let material = module.borrow().interface.material(name, self)?;
            if let Some(material) = material {
                self.registry.write().unwrap()
                    .add_material(name.to_owned(), material.clone())?;
                return Ok(Some(material))
            }
        }
        if let Some(data) = (&*Registry::DEFAULT_MATERIALS).get(name) {
            let mixture = match &data.composition {
                Composition::Formula(formula) => Mixture::from_formula(
                    data.density,
                    formula,
                    None,
                    self,
                ),
                Composition::Mass(composition) => Mixture::from_composition(
                    data.density,
                    composition,
                    None,
                    self,
                ),
            }
                .map_err(|(kind, why)| Error::new(kind).why(&why).to_err())?;
            let material = Material::Mixture(mixture);
            self.registry.write().unwrap()
                .add_material(name.to_owned(), material.clone())?;
            return Ok(Some(material))
        }
        Ok(None)
    }

    pub fn get_mixture(&self, name: &str) -> PyResult<Mixture> {
        self.get_material_opt(name)?
            .and_then(|material| material.into_mixture())
            .ok_or_else(|| {
                let why = format!("undefined base material '{}'", name);
                Error::new(ValueError).why(&why).to_err()
            })
    }

    pub fn load<P: AsRef<Path>>(&self, py: Python, path: P) -> PyResult<()> {
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
                self.registry.write().unwrap().add_element(k, element)?;
            }
        }

        for (k, v) in toml.iter() {
            let k: String = k.extract()
                .map_err(|_| to_err("string", &k))?;
            if k == "elements" { continue }
            let v: Bound<PyDict> = v.extract()
                .map_err(|_| to_err("dict", &v))?;
            let material: Material = (k.as_str(), &v, self).try_into()?;
            self.registry.write().unwrap().add_material(k, material)?;
        }

        Ok(())
    }
}
