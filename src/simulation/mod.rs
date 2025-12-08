use crate::bindings::pumas;
use crate::geometry::{Geometry, GeometryArg, GeometryRef};
use crate::geometry::earth::{EarthGeometry, EarthGeometryStepper, Layer};
use crate::geometry::local;
use crate::materials::set::MaterialsSet;
use crate::utils::convert::TransportMode;
use crate::utils::error::{self, Error};
use crate::utils::error::ErrorKind::{TypeError, ValueError};
use crate::utils::io::PathString;
use crate::utils::notify::{Notifier, NotifyArg};
use crate::utils::numpy::{ArrayMethods, NewArray};
use crate::utils::traits::MinMax;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString, PyTuple};
use std::ffi::{c_uint, c_void};
use std::ops::DerefMut;
use std::ptr::null_mut;
use std::pin::Pin;

pub mod atmosphere;
pub mod coordinates;
pub mod geomagnet;
pub mod physics;
pub mod random;
pub mod reference;
pub mod states;

use atmosphere::{Atmosphere, AtmosphereArg};
use coordinates::{GeographicCoordinates, HorizontalCoordinates, LocalFrame};
use geomagnet::{EarthMagnet, EarthMagnetArg};
use states::{
    ExtractedState, TaggedGeographicState, TaggedLocalState, NewStates, StatesExtractor,
    UntaggedGeographicState, UntaggedLocalState,
};


#[pyclass(module="mulder")]
pub struct Fluxmeter {
    geometry: Geometry,

    /// The atmosphere model.
    #[pyo3(get)]
    pub atmosphere: Py<Atmosphere>,

    /// The geomagnetic field.
    #[pyo3(get)]
    pub geomagnet: Option<Py<EarthMagnet>>,

    /// The transport mode.
    #[pyo3(get, set)]
    mode: TransportMode,

    /// The muon physics.
    #[pyo3(get)]
    physics: Py<physics::Physics>,

    /// The pseudo-random stream.
    #[pyo3(get, set)]
    random: Py<random::Random>,

    /// The reference muon flux.
    #[pyo3(get)]
    reference: Py<reference::Reference>,

    materials: MaterialsSet,
    atmosphere_medium: CMedium,
    media: Vec<CMedium>,
}

unsafe impl Send for Fluxmeter {}
unsafe impl Sync for Fluxmeter {}

struct CMedium (Pin<Box<MediumData>>);

#[repr(C)]
struct MediumData {
    api: pumas::Medium,
    density: f64,
    index: usize,
    use_geomagnet: bool,
}

struct Proxy<'py> {
    geometry: GeometryRef<'py>,
    atmosphere: PyRef<'py, Atmosphere>,
    geomagnet: Option<PyRefMut<'py, EarthMagnet>>,
    physics: PyRefMut<'py, physics::Physics>,
    random: PyRefMut<'py, random::Random>,
    reference: PyRef<'py, reference::Reference>,
}

#[repr(C)]
struct Agent<'a> {
    state: pumas::State,
    geographic: GeographicCoordinates,
    horizontal: HorizontalCoordinates,

    atmosphere: &'a Atmosphere,
    fluxmeter: &'a mut Fluxmeter,
    geomagnet: Option<&'a mut EarthMagnet>,
    geometry: GeometryAgent<'a>,
    physics: &'a physics::Physics,
    reference: &'a reference::Reference,
    context: &'a mut pumas::Context,

    geomagnet_field: [f64; 3],
    geomagnet_position: [f64; 3],
    use_geomagnet: bool,
    opensky: Opensky,
}

enum GeometryAgent<'a> {
    Earth {
        stepper: EarthGeometryStepper,
        zmax: f64,
    },
    Local {
        tracer: local::LocalTracer<'a>,
        frame: &'a LocalFrame,
        direction: [f64; 3],
        step: f64,
        status: TracingStatus,
    },
}

#[derive(Default)]
struct Opensky {
    zmin: f64,
    zmax: f64,
}

#[derive(Debug, PartialEq)]
enum TracingStatus {
    Start,
    Trace,
    Locate,
}

#[derive(FromPyObject)]
enum ReferenceLike {
    Model(PathString),
    Number(f64),
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

        let atmosphere = match extract_field("atmosphere")? {
            Some(arg) => arg.extract::<AtmosphereArg>()?
                .into_atmosphere(py)?,
            None => {
                let atmosphere = Atmosphere::new(None, None)?;
                Py::new(py, atmosphere)?
            },
        };

        let geomagnet = {
            let geomagnet_kwargs = extract_kwargs(
                &["date"]
            )?;
            match extract_field("geomagnet")? {
                Some(arg) => if geomagnet_kwargs.is_none() {
                    arg.extract::<EarthMagnetArg>()?
                        .into_geomagnet(py)?
                } else {
                    let err = Error::new(TypeError)
                        .what("geomagnet argument(s)")
                        .why("geomagnet already provided as **kwargs")
                        .to_err();
                    return Err(err)
                },
                None => match geomagnet_kwargs {
                    Some(kwargs) => {
                        let date: geomagnet::DateArg = kwargs
                            .get_item("date")?
                            .unwrap()
                            .extract()?;
                        let geomagnet = geomagnet::EarthMagnet::new(py, None, Some(date))?;
                        Some(Py::new(py, geomagnet)?)
                    },
                    None => None,
                },
            }
        };

        let geometry = {
            match extract_field("geometry")? {
                Some(geometry) => if layers.is_empty() {
                    geometry.extract::<GeometryArg>()?
                        .into_geometry(py)?
                } else {
                    let err = Error::new(TypeError)
                        .what("geometry argument(s)")
                        .why("geometry already provided as *layers")
                        .to_err();
                    return Err(err)
                },
                None => {
                    let geometry = EarthGeometry::new(layers)?;
                    let geometry = Bound::new(py, geometry)?;
                    Geometry::Earth(geometry.unbind())
                }
            }
        };

        let physics = {
            let physics_kwargs = extract_kwargs(
                &["bremsstrahlung", "pair_production", "photonuclear"]
            )?;
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

        let random = {
            let random_kwargs = extract_kwargs(
                &["seed", "index",]
            )?;
            match extract_field("random")? {
                Some(random) => if random_kwargs.is_none() {
                    let random: Py<random::Random> = random.extract()?;
                    random
                } else {
                    let err = Error::new(TypeError)
                        .what("random argument(s)")
                        .why("random already provided as **kwargs")
                        .to_err();
                    return Err(err)
                },
                None => {
                    let random = match random_kwargs {
                        Some(kwargs) => {
                            let seed: Option<u128> = kwargs
                                .get_item("seed")?
                                .map(|seed| seed.extract())
                                .transpose()?;
                            let index: Option<random::Index> = kwargs
                                .get_item("index")?
                                .map(|index| index.extract())
                                .transpose()?;
                            random::Random::new(seed, index)?
                        },
                        None => random::Random::new(None, None)?,
                    };
                    Py::new(py, random)?
                },
            }
        };

        let mode = TransportMode::default();

        let reference = reference::Reference::new(None, None)?;
        let reference = Py::new(py, reference)?;

        let materials = MaterialsSet::new();
        geometry.subscribe(py, &materials);
        atmosphere.bind(py).borrow_mut().subscribe(&materials);

        let media = Vec::new();
        let atmosphere_medium = CMedium::default();

        let fluxmeter = Self {
            atmosphere, geomagnet, geometry, mode, physics, random, reference, materials,
            atmosphere_medium, media,
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
    fn set_atmosphere(&mut self, py: Python, value: Option<AtmosphereArg>) -> PyResult<()> {
        let new = match value {
            Some(atmosphere) => atmosphere.into_atmosphere(py)?.bind(py).clone(),
            None => Bound::new(py, Atmosphere::new(None, None)?)?,
        };
        let current = self.atmosphere.bind(py);
        if !current.is(&new) {
            current.borrow_mut().unsubscribe(&self.materials);
            new.borrow_mut().subscribe(&self.materials);
            self.atmosphere = new.unbind();
            self.reset();
        }
        Ok(())
    }

    #[setter]
    fn set_geomagnet(&mut self, py: Python, value: Option<EarthMagnetArg>) -> PyResult<()> {
        self.geomagnet = value.and_then(|magnet|
            magnet.into_geomagnet(py).transpose()
        ).transpose()?;
        Ok(())
    }

    /// The local geometry.
    #[getter]
    fn get_geometry(&mut self, py: Python) -> PyObject {
        match &self.geometry {
            Geometry::Earth(geometry) => geometry.clone_ref(py).into_any(),
            Geometry::Local(geometry) => geometry.clone_ref(py).into_any(),
        }
    }

    #[setter]
    fn set_geometry(&mut self, py: Python, value: GeometryArg) -> PyResult<()> {
        let value = value.into_geometry(py)?;
        if !self.geometry.bind(py).is(value.bind(py)) {
            self.geometry.unsubscribe(py, &self.materials);
            self.geometry = value;
            self.geometry.subscribe(py, &self.materials);
            self.reset();
        }
        Ok(())
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
            ReferenceLike::Number(value) => {
                let model = reference::ModelArg::Number(value);
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
    #[pyo3(
        signature=(states=None, /, *, events=None, notify=None, frame=None, **kwargs),
        text_signature="(self, states=None, /, *, events=None, notify=None, **kwargs)",
    )]
    fn flux<'py>(
        &mut self,
        py: Python<'py>,
        states: Option<&Bound<PyAny>>,
        events: Option<usize>,
        notify: Option<NotifyArg>,
        frame: Option<LocalFrame>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<NewArray<'py, f64>> {
        // Configure physics, geometry, samplers etc.
        error::clear();
        let mut proxy = self.borrow(py);
        let mut pinned = proxy.pinned_agent(py, self)?;
        let agent: &mut Agent = &mut pinned.deref_mut();

        // Extract states.
        let frame = coordinates::Maybe::new(frame.as_ref(), agent.geometry.frame());
        let states = StatesExtractor::new(states, kwargs, frame)?;
        let size = states.size();
        let mut shape = states.shape();

        // Uniformise the events parameter.
        let events = events.filter(|events| match agent.fluxmeter.mode {
            TransportMode::Continuous => false,
            _ => if *events > 1  {
                shape.push(2);
                true
            } else {
                false
            },
        });

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

            let state = states.extract(i)?;
            if (state.weight() <= 0.0) || (state.energy() <= 0.0) { continue }

            match &agent.fluxmeter.mode {
                TransportMode::Continuous => {
                    flux[i] = if agent.geomagnet.is_none() ||
                                 (state.energy() >= Agent::HIGH_ENERGY) {
                        let particle = if states.is_tagged() {
                            Particle::from_pid(state.pid())?
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
                            if !states.is_tagged() { agent.randomise_charge(); }
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
                        if !states.is_tagged() { agent.randomise_charge(); }
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

    /// Transport state(s) to the reference flux.
    #[pyo3(
        signature=(states=None, /, *, events=None, notify=None, frame=None, **kwargs),
        text_signature="(self, states=None, /, *, events=None, notify=None, **kwargs)",
    )]
    fn transport<'py>(
        &mut self,
        py: Python<'py>,
        states: Option<&Bound<PyAny>>,
        events: Option<usize>,
        notify: Option<NotifyArg>,
        frame: Option<LocalFrame>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<NewStates<'py>> {
        // Configure physics, geometry, samplers etc.
        error::clear();
        let mut proxy = self.borrow(py);
        let mut pinned = proxy.pinned_agent(py, self)?;
        let agent: &mut Agent = &mut pinned.deref_mut();

        // Extract states.
        let frame = coordinates::Maybe::new(frame.as_ref(), agent.geometry.frame());
        let states = StatesExtractor::new(states, kwargs, frame)?;
        let size = states.size();
        let mut shape = states.shape();

        if agent.geomagnet.is_some() && !states.is_tagged() {
            let err = Error::new(TypeError)
                .what("states")
                .why("a pid is required for a magnetized geometry")
                .to_err();
            return Err(err)
        }

        let events = events
            .map(|events| {
                shape.push(events);
                events
            })
            .unwrap_or_else(|| 1);

        let array = if states.is_tagged() {
            match &states {
                StatesExtractor::Geographic { .. } => {
                    let array = NewArray::<TaggedGeographicState>::empty(py, shape)?;
                    NewStates::TaggedGeographic { array }
                },
                StatesExtractor::Local { frame, .. } => {
                    let array = NewArray::<TaggedLocalState>::empty(py, shape)?;
                    NewStates::TaggedLocal { array, frame: frame.clone() }
                },
            }
        } else {
            match &states {
                StatesExtractor::Geographic { .. } => {
                    let array = NewArray::<UntaggedGeographicState>::empty(py, shape)?;
                    NewStates::UntaggedGeographic { array }
                },
                StatesExtractor::Local { frame, .. } => {
                    let array = NewArray::<UntaggedLocalState>::empty(py, shape)?;
                    NewStates::UntaggedLocal { array, frame: frame.clone() }
                },
            }
        };

        // Setup notifications.
        let notifier = Notifier::from_arg(notify, size * events, "transporting muon(s)");

        for i in 0..size {
            let state = states.extract(i)?;
            if (state.weight() <= 0.0) || (state.energy() <= 0.0) { continue }
            for j in 0..events {
                let index = i * events + j;
                if (index % 100) == 0 { error::check_ctrlc("while transporting muon(s)")?; }

                agent.set_state(&state)?;
                agent.transport()?;

                match &array {
                    NewStates::TaggedGeographic { array } => array.set_item(
                        index,
                        agent.get_flavoured_geographic_state()
                    )?,
                    NewStates::UntaggedGeographic { array } => array.set_item(
                        index,
                        agent.get_unflavoured_geographic_state()
                    )?,
                    NewStates::TaggedLocal { array, frame } => array.set_item(
                        index,
                        agent.get_flavoured_local_state(frame)
                    )?,
                    NewStates::UntaggedLocal { array, frame } => array.set_item(
                        index,
                        agent.get_unflavoured_local_state(frame)
                    )?,
                }

                notifier.tic();
            }
        }

        Ok(array)
    }
}

impl Fluxmeter {
    fn borrow<'py>(&mut self, py: Python<'py>) -> Proxy<'py> {
        let geometry = self.geometry.borrow(py);
        let atmosphere = self.atmosphere.bind(py).borrow();
        let geomagnet = self.geomagnet.as_mut().map(|geomagnet| geomagnet.bind(py).borrow_mut());
        let physics = self.physics.bind(py).borrow_mut();
        let random = self.random.bind(py).borrow_mut();
        let reference = self.reference.bind(py).borrow();
        Proxy { geometry, atmosphere, geomagnet, physics, random, reference }
    }

    fn create_or_update_geometry(
        &mut self,
        py: Python,
        atmosphere: &Atmosphere,
        geometry: &GeometryRef,
        physics: &physics::Physics,
    ) -> PyResult<()> {
        match geometry {
            GeometryRef::Earth(geometry) => if self.media.is_empty() {
                // Map media.
                for (index, layer) in geometry.layers.bind(py).iter().enumerate() {
                    let layer = layer.downcast::<Layer>().unwrap().borrow();
                    let medium = CMedium::from_layer(&layer, index, physics)?;
                    self.media.push(medium);
                }
                self.atmosphere_medium = CMedium::atmosphere(
                    self.media.len(),
                    atmosphere.material.as_str(),
                    physics
                )?;
            } else {
                for (index, layer) in geometry.layers.bind(py).iter().enumerate() {
                    let layer = layer.downcast::<Layer>().unwrap().borrow();
                    self.media[index].update(
                        layer.density,
                        layer.material.as_str(),
                        physics,
                    )?;
                }
                self.atmosphere_medium.update(
                    None,
                    atmosphere.material.as_str(),
                    physics,
                )?;
            },
            GeometryRef::Local(geometry) => if self.media.is_empty() {
                for (index, medium) in geometry.media.bind(py).iter().enumerate() {
                    let medium = medium.downcast::<local::Medium>().unwrap().borrow();
                    let medium = CMedium::from_local(&medium, index, physics)?;
                    self.media.push(medium);
                }
                self.atmosphere_medium = CMedium::atmosphere(
                    self.media.len(),
                    atmosphere.material.as_str(),
                    physics
                )?;
            } else {
                for (index, medium) in geometry.media.bind(py).iter().enumerate() {
                    let medium = medium.downcast::<local::Medium>().unwrap().borrow();
                    self.media[index].update(
                        medium.density,
                        medium.material.as_str(),
                        physics,
                    )?;
                }
                self.atmosphere_medium.update(
                    None,
                    atmosphere.material.as_str(),
                    physics,
                )?;
            },
        }
        Ok(())
    }

    fn reset(&mut self) {
        self.media.clear();
    }
}

#[no_mangle]
extern "C" fn atmosphere_locals(
    _medium: *mut pumas::Medium,
    state: *mut pumas::State,
    locals: *mut pumas::Locals,
) -> f64 {
    let agent: &mut Agent = state.into();
    let locals = unsafe { &mut *locals };

    let density = agent.atmosphere.compute_density(agent.geographic.altitude);
    locals.density = density.value;

    const LAMBDA_MAX: f64 = 1E+09;
    let lambda = if density.lambda.abs() < LAMBDA_MAX {
        let direction = HorizontalCoordinates::from_ecef(
            &agent.state.direction,
            &agent.geographic,
        );
        let c = (direction.elevation.abs() * std::f64::consts::PI / 180.0).sin().max(0.1);
        (density.lambda.abs() / c).min(LAMBDA_MAX)
    } else {
        LAMBDA_MAX
    };

    if agent.use_geomagnet && (agent.state.energy < Agent::HIGH_ENERGY) {
        // Get the local geomagnetic field.
        let lambda_magnet = agent.update_geomagnet();
        locals.magnet = agent.geomagnet_field;
        lambda.min(lambda_magnet)
    } else {
        lambda
    }
}

#[no_mangle]
extern "C" fn uniform_locals(
    medium: *mut pumas::Medium,
    state: *mut pumas::State,
    locals: *mut pumas::Locals,
) -> f64 {
    let agent: &mut Agent = state.into();
    let medium: &MediumData = medium.into();
    let locals = unsafe { &mut *locals };

    locals.density = medium.density;

    if agent.use_geomagnet && medium.use_geomagnet && (agent.state.energy < Agent::HIGH_ENERGY) {
        let lambda_magnet = agent.update_geomagnet();
        locals.magnet = agent.geomagnet_field;
        lambda_magnet
    } else {
        0.0
    }
}

#[no_mangle]
extern "C" fn uniform01(context: *mut pumas::Context) -> f64 {
    let random = unsafe { &mut*((*context).user_data as *mut random::Random) };
    random.open01()
}

#[no_mangle]
extern "C" fn earth_geometry(
    _context: *mut pumas::Context,
    state: *mut pumas::State,
    medium_ptr: *mut *mut pumas::Medium,
    step_ptr: *mut f64,
) -> c_uint {
    let agent: &mut Agent = state.into();
    let GeometryAgent::Earth { stepper, zmax } = &mut agent.geometry
        else { unreachable!() };

    let (step, layer) = stepper.step(
        &mut agent.state.position, &mut agent.geographic
    );

    if step_ptr != null_mut() {
        let step_ptr = unsafe { &mut*step_ptr };
        *step_ptr = if step <= Agent::EPSILON { Agent::EPSILON } else { step };
    }

    if medium_ptr != null_mut() {
        let medium_ptr = unsafe { &mut*medium_ptr };
        if (layer >= 1) && (layer <= stepper.layers) {
            *medium_ptr = agent.fluxmeter.media[layer - 1].as_mut_ptr();
        } else if (layer == stepper.layers + 1) && (agent.geographic.altitude < *zmax) {
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

    if step_ptr != null_mut() {
        let step_ptr = unsafe { &mut*step_ptr };
        *step_ptr = f64::INFINITY;
    }

    if medium_ptr != null_mut() {
        let medium_ptr = unsafe { &mut*medium_ptr };
        let state = unsafe { &*state };
        agent.geographic = GeographicCoordinates::from_ecef(&state.position);
        let Opensky { zmin, zmax } = &agent.opensky;
        if (agent.geographic.altitude > *zmin) && (agent.geographic.altitude < *zmax) {
            *medium_ptr = agent.fluxmeter.atmosphere_medium.as_mut_ptr();
        } else {
            *medium_ptr = null_mut();
        }
    }

    pumas::STEP_CHECK
}

#[no_mangle]
extern "C" fn local_geometry(
    _context: *mut pumas::Context,
    state: *mut pumas::State,
    medium_ptr: *mut *mut pumas::Medium,
    step_ptr: *mut f64,
) -> c_uint {
    let agent: &mut Agent = state.into();
    let GeometryAgent::Local { tracer, direction, step, status, .. } = &mut agent.geometry
        else { unreachable!() };
    let n = agent.fluxmeter.media.len();
    let mut set_medium_ptr = || {
        let medium_ptr = unsafe { &mut*medium_ptr };
        let index = tracer.medium();
        if index < n {
            *medium_ptr = agent.fluxmeter.media[index].as_mut_ptr();
        } else {
            *medium_ptr = null_mut();
        }
    };
    let state = unsafe { &*state };
    let compute_displacement = || {
        let r0 = tracer.position();
        let r1 = &state.position;
        (r1[0] - r0[0]) * direction[0] +
        (r1[1] - r0[1]) * direction[1] +
        (r1[2] - r0[2]) * direction[2]
    };
    if step_ptr != null_mut() {
        // Tracing call.
        let step_ptr = unsafe { &mut*step_ptr };

        match *status {
            TracingStatus::Start => {
                *direction = [-state.direction[0], -state.direction[1], -state.direction[2]];
            },
            TracingStatus::Trace => {
                let length = compute_displacement();
                if length.abs() > pumas::STEP_MIN {
                    tracer.move_(length);
                }
                let u = [-state.direction[0], -state.direction[1], -state.direction[2]];
                tracer.turn(u);
                *direction = u;
            },
            _ => unreachable!(),
        }

        *step = tracer.trace(f64::MAX).max(pumas::STEP_MIN);
        *step_ptr = *step;
        if medium_ptr != null_mut() {
            set_medium_ptr();
        }
        *status = TracingStatus::Locate;
    } else if medium_ptr != null_mut() {
        // Locating call.
        assert!(*status == TracingStatus::Locate);
        let mut length = compute_displacement();
        if (length - *step).abs() < f64::EPSILON {
            length = *step;
        } else {
            *step = length;
        }
        tracer.move_(length);
        set_medium_ptr();
        *status = TracingStatus::Trace;
    }

    pumas::STEP_RAW
}

impl CMedium {
    #[inline]
    fn as_mut_ptr(&mut self) -> *mut pumas::Medium {
        &mut self.0.api
    }

    fn atmosphere(layer: usize, material: &str, physics: &physics::Physics) -> PyResult<Self> {
        Self::new(material, None, layer, Some(atmosphere_locals), physics)
    }

    fn new(
        material: &str,
        density: Option<f64>,
        index: usize,
        locals: pumas::LocalsCallback,
        physics: &physics::Physics,
    ) -> PyResult<Self> {
        let material = physics.material_index(material)?;
        let density = density.unwrap_or(-1.0);
        let api = pumas::Medium { material, locals };
        let use_geomagnet = false;
        let data = MediumData { api, density, index, use_geomagnet };
        let medium = Self(Box::pin(data));
        Ok(medium)
    }

    fn from_layer(
        layer: &Layer,
        layer_index: usize,
        physics: &physics::Physics,
    ) -> PyResult<Self> {
        Self::new(
            layer.material.as_str(), layer.density, layer_index, Some(uniform_locals), physics,
        )
    }

    fn from_local(
        medium: &local::Medium,
        index: usize,
        physics: &physics::Physics,
    ) -> PyResult<Self> {
        Self::new(
            medium.material.as_str(), medium.density, index, Some(uniform_locals), physics,
        )
    }

    fn update(
        &mut self,
        density: Option<f64>,
        material: &str,
        physics: &physics::Physics,
    ) -> PyResult<()> {
        self.0.density = density.unwrap_or(-1.0);
        self.0.api.material = physics.material_index(material)?;
        Ok(())
    }
}

impl From<*mut pumas::Medium> for &MediumData {
    #[inline]
    fn from(value: *mut pumas::Medium) -> Self {
        unsafe { &*(value as *const MediumData) }
    }
}

impl Default for CMedium {
    fn default() -> Self {
        let api = pumas::Medium { material: -1, locals: None };
        let density = 0.0;
        let index = 0;
        let use_geomagnet = false;
        let data = MediumData { api, density, index, use_geomagnet };
        Self(Box::pin(data))
    }
}

impl<'a> Agent<'a> {
    const DELTA_Z: f64 = 300.0;  // min atmospheric depth.
    const EPSILON: f64 = f32::EPSILON as f64;
    const LOW_ENERGY: f64 = 1E+01;  // GeV.
    const HIGH_ENERGY: f64 = 1E+02;  // GeV.

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

    fn get_flavoured_geographic_state(&self) -> TaggedGeographicState {
        let (geographic, horizontal) = match self.geometry {
            GeometryAgent::Earth { .. } => (self.geographic, self.horizontal),
            GeometryAgent::Local { frame, .. } => frame.to_geographic(
                &self.state.position, &self.state.direction
            ),
        };
        TaggedGeographicState {
            pid: Particle::from_charge(self.state.charge).pid(),
            energy: self.state.energy,
            latitude: geographic.latitude,
            longitude: geographic.longitude,
            altitude: geographic.altitude,
            azimuth: horizontal.azimuth,
            elevation: horizontal.elevation,
            weight: self.state.weight,
        }
    }

    fn get_flavoured_local_state(&self, frame: &LocalFrame) -> TaggedLocalState {
        let (position, direction) = match self.geometry {
            GeometryAgent::Earth { .. } => frame.from_geographic(
                self.geographic, self.horizontal
            ),
            GeometryAgent::Local { frame: geometry_frame, .. } => frame.from_local(
                self.state.position, self.state.direction, geometry_frame
            ),
        };
        TaggedLocalState {
            pid: Particle::from_charge(self.state.charge).pid(),
            energy: self.state.energy,
            position,
            direction: [-direction[0], -direction[1], -direction[2]],
            weight: self.state.weight,
        }
    }

    fn get_unflavoured_geographic_state(&self) -> UntaggedGeographicState {
        let (geographic, horizontal) = match self.geometry {
            GeometryAgent::Earth { .. } => (self.geographic, self.horizontal),
            GeometryAgent::Local { frame, .. } => frame.to_geographic(
                &self.state.position, &self.state.direction
            ),
        };
        UntaggedGeographicState {
            energy: self.state.energy,
            latitude: geographic.latitude,
            longitude: geographic.longitude,
            altitude: geographic.altitude,
            azimuth: horizontal.azimuth,
            elevation: horizontal.elevation,
            weight: self.state.weight,
        }
    }

    fn get_unflavoured_local_state(&self, frame: &LocalFrame) -> UntaggedLocalState {
        let (position, direction) = match self.geometry {
            GeometryAgent::Earth { .. } => frame.from_geographic(
                self.geographic, self.horizontal
            ),
            GeometryAgent::Local { frame: geometry_frame, .. } => frame.from_local(
                self.state.position, self.state.direction, geometry_frame
            ),
        };
        UntaggedLocalState {
            energy: self.state.energy,
            position,
            direction: [-direction[0], -direction[1], -direction[2]],
            weight: self.state.weight,
        }
    }

    fn new(
        py: Python,
        atmosphere: &'a Atmosphere,
        fluxmeter: &'a mut Fluxmeter,
        geometry: &'a GeometryRef,
        geomagnet: Option<&'a mut EarthMagnet>,
        physics: &'a mut physics::Physics,
        mut random: &'a mut random::Random,
        reference: &'a reference::Reference,
    ) -> PyResult<Self> {
        // Configure physics and geometry.
        physics.update(py, &fluxmeter.materials, None)?;
        fluxmeter.create_or_update_geometry(py, atmosphere, geometry, &physics)?;
        let geometry = match geometry {
            GeometryRef::Earth(geometry) => GeometryAgent::Earth {
                stepper: geometry.stepper(py)?,
                zmax: geometry.zlim.max() + Self::DELTA_Z,
            },
            GeometryRef::Local(geometry) => GeometryAgent::Local {
                tracer: geometry.tracer()?,
                frame: &geometry.frame,
                direction: [0.0; 3],
                step: 0.0,
                status: TracingStatus::Start,
            },
        };

        // Configure Pumas context.
        let context = physics.borrow_mut_context();
        context.user_data = random.deref_mut() as *mut random::Random as *mut c_void;
        context.random = Some(uniform01);

        // Initialise particles state.
        let state = pumas::State::default();
        let geographic = GeographicCoordinates::default();
        let horizontal = HorizontalCoordinates::default();

        let geomagnet_field = [0.0; 3];
        let geomagnet_position = [0.0; 3];
        let use_geomagnet = false;
        let opensky = Opensky::default();

        let agent = Self {
            state, geographic, horizontal, atmosphere, fluxmeter, geomagnet, geometry, physics,
            reference, context, geomagnet_field, geomagnet_position, use_geomagnet, opensky,
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

    fn set_state<'py>(&mut self, state: &ExtractedState) -> PyResult<()> {
        match &self.geometry {
            GeometryAgent::Earth { .. } => {
                match state {
                    ExtractedState::Geographic { state } => {
                        let TaggedGeographicState {
                            pid, energy, latitude, longitude, altitude, azimuth, elevation, weight
                        } = *state;
                        self.state.charge = Particle::from_pid(pid)?.charge();
                        self.state.energy = energy;
                        self.geographic = GeographicCoordinates { latitude, longitude, altitude };
                        self.state.position = self.geographic.to_ecef();
                        self.horizontal = HorizontalCoordinates { azimuth, elevation };
                        self.state.direction = self.horizontal.to_ecef(&self.geographic);
                        self.state.weight = weight;
                    }
                    ExtractedState::Local { state, frame } => {
                        let TaggedLocalState {
                            pid, energy, position, direction, weight
                        } = *state;
                        self.state.charge = Particle::from_pid(pid)?.charge();
                        self.state.energy = energy;
                        self.state.position = frame.to_ecef_position(&position);
                        self.state.direction = frame.to_ecef_direction(&direction);
                        self.state.weight = weight;
                        self.geographic = GeographicCoordinates::from_ecef(&self.state.position);
                        self.horizontal = HorizontalCoordinates::from_ecef(
                            &self.state.direction, &self.geographic
                        );
                    },
                }
            },
            GeometryAgent::Local { frame: geometry_frame, .. } => {
                match state {
                    ExtractedState::Geographic { state } => {
                        let TaggedGeographicState {
                            pid, energy, latitude, longitude, altitude, azimuth, elevation, weight
                        } = *state;
                        self.state.charge = Particle::from_pid(pid)?.charge();
                        self.state.energy = energy;
                        (self.state.position, self.state.direction) = geometry_frame
                            .from_geographic(
                                GeographicCoordinates { latitude, longitude, altitude },
                                HorizontalCoordinates { azimuth, elevation },
                            );
                        self.state.weight = weight;
                    },
                    ExtractedState::Local { state, frame } => {
                        let TaggedLocalState {
                            pid, energy, position, direction, weight
                        } = *state;
                        self.state.charge = Particle::from_pid(pid)?.charge();
                        self.state.energy = energy;
                        (self.state.position, self.state.direction) = geometry_frame
                            .from_local(position, direction, frame);
                        self.state.weight = weight;
                    },
                }
            },
        }
        for i in 0..3 { self.state.direction[i] = -self.state.direction[i] }  // Observer convention.
        self.state.distance = 0.0;
        self.state.grammage = 0.0;
        self.state.time = 0.0;
        self.state.decayed = 0;
        Ok(())
    }

    fn transport(&mut self) -> PyResult<()> {
        if self.geomagnet.is_some() {
            self.use_geomagnet = true;
            self.geomagnet_position = [0.0; 3];
        }

        self.context.event = pumas::EVENT_LIMIT_ENERGY;

        // Check that the initial state lies within the geometry.
        let n_media = self.fluxmeter.media.len();
        let is_inside = match &mut self.geometry {
            GeometryAgent::Earth { stepper, zmax } => {
                let is_inside =
                    (self.geographic.altitude > EarthGeometry::ZMIN + Self::EPSILON) &&
                    (self.geographic.altitude < *zmax - Self::EPSILON);
                if is_inside {
                    self.context.medium = Some(earth_geometry);
                    stepper.reset();
                }
                is_inside
            },
            GeometryAgent::Local { tracer, status, .. } => {
                let u = [
                    -self.state.direction[0],
                    -self.state.direction[1],
                    -self.state.direction[2],
                ];
                tracer.reset(self.state.position, u);
                *status = TracingStatus::Start;
                let is_inside = tracer.medium() < n_media;
                if is_inside {
                    self.context.medium = Some(local_geometry);
                }
                is_inside
            },
        };

        if is_inside {
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
                    if self.state.energy <= Self::LOW_ENERGY - Self::EPSILON {
                        self.context.mode.energy_loss = pumas::MODE_STRAGGLED;
                        self.context.mode.scattering = pumas::MODE_MIXED;
                        self.context.limit.energy = Self::LOW_ENERGY;
                    } else if self.state.energy <= Self::HIGH_ENERGY - Self::EPSILON {
                        self.context.mode.energy_loss = pumas::MODE_MIXED;
                        self.context.mode.scattering = pumas::MODE_MIXED;
                        self.context.limit.energy = Self::HIGH_ENERGY;
                    } else {
                        // use mixed mode.
                        self.context.mode.energy_loss = pumas::MODE_MIXED;
                        self.context.mode.scattering = pumas::MODE_DISABLED;
                    }
                },
            }
            self.context.mode.direction = pumas::MODE_BACKWARD;

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
                    } else if self.state.energy >= Self::HIGH_ENERGY - Self::EPSILON {
                        self.context.mode.energy_loss = pumas::MODE_MIXED;
                        self.context.mode.scattering = pumas::MODE_DISABLED;
                        self.context.limit.energy = self.reference.energy.max();
                        continue
                    } else {
                        self.context.mode.energy_loss = pumas::MODE_MIXED;
                        self.context.mode.scattering = pumas::MODE_MIXED;
                        self.context.limit.energy = Self::HIGH_ENERGY;
                        continue
                    }
                } else if event != pumas::EVENT_MEDIUM {
                    self.state.weight = 0.0;
                    return Ok(());
                } else {
                    break;
                }
            }
        }

        if let GeometryAgent::Local { frame, .. } = &self.geometry {
            self.state.position = frame.to_ecef_position(&self.state.position);
            self.state.direction = frame.to_ecef_direction(&self.state.direction);
            self.geographic = GeographicCoordinates::from_ecef(&self.state.position);
        }
        {
            let direction = [
                -self.state.direction[0],
                -self.state.direction[1],
                -self.state.direction[2],
            ];
            self.horizontal = HorizontalCoordinates::from_ecef(&direction, &self.geographic);
        }

        let revert_local = |agent: &mut Self| {
            if let GeometryAgent::Local { frame, .. } = &agent.geometry {
                agent.state.position = frame.from_ecef_position(agent.state.position);
                agent.state.direction = frame.from_ecef_direction(&agent.state.direction);
            }
        };

        const EPSILON: f64 = 1E-04;
        if self.geographic.altitude < self.reference.altitude.min() - EPSILON {
            if self.horizontal.elevation < 0.0 {
                self.state.weight = 0.0;
                revert_local(self);
                return Ok(())
            }
            let zref = self.reference.altitude.min();
            self.transport_opensky(zref)?;
            if self.state.weight == 0.0 {
                revert_local(self);
                return Ok(())
            }
        } else if self.geographic.altitude > self.reference.altitude.max() + EPSILON {
            // XXX Check feasability (using elevation angle).
            // Backup proper time and kinetic energy.
            let t0 = self.state.time;
            let e0 = self.state.energy;
            self.state.time = 0.0;

            let zref = self.reference.altitude.max();
            self.transport_opensky(zref)?;
            if self.state.weight == 0.0 {
                revert_local(self);
                return Ok(())
            }

            // Update the proper time and the Jacobian weight */
            self.state.time = t0 - self.state.time;

            let material = self.fluxmeter.atmosphere_medium.0.api.material;
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
        } else if (self.geographic.altitude - self.reference.altitude.min()).abs() < EPSILON {
            self.geographic.altitude = self.reference.altitude.min();
        } else if (self.geographic.altitude - self.reference.altitude.max()).abs() < EPSILON {
            self.geographic.altitude = self.reference.altitude.max();
        };

        revert_local(self);

        // Apply the decay probability.
        const MUON_C_TAU: f64 = 658.654;
        let pdec = (-self.state.time / MUON_C_TAU).exp();
        self.state.weight *= pdec;

        Ok(())
    }

    fn transport_opensky(&mut self, zref: f64) -> PyResult<()> {
        // Transport to the reference height using CSDA.
        self.context.mode.energy_loss = pumas::MODE_CSDA;
        self.context.mode.scattering = pumas::MODE_DISABLED;
        self.context.medium = Some(opensky_geometry);

        const EPSILON: f64 = 1E-04;
        if self.geographic.altitude < zref {
            self.context.mode.direction = pumas::MODE_BACKWARD;
            self.context.limit.energy = self.reference.energy.max();
            self.opensky = Opensky {
                zmin: self.geographic.altitude - EPSILON,
                zmax: zref,
            };
        } else {
            self.context.mode.direction = pumas::MODE_FORWARD;
            self.context.limit.energy = self.reference.energy.min();
            self.opensky = Opensky {
                zmin: zref,
                zmax: self.geographic.altitude + EPSILON,
            };
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

        // Compute the coordinates at end location (expected to be at zref).
        self.geographic = GeographicCoordinates::from_ecef(&self.state.position);
        if (self.geographic.altitude - zref).abs() > EPSILON {
            self.state.weight = 0.0;
            return Ok(())
        } else {
            self.geographic.altitude = zref;
            // due to potential rounding errors.
        }
        let direction = [
            -self.state.direction[0],
            -self.state.direction[1],
            -self.state.direction[2],
        ];
        self.horizontal = HorizontalCoordinates::from_ecef(&direction, &self.geographic);

        Ok(())
    }

    // Update the local geomagnetic field.
    fn update_geomagnet(&mut self) -> f64 {
        const UPDATE_RADIUS: f64 = 1E+03;
        let d2 = {
            let mut d2 = 0.0;
            for i in 0..3 {
                let tmp = self.state.position[i] - self.geomagnet_position[i];
                d2 += tmp * tmp;
            }
            d2
        };
        if d2 > UPDATE_RADIUS.powi(2) {
            // Get the local geomagnetic field (in ENU frame).
            let enu = self.geomagnet.as_mut().unwrap().field(
                self.geographic.latitude,
                self.geographic.longitude,
                self.geographic.altitude,
            ).unwrap();

            let frame = LocalFrame::new(self.geographic, 0.0, 0.0);
            self.geomagnet_field = frame.to_ecef_direction(&enu);
            self.geomagnet_position = self.state.position;
        }

        UPDATE_RADIUS / self.context.accuracy
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

impl<'py> Proxy<'py> {
    fn pinned_agent<'a>(
        &'a mut self,
        py: Python<'py>,
        fluxmeter: &'a mut Fluxmeter,
    ) -> PyResult<Pin<Box<Agent<'a>>>> {
        let pinned = Box::pin(Agent::new(
            py,
            &self.atmosphere,
            fluxmeter,
            &self.geometry,
            self.geomagnet.as_deref_mut(),
            &mut self.physics,
            &mut self.random,
            &self.reference,
        )?);
        Ok(pinned)
    }
}

impl<'a> GeometryAgent<'a> {
    fn frame(&self) -> Option<&'a LocalFrame> {
        match self {
            Self::Earth { .. } => None,
            Self::Local { frame, .. } => Some(frame),
        }
    }
}
