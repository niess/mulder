use crate::utils::coordinates::{GeographicCoordinates, HorizontalCoordinates};
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::TypeError;
use crate::utils::numpy::{AnyArray, ArrayMethods};
use pyo3::prelude::*;
use pyo3::types::PyDict;


// ===============================================================================================
//
// Geographic position(s) wrapper.
//
// ===============================================================================================

pub struct Position<'py> {
    pub latitude: AnyArray<'py, f64>,
    pub longitude: AnyArray<'py, f64> ,
    pub altitude: AnyArray<'py, f64>,
    size: Size,
}

impl<'py> Position<'py> {
    pub fn new(
        latitude: Option<AnyArray<'py, f64>>,
        longitude: Option<AnyArray<'py, f64>>,
        altitude: Option<AnyArray<'py, f64>>,
    ) -> PyResult<Self> {
        let all = latitude.is_some() && longitude.is_some() && altitude.is_some();
        let any = latitude.is_some() || longitude.is_some() || altitude.is_some();
        if all {
            let latitude = latitude.unwrap();
            let longitude = longitude.unwrap();
            let altitude = altitude.unwrap();
            let latitude_size = Size::new(&latitude);
            let longitude_size = Size::new(&longitude);
            let altitude_size = Size::new(&altitude);
            let size = latitude_size
                .common(&longitude_size)
                .and_then(|size| size.common(&altitude_size))
                .ok_or_else(|| Error::new(TypeError)
                    .what("latitude, longitude and altitude")
                    .why("inconsistent arrays sizes")
                    .to_err()
                )?
                .clone();
            let position = Self { latitude, longitude, altitude, size };
            Ok(position)
        } else if any {
            let mut missing: Vec<&str> = Vec::new();
            if latitude.is_none() { missing.push("latitude") };
            if longitude.is_none() { missing.push("longitude") };
            if altitude.is_none() { missing.push("altitude") };
            let why = if missing.len() == 2 {
                format!("missing '{}' and '{}'", missing[0], missing[1])
            } else {
                format!("missing '{}'", missing[0])
            };
            let err = Error::new(TypeError).what("position").why(&why);
            Err(err.to_err())
        } else {
            let err = Error::new(TypeError)
                .what("position")
                .why("missing latitude, longitude and altitude");
            Err(err.to_err())
        }
    }

    pub fn common(&self, direction: &Direction) -> PyResult<(usize, Vec<usize>)> {
        let size = self.size.common(&direction.size)
            .ok_or_else(|| Error::new(TypeError)
                .what("position and direction")
                .why("inconsistent arrays size")
            )?;
        let result = match size {
            Size::Scalar => (1, Vec::new()),
            Size::Array { size, shape } => (*size, shape.clone()),
        };
        Ok(result)
    }

    pub fn get(&self, i: usize) -> PyResult<GeographicCoordinates> {
        let geographic = GeographicCoordinates {
            latitude: self.latitude.get_item(i)?,
            longitude: self.longitude.get_item(i)?,
            altitude: self.altitude.get_item(i)?,
        };
        Ok(geographic)
    }

    pub fn shape(&self) -> Vec<usize> {
        match &self.size {
            Size::Scalar => Vec::new(),
            Size::Array { shape, .. } => shape.clone(),
        }
    }

    pub fn shape3(&self) -> Vec<usize> {
        let mut shape = self.shape();
        shape.push(3);
        shape
    }

    pub fn size(&self) -> usize {
        match &self.size {
            Size::Scalar => 1,
            Size::Array { size, .. } => *size,
        }
    }
}

impl<'py> FromPyObject<'py> for Position<'py> {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        let py = ob.py();
        let latitude = extract(py, ob, "latitude")?;
        let longitude = extract(py, ob, "longitude")?;
        let altitude = extract(py, ob, "altitude")?;
        Self::new(latitude, longitude, altitude)
    }
}


// ===============================================================================================
//
// Generic geographic direction.
//
// ===============================================================================================

pub struct Direction<'py> {
    pub azimuth: AnyArray<'py, f64>,
    pub elevation: AnyArray<'py, f64> ,
    size: Size,
}

impl<'py> Direction<'py> {
    pub fn new(
        azimuth: Option<AnyArray<'py, f64>>,
        elevation: Option<AnyArray<'py, f64>>,
    ) -> PyResult<Self> {
        let all = azimuth.is_some() && elevation.is_some();
        let any = azimuth.is_some() || elevation.is_some();
        if all {
            let azimuth = azimuth.unwrap();
            let elevation = elevation.unwrap();
            let azimuth_size = Size::new(&azimuth);
            let elevation_size = Size::new(&elevation);
            let size = azimuth_size.common(&elevation_size)
                .ok_or_else(|| Error::new(TypeError)
                    .what("azimuth and elevation")
                    .why("inconsistent arrays sizes")
                    .to_err()
                )?
                .clone();
            let direction = Direction { azimuth, elevation, size };
            Ok(direction)
        } else if any {
            let why = if azimuth.is_some() {
                "missing 'elevation'"
            } else {
                "missing 'azimuth'"
            };
            let err = Error::new(TypeError).what("direction").why(why);
            Err(err.to_err())
        } else {
            let err = Error::new(TypeError)
                .what("direction")
                .why("missing azimuth and elevation");
            Err(err.to_err())
        }
    }

    pub fn get(&self, i: usize) -> PyResult<HorizontalCoordinates> {
        let horizontal = HorizontalCoordinates {
            azimuth: self.azimuth.get_item(i)?,
            elevation: self.elevation.get_item(i)?,
        };
        Ok(horizontal)
    }

    pub fn shape(&self) -> Vec<usize> {
        match &self.size {
            Size::Scalar => Vec::new(),
            Size::Array { shape, .. } => shape.clone(),
        }
    }

    pub fn shape3(&self) -> Vec<usize> {
        let mut shape = self.shape();
        shape.push(3);
        shape
    }

    pub fn size(&self) -> usize {
        match &self.size {
            Size::Scalar => 1,
            Size::Array { size, .. } => *size,
        }
    }
}

impl<'py> FromPyObject<'py> for Direction<'py> {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        let py = ob.py();
        let azimuth = extract(py, ob, "azimuth")?;
        let elevation = extract(py, ob, "elevation")?;
        Self::new(azimuth, elevation)
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


// ===============================================================================================
//
// Fields selector.
//
// ===============================================================================================

pub fn select_coordinates<'a, 'py>(
    array: Option<&'a Bound<'py, PyAny>>,
    kwargs: Option<&'a Bound<'py, PyDict>>,
) -> PyResult<Option<&'a Bound<'py, PyAny>>> {
    const FIELDS: &'static [&'static str] = &[
        "latitude", "longitude", "altitude", "azimuth", "elevation",
    ];
    select(array, kwargs, FIELDS)
}

pub fn select_position<'a, 'py>(
    array: Option<&'a Bound<'py, PyAny>>,
    kwargs: Option<&'a Bound<'py, PyDict>>,
) -> PyResult<Option<&'a Bound<'py, PyAny>>> {
    const FIELDS: &'static [&'static str] = &["latitude", "longitude", "altitude"];
    select(array, kwargs, FIELDS)
}

fn select<'a, 'py>(
    array: Option<&'a Bound<'py, PyAny>>,
    kwargs: Option<&'a Bound<'py, PyDict>>,
    fields: &[&str],
) -> PyResult<Option<&'a Bound<'py, PyAny>>> {
    match array {
        Some(_) => match kwargs {
            Some(_) => {
                let err = Error::new(TypeError)
                    .what("arguments")
                    .why("cannot mix positional and keyword only arguments");
                return Err(err.to_err())
            },
            None => Ok(array),
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
                Ok(Some(kwargs.as_any()))
            },
            None => Ok(None),
        },
    }
}
