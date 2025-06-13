use pyo3::prelude::*;
use pyo3::exceptions::PyFileNotFoundError;
use pyo3::sync::GILOnceCell;
use pyo3::types::{PyDict, PyString};
use std::ops::Deref;
use std::path::Path;


// ===============================================================================================
//
// Dict loader(s).
//
// ===============================================================================================

pub trait ConfigFormat {
    fn import_module<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyModule>>;

    fn load_dict<'py>(py: Python<'py>, path: &Path) -> PyResult<Bound<'py, PyDict>> {
        let content = std::fs::read_to_string(path)
            .map_err(|err| match err.kind() {
                std::io::ErrorKind::NotFound => {
                    let path = format!("No such file or directory '{}'", path.display());
                    PyFileNotFoundError::new_err(path)
                },
                _ => err.into(),
            })?;
        let module = Self::import_module(py)?;
        let loads = module.getattr("loads")?;
        let content = loads.call1((content,))?;
        let dict: Bound<PyDict> = content.extract()?;
        Ok(dict)
    }
}

pub struct Toml;

impl ConfigFormat for Toml {
    fn import_module<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyModule>> {
        py.import("tomllib")
            .or_else(|_| py.import("tomli"))
    }
}


// ===============================================================================================
//
// Pathlib.Path wrapper.
//
// ===============================================================================================

pub struct PathString (pub String);

impl<'py> FromPyObject<'py> for PathString {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        static TYPE: GILOnceCell<PyObject> = GILOnceCell::new();
        let py = ob.py();
        let tp = TYPE.get_or_try_init(py, || py.import("pathlib")
            .and_then(|m| m.getattr("Path"))
            .map(|m| m.unbind())
        )?.bind(py);
        let path = if ob.is_instance(tp)? {
            ob.str()?
        } else {
            let path: Bound<PyString> = ob.extract()?;
            path
        };
        Ok(Self(path.to_cow()?.into_owned()))
    }
}

impl Deref for PathString {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ToString for PathString {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}
