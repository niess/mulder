use process_path::get_dylib_path;
use pyo3::prelude::*;
use pyo3::sync::GILOnceCell;
use pyo3::exceptions::PySystemError;
use std::path::Path;

// mod pumas;
mod bindings;
mod camera;
mod geometry;
mod materials;
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

#[pyclass(frozen, module="mulder")]
struct Config ();

#[pymodule(name="_core")]
fn mulder(module: &Bound<PyModule>) -> PyResult<()> {
    let py = module.py();

    // Set the package prefix.
    set_prefix(py)?;

    // Register the C error handlers.
    utils::error::initialise();

    // Initialise the numpy interface.
    utils::numpy::initialise(py)?;

    // Register class object(s).
    module.add_class::<camera::Camera>()?;
    module.add_class::<geometry::earth::EarthGeometry>()?;
    module.add_class::<geometry::earth::Grid>()?;
    module.add_class::<geometry::earth::Layer>()?;
    module.add_class::<geometry::external::ExternalGeometry>()?;
    module.add_class::<simulation::Fluxmeter>()?;
    module.add_class::<simulation::atmosphere::Atmosphere>()?;
    module.add_class::<simulation::geomagnet::EarthMagnet>()?;
    module.add_class::<simulation::physics::Physics>()?;
    module.add_class::<simulation::random::Random>()?;
    module.add_class::<simulation::reference::Reference>()?;
    module.add_class::<simulation::states::GeographicStates>()?;
    module.add_class::<simulation::states::LocalStates>()?;
    module.add_class::<utils::coordinates::LocalFrame>()?;

    // Set config wrapper.
    module.add("config", Config())?;

    // Set the materials submodule.
    let materials = PyModule::new(py, "materials")?;
    materials.add_class::<materials::Element>()?;
    materials.add_class::<materials::Material>()?;
    materials.add_function(wrap_pyfunction!(materials::load, &materials)?)?;
    module.add_submodule(&materials)?;

    // Set the picture submodule.
    let picture = PyModule::new(py, "picture")?;
    picture.add("materials", camera::picture::default_materials(py)?)?;
    picture.add_class::<camera::picture::AmbientLight>()?;
    picture.add_class::<camera::picture::ColourMap>()?;
    picture.add_class::<camera::picture::DirectionalLight>()?;
    picture.add_class::<camera::picture::OpticalProperties>()?;
    picture.add_class::<camera::picture::RawPicture>()?;
    picture.add_class::<camera::picture::SkyProperties>()?;
    picture.add_class::<camera::picture::SunLight>()?;
    module.add_submodule(&picture)?;

    Ok(())
}


#[allow(non_snake_case)]
#[pymethods]
impl Config {
    #[getter]
    fn get_DEFAULT_CACHE(&self, py: Python) -> PyObject {
        utils::cache::default_path()
            .and_then(|cache| cache.into_pyobject(py).map(|cache| cache.unbind()))
            .unwrap_or_else(|_| py.None())
    }

    #[getter]
    fn get_NOTIFY(&self) -> bool {
        utils::notify::get()
    }

    #[setter]
    fn set_NOTIFY(&self, value: bool) {
        utils::notify::set(value)
    }

    #[getter]
    fn get_PREFIX(&self, py: Python) -> &String {
        PREFIX.get(py).unwrap()
    }

    #[getter]
    fn get_VERSION(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
}
