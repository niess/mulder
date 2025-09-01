use crate::bindings::{turtle, pumas};
use crate::utils::coordinates::{GeographicCoordinates, HorizontalCoordinates, LocalFrame};
use crate::utils::error::{self, Error};
use crate::utils::error::ErrorKind::{TypeError, ValueError};
use crate::utils::extract::{Extractor, Field, Name};
use crate::utils::numpy::{ArrayMethods, Dtype, NewArray};
use crate::utils::traits::MinMax;
use crate::geometry::{Doublet, Geometry, GeometryStepper};
use crate::geometry::atmosphere::Atmosphere;
use crate::geometry::magnet::Magnet;
use crate::geometry::layer::Layer;
use crate::utils::convert::TransportMode;
use crate::utils::io::PathString;
use crate::utils::notify::{Notifier, NotifyArg};
use pyo3::prelude::*;
use pyo3::sync::GILOnceCell;
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

    /// The transport mode.
    #[pyo3(get, set)]
    mode: TransportMode,

    /// The Monte Carlo physics.
    #[pyo3(get)]
    physics: Py<physics::Physics>,

    /// The pseudo-random stream.
    #[pyo3(get, set)]
    random: Py<random::Random>,

    /// The reference flux.
    #[pyo3(get)]
    reference: Py<reference::Reference>,

    steppers: Doublet<GeometryStepper>,
    atmosphere_medium: Medium,
    layers_media: Vec<Medium>,
}

unsafe impl Send for Fluxmeter {}
unsafe impl Sync for Fluxmeter {}

struct Medium (Pin<Box<MediumData>>); // XXX Change to RawMedium?

#[repr(C)]
struct MediumData {
    medium: pumas::Medium, // XXX change to pumas?.
    density: f64,
    layer: usize, // XXX change to medium/index.
}

#[repr(C)]
struct Agent<'a> {
    state: pumas::State,
    geographic: GeographicCoordinates,
    horizontal: HorizontalCoordinates,

    atmosphere: &'a Atmosphere,
    fluxmeter: &'a mut Fluxmeter,
    geometry: &'a Geometry,
    magnet: Option<&'a mut Magnet>,
    physics: &'a physics::Physics,
    reference: &'a reference::Reference,
    context: &'a mut pumas::Context,

    use_external_layer: bool,
    magnet_field: [f64; 3],
    magnet_position: [f64; 3],
    use_magnet: bool,
}

#[derive(IntoPyObject)]
enum StatesArray<'py> {
    Flavoured(NewArray<'py, FlavouredState>),
    Unflavoured(NewArray<'py, UnflavouredState>),
}

#[repr(C)]
#[derive(Clone)]
struct FlavouredState {
    pid: i32,
    energy: f64,
    latitude: f64,
    longitude: f64,
    altitude: f64,
    azimuth: f64,
    elevation: f64,
    weight: f64,
}

#[repr(C)]
#[derive(Clone)]
struct UnflavouredState {
    energy: f64,
    latitude: f64,
    longitude: f64,
    altitude: f64,
    azimuth: f64,
    elevation: f64,
    weight: f64,
}

enum GeometryTag {
    Layers,
    Opensky,
}

#[derive(FromPyObject)]
enum ReferenceLike {
    Model(PathString),
    Object(Py<reference::Reference>)
}

#[derive(Clone, Copy, PartialEq)]
enum Particle {
    Anti,
    Any,
    Muon,
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
                &["atmosphere", "magnet"]
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
                    let geometry = Geometry::new(layers, None, None)?;
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

        let mode = TransportMode::default();

        let reference = reference::Reference::new(None, None)?;
        let reference = Py::new(py, reference)?;

        let steppers = Default::default();
        let layers_media = Vec::new();
        let atmosphere_medium = Medium::default();

        let fluxmeter = Self {
            geometry, mode, physics, random, reference, steppers, atmosphere_medium, layers_media,
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

    /// Compute flux estimate(s).
    #[pyo3(signature=(states=None, /, *, events=None, notify=None, **kwargs))]
    fn __call__<'py>(
        &mut self,
        py: Python<'py>,
        states: Option<&Bound<PyAny>>,
        events: Option<usize>,
        notify: Option<NotifyArg>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<NewArray<'py, f64>> {
        // Extract states.
        let states = Extractor::from_args(
            [
                Field::maybe_int(Name::Pid),
                Field::float(Name::Energy),
                Field::float(Name::Latitude),
                Field::float(Name::Longitude),
                Field::float(Name::Altitude),
                Field::float(Name::Azimuth),
                Field::float(Name::Elevation),
                Field::maybe_float(Name::Weight),
            ],
            states,
            kwargs
        )?;
        let size = states.size();
        let mut shape = states.shape();

        // Uniformise the events parameter.
        let events = events.filter(|events| match self.mode {
            TransportMode::Continuous => false,
            _ => if *events > 1  {
                shape.push(2);
                true
            } else {
                false
            },
        });

        // Configure physics, geometry, samplers etc. (XXX pack with a macro?)
        error::clear();
        let mut geometry = self.geometry.bind(py).borrow_mut();
        let atmosphere = geometry.atmosphere.bind(py).borrow();
        let mut magnet = geometry.magnet.as_mut().map(|magnet| magnet.bind(py).borrow_mut());
        let mut physics = self.physics.bind(py).borrow_mut();
        let mut random = self.random.bind(py).borrow_mut();
        let reference = self.reference.bind(py).borrow();
        let mut pinned = Box::pin(Agent::new(
            py,
            &atmosphere,
            self,
            &geometry,
            magnet.as_deref_mut(),
            &mut physics,
            &mut random,
            &reference,
        )?);
        let agent: &mut Agent = &mut pinned.deref_mut();
        let mut array = NewArray::zeros(py, shape)?;
        let flux = array.as_slice_mut();

        // Setup notifications.
        let notifier = {
            let steps = size * events.unwrap_or_else(|| 1);
            Notifier::from_arg(notify, steps, "computing flux")
        };

        // Loop over states.
        for i in 0..size {
            const WHY: &str = "while computing flux(es)";
            if (i % 100) == 0 { error::check_ctrlc(WHY)? }

            let state = FlavouredState::from_transport(&states, i)?;
            if (state.weight <= 0.0) || (state.energy <= 0.0) { continue }

            match &agent.fluxmeter.mode {
                TransportMode::Continuous => {
                    const HIGH_ENERGY: f64 = 1E+02; // XXX Disable in locals as well?
                    flux[i] = if agent.magnet.is_none() || (state.energy >= HIGH_ENERGY) {
                        let particle = if states.contains(Name::Pid) {
                            Particle::from_pid(state.pid)?
                        } else {
                            Particle::Any
                        };
                        agent.set_state(&state)?;
                        agent.flux(particle)?
                    } else {
                        let mut fi = 0.0;
                        for particle in [Particle::Muon, Particle::Anti] {
                            agent.set_state(&state)?;
                            agent.state.charge = particle.charge();
                            fi += agent.flux(particle)?;
                        }
                        fi
                    };
                    notifier.tic();
                },
                _ => match events {
                    Some(events) => {
                        let mut s1 = 0.0;
                        let mut s2 = 0.0;
                        for j in 0..events {
                            agent.set_state(&state)?;
                            if !states.contains(Name::Pid) { agent.randomise_charge(); }
                            let particle = Particle::from_charge(agent.state.charge);
                            let fij = agent.flux(particle)?;
                            s1 += fij;
                            s2 += fij.powi(2);

                            let index = i * events + j;
                            if (index % 100) == 0 { error::check_ctrlc(WHY)? }

                            notifier.tic();
                        }
                        let n = events as f64;
                        s1 /= n;
                        s2 /= n;
                        flux[2 * i] = s1;
                        flux[2 * i + 1] = ((s2 - s1.powi(2)).max(0.0) / n).sqrt();
                    },
                    None => {
                        agent.set_state(&state)?;
                        if !states.contains(Name::Pid) { agent.randomise_charge(); }
                        let particle = Particle::from_charge(agent.state.charge);
                        flux[i] = agent.flux(particle)?;
                        notifier.tic();
                    },
                },
            }
        }
        drop(notifier);

        Ok(array)
    }

    /// Compute grammage(s) along line of sight(s).
    #[pyo3(signature=(states=None, /, *, notify=None, sum=None, **kwargs))]
    fn grammage<'py>(
        &mut self,
        py: Python<'py>,
        states: Option<&Bound<PyAny>>,
        notify: Option<NotifyArg>,
        sum: Option<bool>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<NewArray<'py, f64>> {
        let states = Extractor::from_args(
            [
                Field::float(Name::Latitude),
                Field::float(Name::Longitude),
                Field::float(Name::Altitude),
                Field::float(Name::Azimuth),
                Field::float(Name::Elevation),
            ],
            states,
            kwargs
        )?;
        let size = states.size();
        let shape = states.shape();
        let sum = sum.unwrap_or_else(|| false);

        // Configure physics, geometry, samplers etc.
        error::clear();
        let n = self.layers_media.len() + 1;
        let mut geometry = self.geometry.bind(py).borrow_mut();
        let atmosphere = geometry.atmosphere.bind(py).borrow();
        let mut magnet = geometry.magnet.as_mut().map(|magnet| magnet.bind(py).borrow_mut());
        let mut physics = self.physics.bind(py).borrow_mut();
        let mut random = self.random.bind(py).borrow_mut();
        let reference = self.reference.bind(py).borrow();
        let mut pinned = Box::pin(Agent::new(
            py,
            &atmosphere,
            self,
            &geometry,
            magnet.as_deref_mut(),
            &mut physics,
            &mut random,
            &reference,
        )?);
        let agent: &mut Agent = &mut pinned.deref_mut();

        // Setup notifications.
        let notifier = Notifier::from_arg(notify, size, "computing grammage");

        // Loop over states.
        let mut array = if sum {
            NewArray::empty(py, shape)?
        } else {
            let mut shape = shape.clone();
            shape.push(n);
            NewArray::empty(py, shape)?
        };
        let result = array.as_slice_mut();
        for i in 0..size {
            if (i % 100) == 0 { error::check_ctrlc("while computing grammage(s)")?; }

            let state = FlavouredState::from_grammage(&states, i)?;
            agent.set_state(&state)?;
            let grammage = agent.grammage()?;
            if sum {
                result[i] = grammage.iter().sum();
            } else {
                for j in 0..n {
                    result[i * n + j] = grammage[j];
                }
            }

            notifier.tic();
        }

        Ok(array)
    }

    /// Transport state(s) to the reference flux.
    #[pyo3(signature=(states=None, /, *, events=None, notify=None, **kwargs))]
    fn transport<'py>(
        &mut self,
        py: Python<'py>,
        states: Option<&Bound<PyAny>>,
        events: Option<usize>,
        notify: Option<NotifyArg>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<StatesArray<'py>> {
        let states = Extractor::from_args(
            [
                Field::maybe_int(Name::Pid),
                Field::float(Name::Energy),
                Field::float(Name::Latitude),
                Field::float(Name::Longitude),
                Field::float(Name::Altitude),
                Field::float(Name::Azimuth),
                Field::float(Name::Elevation),
                Field::maybe_float(Name::Weight),
            ],
            states,
            kwargs
        )?;
        let size = states.size();
        let mut shape = states.shape();

        // Configure physics, geometry, samplers etc.
        error::clear();
        let mut geometry = self.geometry.bind(py).borrow_mut();
        let atmosphere = geometry.atmosphere.bind(py).borrow();
        let mut magnet = geometry.magnet.as_mut().map(|magnet| magnet.bind(py).borrow_mut());
        if magnet.is_some() && !states.contains(Name::Pid) {
            let err = Error::new(TypeError)
                .what("states")
                .why("a pid is required for a magnetized geometry")
                .to_err();
            return Err(err)
        }
        let mut physics = self.physics.bind(py).borrow_mut();
        let mut random = self.random.bind(py).borrow_mut();
        let reference = self.reference.bind(py).borrow();
        let mut pinned = Box::pin(Agent::new(
            py,
            &atmosphere,
            self,
            &geometry,
            magnet.as_deref_mut(),
            &mut physics,
            &mut random,
            &reference,
        )?);
        let agent: &mut Agent = &mut pinned.deref_mut();

        let events = events
            .map(|events| {
                shape.push(events);
                events
            })
            .unwrap_or_else(|| 1);
        let array = if states.contains(Name::Pid) {
            StatesArray::Flavoured(NewArray::empty(py, shape)?)
        } else {
            StatesArray::Unflavoured(NewArray::empty(py, shape)?)
        };

        // Setup notifications.
        let notifier = Notifier::from_arg(notify, size * events, "transporting muon(s)");

        for i in 0..size {
            let state = FlavouredState::from_transport(&states, i)?;
            if (state.weight <= 0.0) || (state.energy <= 0.0) { continue }
            for j in 0..events {
                let index = i * events + j;
                if (index % 100) == 0 { error::check_ctrlc("while transporting muon(s)")?; }

                agent.set_state(&state)?;
                agent.transport()?;

                match &array {
                    StatesArray::Flavoured(array) => array.set_item(
                        index,
                        agent.get_flavoured_state()
                    )?,
                    StatesArray::Unflavoured(array) => array.set_item(
                        index,
                        agent.get_unflavoured_state()
                    )?,
                }

                notifier.tic();
            }
        }

        Ok(array)
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
        if self.steppers.layers.stepper == null_mut() {
            // Map media.
            for (index, layer) in geometry.layers.iter().enumerate() {
                let layer = layer.bind(py).borrow();
                let medium = Medium::uniform(&layer, index, physics)?;
                self.layers_media.push(medium);
            }
            self.atmosphere_medium = Medium::atmosphere(geometry.layers.len(), physics)?;

            // Create steppers.
            let zref = reference.altitude.to_range();
            self.steppers = geometry.create_steppers(py, Some(zref))?;
        }
        Ok(())
    }

    fn reset(&mut self) {
        if self.steppers.layers.stepper != null_mut() {
            unsafe {
                turtle::stepper_destroy(&mut self.steppers.layers.stepper); 
                turtle::stepper_destroy(&mut self.steppers.opensky.stepper); 
            }
            self.layers_media.clear();
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
    let agent: &mut Agent = state.into();
    let density = agent.atmosphere.compute_density(agent.geographic.altitude);
    unsafe {
        (*locals).density = density.value;
    }

    const LAMBDA_MAX: f64 = 1E+09;
    let lambda = if density.lambda < LAMBDA_MAX {
        let direction = HorizontalCoordinates::from_ecef(
            &agent.state.direction,
            &agent.geographic,
        );
        let c = (direction.elevation.abs() * std::f64::consts::PI / 180.0).sin().max(0.1);
        (density.lambda / c).min(LAMBDA_MAX)
    } else {
        LAMBDA_MAX
    };

    if !agent.use_magnet {
        return lambda
    }

    // Get the local magnetic field.
    const UPDATE_RADIUS: f64 = 1E+03;
    let d2 = {
        let mut d2 = 0.0;
        for i in 0..3 {
            let tmp = agent.state.position[i] - agent.magnet_position[i];
            d2 += tmp * tmp;
        }
        d2
    };
    if d2 > UPDATE_RADIUS.powi(2) {
        // Get the local magnetic field (in ENU frame).
        let enu = agent.magnet.as_mut().unwrap().field(
            agent.geographic.latitude,
            agent.geographic.longitude,
            agent.geographic.altitude,
        ).unwrap();

        let frame = LocalFrame::new(agent.geographic, 0.0, 0.0);
        agent.magnet_field = frame.to_ecef_direction(&enu);
        agent.magnet_position = agent.state.position;
    }

    // Update the local magnetic field.
    unsafe {
        (*locals).magnet = agent.magnet_field;
    }

    let lambda_magnet = UPDATE_RADIUS / agent.context.accuracy;
    lambda.min(lambda_magnet)
}

#[no_mangle]
extern "C" fn uniform_locals(
    medium: *mut pumas::Medium,
    _state: *mut pumas::State,
    locals: *mut pumas::Locals,
) -> f64 {
    let medium: &MediumData = medium.into();
    unsafe {
        (*locals).density = medium.density;
    }
    0.0
}

#[no_mangle]
extern "C" fn uniform01(context: *mut pumas::Context) -> f64 {
    let random = unsafe { &mut*((*context).user_data as *mut random::Random) };
    random.open01()
}

#[no_mangle]
extern "C" fn layers_geometry(
    _context: *mut pumas::Context,
    state: *mut pumas::State,
    medium_ptr: *mut *mut pumas::Medium,
    step_ptr: *mut f64,
) -> c_uint {
    let agent: &mut Agent = state.into();
    let (step, layer) = agent.step(GeometryTag::Layers);

    if step_ptr != null_mut() {
        let step_ptr = unsafe { &mut*step_ptr };
        *step_ptr = if step <= Agent::EPSILON { Agent::EPSILON } else { step };
    }

    if medium_ptr != null_mut() {
        let medium_ptr = unsafe { &mut*medium_ptr };
        let n = agent.fluxmeter.layers_media.len();
        if (layer >= 1) && (layer <= n) {
            *medium_ptr = agent.fluxmeter.layers_media[layer - 1].as_mut_ptr();
        } else if (layer == n + 1) || (agent.use_external_layer && (layer == n + 2)) {
            *medium_ptr = agent.fluxmeter.atmosphere_medium.as_mut_ptr();
        } else {
            *medium_ptr = null_mut();
        }
    }

    pumas::STEP_CHECK
}

#[no_mangle]
extern "C" fn opensky_geometry(
    _context: *mut pumas::Context,
    state: *mut pumas::State,
    medium_ptr: *mut *mut pumas::Medium,
    step_ptr: *mut f64,
) -> c_uint {
    let agent: &mut Agent = state.into();
    let (step, layer) = agent.step(GeometryTag::Opensky);

    if step_ptr != null_mut() {
        let step_ptr = unsafe { &mut*step_ptr };
        *step_ptr = if step <= Agent::EPSILON { Agent::EPSILON } else { step };
    }

    if medium_ptr != null_mut() {
        let medium_ptr = unsafe { &mut*medium_ptr };
        if layer == 1 {
            *medium_ptr = agent.fluxmeter.atmosphere_medium.as_mut_ptr();
        } else {
            *medium_ptr = null_mut();
        }
    }

    pumas::STEP_CHECK
}

impl Medium {
    #[inline]
    fn as_mut_ptr(&mut self) -> *mut pumas::Medium {
        &mut self.0.medium
    }

    fn atmosphere(layer: usize, physics: &physics::Physics) -> PyResult<Self> {
        Self::new(Fluxmeter::TOP_MATERIAL, None, layer, Some(atmosphere_locals), physics)
    }

    fn new(
        material: &str,
        density: Option<f64>,
        layer: usize,
        locals: pumas::LocalsCallback,
        physics: &physics::Physics,
    ) -> PyResult<Self> {
        let material = physics.material_index(material)?;
        let density = density.unwrap_or(-1.0);
        let medium = pumas::Medium { material, locals };
        let data = MediumData { medium, density, layer };
        let medium = Self(Box::pin(data));
        Ok(medium)
    }

    fn uniform(layer: &Layer, layer_index: usize, physics: &physics::Physics) -> PyResult<Self> {
        Self::new(
            layer.material.as_str(), layer.density, layer_index, Some(uniform_locals), physics,
        )
    }
}

impl From<*mut pumas::Medium> for &MediumData {
    #[inline]
    fn from(value: *mut pumas::Medium) -> Self {
        unsafe { &*(value as *const MediumData) }
    }
}

impl Default for Medium {
    fn default() -> Self {
        let medium = pumas::Medium { material: -1, locals: None };
        let density = 0.0;
        let layer = 0;
        let data = MediumData { medium, density, layer };
        Self(Box::pin(data))
    }
}

impl<'a> Agent<'a> {
    const EPSILON: f64 = f32::EPSILON as f64;

    fn flux(&mut self, particle: Particle) -> PyResult<f64> {
        self.transport()?;
        let f = if self.state.weight <= 0.0 {
            0.0
        } else {
            let f = self.reference.flux(
                self.state.energy, self.horizontal.elevation, self.geographic.altitude
            );
            let f = match particle {
                Particle::Muon => f.muon,
                Particle::Anti => f.anti,
                Particle::Any => f.muon + f.anti,
            };
            f * self.state.weight
        };
        Ok(f)
    }

    fn get_flavoured_state(&self) -> FlavouredState {
        FlavouredState {
            pid: Particle::from_charge(self.state.charge).pid(),
            energy: self.state.energy,
            latitude: self.geographic.latitude,
            longitude: self.geographic.longitude,
            altitude: self.geographic.altitude,
            azimuth: self.horizontal.azimuth,
            elevation: self.horizontal.elevation,
            weight: self.state.weight,
        }
    }

    fn get_unflavoured_state(&self) -> UnflavouredState {
        UnflavouredState {
            energy: self.state.energy,
            latitude: self.geographic.latitude,
            longitude: self.geographic.longitude,
            altitude: self.geographic.altitude,
            azimuth: self.horizontal.azimuth,
            elevation: self.horizontal.elevation,
            weight: self.state.weight,
        }
    }

    fn grammage(&mut self) -> PyResult<Vec<f64>> {
        // Disable any magnetic field.
        self.use_magnet = false;

        // Configure the transport with Pumas.
        self.context.medium = Some(layers_geometry);
        self.context.mode.direction = pumas::MODE_BACKWARD;
        self.context.mode.energy_loss = pumas::MODE_DISABLED;
        self.context.mode.scattering = pumas::MODE_DISABLED;
        self.context.event = pumas::EVENT_MEDIUM;

        unsafe {
            turtle::stepper_reset(self.fluxmeter.steppers.layers.stepper);
        }

        // Compute the grammage.
        let n = self.fluxmeter.layers_media.len();
        let mut grammage = vec![0.0; n + 1];
        let mut last_grammage = 0.0;
        loop {
            let mut event = 0;
            let mut media: [*mut pumas::Medium; 2] = [null_mut(); 2];
            let rc = unsafe {
                pumas::context_transport(
                    self.context,
                    &mut self.state,
                    &mut event,
                    media.as_mut_ptr(),
                )
            };
            error::to_result(rc, None::<&str>)?;
            if media[0] == null_mut() { break }

            let medium: &MediumData = media[0].into();
            grammage[medium.layer] += self.state.grammage - last_grammage;
            last_grammage = self.state.grammage;

            if (event != pumas::EVENT_MEDIUM) || (media[1] == null_mut()) {
                break
            }
        }

        Ok(grammage)
    }

    fn new(
        py: Python,
        atmosphere: &'a Atmosphere,
        fluxmeter: &'a mut Fluxmeter,
        geometry: &'a Geometry,
        magnet: Option<&'a mut Magnet>,
        physics: &'a mut physics::Physics,
        mut random: &'a mut random::Random,
        reference: &'a reference::Reference,
    ) -> PyResult<Self> {
        // Configure physics and geometry.
        physics.update(py, &materials::Materials::default(py)?)?; // XXX get materials from the geometry.
        fluxmeter.create_geometry(py, &geometry, &physics, &reference)?;

        // Configure Pumas context.
        let context = physics.borrow_mut_context();
        context.user_data = random.deref_mut() as *mut random::Random as *mut c_void;
        context.random = Some(uniform01);

        // Initialise particles state.
        let state = pumas::State::default();
        let geographic = GeographicCoordinates::default();
        let horizontal = HorizontalCoordinates::default();

        let use_external_layer = false;
        let magnet_field = [0.0; 3];
        let magnet_position = [0.0; 3];
        let use_magnet = false;

        let agent = Self {
            state, geographic, horizontal, atmosphere, fluxmeter, geometry, magnet,
            physics, reference, context, use_external_layer, magnet_field, magnet_position,
            use_magnet,
        };
        Ok(agent)
    }

    fn random(&mut self) -> &mut random::Random {
        unsafe { &mut*((*self.context).user_data as *mut random::Random) }
    }

    #[inline]
    fn randomise_charge(&mut self) {
        self.state.charge = if self.random().open01() <= 0.5 { -1.0 } else { 1.0 };
        self.state.weight *= 2.0;
    }

    fn set_state<'py>(&mut self, state: &FlavouredState) -> PyResult<()> {
        let FlavouredState {
            pid, energy, latitude, longitude, altitude, azimuth, elevation, weight
        } = *state;
        self.state.charge = Particle::from_pid(pid)?.charge();
        self.state.energy = energy;
        self.geographic = GeographicCoordinates { latitude, longitude, altitude };
        self.state.position = self.geographic.to_ecef();
        self.horizontal = HorizontalCoordinates { azimuth, elevation };
        let direction = self.horizontal.to_ecef(&self.geographic);
        for i in 0..3 { self.state.direction[i] = -direction[i]; }  // Observer convention.
        self.state.weight = weight;
        self.state.distance = 0.0;
        self.state.grammage = 0.0;
        self.state.time = 0.0;
        self.state.decayed = 0;
        self.use_external_layer =
            self.geographic.altitude >= self.fluxmeter.steppers.layers.zlim + Self::EPSILON;
        Ok(())
    }

    fn step(&mut self, tag: GeometryTag) -> (f64, usize) {
        let stepper = match tag {
            GeometryTag::Layers => self.fluxmeter.steppers.layers.stepper,
            GeometryTag::Opensky => self.fluxmeter.steppers.opensky.stepper,
        };
        let mut step = 0.0;
        let mut index = [ -1; 2 ];
        unsafe {
            turtle::stepper_step(
                stepper,
                self.state.position.as_mut_ptr(),
                null(),
                &mut self.geographic.latitude,
                &mut self.geographic.longitude,
                &mut self.geographic.altitude,
                null_mut(),
                &mut step,
                index.as_mut_ptr(),
            );
        }
        (step, index[0] as usize)
    }

    fn transport(&mut self) -> PyResult<()> {
        const LOW_ENERGY: f64 = 1E+01;
        const HIGH_ENERGY: f64 = 1E+02;

        if self.magnet.is_some() {
            self.use_magnet = true;
            self.magnet_position = [0.0; 3];
        }

        self.context.event = pumas::EVENT_LIMIT_ENERGY;

        let zlim = self.fluxmeter.steppers.layers.zlim;
        if self.geographic.altitude < zlim - Self::EPSILON {
            // Transport backward with Pumas.
            self.context.limit.energy = self.reference.energy.max();
            match self.fluxmeter.mode {
                TransportMode::Continuous => {
                    self.context.mode.energy_loss = pumas::MODE_CSDA;
                    self.context.mode.scattering = pumas::MODE_DISABLED;
                },
                TransportMode::Mixed => {
                    self.context.mode.energy_loss = pumas::MODE_MIXED;
                    self.context.mode.scattering = pumas::MODE_DISABLED;
                },
                TransportMode::Discrete => {
                    if self.state.energy <= LOW_ENERGY - Self::EPSILON {
                        self.context.mode.energy_loss = pumas::MODE_STRAGGLED;
                        self.context.mode.scattering = pumas::MODE_MIXED;
                        self.context.limit.energy = LOW_ENERGY;
                    } else if self.state.energy <= HIGH_ENERGY - Self::EPSILON {
                        self.context.mode.energy_loss = pumas::MODE_MIXED;
                        self.context.mode.scattering = pumas::MODE_MIXED;
                        self.context.limit.energy = HIGH_ENERGY;
                    } else {
                        // use mixed mode.
                        self.context.mode.energy_loss = pumas::MODE_MIXED;
                        self.context.mode.scattering = pumas::MODE_DISABLED;
                    }
                },
            }
            self.context.medium = Some(layers_geometry);
            self.context.mode.direction = pumas::MODE_BACKWARD;

            unsafe {
                turtle::stepper_reset(self.fluxmeter.steppers.layers.stepper);
            }

            let mut event: c_uint = 0;
            loop {
                let rc = unsafe {
                    pumas::context_transport(
                        self.context, &mut self.state, &mut event, null_mut(),
                    )
                };
                error::to_result(rc, None::<&str>)?;

                if (self.fluxmeter.mode == TransportMode::Discrete) &&
                    (event == pumas::EVENT_LIMIT_ENERGY) {
                    if self.state.energy >= self.reference.energy.max() - Self::EPSILON {
                        self.state.weight = 0.0;
                        return Ok(())
                    } else if self.state.energy >= HIGH_ENERGY - Self::EPSILON {
                        self.context.mode.energy_loss = pumas::MODE_MIXED;
                        self.context.mode.scattering = pumas::MODE_DISABLED;
                        self.context.limit.energy = self.reference.energy.max();
                        continue
                    } else {
                        self.context.mode.energy_loss = pumas::MODE_MIXED;
                        self.context.mode.scattering = pumas::MODE_MIXED;
                        self.context.limit.energy = HIGH_ENERGY;
                        continue
                    }
                } else if event != pumas::EVENT_MEDIUM {
                    self.state.weight = 0.0;
                    return Ok(());
                } else {
                    break;
                }
            }

            // Compute the coordinates at the end location (expected to be at zlim).
            self.geographic = GeographicCoordinates::from_ecef(&self.state.position);
            if (self.geographic.altitude - zlim).abs() > 1E-04 {
                self.state.weight = 0.0;
                return Ok(())
            }
        }

        // XXX Check feasability.
        let zlim = self.fluxmeter.steppers.opensky.zlim;
        if self.geographic.altitude > self.reference.altitude.min() + Self::EPSILON {
            // Backup proper time and kinetic energy.
            let t0 = self.state.time;
            let e0 = self.state.energy;
            self.state.time = 0.0;

            // Transport forward to the reference height using CSDA.
            self.context.mode.energy_loss = pumas::MODE_CSDA;
            self.context.mode.scattering = pumas::MODE_DISABLED;
            self.context.medium = Some(opensky_geometry);
            self.context.mode.direction = pumas::MODE_FORWARD;
            self.context.limit.energy = self.reference.energy.0;

            unsafe {
                turtle::stepper_reset(self.fluxmeter.steppers.opensky.stepper);
            }

            let mut event: c_uint = 0;
            let rc = unsafe {
                pumas::context_transport(self.context, &mut self.state, &mut event, null_mut())
            };
            error::to_result(rc, None::<&str>)?;
            if event != pumas::EVENT_MEDIUM {
                self.state.weight = 0.0;
                return Ok(())
            }

            // Compute the coordinates at end location (expected to be at zlim).
            self.geographic = GeographicCoordinates::from_ecef(&self.state.position);
            if (self.geographic.altitude - zlim).abs() > 1E-04 {
                self.state.weight = 0.0;
                return Ok(())
            } else {
                self.geographic.altitude = zlim;
                // due to potential rounding errors.
            }

            // Update the proper time and the Jacobian weight */
            self.state.time = t0 - self.state.time;

            let material = self.fluxmeter.atmosphere_medium.0.medium.material;
            let mut dedx0 = 0.0;
            let mut dedx1 = 0.0;
            unsafe {
                pumas::physics_property_stopping_power(
                    self.physics.borrow_physics_ptr(), pumas::MODE_CSDA, material, e0, &mut dedx0,
                );
                pumas::physics_property_stopping_power(
                    self.physics.borrow_physics_ptr(), pumas::MODE_CSDA, material,
                    self.state.energy, &mut dedx1,
                );
            }
            if (dedx0 <= 0.0) || (dedx1 <= 0.0) {
                self.state.weight = 0.0;
                return Ok(())
            }
            self.state.weight *= dedx1 / dedx0;
        } else if (self.geographic.altitude - zlim).abs() <= 10.0 * Self::EPSILON {
            self.geographic.altitude = zlim;  // due to rounding errors.
        }


        // Compute the direction at the reference height.
        let direction = [
            -self.state.direction[0],
            -self.state.direction[1],
            -self.state.direction[2],
        ];
        self.horizontal = HorizontalCoordinates::from_ecef(&direction, &self.geographic);

        // Apply the decay probability.
        const MUON_C_TAU: f64 = 658.654;
        let pdec = (-self.state.time / MUON_C_TAU).exp();
        self.state.weight *= pdec;

        Ok(())
    }
}

impl<'a> From<*mut pumas::State> for &Agent<'a> {
    #[inline]
    fn from(value: *mut pumas::State) -> Self {
        unsafe { &*(value as *const Agent) }
    }
}

impl<'a> From<*mut pumas::State> for &mut Agent<'a> {
    #[inline]
    fn from(value: *mut pumas::State) -> Self {
        unsafe { &mut*(value as *mut Agent) }
    }
}

impl FlavouredState {
    fn from_grammage(states: &Extractor<5>, index: usize) -> PyResult<Self> {
        let pid = Particle::Muon.pid();
        let energy = 1E+03;
        let latitude = states.get_f64(Name::Latitude, index)?;
        let longitude = states.get_f64(Name::Longitude, index)?;
        let altitude = states.get_f64(Name::Altitude, index)?;
        let azimuth = states.get_f64(Name::Azimuth, index)?;
        let elevation = states.get_f64(Name::Elevation, index)?;
        let weight = 1.0;
        Ok(Self { pid, energy, latitude, longitude, altitude, azimuth, elevation, weight })
    }

    fn from_transport(states: &Extractor<8>, index: usize) -> PyResult<Self> {
        let pid = states.get_i32_opt(Name::Pid, index)?.unwrap_or_else(|| Particle::Muon.pid());
        let energy = states.get_f64(Name::Energy, index)?;
        let latitude = states.get_f64(Name::Latitude, index)?;
        let longitude = states.get_f64(Name::Longitude, index)?;
        let altitude = states.get_f64(Name::Altitude, index)?;
        let azimuth = states.get_f64(Name::Azimuth, index)?;
        let elevation = states.get_f64(Name::Elevation, index)?;
        let weight = states.get_f64_opt(Name::Weight, index)?.unwrap_or_else(|| 1.0);
        Ok(Self { pid, energy, latitude, longitude, altitude, azimuth, elevation, weight })
    }
}

static FLAVOURED_STATE_DTYPE: GILOnceCell<PyObject> = GILOnceCell::new();

impl Dtype for FlavouredState {
    fn dtype<'py>(py: Python<'py>) -> PyResult<&'py Bound<'py, PyAny>> {
        let ob = FLAVOURED_STATE_DTYPE.get_or_try_init(py, || -> PyResult<_> {
            let ob = PyModule::import(py, "numpy")?
                .getattr("dtype")?
                .call1(([
                        ("pid",       "i4"),
                        ("energy",    "f8"),
                        ("latitude",  "f8"),
                        ("longitude", "f8"),
                        ("altitude",  "f8"),
                        ("azimuth",   "f8"),
                        ("elevation", "f8"),
                        ("weight",    "f8"),
                    ],
                    true,
                ))?
                .unbind();
            Ok(ob)
        })?
        .bind(py);
        Ok(ob)
    }
}

static UNFLAVOURED_STATE_DTYPE: GILOnceCell<PyObject> = GILOnceCell::new();

impl Dtype for UnflavouredState {
    fn dtype<'py>(py: Python<'py>) -> PyResult<&'py Bound<'py, PyAny>> {
        let ob = UNFLAVOURED_STATE_DTYPE.get_or_try_init(py, || -> PyResult<_> {
            let ob = PyModule::import(py, "numpy")?
                .getattr("dtype")?
                .call1(([
                        ("energy",    "f8"),
                        ("latitude",  "f8"),
                        ("longitude", "f8"),
                        ("altitude",  "f8"),
                        ("azimuth",   "f8"),
                        ("elevation", "f8"),
                        ("weight",    "f8"),
                    ],
                    true,
                ))?
                .unbind();
            Ok(ob)
        })?
        .bind(py);
        Ok(ob)
    }
}

impl Particle {
    #[inline]
    const fn charge(&self) -> f64 {
        match self {
            Self::Anti => 1.0,
            Self::Muon => -1.0,
            Self::Any => unreachable!(),
        }
    }

    #[inline]
    fn from_charge(value: f64) -> Self {
        if value > 0.0 { Self::Anti }
        else { Self::Muon }
    }

    #[inline]
    fn from_pid(value: i32) -> PyResult<Self> {
        match value {
            13 => Ok(Self::Muon),
            -13 => Ok(Self::Anti),
            _ => {
                let why = format!("expected '13' or '-13', found {}", value);
                let err = Error::new(ValueError)
                    .what("pid")
                    .why(&why)
                    .to_err();
                Err(err)
            },
        }
    }

    #[inline]
    const fn pid(&self) -> i32 {
        match self {
            Self::Anti => -13,
            Self::Muon => 13,
            Self::Any => unreachable!(),
        }
    }
}
