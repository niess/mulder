use process_path::get_dylib_path;
use pyo3::prelude::*;
use pyo3::sync::GILOnceCell;
use pyo3::exceptions::PySystemError;
use std::path::Path;

// mod pumas;
mod bindings;
mod simulation;
mod utils;


static PREFIX: GILOnceCell<String> = GILOnceCell::new();

fn set_prefix(py: Python) -> PyResult<()> {
    let filename = match get_dylib_path() {
        Some(path) => path
                        .to_string_lossy()
                        .to_string(),
        None => return Err(PySystemError::new_err("could not resolve module path")),
    };
    let prefix = match Path::new(&filename).parent() {
        None => ".",
        Some(path) => path.to_str().unwrap(),
    };
    PREFIX
        .set(py, prefix.to_string()).unwrap();
    Ok(())
}

#[pymodule]
fn mulder(module: &Bound<PyModule>) -> PyResult<()> {
    let py = module.py();

    // Set the package prefix.
    set_prefix(py)?;

    // Register the C error handlers.
    utils::error::initialise();

    // Initialise the materials.
    simulation::materials::initialise(py)?;

    // Register class object(s).
    module.add_class::<simulation::Fluxmeter>()?;

    // Register function(s).
    module.add_function(wrap_pyfunction!(simulation::physics::compute, module)?)?;

    // Register constants.
    let default_cache = utils::cache::default_path()
        .and_then(|cache| cache.into_pyobject(py).map(|cache| cache.unbind()))
        .unwrap_or_else(|_| py.None());
    module.add("DEFAULT_CACHE", default_cache)?;

    Ok(())
}
