use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use crate::simulation::coordinates::{GeographicCoordinates, HorizontalCoordinates};
use crate::utils::convert::LightModel;
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::ValueError;
use pyo3::prelude::*;
use super::colours::{LinearRgb, StandardRgb};
use super::vec3::Vec3;


const AMBIENT_SCALING: f64 = 1.0 / (2.0 * std::f64::consts::PI);

#[derive(Clone, Debug, FromPyObject)]
pub enum Light {
    Ambient(AmbientLight),
    Directional(DirectionalLight),
    Sun(SunLight),
}

#[derive(Clone, Debug, FromPyObject)]
pub enum LightArg {
    Model(LightModel),
    Light(Light),
}

#[derive(Clone, Debug, FromPyObject)]
pub enum Lights {
    Single(LightArg),
    Sequence(Vec<LightArg>),
}

#[pyclass(module="mulder.picture")]
#[derive(Clone, Debug)]
pub struct AmbientLight {
    /// The light colour.
    #[pyo3(get, set)]
    pub colour: StandardRgb,

    /// The light intensity.
    #[pyo3(get, set)]
    pub intensity: f64,
}

#[pyclass(module="mulder.picture")]
#[derive(Clone, Debug)]
pub struct DirectionalLight {
    /// The light colour.
    #[pyo3(get, set)]
    pub colour: StandardRgb,

    /// The light intensity.
    #[pyo3(get, set)]
    pub intensity: f64,

    /// The light azimuth direction, in deg.
    #[pyo3(get, set)]
    pub azimuth: f64,

    /// The light elevation direction, in deg.
    #[pyo3(get, set)]
    pub elevation: f64,
}

#[pyclass(module="mulder.picture")]
#[derive(Clone, Debug)]
pub struct SunLight {
    /// The sun light colour.
    #[pyo3(get, set)]
    pub colour: StandardRgb,

    /// The sun light intensity.
    #[pyo3(get, set)]
    pub intensity: f64,

    /// The local date and solar time.
    #[pyo3(get, set)]
    datetime: NaiveDateTime,
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

#[derive(Clone, Debug)]
pub struct ResolvedLight {
    pub azimuth: f64,
    pub elevation: f64,
    pub direction: [f64; 3],
    pub illuminance: Vec3,
}

#[pymethods]
impl AmbientLight {
    #[new]
    #[pyo3(signature=(intensity=None, colour=None))]
    fn new(intensity: Option<f64>, colour: Option<StandardRgb>) -> Self {
        let intensity = intensity.unwrap_or(Self::DEFAULT_INTENSITY);
        let colour = colour.unwrap_or(Self::DEFAULT_COLOUR);
        Self { colour, intensity }
    }
}

impl AmbientLight {
    const DEFAULT_COLOUR: StandardRgb = StandardRgb::WHITE;
    const DEFAULT_INTENSITY: f64 = 1.0;

    pub fn luminance(&self) -> Vec3 {
        let colour: LinearRgb = self.colour.into();
        Vec3(colour.0) * (self.intensity * AMBIENT_SCALING)
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
        colour: Option<StandardRgb>,
        intensity: Option<f64>,
    ) -> Self {
        let colour = colour.unwrap_or_else(|| Self::DEFAULT_COLOUR);
        let intensity = intensity.unwrap_or(Self::DEFAULT_INTENSITY);
        Self { azimuth, elevation, colour, intensity }
    }
}

impl DirectionalLight {
    const DEFAULT_COLOUR: StandardRgb = StandardRgb::WHITE;
    const DEFAULT_INTENSITY: f64 = 1.0;

    #[inline]
    fn direction(&self) -> HorizontalCoordinates {
        HorizontalCoordinates { azimuth: self.azimuth, elevation: self.elevation }
    }

    pub(super) fn resolve(&self, position: &GeographicCoordinates) -> ResolvedLight {
        let direction = self.direction().to_ecef(&position);
        let colour: LinearRgb = self.colour.into();
        ResolvedLight {
            azimuth: self.azimuth, elevation: self.elevation, direction,
            illuminance: Vec3(colour.0) * self.intensity,
        }
    }
}

#[pymethods]
impl SunLight {
    #[new]
    #[pyo3(signature=(/, *, colour=None, datetime=None, intensity=None))]
    fn new(
        colour: Option<StandardRgb>,
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
        let intensity = intensity.unwrap_or(Self::DEFAULT_INTENSITY);
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
    const DEFAULT_COLOUR: StandardRgb = StandardRgb::WHITE;
    const DEFAULT_INTENSITY: f64 = 1.0;

    pub fn to_directional(&self, latitude: f64) -> PyResult<DirectionalLight> {
        let datetime = self.datetime.and_utc();
        let position = spa::solar_position::<spa::StdFloatOps>(
            datetime, latitude, 0.0,
        ).unwrap(); 
        let elevation = 90.0 - position.zenith_angle;
        let azimuth = position.azimuth;
        Ok(DirectionalLight { azimuth, elevation, colour: self.colour, intensity: self.intensity })
    }
}

impl Default for SunLight {
    fn default() -> Self {
        Self::new(None, None, None).unwrap()
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

impl LightArg {
    fn resolve(self, camera_direction: HorizontalCoordinates) -> Light {
        match self {
            Self::Light(light) => light,
            Self::Model(model) => match model {
                LightModel::Ambient => Light::Ambient(AmbientLight::default()),
                LightModel::Directional => Light::Directional(
                    DirectionalLight::new(
                        camera_direction.azimuth + 180.0,
                        -camera_direction.elevation,
                        None,
                        None,
                    )
                ),
                LightModel::Sun => Light::Sun(SunLight::default()),
            },
        }
    }
}

impl Lights {
    pub const DIRECTIONAL: Self = Self::Single(LightArg::Model(LightModel::Directional));
    pub const SUN: Self = Self::Single(LightArg::Model(LightModel::Sun));

    pub fn into_vec(self, camera_direction: HorizontalCoordinates) -> Vec<Light> {
        match self {
            Self::Single(light) => vec![light.resolve(camera_direction)],
            Self::Sequence(lights) => lights
                .into_iter()
                .map(|light| light.resolve(camera_direction))
                .collect(),
        }
    }
}
