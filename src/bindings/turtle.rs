#![allow(unused)]

use ::std::ffi::{c_char, c_int, c_uint, c_void};
use ::std::ptr::null;

pub const SUCCESS: c_uint = 0;

#[repr(C)]
pub struct List {
    pub head: *mut c_void,
    pub tail: *mut c_void,
    pub size: c_int,
}

#[repr(C)]
pub struct ListElement {
    pub previous: *mut c_void,
    pub next: *mut c_void,
}

#[repr(C)]
pub struct Map {
    pub element: ListElement,
    pub meta: MapMeta,
}

#[repr(C)]
pub struct MapInfo {
    pub nx: c_int,
    pub ny: c_int,
    pub x: [f64; 2],
    pub y: [f64; 2],
    pub z: [f64; 2],
    pub encoding: *const c_char,
}

#[repr(C)]
pub struct MapMeta {
    pub nx: c_int,
    pub ny: c_int,
    pub x0: f64,
    pub y0: f64,
    pub z0: f64,
    pub dx: f64,
    pub dy: f64,
    pub dz: f64,
    pub get_z: GetZ,
    pub set_z: SetZ,
    pub encoding: [c_char; 8],
    pub projection: Projection,
}

impl Default for MapInfo {
    fn default() -> Self {
        Self {
            nx: 0,
            ny: 0,
            x: [f64::NAN; 2],
            y: [f64::NAN; 2],
            z: [f64::NAN; 2],
            encoding: null(),
        }
    }
}

#[repr(C)]
pub struct Projection {
    _unused: [u8; 0]
}

#[repr(C)]
pub struct Stack {
    pub list: List,
}

#[repr(C)]
pub struct Stepper {
    _unused: [u8; 0]
}

pub type ErrorHandler = Option<
    unsafe extern "C" fn(rc: c_uint, function: Function, message: *const c_char)
>;

pub type Function = Option<
    unsafe extern "C" fn()
>;

pub type GetZ = Option<
    unsafe extern "C" fn() -> c_int,
>;

pub type Lock = Option<
    unsafe extern "C" fn() -> c_int,
>;

pub type SetZ = Option<
    unsafe extern "C" fn() -> c_int,
>;

pub type Unlock = Option<
    unsafe extern "C" fn() -> c_int,
>;

#[link(name = "c-libs")]
extern "C" {
    #[link_name="turtle_ecef_from_geodetic"]
    pub fn ecef_from_geodetic(
        latitude: f64,
        longitude: f64,
        elevation: f64,
        ecef: *mut f64
    );

    #[link_name="turtle_ecef_from_horizontal"]
    pub fn ecef_from_horizontal(
        latitude: f64,
        longitude: f64,
        azimuth: f64,
        elevation: f64,
        direction: *mut f64,
    );

    #[link_name="turtle_ecef_to_geodetic"]
    pub fn ecef_to_geodetic(
        ecef: *const f64,
        latitude: *mut f64,
        longitude: *mut f64,
        altitude: *mut f64,
    );

    #[link_name="turtle_ecef_to_horizontal"]
    pub fn ecef_to_horizontal(
        latitude: f64,
        longitude: f64,
        direction: *const f64,
        azimuth: *mut f64,
        elevation: *mut f64,
    );

    #[link_name="turtle_error_handler_set"]
    pub fn error_handler_set(handler: ErrorHandler);

    #[link_name="turtle_error_function"]
    pub fn error_function(function: Function) -> *const c_char;

    #[link_name="turtle_map_create"]
    pub fn map_create(
        map: *mut *mut Map,
        info: *const MapInfo,
        projection: *const c_char,
    ) -> c_uint;

    #[link_name="turtle_map_destroy"]
    pub fn map_destroy(map: *mut *mut Map);

    #[link_name="turtle_map_elevation"]
    pub fn map_elevation(
        map: *const Map,
        x: f64,
        y: f64,
        elevation: *mut f64,
        inside: *mut c_int,
    ) -> c_uint;

    #[link_name="turtle_map_fill"]
    pub fn map_fill(
        map: *const Map,
        ix: c_int,
        iy: c_int,
        elevation: f64,
    ) -> c_uint;

    #[link_name="turtle_map_gradient"]
    pub fn map_gradient(
        map: *const Map,
        x: f64,
        y: f64,
        gx: *mut f64,
        gy: *mut f64,
        inside: *mut c_int,
    ) -> c_uint;

    #[link_name="turtle_map_load"]
    pub fn map_load(
        map: *mut *mut Map,
        path: *const c_char,
    ) -> c_uint;

    #[link_name="turtle_map_meta"]
    pub fn map_meta(
        map: *const Map,
        info: *mut MapInfo,
        projection: *mut *const c_char,
    );

    #[link_name="turtle_map_node"]
    pub fn map_node(
        map: *const Map,
        ix: c_int,
        iy: c_int,
        x: *mut f64,
        y: *mut f64,
        elevation: *mut f64,
    ) -> c_uint;

    #[link_name="turtle_map_projection"]
    pub fn map_projection(map: *const Map) -> *const Projection;

    #[link_name="turtle_projection_configure"]
    pub fn projection_configure(projection: *mut Projection, name: *const c_char) -> c_uint;

    #[link_name="turtle_projection_create"]
    pub fn projection_create(projection: *mut *mut Projection, name: *const c_char) -> c_uint;

    #[link_name="turtle_projection_destroy"]
    pub fn projection_destroy(projection: *mut *mut Projection) -> c_uint;

    #[link_name="turtle_projection_project"]
    pub fn projection_project(
        projection: *const Projection,
        latitude: f64,
        longitude: f64,
        x: &mut f64,
        y: &mut f64,
    ) -> c_uint;

    #[link_name="turtle_stack_create"]
    pub fn stack_create(
        stack: *mut *mut Stack,
        path: *const c_char,
        size: c_int,
        lock: Lock,
        unlock: Unlock,
    ) -> c_uint;

    #[link_name="turtle_stack_destroy"]
    pub fn stack_destroy(stack: *mut *mut Stack);

    #[link_name="turtle_stack_elevation"]
    pub fn stack_elevation(
        stack: *const Stack,
        latitude: f64,
        longitude: f64,
        elevation: *mut f64,
        inside: *mut c_int,
    ) -> c_uint;

    #[link_name="turtle_stack_gradient"]
    pub fn stack_gradient(
        stack: *const Stack,
        latitude: f64,
        longitude: f64,
        glat: *mut f64,
        glon: *mut f64,
        inside: *mut c_int,
    ) -> c_uint;

    #[link_name="turtle_stack_info"]
    pub fn stack_info(
        stack: *const Stack,
        shape: *mut c_int,
        latitude: *mut f64,
        longitude: *mut f64,
    );

    #[link_name="turtle_stack_load"]
    pub fn stack_load(stack: *mut Stack) -> c_uint;

    #[link_name="turtle_stepper_add_flat"]
    pub fn stepper_add_flat(stepper: *mut Stepper, ground_level: f64) -> c_uint;

    #[link_name="turtle_stepper_add_layer"]
    pub fn stepper_add_layer(stepper: *mut Stepper) -> c_uint;

    #[link_name="turtle_stepper_add_map"]
    pub fn stepper_add_map(stepper: *mut Stepper, map: *mut Map, offset: f64) -> c_uint;

    #[link_name="turtle_stepper_add_stack"]
    pub fn stepper_add_stack(stepper: *mut Stepper, stack: *mut Stack, offset: f64) -> c_uint;

    #[link_name="turtle_stepper_create"]
    pub fn stepper_create(stepper: *mut *mut Stepper) -> c_uint;

    #[link_name="turtle_stepper_destroy"]
    pub fn stepper_destroy(stepper: *mut *mut Stepper) -> c_uint;

    #[link_name="turtle_stepper_reset"]
    pub fn stepper_reset(stepper: *mut Stepper);

    #[link_name="turtle_stepper_step"]
    pub fn stepper_step(
        stepper: *mut Stepper,
        position: *mut f64,
        direction: *const f64,
        latitude: *mut f64,
        longitude: *mut f64,
        altitude: *mut f64,
        elevation: *mut f64,
        step: *mut f64,
        index: *mut c_int,
    ) -> c_uint;
}
