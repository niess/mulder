use crate::geometry::Geometry;
use crate::utils::coordinates::{GeographicCoordinates, HorizontalCoordinates, LocalFrame};
use crate::utils::error::{self, Error};
use crate::utils::error::ErrorKind::{TypeError, ValueError};
use crate::utils::extract::{Field, Extractor, Name};
use crate::utils::notify::{Notifier, NotifyArg};
use crate::utils::numpy::{NewArray, PyArray};
use pyo3::prelude::*;
use pyo3::types::PyDict;


#[pyclass(module="mulder")]
pub struct Camera {
    /// The camera latitude coordinate, in degrees.
    #[pyo3(get)]
    latitude: f64,

    /// The camera longitude coordinate, in degrees.
    #[pyo3(get)]
    longitude: f64,

    /// The camera altitude coordinate, in degrees.
    #[pyo3(get)]
    altitude: f64,

    /// The camera azimuth direction, in degrees.
    #[pyo3(get)]
    azimuth: f64,

    /// The camera elevation direction, in degrees.
    #[pyo3(get)]
    elevation: f64,

    /// The camera diagonal field-of-view, in degrees.
    #[pyo3(get)]
    fov: f64,

    /// The camera screen ratio.
    #[pyo3(get)]
    ratio: f64,

    /// The camera screen resolution, in pixels.
    #[pyo3(get)]
    resolution: (usize, usize),

    pixels: Option<Py<PixelsCoordinates>>,
}

#[pyclass(module="mulder")]
pub struct PixelsCoordinates {
    /// The pixels latitude coordinate, in degrees.
    #[pyo3(get)]
    latitude: f64,

    /// The pixels longitude coordinate, in degrees.
    #[pyo3(get)]
    longitude: f64,

    /// The pixels altitude coordinate, in degrees.
    #[pyo3(get)]
    altitude: f64,

    /// The pixels azimuth direction, in degrees.
    #[pyo3(get)]
    azimuth: Py<PyArray<f64>>,

    /// The pixels elevation direction, in degrees.
    #[pyo3(get)]
    elevation: Py<PyArray<f64>>,

    /// The pixels u coordinates.
    #[pyo3(get)]
    u: Py<PyArray<f64>>,

    /// The pixels v coordinates.
    #[pyo3(get)]
    v: Py<PyArray<f64>>,

    /// The screen ratio.
    #[pyo3(get)]
    ratio: f64,

    /// The screen resolution.
    #[pyo3(get)]
    resolution: (usize, usize),
}

struct Iter {
    frame: LocalFrame,
    ratio: f64,
    f: f64,
    nu: usize,
    nv: usize,
    index: usize,
}

#[pymethods]
impl Camera {
    #[new]
    #[pyo3(signature=(coordinates=None, /, *, resolution=None, fov=None, ratio=None, **kwargs))]
    fn new(
        coordinates: Option<&Bound<PyAny>>,
        resolution: Option<[usize; 2]>,
        fov: Option<f64>,
        ratio: Option<f64>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<Self> {
        let coordinates = Extractor::from_args(
            [
                Field::float(Name::Latitude),
                Field::float(Name::Longitude),
                Field::float(Name::Altitude),
                Field::float(Name::Azimuth),
                Field::float(Name::Elevation),
            ],
            coordinates,
            kwargs,
        )?;

        if coordinates.size() != 1 {
            let why = format!("expected a scalar, found size = {}", coordinates.size());
            let err = Error::new(TypeError)
                .what("camera coordinates")
                .why(&why)
                .to_err();
            return Err(err)
        }

        let latitude = coordinates.get_f64(Name::Latitude, 0)?;
        let longitude = coordinates.get_f64(Name::Longitude, 0)?;
        let altitude = coordinates.get_f64(Name::Altitude, 0)?;
        let azimuth = coordinates.get_f64(Name::Azimuth, 0)?;
        let elevation = coordinates.get_f64(Name::Elevation, 0)?;

        let resolution = resolution.unwrap_or_else(|| [90, 120]);
        let resolution = Self::checked_resolution(resolution)?;
        let ratio = ratio.unwrap_or_else(||
            (resolution.width() as f64) / (resolution.height() as f64)
        );
        let fov = fov.unwrap_or_else(|| 60.0);
        let pixels = None;

        Ok(Self {
            latitude, longitude, altitude, azimuth, elevation, resolution, fov, ratio, pixels
        })
    }

    /// The camera focal length.
    #[getter]
    fn get_focal_length(&mut self) -> f64 { // XXX setter as well?
        self.focal_length()
    }

    #[getter]
    fn get_pixels<'py>(&mut self, py: Python<'py>) -> PyResult<Py<PixelsCoordinates>> {
        if self.pixels.is_none() {
            let pixels = PixelsCoordinates::new(py, self)?;
            self.pixels = Some(Py::new(py, pixels)?);
        };
        Ok(self.pixels.as_ref().unwrap().clone_ref(py))
    }

    #[setter]
    fn set_latitude(&mut self, value: f64) {
        if value != self.latitude {
            self.latitude = value;
            self.pixels = None;
        }
    }

    #[setter]
    fn set_longitude(&mut self, value: f64) {
        if value != self.longitude {
            self.longitude = value;
            self.pixels = None;
        }
    }

    #[setter]
    fn set_altitude(&mut self, value: f64) {
        if value != self.altitude {
            self.altitude = value;
            self.pixels = None;
        }
    }

    #[setter]
    fn set_azimuth(&mut self, value: f64) {
        if value != self.azimuth {
            self.azimuth = value;
            self.pixels = None;
        }
    }

    #[setter]
    fn set_elevation(&mut self, value: f64) {
        if value != self.elevation {
            self.elevation = value;
            self.pixels = None;
        }
    }

    #[setter]
    fn set_fov(&mut self, value: f64) {
        if value != self.fov {
            self.fov = value;
            self.pixels = None;
        }
    }

    #[setter]
    fn set_ratio(&mut self, value: f64) {
        if value != self.ratio {
            self.ratio = value;
            self.pixels = None;
        }
    }

    #[setter]
    fn set_resolution(&mut self, value: [usize; 2]) -> PyResult<()> {
        let value = Self::checked_resolution(value)?;
        if value != self.resolution {
            self.resolution = value;
            self.pixels = None;
        }
        Ok(())
    }

    #[pyo3(signature=(geometry, /, *, notify=None))]
    fn shoot<'py>(
        &mut self,
        py: Python<'py>,
        geometry: &mut Geometry,
        notify: Option<NotifyArg>,
    ) -> PyResult<NewArray<'py, i32>> {
        let nu = self.resolution.width();
        let nv = self.resolution.height();
        let mut array = NewArray::empty(py, [nv, nu])?;
        let picture = array.as_slice_mut();

        geometry.ensure_stepper(py)?;
        let notifier = Notifier::from_arg(notify, picture.len(), "shooting geometry");

        for (i, direction) in self.iter().enumerate() {
            const WHY: &str = "while shooting geometry";
            if (i % 100) == 0 { error::check_ctrlc(WHY)? }

            geometry.reset_stepper();

            let intersection = geometry.trace(self.position(), direction)?;
            picture[i] = intersection.after;
            notifier.tic();
        }

        Ok(array)
    }
}

impl Camera {
    const DEG: f64 = std::f64::consts::PI / 180.0;

    fn checked_resolution(resolution: [usize; 2]) -> PyResult<(usize, usize)> {
        if (resolution[0] <= 0) || (resolution[1] <= 0) {
            let why = format!("expected strictly positive values, found {:?}", resolution);
            let err = Error::new(ValueError).what("resolution").why(&why).to_err();
            Err(err)
        } else {
            Ok((resolution[0], resolution[1]))
        }
    }

    fn focal_length(&self) -> f64 {
        0.5 * (1.0 + self.ratio.powi(2)).sqrt() / (0.5 * (self.fov * Self::DEG)).tan()
    }

    fn iter(&self) -> Iter {
        Iter {
            frame: self.local_frame(),
            ratio: self.ratio,
            f: self.focal_length(),
            nu: self.resolution.width(),
            nv: self.resolution.height(),
            index: 0,
        }
    }

    fn local_frame(&self) -> LocalFrame {
        let origin = GeographicCoordinates {
            latitude: self.latitude,
            longitude: self.longitude,
            altitude: self.altitude,
        };
        LocalFrame::new(origin, self.azimuth, -self.elevation)
    }

    #[inline]
    fn position(&self) -> GeographicCoordinates {
        GeographicCoordinates {
            latitude: self.latitude,
            longitude: self.longitude,
            altitude: self.altitude,
        }
    }
}

#[pymethods]
impl PixelsCoordinates {
    /// The pixels coordinates wrapped by a dict.
    #[getter]
    fn get_coordinates<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("latitude", self.latitude)?;
        dict.set_item("longitude", self.longitude)?;
        dict.set_item("altitude", self.altitude)?;
        dict.set_item("azimuth", self.azimuth.clone_ref(py))?;
        dict.set_item("elevation", self.elevation.clone_ref(py))?;
        Ok(dict)
    }
}

impl PixelsCoordinates {
    fn new(py: Python, camera: &Camera) -> PyResult<Self> {
        let nu = camera.resolution.width();
        let nv = camera.resolution.height();
        let mut az_array = NewArray::<f64>::empty(py, [nv, nu])?;
        let mut el_array = NewArray::<f64>::empty(py, [nv, nu])?;
        let mut u_array = NewArray::<f64>::empty(py, [nu,])?;
        let mut v_array = NewArray::<f64>::empty(py, [nv,])?;
        let azimuth = az_array.as_slice_mut();
        let elevation = el_array.as_slice_mut();
        let u = u_array.as_slice_mut();
        let v = v_array.as_slice_mut();

        let iter = camera.iter();
        for i in 0..nv {
            v[i] = iter.v(i);
        }
        for j in 0..nu {
            u[j] = iter.u(j);
        }
        for (i, direction) in iter.enumerate() {
            azimuth[i] = direction.azimuth;
            elevation[i] = direction.elevation;
        }

        let latitude = camera.latitude;
        let longitude = camera.longitude;
        let altitude = camera.altitude;
        let azimuth = az_array.into_bound().unbind();
        let elevation = el_array.into_bound().unbind();
        let u = u_array.into_bound().unbind();
        let v = v_array.into_bound().unbind();
        let ratio = camera.ratio;
        let resolution = (camera.resolution.height(), camera.resolution.width());

        Ok(Self { latitude, longitude, altitude, azimuth, elevation, u, v, ratio, resolution })
    }
}

impl Iter {
    #[inline]
    fn u(&self, j: usize) -> f64 {
        if self.nu == 1 { 0.0 } else { self.ratio * ((j as f64) / ((self.nu - 1) as f64) - 0.5)}
    }

    #[inline]
    fn v(&self, i: usize) -> f64 {
        if self.nv == 1 { 0.0 } else { (i as f64) / ((self.nv - 1) as f64) - 0.5 }
    }
}

impl Iterator for Iter {
    type Item = HorizontalCoordinates;

    fn next(&mut self) -> Option<Self::Item> {
        let i = self.index / self.nu;
        let j = self.index % self.nu;
        self.index += 1;

        if (i < self.nv) && (j < self.nu) {
            let uj = self.u(j);
            let vi = self.v(i);
            let horizontal = self.frame.to_horizontal(&[uj, self.f, vi]);
            Some(horizontal)
        } else {
            None
        }
    }
}

trait HeightWidth {
    fn height(&self) -> usize;
    fn width(&self) -> usize;
}

impl HeightWidth for [usize; 2] {
    #[inline]
    fn height(&self) -> usize {
        self[0]
    }

    #[inline]
    fn width(&self) -> usize {
        self[1]
    }
}

impl HeightWidth for (usize, usize) {
    #[inline]
    fn height(&self) -> usize {
        self.0
    }

    #[inline]
    fn width(&self) -> usize {
        self.1
    }
}
