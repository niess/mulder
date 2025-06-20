use crate::utils::error::Error;
use crate::utils::error::ErrorKind::TypeError;
use crate::utils::numpy::{AnyArray, ArrayMethods};
use pyo3::prelude::*;
use pyo3::types::PyDict;


// ===============================================================================================
//
// Generic attributes extractor.
//
// ===============================================================================================

pub struct Extractor<'py, const N: usize> {
    pub data: Vec<AnyArray<'py, f64>>,
    size: Size,
}

impl<'py, const N: usize> Extractor<'py, N> {
    pub fn new(fields: [&str; N], ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        let py = ob.py();
        let mut data = Vec::with_capacity(N);
        for field in &fields {
            match extract(py, ob, field)? {
                Some(array) => data.push(array),
                None => {
                    let why = format!("missing '{}'", field);
                    let err = Error::new(TypeError).why(&why);
                    return Err(err.to_err())
                },
            }
        }
        let mut size = Size::new(&data[0]);
        for i in 1..N {
            size = size.common(&Size::new(&data[i]))
                .ok_or_else(|| Error::new(TypeError)
                    .why("inconsistent arrays sizes")
                    .to_err()
                )?.clone();
        }

        Ok(Self { data, size })
    }

    pub fn from_args(
        fields: [&str; N],
        array: Option<&Bound<'py, PyAny>>,
        kwargs: Option<&Bound<'py, PyDict>>,
    ) -> PyResult<Self> {
        let ob = match array {
            Some(array) => match kwargs {
                Some(_) => {
                    let err = Error::new(TypeError)
                        .what("arguments")
                        .why("cannot mix positional and keyword only arguments");
                    return Err(err.to_err())
                },
                None => array,
            },
            None => match kwargs {
                Some(kwargs) => {
                    for key in kwargs.keys() {
                        let key: String = key.extract()?;
                        if !fields.contains(&key.as_str()) {
                            let why = format!("invalid keyword argument '{}'", key);
                            let err = Error::new(TypeError)
                                .what("kwargs")
                                .why(&why);
                            return Err(err.to_err())
                        }
                    }
                    kwargs.as_any()
                },
                None => {
                    let why = format!("missing '{}'", fields[0]);
                    let err = Error::new(TypeError).why(&why);
                    return Err(err.to_err())
                },
            },
        };
        Self::new(fields, ob)
    }

    pub fn get(&self, i: usize) -> PyResult<[f64; N]> {
        let mut data = [0.0; N];
        for j in 0..N {
            data[j] = self.data[j].get_item(i)?;
        }
        Ok(data)
    }

    pub fn shape(&self) -> Vec<usize> {
        match &self.size {
            Size::Scalar => Vec::new(),
            Size::Array { shape, .. } => shape.clone(),
        }
    }

    pub fn size(&self) -> usize {
        match &self.size {
            Size::Scalar => 1,
            Size::Array { size, .. } => *size,
        }
    }
}


// ===============================================================================================
//
// Managed array size.
//
// ===============================================================================================

#[derive(Clone)]
enum Size {
    Scalar,
    Array { size: usize, shape: Vec<usize> },
}

impl Size {
    fn new(array: &AnyArray<f64>) -> Self {
        if array.ndim() == 0 {
            Self::Scalar
        } else {
            Self::Array { size: array.size(), shape: array.shape() }
        }
    }

    fn common<'a>(&'a self, other: &'a Self) -> Option<&'a Self> {
        match self {
            Self::Scalar => Some(other),
            Self::Array { size, .. } => match other {
                Self::Scalar => Some(self),
                Self::Array { size: other_size, .. } => if size == other_size {
                    Some(self)
                } else {
                    None
                }
            }
        }
    }
}


// ===============================================================================================
//
// Generic extraction.
//
// ===============================================================================================

fn extract<'py>(
    py: Python<'py>,
    ob: &Bound<'py, PyAny>,
    key: &str
) -> PyResult<Option<AnyArray<'py, f64>>> {
    let value: Option<AnyArray<'py, f64>> = ob
        .get_item(key)
        .ok()
        .and_then(|a| Some(a.extract())).transpose()
        .map_err(|err| {
            Error::new(TypeError)
                .what(key)
                .why(&err.value(py).to_string()).to_err()
        })?;
    Ok(value)
}
