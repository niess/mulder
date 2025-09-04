use crate::bindings::turtle;
use crate::utils::numpy::{Dtype, impl_dtype};
use pyo3::prelude::*;
use pyo3::types::PyDict;


// ===============================================================================================
//
// Geographic coordinates.
//
// ===============================================================================================

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GeographicCoordinates {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
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


// ===============================================================================================
//
// Horizontal angular coordinates.
//
// ===============================================================================================

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HorizontalCoordinates {
    pub azimuth: f64,
    pub elevation: f64,
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


// ===============================================================================================
//
// Local frame (ENU like).
//
// ===============================================================================================

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

#[pymethods]
impl LocalFrame {
    #[new]
    #[pyo3(signature=(
        latitude=None, longitude=None, altitude=None, *, declination=None, inclination=None
    ))]
    fn py_new( // XXX use kwargs?
        latitude: Option<f64>,
        longitude: Option<f64>,
        altitude: Option<f64>,
        declination: Option<f64>,
        inclination: Option<f64>,
    ) -> Self {
        let latitude = latitude.unwrap_or(Self::DEFAULT_LATITUDE);
        let longitude = longitude.unwrap_or(Self::DEFAULT_LONGITUDE);
        let altitude = altitude.unwrap_or(0.0);
        let declination = declination.unwrap_or(0.0);
        let inclination = inclination.unwrap_or(0.0);
        let origin = GeographicCoordinates { latitude, longitude, altitude };
        Self::new(origin, declination, inclination)
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
