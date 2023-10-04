#![allow(unused)]

use ::std::ffi::{c_char, c_int, c_uint};

pub const SUCCESS: c_uint = 0;

#[repr(C)]
pub struct Stack {
    _unused: [u8; 0],
}

pub type ErrorHandler = Option<
    unsafe extern "C" fn(rc: c_uint, function: Function, message: *const c_char)
>;

pub type Function = Option<
    unsafe extern "C" fn()
>;

pub type Lock = Option<
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

    #[link_name="turtle_stack_create"]
    pub fn stack_create(
        stack: *mut *mut Stack,
        path: *const c_char,
        size: c_int,
        lock: Lock,
        unlock: Lock,
    ) -> c_uint;

    #[link_name="turtle_stack_destroy"]
    pub fn stack_destroy(stack: *mut *mut Stack);

    #[link_name="turtle_stack_info"]
    pub fn stack_info(
        stack: *const Stack,
        shape: *mut c_int,
        latitude: *mut f64,
        longitude: *mut f64,
    );
}
