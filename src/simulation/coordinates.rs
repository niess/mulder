use crate::bindings::turtle;
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::TypeError;
use crate::utils::extract::{Extractor, Field, Name};
use crate::utils::numpy::{Dtype, impl_dtype};
use pyo3::prelude::*;
use pyo3::types::PyDict;


#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GeographicCoordinates {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HorizontalCoordinates {
    pub azimuth: f64,
    pub elevation: f64,
}

#[derive(Clone, Debug, PartialEq)]
#[pyclass(module="mulder")]
pub struct LocalFrame {
    pub origin: GeographicCoordinates,

    /// The frame declination angle (w.r.t. the geographic north), in deg.
    #[pyo3(get)]
    pub declination: f64,

    /// The frame inclination angle (w.r.t. the local vertical), in deg.
    #[pyo3(get)]
    pub inclination: f64,

    pub rotation: [[f64; 3]; 3],
    pub translation: [f64; 3],
}

pub enum PositionExtractor<'py> {
    Geographic {
        extractor: Extractor<'py, 3>,
        default_latitude: f64,
        default_longitude: f64,
    },
    Local {
        extractor: Extractor<'py, 1>,
        frame: LocalFrame,
    },
}

pub enum ExtractedPosition<'a> {
    Geographic { position: GeographicCoordinates },
    Local { position: [f64; 3], frame: &'a LocalFrame },
}

impl GeographicCoordinates {
    pub fn from_ecef(position: &[f64; 3]) -> Self {
        let mut latitude = 0.0;
        let mut longitude = 0.0;
        let mut altitude = 0.0;
        unsafe {
            turtle::ecef_to_geodetic(
                position.as_ptr(),
                &mut latitude,
                &mut longitude,
                &mut altitude
            );
        }
        Self { latitude, longitude, altitude }
    }

    pub fn to_ecef(&self) -> [f64; 3] {
        let mut position = [0_f64; 3];
        unsafe {
            turtle::ecef_from_geodetic(
                self.latitude,
                self.longitude,
                self.altitude,
                position.as_mut_ptr(),
            );
        }
        position
    }
}

impl HorizontalCoordinates {
    pub fn from_ecef(
        direction: &[f64; 3],
        origin: &GeographicCoordinates
    ) -> Self {
        let mut azimuth: f64 = 0.0;
        let mut elevation: f64 = 0.0;
        unsafe {
            turtle::ecef_to_horizontal(
                origin.latitude,
                origin.longitude,
                direction.as_ptr(),
                &mut azimuth,
                &mut elevation,
            );
        }
        Self { azimuth, elevation }
    }

    pub fn to_ecef(
        &self,
        origin: &GeographicCoordinates
    ) -> [f64; 3] {
        let mut direction = [0.0; 3];
        unsafe {
            turtle::ecef_from_horizontal(
                origin.latitude,
                origin.longitude,
                self.azimuth,
                self.elevation,
                (&mut direction) as *mut f64,
            );
        }
        direction
    }
}

impl_dtype!(
    HorizontalCoordinates,
    [
        ("azimuth", "f8"),
        ("elevation", "f8")
    ]
);

#[pymethods]
impl LocalFrame {
    #[new]
    #[pyo3(signature=(position=None, /, *, declination=None, inclination=None, **kwargs))]
    fn py_new(
        py: Python,
        position: Option<&Bound<PyAny>>,
        declination: Option<f64>,
        inclination: Option<f64>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<Self> {
        let position = PositionExtractor::new(py, position, kwargs, None, Some(1))?
            .with_default_latitude(Self::DEFAULT_LATITUDE)
            .with_default_longitude(Self::DEFAULT_LONGITUDE);
        let position = position.extract(0)?;
        let origin = match position {
            ExtractedPosition::Geographic { position } => position,
            ExtractedPosition::Local { position, frame } => {
                frame.to_geographic_position(&position)
            },
        };
        let declination = declination.unwrap_or(0.0);
        let inclination = inclination.unwrap_or(0.0);
        let frame = Self::new(origin, declination, inclination);
        Ok(frame)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.eq(other)
    }

    fn __getstate__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        // This ensures that no field is omitted.
        let Self { origin, declination, inclination, rotation, translation } = self;
        let GeographicCoordinates { latitude, longitude, altitude } = origin;

        let state = PyDict::new(py);
        state.set_item("latitude", latitude)?;
        state.set_item("longitude", longitude)?;
        state.set_item("altitude", altitude)?;
        state.set_item("declination", declination)?;
        state.set_item("inclination", inclination)?;
        state.set_item("rotation", rotation)?;
        state.set_item("translation", translation)?;
        Ok(state)
    }

    fn __setstate__(&mut self, state: Bound<PyDict>) -> PyResult<()> {
        let origin = GeographicCoordinates {
            latitude: state.get_item("latitude")?.unwrap().extract()?,
            longitude: state.get_item("longitude")?.unwrap().extract()?,
            altitude: state.get_item("altitude")?.unwrap().extract()?,
        };
        *self = Self { // This ensures that no field is omitted.
            origin,
            declination: state.get_item("declination")?.unwrap().extract()?,
            inclination: state.get_item("inclination")?.unwrap().extract()?,
            rotation: state.get_item("rotation")?.unwrap().extract()?,
            translation: state.get_item("translation")?.unwrap().extract()?,
        };
        Ok(())
    }

    fn __repr__(&self) -> String {
        let mut args = vec![
            format!("{}, {}", self.origin.latitude, self.origin.longitude)
        ];
        if self.origin.altitude != 0.0 {
            args.push(format!("{}", self.origin.altitude));
        }
        if self.declination != 0.0 {
            args.push(format!("declination={}", self.declination));
        }
        if self.inclination != 0.0 {
            args.push(format!("inclination={}", self.declination));
        }
        let args = args.join(", ");
        format!("LocalFrame({})", args)
    }

    // XXX add point/vector transforms between frames?

    /// The latitude coordinate of the frame origin, in deg.
    #[getter]
    fn get_latitude(&self) -> f64 {
        self.origin.latitude
    }

    /// The longitude coordinate of the frame origin, in deg.
    #[getter]
    fn get_longitude(&self) -> f64 {
        self.origin.longitude
    }

    /// The altitude coordinate of the frame origin, in m.
    #[getter]
    fn get_altitude(&self) -> f64 {
        self.origin.altitude
    }
}

impl LocalFrame {
    pub const DEFAULT_LATITUDE: f64 = 45.0;
    pub const DEFAULT_LONGITUDE: f64 = 0.0;

    pub fn from_ecef_direction(&self, ecef: &[f64; 3]) -> [f64; 3] {
        let mut enu = [0.0; 3];
        for i in 0..3 {
            for j in 0..3 {
                enu[i] += self.rotation[i][j] * ecef[j];
            }
        }
        enu
    }

    pub fn from_ecef_position(&self, mut ecef: [f64; 3]) -> [f64; 3] {
        for i in 0..3 {
            ecef[i] -= self.translation[i];
        }
        self.from_ecef_direction(&ecef)
    }

    pub fn from_geographic(
        &self,
        position: GeographicCoordinates,
        direction: HorizontalCoordinates,
    ) -> ([f64; 3], [f64; 3]) {
        let direction = self.from_ecef_direction(&direction.to_ecef(&position));
        let position = self.from_ecef_position(position.to_ecef());
        (position, direction)
    }

    pub fn from_local(
        &self,
        position: [f64; 3],
        direction: [f64; 3],
        frame: &LocalFrame,
    ) -> ([f64; 3], [f64; 3]) {
        if self.ne(frame) {
            let position = self.from_ecef_position(frame.to_ecef_position(&position));
            let direction = self.from_ecef_direction(&frame.to_ecef_direction(&direction));
            (position, direction)
        } else {
            (position, direction)
        }
    }

    pub fn to_ecef_direction(&self, enu: &[f64; 3]) -> [f64; 3] {
        let mut ecef = [0.0; 3];
        for i in 0..3 {
            for j in 0..3 {
                ecef[i] += self.rotation[j][i] * enu[j];
            }
        }
        ecef
    }

    pub fn to_ecef_position(&self, enu: &[f64; 3]) -> [f64; 3] {
        let mut ecef = self.to_ecef_direction(enu);
        for i in 0..3 {
            ecef[i] += self.translation[i];
        }
        ecef
    }

    pub fn to_geographic(
        &self,
        position: &[f64; 3],
        direction: &[f64; 3],
    ) -> (GeographicCoordinates, HorizontalCoordinates) {
        let position = GeographicCoordinates::from_ecef(&self.to_ecef_position(position));
        let direction = HorizontalCoordinates::from_ecef(
            &self.to_ecef_direction(direction),
            &position
        );
        (position, direction)
    }

    #[inline]
    pub fn to_geographic_position(&self, position: &[f64; 3]) -> GeographicCoordinates {
        GeographicCoordinates::from_ecef(&self.to_ecef_position(position))
    }

    pub fn to_horizontal(&self, enu: &[f64; 3]) -> HorizontalCoordinates {
        let ecef = self.to_ecef_direction(enu);
        HorizontalCoordinates::from_ecef(&ecef, &self.origin)
    }

    pub fn new(origin: GeographicCoordinates, declination: f64, inclination: f64) -> Self {
        // Compute transform from ECEF to ENU.
        let mut rotation = [[0.0; 3]; 3];
        unsafe {
            turtle::ecef_from_horizontal(
                 origin.latitude,
                 origin.longitude,
                 90.0 + declination,
                 0.0,
                 rotation[0].as_mut_ptr(),
            );

            turtle::ecef_from_horizontal(
                origin.latitude,
                origin.longitude,
                declination,
                -inclination,
                rotation[1].as_mut_ptr(),
            );

            turtle::ecef_from_horizontal(
                origin.latitude,
                origin.longitude,
                declination,
                90.0 - inclination,
                rotation[2].as_mut_ptr(),
            );
        }

        let translation = origin.to_ecef();
        Self { rotation, translation, origin, declination, inclination }
    }
}

impl Default for LocalFrame {
    fn default() -> Self {
        let origin = GeographicCoordinates {
            latitude: Self::DEFAULT_LATITUDE,
            longitude: Self::DEFAULT_LONGITUDE,
            altitude: 0.0
        };
        Self::new(origin, 0.0, 0.0)
    }
}

impl<'py> PositionExtractor<'py> {
    pub fn new(
        py: Python<'py>,
        states: Option<&Bound<'py, PyAny>>,
        kwargs: Option<&Bound<'py, PyDict>>,
        frame: Option<&LocalFrame>,
        expected_size: Option<usize>,
    ) -> PyResult<Self> {
        const DEFAULT_LATITUDE: f64 = 0.0;
        const DEFAULT_LONGITUDE: f64 = 0.0;

        let extractor = match states {
            Some(states) => match states.getattr_opt("frame")? {
                Some(frame) => {
                    let frame: LocalFrame = frame.extract()
                        .map_err(|err| {
                            let why = err.value(py).to_string();
                            Error::new(TypeError).what("states' frame").why(&why).to_err()
                        })?;
                    let extractor = Self::local_extractor(Some(states), kwargs)?;
                    Self::Local { extractor, frame }
                },
                None => {
                    let extractor = Self::geographic_extractor(Some(states), kwargs)?;
                    Self::Geographic {
                        extractor,
                        default_latitude: DEFAULT_LATITUDE,
                        default_longitude: DEFAULT_LONGITUDE,
                    }
                },
            },
            None => match frame {
                Some(frame) => {
                    let extractor = Self::local_extractor(None, kwargs)?;
                    Self::Local { extractor, frame: frame.clone() }
                },
                None => {
                    let extractor = Self::geographic_extractor(None, kwargs)?;
                    Self::Geographic {
                        extractor,
                        default_latitude: DEFAULT_LATITUDE,
                        default_longitude: DEFAULT_LONGITUDE,
                    }
                },
            },
        };
        if let Some(expected_size) = expected_size {
            let found_size = extractor.size();
            if found_size != expected_size {
                let why = format!(
                    "expected size={}, found size={}",
                    expected_size,
                    found_size,
                );
                let err = Error::new(TypeError).what("position").why(&why).to_err();
                return Err(err)
            }
        }
        Ok(extractor)
    }

    pub fn extract<'a>(&'a self, index: usize) -> PyResult<ExtractedPosition<'a>> {
        let extracted = match self {
            Self::Geographic { extractor, default_latitude, default_longitude } => {
                let position = GeographicCoordinates {
                    latitude: extractor.get_f64_opt(Name::Latitude, index)?
                        .unwrap_or(*default_latitude),
                    longitude: extractor.get_f64_opt(Name::Longitude, index)?
                        .unwrap_or(*default_longitude),
                    altitude: extractor.get_f64_opt(Name::Altitude, index)?
                        .unwrap_or(0.0),
                };
                ExtractedPosition::Geographic { position }
            },
            Self::Local { extractor, frame } => {
                let position = extractor.get_vec3_opt(Name::Position, index)?
                    .unwrap_or([0.0; 3]);
                ExtractedPosition::Local { position, frame }
            },
        };
        Ok(extracted)
    }

    fn geographic_extractor(
        states: Option<&Bound<'py, PyAny>>,
        kwargs: Option<&Bound<'py, PyDict>>,
    ) -> PyResult<Extractor<'py, 3>> {
        Extractor::from_args(
            [
                Field::maybe_float(Name::Latitude),
                Field::maybe_float(Name::Longitude),
                Field::maybe_float(Name::Altitude),
            ],
            states,
            kwargs,
        )
    }

    fn local_extractor(
        states: Option<&Bound<'py, PyAny>>,
        kwargs: Option<&Bound<'py, PyDict>>,
    ) -> PyResult<Extractor<'py, 1>> {
        Extractor::from_args(
            [ Field::maybe_vec3(Name::Position) ],
            states,
            kwargs,
        )
    }

    #[allow(unused)] // XXX needed?
    #[inline]
    pub fn shape(&self) -> Vec<usize> {
        match self {
            Self::Geographic { extractor, .. } => extractor.shape(),
            Self::Local { extractor, .. } => extractor.shape(),
        }
    }

    #[inline]
    pub fn size(&self) -> usize {
        match self {
            Self::Geographic { extractor, .. } => extractor.size(),
            Self::Local { extractor, .. } => extractor.size(),
        }
    }

    pub fn with_default_latitude(mut self, value: f64) -> Self {
        match &mut self {
            Self::Geographic { default_latitude, .. } => {
                *default_latitude = value;
            }
            _ => (),
        }
        self
    }

    pub fn with_default_longitude(mut self, value: f64) -> Self {
        match &mut self {
            Self::Geographic { default_longitude, .. } => {
                *default_longitude = value;
            }
            _ => (),
        }
        self
    }
}

impl<'a> ExtractedPosition<'a> {
    pub fn into_geographic(self) -> GeographicCoordinates {
        match self {
            Self::Geographic { position } => position,
            Self::Local { position, frame } => frame.to_geographic_position(&position),
        }
    }
}
