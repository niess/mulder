use crate::utils::error::Error;
use crate::utils::error::ErrorKind::TypeError;
use crate::utils::numpy::{AnyArray, ArrayMethods, Dtype};
use enum_variants_strings::EnumVariantsStrings;
use pyo3::prelude::*;
use pyo3::types::PyDict;


// ===============================================================================================
//
// Generic attributes extractor.
//
// ===============================================================================================

pub struct Extractor<'py, const N: usize> {
    data: Vec<FieldArray<'py>>,
    map: [usize; NUMBER_OF_NAMES],
    size: Size,
}

pub struct Field {
    name: Name,
    kind: Kind,
}

#[derive(Clone, Copy, EnumVariantsStrings, PartialEq)]
#[enum_variants_strings_transform(transform="lower_case")]
pub enum Name {
    Altitude = 0,
    Azimuth,
    Direction,
    Elevation,
    Energy,
    Latitude,
    Longitude,
    Pid,
    Position,
    Weight,
}

const NUMBER_OF_NAMES: usize = (Name::Weight as usize) + 1;

enum Kind {
    Float,
    Int,
    Vec3,
    MaybeFloat,
    MaybeInt,
    MaybeVec3,
}

enum FieldArray<'py> {
    Float(AnyArray<'py, f64>),
    Int(AnyArray<'py, i32>),
    Vec3(AnyArray<'py, f64>),
    MaybeFloat(Option<AnyArray<'py, f64>>),
    MaybeInt(Option<AnyArray<'py, i32>>),
    MaybeVec3(Option<AnyArray<'py, f64>>),
}

impl<'a, 'py, const N: usize> Extractor<'py, N> {
    pub fn new(fields: [Field; N], ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        let py = ob.py();
        let mut map = [0; NUMBER_OF_NAMES];
        let mut data = Vec::with_capacity(N);
        for (i, field) in fields.iter().enumerate() {
            map[field.name as usize] = i;
            data.push(field.extract(py, ob)?);
        }
        let mut size = Size::new(&data[0])
            .map_err(|why|
                Error::new(TypeError).what(fields[0].name.to_str()).why(&why).to_err()
            )?;
        for i in 1..N {
            let si = Size::new(&data[i])
                .map_err(|why|
                    Error::new(TypeError).what(fields[i].name.to_str()).why(&why).to_err()
                )?;
            size = size.common(&si)
                .ok_or_else(|| Error::new(TypeError)
                    .why("inconsistent arrays sizes")
                    .to_err()
                )?.clone();
        }

        Ok(Self { data, map, size })
    }

    pub fn from_args(
        fields: [Field; N],
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
                        if !fields.iter().any(|field| field.name.to_str().eq(key.as_str())) {
                            let why = format!("invalid keyword argument '{}'", key);
                            let err = Error::new(TypeError)
                                .what("kwargs")
                                .why(&why);
                            return Err(err.to_err())
                        }
                    }
                    kwargs.as_any()
                },
                None => return Self::from_none(fields),
            },
        };
        Self::new(fields, ob)
    }

    pub fn from_none(fields: [Field; N]) -> PyResult<Self> {
        let mut map = [0; NUMBER_OF_NAMES];
        let mut data = Vec::with_capacity(N);
        for (i, field) in fields.iter().enumerate() {
            if !field.kind.is_opt() {
                let why = format!("missing '{}'", field.name.to_str());
                return Err(Error::new(TypeError).why(&why).to_err())
            }
            map[field.name as usize] = i;
            data.push(field.extract_none());
        }
        let size = Size::Scalar;

        Ok(Self { data, map, size })
    }

    pub fn contains(&self, name: Name) -> bool {
        let j = self.map[name as usize];
        match &self.data[j] {
            FieldArray::MaybeFloat(opt) => opt.is_some(),
            FieldArray::MaybeInt(opt) => opt.is_some(),
            _ => true,
        }
    }

    pub fn get_i32(&self, name: Name, i: usize) -> PyResult<i32> {
        let j = self.map[name as usize];
        match &self.data[j] {
            FieldArray::Int(array) => array.get_item(i),
            _ => unreachable!(),
        }
    }

    pub fn get_i32_opt(&self, name: Name, i: usize) -> PyResult<Option<i32>> {
        let j = self.map[name as usize];
        match &self.data[j] {
            FieldArray::MaybeInt(array) => array
                .as_ref()
                .map(|array| array.get_item(i))
                .transpose(),
            _ => unreachable!(),
        }
    }

    pub fn get_f64(&self, name: Name, i: usize) -> PyResult<f64> {
        let j = self.map[name as usize];
        match &self.data[j] {
            FieldArray::Float(array) => array.get_item(i),
            _ => unreachable!(),
        }
    }

    pub fn get_f64_opt(&self, name: Name, i: usize) -> PyResult<Option<f64>> {
        let j = self.map[name as usize];
        match &self.data[j] {
            FieldArray::MaybeFloat(array) => array
                .as_ref()
                .map(|array| array.get_item(i))
                .transpose(),
            _ => unreachable!(),
        }
    }

    pub fn get_vec3(&self, name: Name, i: usize) -> PyResult<[f64; 3]> {
        let j = self.map[name as usize];
        let vec3 = match &self.data[j] {
            FieldArray::Vec3(array) => {
                let i = if array.ndim() == 1 { 0 } else { i };
                [
                    array.get_item(3 * i + 0)?,
                    array.get_item(3 * i + 1)?,
                    array.get_item(3 * i + 2)?,
                ]
            },
            _ => unreachable!(),
        };
        Ok(vec3)
    }

    pub fn get_vec3_opt(&self, name: Name, i: usize) -> PyResult<Option<[f64; 3]>> {
        let j = self.map[name as usize];
        match &self.data[j] {
            FieldArray::MaybeVec3(array) => array
                .as_ref()
                .map(|array| {
                    let i = if array.ndim() == 1 { 0 } else { i };
                    Ok([
                        array.get_item(3 * i + 0)?,
                        array.get_item(3 * i + 1)?,
                        array.get_item(3 * i + 2)?,
                    ])
                })
                .transpose(),
            _ => unreachable!(),
        }
    }

    pub fn shape(&self) -> Vec<usize> {
        self.size.shape()
    }

    pub fn size(&self) -> usize {
        self.size.size()
    }
}

impl Field {
    fn extract<'py>(
        &self,
        py: Python<'py>,
        ob: &Bound<'py, PyAny>,
    ) -> PyResult<FieldArray<'py>> {
        let key = self.name.to_str();
        let array = match self.kind {
            Kind::Float => FieldArray::Float(require::<f64>(py, ob, key)?),
            Kind::Int => FieldArray::Int(require::<i32>(py, ob, key)?),
            Kind::Vec3 => FieldArray::Vec3(require::<f64>(py, ob, key)?),
            Kind::MaybeFloat => FieldArray::MaybeFloat(extract::<f64>(py, ob, key)?),
            Kind::MaybeInt => FieldArray::MaybeInt(extract::<i32>(py, ob, key)?),
            Kind::MaybeVec3 => FieldArray::MaybeVec3(extract::<f64>(py, ob, key)?),
        };
        Ok(array)
    }

    fn extract_none(&self) -> FieldArray<'static> {
        match self.kind {
            Kind::MaybeFloat => FieldArray::MaybeFloat(None),
            Kind::MaybeInt => FieldArray::MaybeInt(None),
            Kind::MaybeVec3 => FieldArray::MaybeVec3(None),
            _ => unreachable!(),
        }
    }

    pub fn float(name: Name) -> Self {
        Self { name, kind: Kind::Float }
    }

    pub fn int(name: Name) -> Self {
        Self { name, kind: Kind::Int }
    }

    pub fn vec3(name: Name) -> Self {
        Self { name, kind: Kind::Vec3 }
    }

    pub fn maybe_float(name: Name) -> Self {
        Self { name, kind: Kind::MaybeFloat }
    }

    pub fn maybe_int(name: Name) -> Self {
        Self { name, kind: Kind::MaybeInt }
    }

    pub fn maybe_vec3(name: Name) -> Self {
        Self { name, kind: Kind::MaybeVec3 }
    }
}

impl Kind {
    fn is_opt(&self) -> bool {
        match self {
            Self::Int => false,
            Self::Float => false,
            Self::Vec3 => false,
            Self::MaybeInt => true,
            Self::MaybeFloat => true,
            Self::MaybeVec3 => true,
        }
    }
}

// ===============================================================================================
//
// Managed array size.
//
// ===============================================================================================

#[derive(Clone)]
pub enum Size {
    Scalar,
    Array { size: usize, shape: Vec<usize> },
}

impl Size {
    pub fn shape(&self) -> Vec<usize> {
        match self {
            Size::Scalar => Vec::new(),
            Size::Array { shape, .. } => shape.clone(),
        }
    }

    pub fn size(&self) -> usize {
        match self {
            Size::Scalar => 1,
            Size::Array { size, .. } => *size,
        }
    }

    fn new(array: &FieldArray) -> Result<Self, String> {
        let size = match array {
            FieldArray::Float(array) => Self::from_typed::<f64>(Some(array)),
            FieldArray::Int(array) => Self::from_typed::<i32>(Some(array)),
            FieldArray::Vec3(array) => Self::from_typed_vec::<f64>(Some(array), 3)?,
            FieldArray::MaybeFloat(array) => Self::from_typed::<f64>(array.as_ref()),
            FieldArray::MaybeInt(array) => Self::from_typed::<i32>(array.as_ref()),
            FieldArray::MaybeVec3(array) => Self::from_typed_vec::<f64>(array.as_ref(), 3)?,
        };
        Ok(size)
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

    fn from_typed_vec<'py, T: Clone + Dtype>(
        array: Option<&AnyArray<'py, T>>,
        n: usize,
    ) -> Result<Self, String> {
        match array {
            Some(array) => {
                let mut shape = array.shape();
                let m = shape.pop().unwrap_or(0);
                if m != n {
                    let why = format!(
                        "expected a shape [.., {}] array, found [.., {}]",
                        n,
                        m,
                    );
                    return Err(why)
                } else if shape.is_empty() {
                    Ok(Self::Scalar)
                } else {
                    let size = array.size() / n;
                    Ok(Self::Array { size, shape })
                }
            },
            None => Ok(Self::Scalar),
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
