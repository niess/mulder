use crate::utils::error::Error;
use crate::utils::error::ErrorKind::ValueError;
use pyo3::prelude::*;
use ::std::env;
use ::std::path::{Path, PathBuf};
use ::std::sync::{OnceLock, RwLock};
use ::std::sync::atomic::{AtomicBool, Ordering};


static CACHE_PATH: OnceLock<RwLock<PathBuf>> = OnceLock::new();

static INITIALISED: AtomicBool = AtomicBool::new(false);

fn default_path() -> PyResult<PathBuf> {
    let home = env::var("HOME")
        .map_err(|_| Error::new(ValueError)
            .what("cache")
            .why("could not resolve $HOME")
        )?;
    let cache = Path::new(&home)
        .join(".cache/mulder");
    Ok(cache)
}

pub fn set_path(value: PathBuf) {
    let mut path = CACHE_PATH
        .get_or_init(|| RwLock::new(PathBuf::new()))
        .write()
        .unwrap();
    *path = value;
    INITIALISED.store(true, Ordering::Relaxed);
}

pub fn get_path() -> PyResult<PathBuf> {
    if INITIALISED.load(Ordering::Relaxed) {
        let cache = CACHE_PATH.get().unwrap().read().unwrap().clone();
        Ok(cache)
    } else {
        let cache = match env::var("MULDER_CACHE") {
            Ok(cache) => Path::new(&cache).to_path_buf(),
            Err(_) => default_path()?,
        };
        set_path(cache.clone());
        Ok(cache)
    }
}
