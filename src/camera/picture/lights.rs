use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use crate::utils::coordinates::{GeographicCoordinates, HorizontalCoordinates};
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::ValueError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use super::materials::LinearRgb;
use super::vec3::Vec3;


#[inline]
pub fn default_lights(py: Python) -> PyResult<PyObject> {
    let lights = PyList::new(py, [
        AmbientLight::default().into_pyobject(py)?.into_any(),
        SunLight::new(None, None, None)?.into_pyobject(py)?.into_any(),
    ])?.into_any().unbind();
    Ok(lights)
}

#[derive(FromPyObject)]
pub enum Light {
    Ambient(AmbientLight),
    Directional(DirectionalLight),
    Sun(SunLight),
}

#[pyclass(module="mulder")]
#[derive(Clone, Debug)]
pub struct AmbientLight {
    #[pyo3(get, set)]
    pub colour: (u8, u8, u8),

    #[pyo3(get, set)]
    pub intensity: f64,
}

#[pyclass(module="mulder")]
#[derive(Clone, Debug)]
pub struct DirectionalLight {
    #[pyo3(get, set)]
    pub colour: (u8, u8, u8),

    #[pyo3(get, set)]
    pub azimuth: f64,

    #[pyo3(get, set)]
    pub elevation: f64,

    #[pyo3(get, set)]
    pub intensity: f64,
}

#[pyclass(module="mulder")]
#[derive(Clone, Debug)]
pub struct SunLight {
    #[pyo3(get, set)]
    pub colour: (u8, u8, u8),

    /// The local date and solar time.
    #[pyo3(get, set)]
    datetime: NaiveDateTime,

    /// The sun light intensity.
    #[pyo3(get, set)]
    intensity: f64,
}

#[derive(FromPyObject)]
enum DateArg {
    NaiveDate(NaiveDate),
    String(String),
}

#[derive(FromPyObject)]
enum DateTimeArg {
    NaiveDateTime(NaiveDateTime),
    String(String),
}

#[derive(FromPyObject)]
enum TimeArg {
    NaiveTime(NaiveTime),
    Number(f64),
    String(String),
}

pub struct ResolvedAmbient {
    pub colour: LinearRgb,
    pub intensity: f64,
}

pub struct ResolvedLight {
    pub azimuth: f64,
    pub elevation: f64,
    pub direction: [f64; 3],
    pub colour: LinearRgb,
    pub intensity: f64,
}

#[pymethods]
impl AmbientLight {
    #[new]
    #[pyo3(signature=(colour=None, intensity=None))]
    fn new(colour: Option<(u8, u8, u8)>, intensity: Option<f64>) -> Self {
        let colour = colour.unwrap_or_else(|| Self::DEFAULT_COLOUR);
        let intensity = intensity.unwrap_or_else(|| Self::DEFAULT_INTENSITY);
        Self { colour, intensity }
    }
}

impl AmbientLight {
    const DEFAULT_COLOUR: (u8, u8, u8) = (255, 255, 255);
    const DEFAULT_INTENSITY: f64 = 0.3;

    pub fn luminance(&self) -> Vec3 {
        let colour: LinearRgb = self.colour.into();
        Vec3(colour.0) * self.intensity
    }
}

impl Default for AmbientLight {
    fn default() -> Self {
        Self { colour: Self::DEFAULT_COLOUR, intensity: Self::DEFAULT_INTENSITY }
    }
}

#[pymethods]
impl DirectionalLight {
    #[new]
    #[pyo3(signature=(azimuth, elevation, *, colour=None, intensity=None))]
    fn new(
        azimuth: f64,
        elevation: f64,
        colour: Option<(u8, u8, u8)>,
        intensity: Option<f64>,
    ) -> Self {
        let colour = colour.unwrap_or_else(|| Self::DEFAULT_COLOUR);
        let intensity = intensity.unwrap_or_else(|| 1.0 - AmbientLight::DEFAULT_INTENSITY);
        Self { azimuth, elevation, colour, intensity }
    }
}

impl DirectionalLight {
    const DEFAULT_COLOUR: (u8, u8, u8) = (255, 255, 255);

    #[inline]
    fn direction(&self) -> HorizontalCoordinates {
        HorizontalCoordinates { azimuth: self.azimuth, elevation: self.elevation }
    }

    pub(super) fn resolve(&self, position: &GeographicCoordinates) -> ResolvedLight {
        let direction = self.direction().to_ecef(&position);
        ResolvedLight {
            azimuth: self.azimuth, elevation: self.elevation, colour: self.colour.into(),
            direction, intensity: self.intensity,
        }
    }
}

#[pymethods]
impl SunLight {
    #[new]
    #[pyo3(signature=(/, *, colour=None, datetime=None, intensity=None))]
    fn new(
        colour: Option<(u8, u8, u8)>,
        datetime: Option<DateTimeArg>,
        intensity: Option<f64>,
    ) -> PyResult<Self> {
        let colour = colour.unwrap_or_else(|| Self::DEFAULT_COLOUR);
        let datetime = datetime
            .unwrap_or_else(|| {
                let time = NaiveTime::from_hms_opt(12, 0, 0)
                    .unwrap();
                let datetime = NaiveDate::from_ymd_opt(2025, 3, 20)
                    .unwrap()
                    .and_time(time);
                DateTimeArg::NaiveDateTime(datetime)
            })
            .into_datetime()?;
        let intensity = intensity.unwrap_or_else(|| 1.0 - AmbientLight::DEFAULT_INTENSITY);
        Ok(Self { colour, datetime, intensity })
    }

    /// The local date.
    #[getter]
    fn get_date(&self) -> NaiveDate {
        self.datetime.date()
    }

    #[setter]
    fn set_date(&mut self, value: DateArg) -> PyResult<()> {
        let date = value.into_date()?;
        let time = self.datetime.time();
        self.datetime = NaiveDateTime::new(date, time);
        Ok(())
    }


    #[setter]
    fn set_datetime(&mut self, value: DateTimeArg) -> PyResult<()> {
        self.datetime = value.into_datetime()?;
        Ok(())
    }

    /// The solar time.
    #[getter]
    fn get_time(&self) -> NaiveTime {
        self.datetime.time()
    }

    #[setter]
    fn set_time(&mut self, value: TimeArg) -> PyResult<()> {
        let date = self.datetime.date();
        let time = value.into_time()?;
        self.datetime = NaiveDateTime::new(date, time);
        Ok(())
    }
}

impl SunLight {
    const DEFAULT_COLOUR: (u8, u8, u8) = (255, 255, 255);

    pub fn to_directional(&self, latitude: f64) -> PyResult<DirectionalLight> {
        let datetime = self.datetime.and_utc();
        let position = spa::solar_position::<spa::StdFloatOps>(
            datetime, latitude, 0.0,
        ).unwrap(); 
        let elevation = 90.0 - position.zenith_angle;
        let azimuth = position.azimuth;
        let intensity = self.intensity;
        Ok(DirectionalLight { azimuth, elevation, intensity, colour: self.colour })
    }
}

impl DateArg {
    fn into_date(self) -> PyResult<NaiveDate> {
        match self {
            Self::NaiveDate(date) => Ok(date),
            Self::String(date) => {
                NaiveDate::parse_from_str(&date, "%Y-%m-%d")
                    .map_err(|err| {
                        let why = format!("{}", err);
                        Error::new(ValueError).what("date").why(&why).to_err()
                    })
            },
        }
    }
}

impl DateTimeArg {
    fn into_datetime(self) -> PyResult<NaiveDateTime> {
        match self {
            Self::NaiveDateTime(datetime) => Ok(datetime),
            Self::String(datetime) => {
                NaiveDateTime::parse_from_str(&datetime, "%Y-%m-%d %H:%M:%S")
                    .map_err(|err| {
                        let why = format!("{}", err);
                        Error::new(ValueError).what("datetime").why(&why).to_err()
                    })
            },
        }
    }
}

impl TimeArg {
    fn into_time(self) -> PyResult<NaiveTime> {
        match self {
            Self::NaiveTime(time) => Ok(time),
            Self::Number(time) => {
                let seconds = (time * 3600.0) as u32;
                NaiveTime::from_num_seconds_from_midnight_opt(seconds, 0)
                    .ok_or_else(|| {
                        let why = format!("expected a value in [0, 24), found {}", time);
                        Error::new(ValueError).what("time").why(&why).to_err()
                    })
            },
            Self::String(time) => {
                NaiveTime::parse_from_str(&time, "%H:%M:%S")
                    .map_err(|err| {
                        let why = format!("{}", err);
                        Error::new(ValueError).what("time").why(&why).to_err()
                    })
            },
        }
    }
}
