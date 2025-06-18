use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString};

pub mod materials;
pub mod physics;
pub mod reference;


#[pyclass(module="mulder")]
pub struct Fluxmeter {
    /// The Monte Carlo physics.
    #[pyo3(get)]
    physics: Py<physics::Physics>,
}

unsafe impl Send for Fluxmeter {}

#[pymethods]
impl Fluxmeter {
    #[pyo3(signature=(/, *, **kwargs))]
    #[new]
    pub fn new<'py>(
        py: Python<'py>,
        kwargs: Option<&Bound<'py, PyDict>>,
    ) -> PyResult<Py<Self>> {
        let physics_kwargs = match kwargs {
            None => None,
            Some(kwargs) => {
                let extract = |fields: &[&str]| -> PyResult<Option<Bound<'py, PyDict>>> {
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
                };
                let mut physics_kwargs = extract(
                    &["bremsstrahlung", "pair_production", "photonuclear"]
                )?;
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
                physics_kwargs
            },
        };

        let physics = physics::Physics::new(py, physics_kwargs.as_ref())?;
        let fluxmeter = Self { physics };
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
    fn set_physics(&mut self, value: &Bound<physics::Physics>) {
        if !self.physics.is(value) {
            self.physics = value.clone().unbind();
        }
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
