use crate::geometry::ExternalGeometry;
use crate::materials::{Element, Material, Registry};
use crate::simulation::coordinates::LocalFrame;
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::{TypeError, ValueError};
use crate::utils::io::PathString;
use libloading::Library;
use pyo3::prelude::*;
use pyo3::sync::GILOnceCell;
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

mod types;

pub use types::{CGeometry, CMedium, CModule, CTracer, CVec3};


#[pyclass(module="mulder")]
pub struct Module {
    /// The module location.
    #[pyo3(get)]
    path: String,

    #[allow(dead_code)]
    lib: Library, // for keeping the library alive.
    interface: CModule,
}

type Modules = HashMap<String, Py<Module>>;

#[inline]
fn type_error(what: &str, why: &str) -> PyErr {
    Error::new(TypeError).what(what).why(&why).to_err()
}

static MODULES: GILOnceCell<RwLock<Modules>> = GILOnceCell::new();

fn modules(py: Python) -> PyResult<&'static RwLock<Modules>> {
    MODULES.get_or_try_init(py, || Ok::<_, PyErr>(RwLock::new(Modules::new())))
}

#[pymethods]
impl Module {
    #[new]
    #[pyo3(signature=(path, /))]
    pub unsafe fn new(py: Python<'_>, path: PathString) -> PyResult<Py<Self>> {
        let path = Path::new(&path.0)
            .canonicalize()?
            .to_str()
            .ok_or_else(|| Error::new(ValueError).what("path").why(&path.0).to_err())?
            .to_owned();

        if let Some(module) = modules(py)?.read().unwrap().get(&path) {
            return Ok(module.clone_ref(py))
        }

        // Fetch interface from entry point.
        type Initialise = unsafe fn() -> types::CModule;
        const INITIALISE: &[u8] = b"mulder_initialise\0";

        let library = Library::new(path.as_str())
            .map_err(|err| type_error(
                "CModule",
                &format!("{}: {}", path.as_str(), err)
            ))?;
        let initialise = library.get::<Initialise>(INITIALISE)
            .map_err(|err| type_error(
                "CModule",
                &format!("{}: {}", path.as_str(), err)
            ))?;
        let interface = unsafe { initialise() };

        let module = Py::new(py, Self {
            path: path.clone(),
            lib: library,
            interface,
        })?;

        modules(py)?
            .write()
            .unwrap()
            .insert(path, module.clone_ref(py));

        Ok(module)
    }

    fn __repr__(&self) -> String {
        format!("Module(\"{}\")", self.path)
    }

    /// Pointer to the C interface.
    #[getter]
    fn get_ptr(&self) -> PyObject {
        unimplemented!() // XXX return ctype pointer.
    }

    /// Fetches a module atomic element.
    #[pyo3(signature=(symbol, /))]
    fn element(&self, py: Python<'_>, symbol: &str) -> PyResult<Option<Element>> {
        self.interface
            .element(symbol)?
            .map(|element| {
                let registry = &mut Registry::get(py)?.write().unwrap();
                registry.add_element(symbol.to_owned(), element.clone())?;
                Ok(element)
            })
            .transpose()
    }

    /// Creates a new geometry.
    #[pyo3(signature=(*, frame=None))]
    fn geometry(
        &self,
        py: Python<'_>,
        frame: Option<LocalFrame>,
    ) -> PyResult<Py<ExternalGeometry>> {
        ExternalGeometry::from_module(py, &self.interface, frame)
    }

    /// Feches a module material.
    #[pyo3(signature=(name, /))]
    fn material(&self, py: Python<'_>, name: &str) -> PyResult<Option<Material>> {
        let registry = &mut Registry::get(py)?.write().unwrap();
        self.interface
            .material(name, registry)?
            .map(|material| {
                registry.add_material(name.to_owned(), material.clone())?;
                Ok(material)
            })
            .transpose()
    }
}
