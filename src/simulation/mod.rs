use crate::bindings::{turtle, pumas};
use crate::utils::coordinates::{GeographicCoordinates, HorizontalCoordinates};
use crate::utils::error::{self, Error};
use crate::utils::error::ErrorKind::{KeyboardInterrupt, TypeError, ValueError};
use crate::utils::extract::{Extractor, Field};
use crate::utils::numpy::{Dtype, NewArray};
use crate::utils::traits::MinMax;
use crate::geometry::{Doublet, Geometry, GeometryStepper};
use crate::geometry::atmosphere::Atmosphere;
use crate::geometry::layer::Layer;
use crate::utils::convert::TransportMode;
use crate::utils::io::PathString;
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
    use_external_layer: bool,
}

unsafe impl Send for Fluxmeter {}
unsafe impl Sync for Fluxmeter {}

struct Medium (Pin<Box<MediumData>>);

#[repr(C)]
struct MediumData {
    medium: pumas::Medium,
    density: f64,
}

#[repr(C)]
struct Agent<'a> {
    state: pumas::State,
    geographic: GeographicCoordinates,
    horizontal: HorizontalCoordinates,
    atmosphere: &'a Atmosphere,
    fluxmeter: &'a mut Fluxmeter,
    geometry: &'a Geometry,
    physics: &'a physics::Physics,
    reference: &'a reference::Reference,
    context: &'a mut pumas::Context,
}

#[repr(C)]
struct State {
    pid: f64,
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

        let mode = TransportMode::default();

        let reference = reference::Reference::new(None, None)?;
        let reference = Py::new(py, reference)?;

        let steppers = Default::default();
        let layers_media = Vec::new();
        let atmosphere_medium = Medium::default();
        let use_external_layer = false;

        let fluxmeter = Self {
            geometry, mode, physics, random, reference, steppers, atmosphere_medium, layers_media,
            use_external_layer,
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
    #[pyo3(signature=(states=None, /, **kwargs))]
    fn transport<'py>(
        &mut self,
        py: Python<'py>,
        states: Option<&Bound<PyAny>>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<NewArray<'py, State>> {
        let states = Extractor::from_args(
            [
                Field::maybe_int("pid"),
                Field::float("energy"),
                Field::float("latitude"),
                Field::float("longitude"),
                Field::float("altitude"),
                Field::float("azimuth"),
                Field::float("elevation"),
                Field::maybe_float("weight"),
            ],
            states,
            kwargs
        )?;
        let size = states.size();
        let shape = states.shape();

        // Configure physics, geometry, samplers etc.
        error::clear();
        let geometry = self.geometry.bind(py).borrow();
        let atmosphere = geometry.atmosphere.bind(py).borrow();
        let mut physics = self.physics.bind(py).borrow_mut();
        let mut random = self.random.bind(py).borrow_mut();
        let reference = self.reference.bind(py).borrow();
        let mut pinned = Box::pin(Agent::new(
            py,
            &atmosphere,
            self,
            &geometry,
            &mut physics,
            &mut random,
            &reference,
        )?);
        let agent: &mut Agent = &mut pinned.deref_mut();

        let mut array = NewArray::empty(py, shape)?;
        let result = array.as_slice_mut();
        for i in 0..size {
            agent.set_state(i, &states)?;
            if (agent.state.weight > 0.0) & (agent.state.energy > 0.0) {
                agent.transport()?;
            }
            result[i] = agent.get_state();
            if ((i % 100) == 0) && error::ctrlc_catched() {
                error::clear();
                let err = Error::new(KeyboardInterrupt)
                    .why("while transporting muon(s)");
                return Err(err.to_err())
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
            for layer in &geometry.layers {
                let layer = layer.bind(py).borrow();
                let medium = Medium::uniform(&layer, physics)?;
                self.layers_media.push(medium);
            }
            self.atmosphere_medium = Medium::atmosphere(physics)?;

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
    let agent: &Agent = state.into();
    let density = agent.atmosphere.compute_density(agent.geographic.altitude);
    unsafe {
        (*locals).density = density.value;
    }

    const LAMBDA_MAX: f64 = 1E+09;
    if density.lambda < LAMBDA_MAX {
        let direction = HorizontalCoordinates::from_ecef(
            &agent.state.direction,
            &agent.geographic,
        );
        let c = (direction.elevation.abs() * std::f64::consts::PI / 180.0).sin().max(0.1);
        (density.lambda / c).min(LAMBDA_MAX)
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
        } else if (layer == n + 1) || (agent.fluxmeter.use_external_layer && (layer == n + 2)) {
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
        let data = MediumData { medium, density };
        Self(Box::pin(data))
    }
}

impl<'a> Agent<'a> {
    const EPSILON: f64 = f32::EPSILON as f64;

    fn get_state(&self) -> State {
        State {
            pid: if self.state.charge > 0.0 { -13.0 } else { 13.0 },
            energy: self.state.energy,
            latitude: self.geographic.latitude,
            longitude: self.geographic.longitude,
            altitude: self.geographic.altitude,
            azimuth: self.horizontal.azimuth,
            elevation: self.horizontal.elevation,
            weight: self.state.weight,
        }
    }

    fn new(
        py: Python,
        atmosphere: &'a Atmosphere,
        fluxmeter: &'a mut Fluxmeter,
        geometry: &'a Geometry,
        physics: &'a mut physics::Physics,
        mut random: &'a mut random::Random,
        reference: &'a reference::Reference,
    ) -> PyResult<Self> {
        // Configure physics and geometry.
        physics.compile(py, None)?;
        fluxmeter.create_geometry(py, &geometry, &physics, &reference)?;

        // Configure Pumas context.
        let context = unsafe { &mut *physics.context };
        context.user_data = random.deref_mut() as *mut random::Random as *mut c_void;
        context.random = Some(uniform01);
        context.event = pumas::EVENT_LIMIT_ENERGY;

        // Initialise particles state.
        let state = pumas::State::default();
        let geographic = GeographicCoordinates::default();
        let horizontal = HorizontalCoordinates::default();

        let agent = Self {
            state, geographic, horizontal, atmosphere, fluxmeter, geometry, physics, reference,
            context
        };
        Ok(agent)
    }

    fn set_state<'py>(
        &mut self,
        index: usize,
        extractor: &Extractor<'py, 8>,
    ) -> PyResult<()> {
        let [ pid, energy, latitude, longitude, altitude, azimuth, elevation, weight ] =
            extractor.get(index)?;
        let pid = pid.into_i32_opt().unwrap_or_else(|| 13);
        self.state.charge = if pid == 13 { -1.0 } else if pid == -13 { 1.0 } else {
            let why = format!("expected '13' or '-13', found {}", pid);
            let err = Error::new(ValueError).what("pid").why(&why).to_err();
            return Err(err)
        };
        self.state.energy = energy.into_f64();
        self.geographic = GeographicCoordinates {
            latitude: latitude.into_f64(),
            longitude: longitude.into_f64(),
            altitude: altitude.into_f64(),
        };
        self.state.position = self.geographic.to_ecef();
        self.horizontal = HorizontalCoordinates {
            azimuth: azimuth.into_f64(),
            elevation: elevation.into_f64(),
        };
        let direction = self.horizontal.to_ecef(&self.geographic);
        for i in 0..3 { self.state.direction[i] = -direction[i]; }  // Observer convention.
        self.state.weight = weight.into_f64_opt().unwrap_or_else(|| 1.0);
        self.state.distance = 0.0;
        self.state.grammage = 0.0;
        self.state.time = 0.0;
        self.state.decayed = 0;
        self.fluxmeter.use_external_layer =
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
                    self.physics.physics, pumas::MODE_CSDA, material, e0, &mut dedx0,
                );
                pumas::physics_property_stopping_power(
                    self.physics.physics, pumas::MODE_CSDA, material, self.state.energy,
                    &mut dedx1,
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

static STATE_DTYPE: GILOnceCell<PyObject> = GILOnceCell::new();

impl Dtype for State {
    fn dtype<'py>(py: Python<'py>) -> PyResult<&'py Bound<'py, PyAny>> {
        let ob = STATE_DTYPE.get_or_try_init(py, || -> PyResult<_> {
            let ob = PyModule::import(py, "numpy")?
                .getattr("dtype")?
                .call1(([
                        ("pid",       "f8"),
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
