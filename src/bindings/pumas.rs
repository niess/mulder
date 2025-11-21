#![allow(unused)]

use ::std::ffi::{c_char, c_double, c_int, c_long, c_uint, c_void};

pub const MUON: c_uint = 0;
pub const TAU: c_uint = 1;

pub const SUCCESS: c_uint = 0;
pub const ACCURACY_ERROR: c_uint = 1;
pub const END_OF_FILE: c_uint = 2;
pub const DECAY_ERROR: c_uint = 3;
pub const DENSITY_ERROR: c_uint = 4;
pub const DIRECTION_ERROR: c_uint = 5;
pub const INCOMPLETE_FILE: c_uint = 6;
pub const INDEX_ERROR: c_uint = 7;
pub const PHYSICS_ERROR: c_uint = 8;
pub const INTERNAL_ERROR: c_uint = 9;
pub const IO_ERROR: c_uint = 10;
pub const FORMAT_ERROR: c_uint = 11;
pub const MEDIUM_ERROR: c_uint = 12;
pub const MEMORY_ERROR: c_uint = 13;
pub const MODEL_ERROR: c_uint = 14;
pub const MISSING_LIMIT: c_uint = 15;
pub const MISSING_RANDOM: c_uint = 16;
pub const PATH_ERROR: c_uint = 17;
pub const RAISE_ERROR: c_uint = 18;
pub const TOO_LONG: c_uint = 19;
pub const UNDEFINED_MDF: c_uint = 20;
pub const UNKNOWN_ELEMENT: c_uint = 21;
pub const UNKNOWN_MATERIAL: c_uint = 22;
pub const UNKNOWN_PARTICLE: c_uint = 23;
pub const VALUE_ERROR: c_uint = 24;
pub const INTERRUPT: c_uint = 25;

pub const BREMSSTRAHLUNG: c_uint = 0;
pub const PAIR_PRODUCTION: c_uint = 1;
pub const PHOTONUCLEAR: c_uint = 2;

pub const EVENT_NONE: c_uint = 0;
pub const EVENT_LIMIT_ENERGY: c_uint = 1;
pub const EVENT_LIMIT_DISTANCE: c_uint = 2;
pub const EVENT_LIMIT_GRAMMAGE: c_uint = 4;
pub const EVENT_LIMIT_TIME: c_uint = 8;
pub const EVENT_LIMIT: c_uint = 15;
pub const EVENT_MEDIUM: c_uint = 16;
pub const EVENT_VERTEX_BREMSSTRAHLUNG: c_uint = 32;
pub const EVENT_VERTEX_PAIR_CREATION: c_uint = 64;
pub const EVENT_VERTEX_PHOTONUCLEAR: c_uint = 128;
pub const EVENT_VERTEX_DELTA_RAY: c_uint = 256;
pub const EVENT_VERTEX_DEL: c_uint = 480;
pub const EVENT_VERTEX_COULOMB: c_uint = 512;
pub const EVENT_VERTEX_DECAY: c_uint = 1024;
pub const EVENT_VERTEX: c_uint = 2016;
pub const EVENT_WEIGHT: c_uint = 2048;
pub const EVENT_START: c_uint = 4096;
pub const EVENT_STOP: c_uint = 8192;

pub const MODE_DISABLED: c_int = -1;
pub const MODE_CSDA: c_int = 0;
pub const MODE_MIXED: c_int = 1;
pub const MODE_STRAGGLED: c_int = 2;
pub const MODE_WEIGHTED: c_int = 0;
pub const MODE_RANDOMISED: c_int = 1;
pub const MODE_FORWARD: c_int = 0;
pub const MODE_BACKWARD: c_int = 1;

pub const STEP_CHECK: c_uint = 0;
pub const STEP_RAW: c_uint = 1;

pub const STEP_MIN: c_double = 1E-07;

#[repr(C)]
pub struct Context {
    pub medium: MediumCallback,
    pub random: RandomCallback,
    pub recorder: Recorder,
    pub user_data: *mut c_void,
    pub mode: ContextMode,
    pub event: c_uint,
    pub limit: ContextLimit,
    pub accuracy: c_double,
}

#[repr(C)]
pub struct ContextLimit {
    pub energy: c_double,
    pub distance: c_double,
    pub grammage: c_double,
    pub time: c_double,
}

#[repr(C)]
pub struct ContextMode {
    pub energy_loss: c_int,
    pub decay: c_int,
    pub direction: c_int,
    pub scattering: c_int,
}

#[repr(C)]
pub struct Locals {
    pub density: c_double,
    pub magnet: [c_double; 3],
}

#[repr(C)]
pub struct Medium {
    pub material: c_int,
    pub locals: LocalsCallback,
}

#[repr(C)]
pub struct Physics {
    _unused: [u8; 0],
}

#[repr(C)]
pub struct PhysicsSettings {
    pub cutoff: c_double,
    pub elastic_ratio: c_double,
    pub bremsstrahlung: *const c_char,
    pub pair_production: *const c_char,
    pub photonuclear: *const c_char,
    pub n_energies: c_int,
    pub energy: *mut c_double,
    pub update: c_int,
    pub dry: c_int,
}

#[repr(C)]
pub struct Recorder {
    // XXX Implement recording?
    length: c_int,
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct State {
    pub charge: c_double,
    pub energy: c_double,
    pub distance: c_double,
    pub grammage: c_double,
    pub time: c_double,
    pub weight: c_double,
    pub position: [c_double; 3],
    pub direction: [c_double; 3],
    pub decayed: c_int,
}

#[allow(non_snake_case)]
pub type Dcs = Option<
    unsafe extern "C" fn(
        Z: c_double, A: c_double, m: c_double, K: c_double, q: c_double,
    ) -> c_double,
>;

#[repr(C)]
pub struct PhysicsNotifier {
    pub configure: Configure,
    pub notify: Notify,
}

pub type Configure = Option<
    unsafe extern "C" fn(slf: *mut PhysicsNotifier, *const c_char, steps: c_int) -> c_uint,
>;

pub type ErrorHandler = Option<
    unsafe extern "C" fn(rc: c_uint, function: Function, message: *const c_char)
>;

pub type Function = Option<
    unsafe extern "C" fn()
>;

pub type LocalsCallback = Option<
    unsafe extern "C" fn(medium: *mut Medium, state: *mut State, locals: *mut Locals) -> c_double
>;

pub type MediumCallback = Option<
    unsafe extern "C" fn(
        context: *mut Context,
        state: *mut State,
        medium: *mut *mut Medium,
        step: *mut c_double,
    ) -> c_uint
>;

pub type Notify = Option<
    unsafe extern "C" fn(slf: *mut PhysicsNotifier) -> c_uint,
>;

pub type RandomCallback = Option<
    unsafe extern "C" fn(context: *mut Context) -> c_double
>;

#[link(name = "c-libs")]
extern "C" {
    #[link_name="pumas_context_create"]
    pub fn context_create(
        context: *mut *mut Context,
        physics: *const Physics,
        extra_memory: c_int
    ) -> c_uint;

    #[link_name="pumas_context_destroy"]
    pub fn context_destroy(context: *mut *mut Context);

    #[link_name="pumas_context_transport"]
    pub fn context_transport(
        context: *mut Context,
        state: *mut State,
        event: *mut c_uint,
        media: *mut *mut Medium,
    ) -> c_uint;

    #[link_name="pumas_error_handler_set"]
    pub fn error_handler_set(handler: ErrorHandler);

    #[link_name="pumas_error_function"]
    pub fn error_function(function: Function) -> *const c_char;

    #[link_name="pumas_physics_composite_properties"]
    pub fn physics_composite_properties(
        physics: *const Physics,
        index: c_int,
        length: *mut c_int,
        components: *mut c_int,
        fractions: *mut c_double,
    ) -> c_uint;

    #[link_name="pumas_physics_composite_update"]
    pub fn physics_composite_update(
        physics: *const Physics,
        material: c_int,
        fractions: *const c_double,
    ) -> c_uint;

    #[link_name="pumas_physics_create"]
    pub fn physics_create(
        physics: *mut *mut Physics,
        particle: c_uint,
        mdf_path: *const c_char,
        dedx_path: *const c_char,
        settings: *const PhysicsSettings,
        notifier: *mut PhysicsNotifier,
    ) -> c_uint;

    #[link_name="pumas_physics_cutoff"]
    pub fn physics_cutoff(physics: *const Physics) -> f64;

    #[link_name="pumas_physics_dcs"]
    pub fn physics_dcs(
        physics: *const Physics,
        process: c_uint,
        model: *mut *const c_char,
        dcs: *mut Dcs,
    ) -> c_uint;

    #[link_name="pumas_physics_destroy"]
    pub fn physics_destroy(physics: *mut *mut Physics);

    #[link_name="pumas_physics_dump"]
    pub fn physics_dump(physics: *const Physics, stream: *mut libc::FILE) -> c_uint;

    #[link_name="pumas_physics_elastic_ratio"]
    pub fn physics_elastic_ratio(physics: *const Physics) -> f64;

    #[link_name="pumas_physics_load"]
    pub fn physics_load(physics: *mut *mut Physics, stream: *mut libc::FILE) -> c_uint;

    #[link_name="pumas_physics_material_index"]
    pub fn physics_material_index(
        physics: *const Physics,
        name: *const c_char,
        index: *mut c_int
    ) -> c_uint;

    #[link_name="pumas_physics_particle"]
    pub fn physics_particle(
        physics: *const Physics,
        particle: *mut c_uint,
        ctau: *mut c_double,
        mass: *mut c_double
    ) -> c_uint;

    #[link_name="pumas_physics_property_cross_section"]
    pub fn physics_property_cross_section(
        physics: *const Physics,
        material: c_int,
        energy: c_double,
        value: *mut c_double,
    ) -> c_uint;

    #[link_name="pumas_physics_property_elastic_cutoff_angle"]
    pub fn physics_property_elastic_cutoff_angle(
        physics: *const Physics,
        material: c_int,
        energy: c_double,
        value: *mut c_double,
    ) -> c_uint;

    #[link_name="pumas_physics_property_elastic_path"]
    pub fn physics_property_elastic_path(
        physics: *const Physics,
        material: c_int,
        energy: c_double,
        value: *mut c_double,
    ) -> c_uint;

    #[link_name="pumas_physics_property_range"]
    pub fn physics_property_range(
        physics: *const Physics,
        mode: c_int,
        material: c_int,
        energy: c_double,
        value: *mut c_double,
    ) -> c_uint;

    #[link_name="pumas_physics_property_stopping_power"]
    pub fn physics_property_stopping_power(
        physics: *const Physics,
        mode: c_int,
        material: c_int,
        energy: c_double,
        value: *mut c_double,
    ) -> c_uint;

    #[link_name="pumas_physics_property_transport_path"]
    pub fn physics_property_transport_path(
        physics: *const Physics,
        mode: c_int,
        material: c_int,
        energy: c_double,
        value: *mut c_double,
    ) -> c_uint;
}
