use crate::geometry::Geometry;
use crate::utils::io::PathString;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString, PyTuple};

pub mod materials;
pub mod physics;
pub mod reference;


#[pyclass(module="mulder")]
pub struct Fluxmeter {
    /// The Monte Carlo geometry.
    #[pyo3(get)]
    geometry: Py<Geometry>,

    /// The Monte Carlo physics.
    #[pyo3(get, set)]
    physics: Py<physics::Physics>,

    /// The reference flux.
    #[pyo3(get)]
    reference: Py<reference::Reference>,
}

unsafe impl Send for Fluxmeter {}

#[derive(FromPyObject)]
enum ReferenceLike {
    Model(PathString),
    Object(Py<reference::Reference>)
}

#[pymethods]
impl Fluxmeter {
    #[pyo3(signature=(*layers, **kwargs))]
    #[new]
    pub fn new<'py>(
        layers: &Bound<'py, PyTuple>,
        kwargs: Option<&Bound<'py, PyDict>>,
    ) -> PyResult<Py<Self>> {
        let py = layers.py();
        let extract_field = |field: &str| -> PyResult<Option<Bound<'py, PyAny>>> {
            match kwargs {
                Some(kwargs) => {
                    let value = kwargs.get_item(field)?;
                    if !value.is_none() {
                            kwargs.del_item(field)?;
                    }
                    Ok(value)
                },
                None => Ok(None),
            }
        };
        let extract_kwargs = |fields: &[&str]| -> PyResult<Option<Bound<'py, PyDict>>> {
            match kwargs {
                Some(kwargs) => {
                    let mut result = None;
                    for field in fields {
                        if let Some(value) = kwargs.get_item(field)? {
                            result
                                .get_or_insert_with(|| PyDict::new(py))
                                .set_item(field, value)?;
                            kwargs.del_item(field)?;
                        }
                    }
                    Ok(result)
                },
                None => Ok(None),
            }
        };

        let geometry = {
            let geometry_kwargs = extract_kwargs(
                &["atmosphere",]
            )?;
            let geometry = match extract_field("geometry")? {
                Some(geometry) => if layers.is_empty() && geometry_kwargs.is_none() {
                    let geometry: Bound<Geometry> = geometry.extract()?;
                    geometry
                } else {
                    unimplemented!()
                },
                None => {
                    let geometry = Geometry::new(layers, None)?;
                    let geometry = Bound::new(py, geometry)?;
                    if let Some(kwargs) = geometry_kwargs {
                        for (key, value) in kwargs.iter() {
                            let key: Bound<PyString> = key.extract()?;
                            geometry.setattr(key, value)?
                        }
                    }
                    geometry
                }
            };
            geometry.unbind()
        };

        let physics = {
            let mut physics_kwargs = extract_kwargs(
                &["bremsstrahlung", "pair_production", "photonuclear"]
            )?;
            if let Some(kwargs) = kwargs {
                if let Some(materials) = kwargs.get_item("materials")? {
                    kwargs.del_item("materials")?;
                    match physics_kwargs.as_mut() {
                        None => {
                            let dict = PyDict::new(py);
                            dict.set_item("materials", materials)?;
                            physics_kwargs.replace(dict);
                        },
                        Some(physics_kwargs) => {
                            physics_kwargs.set_item("materials", materials)?;
                        }
                    }
                }
            }
            match extract_field("physics")? {
                Some(physics) => if physics_kwargs.is_none() {
                    let physics: Py<physics::Physics> = physics.extract()?;
                    physics
                } else {
                    unimplemented!()
                },
                None => physics::Physics::new(py, physics_kwargs.as_ref())?,
            }
        };

        let reference = reference::Reference::new(None, None)?;
        let reference = Py::new(py, reference)?;

        let fluxmeter = Self { geometry, physics, reference };
        let fluxmeter = Bound::new(py, fluxmeter)?;

        if let Some(kwargs) = kwargs {
            for (key, value) in kwargs.iter() {
                let key: Bound<PyString> = key.extract()?;
                fluxmeter.setattr(key, value)?
            }
        }

        Ok(fluxmeter.unbind())
    }

    #[setter]
    fn set_geometry(&mut self, value: &Bound<Geometry>) {
        if !self.geometry.is(value) {
            self.geometry = value.clone().unbind();
        }
    }

    #[setter]
    fn set_reference(&mut self, py: Python, value: ReferenceLike) -> PyResult<()> {
        match value {
            ReferenceLike::Model(model) => {
                let model = reference::ModelArg::Path(model);
                let reference = reference::Reference::new(Some(model), None)?;
                self.reference = Py::new(py, reference)?;
            },
            ReferenceLike::Object(reference) => if !self.reference.is(&reference) {
                self.reference = reference;
            }
        }
        Ok(())
    }

    /// Compute the muon flux.
    #[pyo3(signature=(states, /))]
    fn flux(&mut self, states: &Bound<PyAny>) -> PyResult<()> {
        // Configure physics, geometry, samplers etc.
        let py = states.py();
        let mut physics = self.physics.bind(py).borrow_mut();
        physics.compile(py, None)?;

        Ok(())
    }
}
