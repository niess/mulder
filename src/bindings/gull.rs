#![allow(unused)]

use ::std::ffi::{c_char, c_int, c_uint};

pub const SUCCESS: c_uint = 0;

pub type ErrorHandler = Option<
    unsafe extern "C" fn(rc: c_uint, function: Function, file: *const c_char, line: c_int)
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
}
