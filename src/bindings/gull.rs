#![allow(unused)]

use ::std::ffi::{c_char, c_int, c_uint};

pub const SUCCESS: c_uint = 0;

#[repr(C)]
pub struct Snapshot {
    _unused: [u8; 0],
}

pub type ErrorHandler = Option<
    unsafe extern "C" fn(rc: c_uint, function: Function, file: *const c_char)
>;

pub type Function = Option<
    unsafe extern "C" fn()
>;

#[link(name = "c-libs")]
extern "C" {
    #[link_name="gull_error_handler_set"]
    pub fn error_handler_set(handler: ErrorHandler);

    #[link_name="gull_error_function"]
    pub fn error_function(function: Function) -> *const c_char;

    #[link_name="gull_snapshot_create"]
    pub fn snapshot_create(
        snapshot: *mut *mut Snapshot,
        model: *const c_char,
        day: c_int,
        month: c_int,
        year: c_int,
    ) -> c_uint;

    #[link_name="gull_snapshot_destroy"]
    pub fn snapshot_destroy(snapshot: *mut *mut Snapshot);

    #[link_name="gull_snapshot_field"]
    pub fn snapshot_field(
        snapshot: *mut Snapshot,
        latitude: f64,
        longitude: f64,
        altitude: f64,
        magnet: *mut f64,
        workspace: *mut *mut f64,
    ) -> c_uint;

    #[link_name="gull_snapshot_info"]
    pub fn snapshot_info(
        snapshot: *mut Snapshot,
        order: *mut c_int,
        altitude_min: *mut f64,
        altitude_max: *mut f64,
    );
}
