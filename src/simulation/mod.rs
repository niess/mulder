use crate::utils::error::Error;
use crate::utils::error::ErrorKind::TypeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString};

pub mod materials;
pub mod physics;


#[pyclass(module="mulder")]
pub struct Fluxmeter {
    /// The Monte Carlo materials.
    #[pyo3(get)]
    materials: Py<materials::Materials>,
    /// The Monte Carlo physics models.
    #[pyo3(get)]
    physics: Py<physics::Physics>,
}

unsafe impl Send for Fluxmeter {}

#[derive(FromPyObject)]
enum MaterialsArg<'py> {
    Materials(Bound<'py, materials::Materials>),
    String(String),
}

#[pymethods]
impl Fluxmeter {
    #[pyo3(signature=(/, *, **kwargs))]
    #[new]
    pub fn new<'py>(
        py: Python<'py>,
        kwargs: Option<&Bound<'py, PyDict>>,
    ) -> PyResult<Py<Self>> {
        let (materials, physics_kwargs) = match kwargs {
            None => (None, None),
            Some(kwargs) => {
                let extract = |fields: &[&str]| -> PyResult<Option<Bound<'py, PyDict>>> {
                    let mut result = None;
                    for field in fields {
                        if let Some(value) = kwargs.get_item(field)? {
                            result
                                .get_or_insert_with(|| {
                                    PyDict::new_bound(py)
                                })
                                .set_item(field, value)?;
                            kwargs.del_item(field)?;
                        }
                    }
                    Ok(result)
                };
                let physics_kwargs = extract(
                    &["bremsstrahlung", "pair_production", "photonuclear"]
                )?;
                let materials = match kwargs.get_item("materials")? {
                    Some(materials) => {
                        let materials: String = materials.extract()
                            .map_err(|_| {
                                let tp = materials.get_type();
                                let why = format!("expected a 'string', found a '{:?}'", tp);
                                Error::new(TypeError)
                                    .what("materials")
                                    .why(&why)
                                    .to_err()
                            })?;
                        kwargs.del_item("materials")?;
                        Some(materials)
                    },
                    None => None,
                };
                (materials, physics_kwargs)
            },
        };

        let materials = Py::new(py,
            materials::Materials::new(py, materials.as_ref().map(|x| x.as_str()))?
        )?;
        let physics = physics::Physics::new(py, physics_kwargs.as_ref())?;

        let fluxmeter = Self { materials, physics };
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
    fn set_materials(&mut self, py: Python, materials: Option<MaterialsArg>) -> PyResult<()> {
        let materials = materials
            .unwrap_or(MaterialsArg::String(materials::DEFAULT_MATERIALS.to_string()));
        match materials {
            MaterialsArg::String(materials) => {
                self.materials = Py::new(py,
                    materials::Materials::new(py, Some(materials.as_str()))?
                )?;
            },
            MaterialsArg::Materials(materials) => {
                self.materials = materials.unbind();
            },
        }
        Ok(())
    }

    #[setter]
    fn set_physics(&mut self, value: &Bound<physics::Physics>) {
        // XXX reset the pumas context.
        self.physics = value.clone().unbind();
    }

    /// Compute the muon flux.
    #[pyo3(signature=(states, /))]
    fn flux(&mut self, states: &Bound<PyAny>) -> PyResult<()> {
        // Configure physics, geometry, samplers etc.
        let py = states.py();
        let mut physics = self.physics.bind(py).borrow_mut();
        let materials = self.materials.bind(py).borrow();
        if physics.apply(py, &materials)? {
            // XXX Reset the pumas context.
        }

        Ok(())
    }
}
