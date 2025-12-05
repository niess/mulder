use crate::utils::error::Error;
use crate::utils::error::ErrorKind::ValueError;
use pyo3::prelude::*;
use std::ops::{Add, Mul, Sub};
use std::path::Path;


pub trait EnsureFile: Sized {
    fn ensure_file(self, what: &str) -> PyResult<Self>;
}

impl EnsureFile for &Path {
    fn ensure_file(self, what: &str) -> PyResult<Self> {
        if !self.is_file() {
            let why = if !self.exists() {
                format!("no such file '{}'", self.display())
            } else {
                format!("not a file '{}'", self.display())
            };
            let err = Error::new(ValueError).what(what).why(&why);
            return Err(err.to_err())
        }
        Ok(self)
    }
}

pub trait MinMax {
    type Number;

    fn min(&self) -> Self::Number;
    fn max(&self) -> Self::Number;

    fn mut_min(&mut self) -> &mut Self::Number;
    fn mut_max(&mut self) -> &mut Self::Number;
}

impl MinMax for (f64, f64) {
    type Number = f64;

    #[inline]
    fn min(&self) -> Self::Number {
        self.0
    }

    #[inline]
    fn max(&self) -> Self::Number {
        self.1
    }

    #[inline]
    fn mut_min(&mut self) -> &mut Self::Number {
        &mut self.0
    }

    #[inline]
    fn mut_max(&mut self) -> &mut Self::Number {
        &mut self.1
    }
}

#[allow(unused)]
pub trait Vector3: Sized {
    type Number:
        Copy +
        Add<Output=Self::Number> +
        Mul<Output=Self::Number> +
        Sub<Output=Self::Number>;

    fn vector3(x: Self::Number, y: Self::Number, z: Self::Number) -> Self;
    fn x(&self) -> Self::Number;
    fn y(&self) -> Self::Number;
    fn z(&self) -> Self::Number;

    #[inline]
    fn add(&self, rhs: &Self) -> Self {
        Self::vector3(self.x() + rhs.x(), self.y() + rhs.y(), self.z() + rhs.z())
    }

    #[inline]
    fn mul(&self, rhs: Self::Number) -> Self {
        Self::vector3(self.x() * rhs, self.y() * rhs, self.z() * rhs)
    }

    #[inline]
    fn sub(&self, rhs: &Self) -> Self {
        Self::vector3(self.x() - rhs.x(), self.y() - rhs.y(), self.z() - rhs.z())
    }

    #[inline]
    fn dot(&self, rhs: &Self) -> Self::Number {
        self.x() * rhs.x() +  self.y() * rhs.y() + self.z() * rhs.z()
    }
}

impl Vector3 for [f64; 3] {
    type Number = f64;

    #[inline]
    fn vector3(x: Self::Number, y: Self::Number, z: Self::Number) -> Self {
        [x, y, z]
    }

    #[inline]
    fn x(&self) -> Self::Number {
        self[0]
    }

    #[inline]
    fn y(&self) -> Self::Number {
        self[1]
    }

    #[inline]
    fn z(&self) -> Self::Number {
        self[2]
    }
}

pub trait TypeName {
    fn type_name(&self) -> String;
}

impl<'py> TypeName for Bound<'py, PyAny> {
    fn type_name(&self) -> String {
        const UNKNOWN: &str = "unknown";
        match self.get_type().getattr("__name__").ok() {
            Some(name) => {
                let name: String = name
                    .extract()
                    .ok()
                    .unwrap_or_else(|| UNKNOWN.to_owned());
                name
            },
            None => UNKNOWN.to_owned(),
        }
    }
}
