use crate::utils::cache;
use pyo3::prelude::*;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::{Arc, RwLock, RwLockReadGuard, Weak};
use std::sync::atomic::{AtomicUsize, Ordering};
use super::definitions::{Component, Material};
use super::registry::MaterialsBroker;
use super::toml::ToToml;


#[derive(Clone)]
pub struct MaterialsSet {
    inner: Arc<RwLock<Set>>
}

pub struct MaterialsSubscriber {
    inner: Weak<RwLock<Set>>
}

pub struct MaterialsBorrow<'a> (RwLockReadGuard<'a, Set>);

pub struct UnpackedMaterials {
    pub composites: Vec<String>,
    pub elements: Vec<String>,
    pub mixtures: Vec<String>,
}

pub struct CachePath {
    hash: u64,
    suffix: &'static str,
    tag: Option<String>,
    makedirs: bool,
}

struct Set {
    pub data: HashMap<String, usize>,
    pub version: usize,
    pub hash: Option<u64>,
}

static VERSION: AtomicUsize = AtomicUsize::new(0);

impl MaterialsSet {
    pub fn new() -> Self {
        let set = Set::new();
        let inner = Arc::new(RwLock::new(set));
        Self { inner }
    }

    pub fn subscribe(&self) -> MaterialsSubscriber {
        let inner = Arc::downgrade(&self.inner);
        MaterialsSubscriber { inner }
    }

    pub fn add(&self, material: &str) {
        let mut set = self.inner.write().unwrap();
        match set.data.get_mut(material) {
            Some(rc) => *rc += 1,
            None => {
                set.data.insert(material.to_owned(), 1);
                set.update();
            },
        }
    }

    pub fn remove(&self, material: &str) {
        let mut set = self.inner.write().unwrap();
        if let Some(rc) = set.data.get_mut(material) {
            if *rc > 1 {
                *rc -= 1;
            } else {
                set.data.remove(material);
                set.update();
            }
        }
    }

    pub fn hash(&self, py: Python) -> PyResult<u64> {
        let mut set = self.inner.write().unwrap();
        if set.hash.is_none() {
            let broker = MaterialsBroker::new(py)?;
            let UnpackedMaterials { composites, elements, mixtures } = set.unpack(&broker)?;

            let registry = &broker.registry.read().unwrap();
            let mut state = DefaultHasher::new();
            for element in elements {
                element.hash(&mut state);
                let definition = registry.element(&element);
                definition.hash(&mut state);
            }
            for mixture in mixtures {
                mixture.hash(&mut state);
                let definition = registry.mixture(&mixture);
                definition.hash(&mut state);
            }
            for composite in composites {
                composite.hash(&mut state);
                let definition = registry.composite(&composite);
                definition.hash(&mut state);
            }
            set.hash = Some(state.finish());
        }
        let hash = set.hash.unwrap();
        Ok(hash)
    }

    pub fn version(&self) -> usize {
        self.inner.read().unwrap().version
    }

    pub fn is_cached(&self, py: Python) -> PyResult<bool> {
        let path = self.cache_path(py, "toml")?
            .into_path()?;
        let cached = if path.try_exists().unwrap_or(false) {
            let broker = MaterialsBroker::new(py)?;
            if broker.load(py, &path).is_ok() {
                true
            } else {
                self.delete_dumps(py)?;
                false
            }
        } else {
            false
        };
        Ok(cached)
    }

    pub fn cache_path(&self, py: Python, suffix: &'static str) -> PyResult<CachePath> {
        let path = CachePath::new(self.hash(py)?, suffix);
        Ok(path)
    }

    pub fn cache_definitions(&self, py: Python) -> PyResult<()> {
        let path = self.cache_path(py, "toml")?
            .into_path()?;
        std::fs::write(path, self.to_toml(py)?)?;
        Ok(())
    }

    pub fn borrow<'a>(&'a self) -> MaterialsBorrow<'a> {
        MaterialsBorrow(self.inner.read().unwrap())
    }

    fn delete_dumps(&self, py: Python) -> PyResult<()> {
        let materials_cache = cache::get_path()?.join("materials");
        let hash = format!("{:016x}", self.hash(py)?);
        if let Ok(content) = std::fs::read_dir(&materials_cache) {
            // Remove any cached pumas dumps.
            for entry in content {
                if let Ok(entry) = entry {
                    if let Some(filename) = entry.file_name().to_str() {
                        if filename.starts_with(&hash) &&
                           filename.ends_with(".pumas") {
                            std::fs::remove_file(&entry.path())?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl MaterialsSubscriber {
    pub fn is_alive(&self) -> bool {
        self.inner.strong_count() > 0
    }

    pub fn is_subscribed(&self, set: &MaterialsSet) -> bool {
        std::ptr::addr_eq(
            Weak::as_ptr(&self.inner),
            Arc::as_ptr(&set.inner),
        )
    }

    pub fn replace(&self, current: &str, new: &str) -> bool {
        let Some(set) = self.inner.upgrade() else { return false };
        let mut set = set.write().unwrap();
        let mut update = false;
        if let Some(rc) = set.data.get_mut(current) {
            if *rc > 1 {
                *rc -= 1;
            } else {
                set.data.remove(current);
                update = true;
            }
        }
        match set.data.get_mut(new) {
            Some(rc) => *rc += 1,
            None => {
                set.data.insert(new.to_owned(), 1);
                update = true;
            },
        }
        if update { set.update() }
        true
    }
}

impl Set {
    fn new() -> Self {
        let data = HashMap::new();
        let version = VERSION.fetch_add(1, Ordering::SeqCst);
        let hash = None;
        Self { data, version, hash }
    }

    fn unpack(&self, broker: &MaterialsBroker) -> PyResult<UnpackedMaterials> {
        let mut composites = Vec::new();
        let mut mixtures = Vec::new();
        for key in self.data.keys() {
            match broker.get_material(key)? {
                Material::Mixture(_) => {
                    mixtures.push(key.clone());
                },
                Material::Composite(composite) => {
                    let data = composite.read();
                    for Component { name, .. } in data.composition.iter() {
                        mixtures.push(name.clone());
                    }
                    composites.push(key.clone());
                },
            }
        }
        composites.sort();
        mixtures.sort();
        mixtures.dedup();

        let registry = &broker.registry.read().unwrap();
        let mut elements = Vec::new();
        for mixture in mixtures.iter() {
            let mixture = registry.mixture(mixture);
            for Component { name, .. } in mixture.composition.iter() {
                elements.push(name.clone())
            }
        }
        elements.sort();
        elements.dedup();

        let unpacked = UnpackedMaterials { composites, elements, mixtures };
        Ok(unpacked)
    }

    fn update(&mut self) {
        self.version = VERSION.fetch_add(1, Ordering::SeqCst);
        self.hash = None;
    }
}

impl CachePath {
    pub fn new(hash: u64, suffix: &'static str) -> Self {
        let tag = None;
        let makedirs = false;
        Self { hash, suffix, tag, makedirs }
    }

    pub fn with_tag(mut self, value: String) -> Self {
        self.tag = Some(value);
        self
    }

    pub fn with_makedirs(mut self) -> Self {
        self.makedirs = true;
        self
    }

    pub fn into_path(self) -> PyResult<PathBuf> {
        let path = cache::get_path()?.join("materials");
        if self.makedirs {
            std::fs::create_dir_all(&path)?;
        }
        let path = match self.tag {
            Some(tag) => path.join(format!("{:016x}-{}.{}", self.hash, tag, self.suffix)),
            None => path.join(format!("{:016x}.{}", self.hash, self.suffix)),
        };
        Ok(path)
    }
}

impl<'a> MaterialsBorrow<'a> {
    pub fn iter(&self) -> impl Iterator<Item=&String> {
        self.0.data.keys()
    }

    #[inline]
    pub fn unpack(&self, broker: &MaterialsBroker) -> PyResult<UnpackedMaterials> {
        self.0.unpack(broker)
    }
}

impl<'a, I> From<I> for MaterialsSet
where
    I: IntoIterator<Item=String>
{
    fn from(materials: I) -> Self {
        let mut set = Set::new();
        for material in materials {
            set.data.entry(material)
                .and_modify(|rc| { *rc += 1 })
                .or_insert(1);
        }
        let inner = Arc::new(RwLock::new(set));
        Self { inner }
    }
}
