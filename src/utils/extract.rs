use crate::utils::error::Error;
use crate::utils::error::ErrorKind::TypeError;
use crate::utils::numpy::{AnyArray, ArrayMethods, Dtype};
use pyo3::prelude::*;
use pyo3::types::PyDict;


// ===============================================================================================
//
// Generic attributes extractor.
//
// ===============================================================================================

pub struct Extractor<'py, const N: usize> {
    pub data: Vec<FieldArray<'py>>,
    size: Size,
}

pub struct Field<'a> {
    pub name: &'a str,
    pub kind: FieldKind,
}

pub enum FieldKind {
    Float,
    #[allow(unused)] // XXX needed?
    Int,
    MaybeFloat,
    MaybeInt,
}

pub enum FieldArray<'py> {
    Float(AnyArray<'py, f64>),
    Int(AnyArray<'py, i32>),
    MaybeFloat(Option<AnyArray<'py, f64>>),
    MaybeInt(Option<AnyArray<'py, i32>>),
}

#[derive(Clone, Copy)]
pub enum FieldValue {
    Float(f64),
    #[allow(unused)] // XXX needed?
    Int(i32),
    MaybeFloat(Option<f64>),
    MaybeInt(Option<i32>),
}

impl<'a, 'py, const N: usize> Extractor<'py, N> {
    pub fn new(fields: [Field<'a>; N], ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        let py = ob.py();
        let mut data = Vec::with_capacity(N);
        for field in &fields {
            data.push(field.kind.extract(py, ob, field.name)?);
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
        fields: [Field<'a>; N],
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
                        if !fields.iter().any(|field| field.name.eq(key.as_str())) {
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
                    let why = format!("missing '{}'", fields[0].name);
                    let err = Error::new(TypeError).why(&why);
                    return Err(err.to_err())
                },
            },
        };
        Self::new(fields, ob)
    }

    pub fn get(&self, i: usize) -> PyResult<[FieldValue; N]> {
        let mut data = [FieldValue::default(); N];
        for j in 0..N {
            data[j] = match &self.data[j] {
                FieldArray::Float(array) => FieldValue::Float(array.get_item(i)?),
                FieldArray::Int(array) => FieldValue::Int(array.get_item(i)?),
                FieldArray::MaybeFloat(array) => FieldValue::MaybeFloat(
                    array.as_ref().map(|array| array.get_item(i)).transpose()?
                ),
                FieldArray::MaybeInt(array) => FieldValue::MaybeInt(
                    array.as_ref().map(|array| array.get_item(i)).transpose()?
                ),
            };
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

impl<'a> Field<'a> {
    pub fn float(name: &'a str) -> Self {
        Self { name, kind: FieldKind::Float }
    }

    #[allow(unused)] // XXX needed?
    pub fn int(name: &'a str) -> Self {
        Self { name, kind: FieldKind::Int }
    }

    pub fn maybe_float(name: &'a str) -> Self {
        Self { name, kind: FieldKind::MaybeFloat }
    }

    pub fn maybe_int(name: &'a str) -> Self {
        Self { name, kind: FieldKind::MaybeInt }
    }
}

impl FieldValue {
    pub fn into_f64(self) -> f64 {
        match self {
            Self::Float(value) => value,
            _ => unreachable!(),
        }
    }

    pub fn into_f64_opt(self) -> Option<f64> {
        match self {
            Self::MaybeFloat(value) => value,
            _ => unreachable!(),
        }
    }

    pub fn into_i32_opt(self) -> Option<i32> {
        match self {
            Self::MaybeInt(value) => value,
            _ => unreachable!(),
        }
    }
}

impl Default for FieldValue {
    fn default() -> Self {
        Self::MaybeInt(None)
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
    fn new(array: &FieldArray) -> Self {
        match array {
            FieldArray::Float(array) => Self::from_typed::<f64>(Some(array)),
            FieldArray::Int(array) => Self::from_typed::<i32>(Some(array)),
            FieldArray::MaybeFloat(array) => Self::from_typed::<f64>(array.as_ref()),
            FieldArray::MaybeInt(array) => Self::from_typed::<i32>(array.as_ref()),
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

    fn from_typed<'py, T: Clone + Dtype>(array: Option<&AnyArray<'py, T>>) -> Self {
        match array {
            Some(array) => if array.ndim() == 0 {
                Self::Scalar
            } else {
                Self::Array { size: array.size(), shape: array.shape() }
            },
            None => Self::Scalar,
        }
    }
}


// ===============================================================================================
//
// Generic extraction.
//
// ===============================================================================================

fn extract<'py, T: Clone + Dtype>(
    py: Python<'py>,
    ob: &Bound<'py, PyAny>,
    key: &str
) -> PyResult<Option<AnyArray<'py, T>>> {
    let value: Option<AnyArray<'py, T>> = ob
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

fn require<'py, T: Copy + Dtype>(
    py: Python<'py>,
    ob: &Bound<'py, PyAny>,
    key: &str
) -> PyResult<AnyArray<'py, T>> {
    extract(py, ob, key)?
        .ok_or_else(|| {
            let why = format!("missing '{}'", key);
            Error::new(TypeError).why(&why).to_err()
        })
}

impl FieldKind {
    fn extract<'py>(
        &self,
        py: Python<'py>,
        ob: &Bound<'py, PyAny>,
        key: &str,
    ) -> PyResult<FieldArray<'py>> {
        let array = match self {
            Self::Float => FieldArray::Float(require::<f64>(py, ob, key)?),
            Self::Int => FieldArray::Int(require::<i32>(py, ob, key)?),
            Self::MaybeFloat => FieldArray::MaybeFloat(extract::<f64>(py, ob, key)?),
            Self::MaybeInt => FieldArray::MaybeInt(extract::<i32>(py, ob, key)?),
        };
        Ok(array)
    }
}
