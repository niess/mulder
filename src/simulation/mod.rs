use crate::bindings::{turtle, pumas};
use crate::utils::coordinates::{GeographicCoordinates, HorizontalCoordinates};
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::TypeError;
use crate::geometry::Geometry;
use crate::geometry::atmosphere::Atmosphere;
use crate::geometry::layer::Layer;
use crate::utils::io::PathString;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString, PyTuple};
use std::ffi::{c_uint, c_void};
use std::ops::DerefMut;
use std::ptr::{null, null_mut};
use std::pin::Pin;

pub mod materials;
pub mod physics;
pub mod random;
pub mod reference;


#[pyclass(module="mulder")]
pub struct Fluxmeter {
    /// The Monte Carlo geometry.
    #[pyo3(get)]
    geometry: Py<Geometry>,

    /// The Monte Carlo physics.
    #[pyo3(get)]
    physics: Py<physics::Physics>,

    /// The pseudo-random stream.
    #[pyo3(get, set)]
    random: Py<random::Random>,

    /// The reference flux.
    #[pyo3(get)]
    reference: Py<reference::Reference>,

    layers_stepper: *mut turtle::Stepper,
    opensky_stepper: *mut turtle::Stepper,
    atmosphere_medium: Medium,
    layers_media: Vec<Medium>,
    use_external_layer: bool, // XXX initialise this field.
}

unsafe impl Send for Fluxmeter {}
unsafe impl Sync for Fluxmeter {}

struct Medium (Pin<Box<MediumData>>);

#[repr(C)]
struct MediumData {
    medium: pumas::Medium,
    density: f64,
}

struct State (Pin<Box<StateData>>);

#[repr(C)]
struct StateData {
    state: pumas::State,
    geographic: GeographicCoordinates, // XXX Update in medium callback.
    atmosphere: *const Atmosphere,
    fluxmeter: *mut Fluxmeter,
}

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
                    let err = Error::new(TypeError)
                        .what("geometry argument(s)")
                        .why("geometry already provided as **kwargs")
                        .to_err();
                    return Err(err)
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
                    let err = Error::new(TypeError)
                        .what("physics argument(s)")
                        .why("physics already provided as **kwargs")
                        .to_err();
                    return Err(err)
                },
                None => physics::Physics::new(py, physics_kwargs.as_ref())?,
            }
        };

        let random = match extract_field("random")? {
            Some(random) => {
                let random: Py<random::Random> = random.extract()?;
                random
            },
            None => {
                let random = random::Random::new(None, None)?;
                Py::new(py, random)?
            },
        };

        let reference = reference::Reference::new(None, None)?;
        let reference = Py::new(py, reference)?;

        let layers_stepper = null_mut();
        let opensky_stepper = null_mut();
        let layers_media = Vec::new();
        let atmosphere_medium = Medium::default();
        let use_external_layer = false;

        let fluxmeter = Self {
            geometry, physics, random, reference, layers_stepper, opensky_stepper,
            atmosphere_medium, layers_media, use_external_layer,
        };
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
            self.reset();
        }
    }

    #[setter]
    fn set_physics(&mut self, value: &Bound<physics::Physics>) {
        if !self.physics.is(value) {
            self.physics = value.clone().unbind();
            self.reset();
        }
    }

    #[setter]
    fn set_reference(&mut self, py: Python, value: ReferenceLike) -> PyResult<()> {
        match value {
            ReferenceLike::Model(model) => {
                let model = reference::ModelArg::Path(model);
                let reference = reference::Reference::new(Some(model), None)?;
                self.reference = Py::new(py, reference)?;
                self.reset();
            },
            ReferenceLike::Object(reference) => if !self.reference.is(&reference) {
                self.reference = reference;
                self.reset();
            }
        }
        Ok(())
    }

    /// Transport state(s) to the reference flux.
    #[pyo3(signature=(states, /))]
    fn transport(&mut self, states: &Bound<PyAny>) -> PyResult<()> {
        // Configure physics, geometry, samplers etc.
        let py = states.py();
        let mut physics = self.physics.bind(py).borrow_mut();
        physics.compile(py, None)?;

        let geometry = self.geometry.bind(py).borrow();
        let reference = self.reference.bind(py).borrow();
        self.create_geometry(py, &geometry, &physics, &reference)?;

        let mut random = self.random.bind(py).borrow_mut();
        let context = unsafe { &mut *physics.context };
        context.user_data = random.deref_mut() as *mut random::Random as *mut c_void;
        context.random = Some(uniform01);
        context.medium = Some(layers_geometry);

        Ok(())
    }
}

impl Fluxmeter {
    const TOP_MATERIAL: &str = "Air";

    fn create_geometry(
        &mut self,
        py: Python,
        geometry: &Geometry,
        physics: &physics::Physics,
        reference: &reference::Reference,
    ) -> PyResult<()> {
        if self.layers_stepper == null_mut() {
            // Map media.
            for layer in &geometry.layers {
                let layer = layer.bind(py).borrow();
                let medium = Medium::uniform(&layer, physics)?;
                self.layers_media.push(medium);
            }
            self.atmosphere_medium = Medium::atmosphere(physics)?;

            // Create steppers.
            let zref = match reference.altitude {
                reference::Altitude::Scalar(z) => (z, z),
                reference::Altitude::Range(z) => z,
            };
            let steppers = geometry.create_steppers(py, Some(zref))?;
            self.layers_stepper = steppers.0;
            self.opensky_stepper = steppers.1;
        }
        Ok(())
    }

    fn reset(&mut self) {
        if self.layers_stepper != null_mut() {
            unsafe {
                turtle::stepper_destroy(&mut self.layers_stepper); 
                turtle::stepper_destroy(&mut self.opensky_stepper); 
            }
            self.layers_media.clear();
            self.use_external_layer = false;
        }
    }
}

impl Drop for Fluxmeter {
    fn drop(&mut self) {
        self.reset()
    }
}

#[no_mangle]
extern "C" fn atmosphere_locals(
    _medium: *mut pumas::Medium,
    state: *mut pumas::State,
    locals: *mut pumas::Locals,
) -> f64 {
    let state = unsafe { &*(state as *const State) };
    let atmosphere = unsafe { &*state.0.atmosphere };
    let locals = unsafe { &mut*locals };
    let r = atmosphere.compute_density(state.0.geographic.altitude);
    locals.density = r.0;

    const LAMBDA_MAX: f64 = 1E+09;
    if r.1 < LAMBDA_MAX {
        let direction = HorizontalCoordinates::from_ecef(
            &state.0.state.direction,
            &state.0.geographic,
        );
        let c = (direction.elevation.abs() * std::f64::consts::PI / 180.0).sin().max(0.1);
        (r.1 / c).min(LAMBDA_MAX)
    } else {
        LAMBDA_MAX
    }
}

#[no_mangle]
extern "C" fn uniform_locals(
    medium: *mut pumas::Medium,
    _state: *mut pumas::State,
    locals: *mut pumas::Locals,
) -> f64 {
    let medium = unsafe { &*(medium as *const Medium) };
    let locals = unsafe { &mut*locals };
    locals.density = medium.0.density;
    0.0
}

#[no_mangle]
extern "C" fn uniform01(context: *mut pumas::Context) -> f64 {
    let context = unsafe { &*context };
    let random = unsafe { &mut*(context.user_data as *mut random::Random) };
    random.open01()
}

#[no_mangle]
extern "C" fn layers_geometry(
    _context: *mut pumas::Context,
    state: *mut pumas::State,
    medium_ptr: *mut *mut pumas::Medium,
    step_ptr: *mut f64,
) -> c_uint {
    let state = unsafe { &mut *(state as *mut State) };
    let fluxmeter = unsafe { &mut *state.0.fluxmeter };

    let mut step = 0.0;
    let mut index = [ -1; 2 ];
    unsafe {
        turtle::stepper_step(
            fluxmeter.layers_stepper,
            state.0.state.position.as_mut_ptr(),
            null(),
            &mut state.0.geographic.latitude,
            &mut state.0.geographic.longitude,
            &mut state.0.geographic.altitude,
            null_mut(),
            &mut step,
            index.as_mut_ptr(),
        );
    }

    const EPS: f64 = f32::EPSILON as f64;
    if step_ptr != null_mut() {
        let step_ptr = unsafe { &mut*step_ptr };
        *step_ptr = if step <= EPS { EPS } else { step };
    }

    if medium_ptr != null_mut() {
        let medium_ptr = unsafe { &mut*medium_ptr };
        let n = fluxmeter.layers_media.len();
        let i = index[0] as usize;
        if (i >= 1) && (i <= n) {
            *medium_ptr = &mut fluxmeter.layers_media[i - 1].0.medium as *mut pumas::Medium;
        } else if (i == n + 1) || (fluxmeter.use_external_layer && (i == n + 2)) {
            *medium_ptr = &mut fluxmeter.atmosphere_medium.0.medium as *mut pumas::Medium;
        } else {
            *medium_ptr = null_mut();
        }
    }

    pumas::STEP_CHECK
}

impl Medium {
    fn atmosphere(physics: &physics::Physics) -> PyResult<Self> {
        Self::new(Fluxmeter::TOP_MATERIAL, None, Some(atmosphere_locals), physics)
    }

    fn new(
        material: &str,
        density: Option<f64>,
        locals: pumas::LocalsCallback,
        physics: &physics::Physics,
    ) -> PyResult<Self> {
        let material = physics.material_index(material)?;
        let density = density.unwrap_or(-1.0);
        let medium = pumas::Medium { material, locals };
        let data = MediumData { medium, density };
        let medium = Self(Box::pin(data));
        Ok(medium)
    }

    fn uniform(layer: &Layer, physics: &physics::Physics) -> PyResult<Self> {
        Self::new(layer.material.as_str(), layer.density, Some(uniform_locals), physics)
    }
}

impl Default for Medium {
    fn default() -> Self {
        let medium = pumas::Medium { material: -1, locals: None };
        let density = 0.0;
        let data = MediumData { medium, density };
        Self(Box::pin(data))
    }
}
