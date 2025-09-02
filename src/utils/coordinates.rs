use crate::bindings::turtle;
use crate::utils::numpy::{Dtype, impl_dtype};
use pyo3::prelude::*;


// ===============================================================================================
//
// Geographic coordinates.
//
// ===============================================================================================

#[derive(Clone, Copy, Debug, Default, IntoPyObject)]
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
#[derive(Clone, Copy, Debug, Default)]
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

#[derive(Clone, Default)]
#[pyclass(module="mulder")]
pub struct LocalFrame {
    #[pyo3(get)]
    pub origin: GeographicCoordinates,

    pub rotation: [[f64; 3]; 3],
    #[allow(unused)] // XXX needed?
    pub translation: [f64; 3],
}

#[pymethods]
impl LocalFrame {

    #[new]
    #[pyo3(signature=(*, latitude, longitude, altitude=None, declination=None, inclination=None))]
    fn py_new(
        latitude: f64,
        longitude: f64,
        altitude: Option<f64>,
        declination: Option<f64>,
        inclination: Option<f64>,
    ) -> Self {
        let altitude = altitude.unwrap_or(0.0);
        let declination = declination.unwrap_or(0.0);
        let inclination = inclination.unwrap_or(0.0);
        let origin = GeographicCoordinates { latitude, longitude, altitude };
        Self::new(origin, declination, inclination)
    }
}

impl LocalFrame {
    pub fn to_ecef_direction(&self, enu: &[f64; 3]) -> [f64; 3] {
        let mut ecef = [0.0; 3];
        for i in 0..3 {
            for j in 0..3 {
                ecef[i] += self.rotation[j][i] * enu[j];
            }
        }
        ecef
    }

    #[allow(unused)] // XXX needed?
    pub fn to_ecef_position(&self, enu: &[f64; 3]) -> [f64; 3] {
        let mut ecef = self.to_ecef_direction(enu);
        for i in 0..3 {
            ecef[i] += self.translation[i];
        }
        ecef
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
        Self { rotation, translation, origin }
    }
}
