use crate::utils::error::Error;
use crate::utils::error::ErrorKind::ValueError;
use pyo3::prelude::*;
use std::ptr::NonNull;

pub struct OwnedPtr<T> (pub NonNull<T>) where NonNull<T>: Destroy;

unsafe impl<T> Send for OwnedPtr<T> where NonNull<T>: Destroy {}
unsafe impl<T> Sync for OwnedPtr<T> where NonNull<T>: Destroy {}

pub trait Destroy {
    fn destroy(self);
}

#[inline]
pub fn null_pointer_err() -> PyErr {
    Error::new(ValueError).what("pointer").why("null").to_err()
}

impl<T> OwnedPtr<T>
where
    NonNull<T>: Destroy
{
    pub fn new(ptr: *mut T) -> PyResult<Self> {
        NonNull::new(ptr)
            .map(|ptr| Self(ptr))
            .ok_or_else(|| null_pointer_err())
    }
}

impl<T> Drop for OwnedPtr<T>
where
    NonNull<T>: Destroy
{
    fn drop(&mut self) {
        self.0.destroy();
    }
}
