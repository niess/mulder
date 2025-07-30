use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use crate::geometry::Geometry;
use crate::utils::coordinates::{GeographicCoordinates, HorizontalCoordinates, LocalFrame};
use crate::utils::error::{self, Error};
use crate::utils::error::ErrorKind::{TypeError, ValueError};
use crate::utils::extract::{Field, Extractor, Name};
use crate::utils::notify::{Notifier, NotifyArg};
use crate::utils::numpy::{ArrayMethods, Dtype, NewArray, PyArray};
use crate::utils::traits::Vector3;
use pyo3::prelude::*;
use pyo3::sync::GILOnceCell;
use pyo3::type_object::PyTypeInfo;
use pyo3::types::{PyDict, PyList, PyTuple};
use std::collections::HashMap;


pub fn initialise(py: Python) -> PyResult<()> {
    let lights = PyList::new(py, [
        AmbientLight::default().into_pyobject(py)?.into_any(),
        SunLight::new(None, None)?.into_pyobject(py)?.into_any(),
    ])?.unbind();

    let palette = PyDict::new(py);
    palette.set_item("Rock", Colour {
        rgb: (139, 69, 19),
        specularity: 0.5,
    })?;
    palette.set_item("Water", Colour {
        rgb: (212, 241, 249),
        specularity: 0.8,
    })?;
    let palette = palette.unbind();

    let raw_picture = RawPicture::type_object(py);
    raw_picture.setattr("lights", lights)?;
    raw_picture.setattr("palette", palette)?;

    Ok(())
}

#[pyclass(module="mulder")]
pub struct Camera {
    /// The camera latitude coordinate, in degrees.
    #[pyo3(get)]
    latitude: f64,

    /// The camera longitude coordinate, in degrees.
    #[pyo3(get)]
    longitude: f64,

    /// The camera altitude coordinate, in m.
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

    /// The pixels altitude coordinate, in m.
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

#[pyclass(module="mulder")]
pub struct RawPicture {
    /// The picture latitude coordinate, in degrees.
    #[pyo3(get)]
    latitude: f64,

    /// The picture longitude coordinate, in degrees.
    #[pyo3(get)]
    longitude: f64,

    /// The picture altitude coordinate, in m.
    #[pyo3(get)]
    altitude: f64,

    /// The layers' materials.
    #[pyo3(set)]
    materials: Vec<String>,

    /// The pixels data.
    #[pyo3(get)]
    pixels: Py<PyArray<PictureData>>,
}

#[repr(C)]
#[derive(Clone)]
struct PictureData {
    layer: i32,
    direction: [f64; 3],
    normal: [f64; 3],
}

#[derive(FromPyObject)]
enum Light {
    Ambient(AmbientLight),
    Directional(DirectionalLight),
    Sun(SunLight),
}

#[pyclass(module="mulder")]
#[derive(Clone)]
pub struct AmbientLight {
    #[pyo3(get, set)]
    intensity: f64,
}

#[pyclass(module="mulder")]
#[derive(Clone)]
pub struct DirectionalLight {
    #[pyo3(get, set)]
    azimuth: f64,

    #[pyo3(get, set)]
    elevation: f64,

    #[pyo3(get, set)]
    intensity: f64,
}

#[pyclass(module="mulder")]
#[derive(Clone)]
pub struct SunLight {
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

struct ResolvedLight {
    direction: [f64; 3],
    intensity: f64,
}

#[pyclass(module="mulder")]
#[derive(Clone)]
struct Colour {
    #[pyo3(get, set)]
    rgb: (u8, u8, u8),

    #[pyo3(get)]
    specularity: f64,
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
            latitude, longitude, altitude, azimuth, elevation, resolution, fov, ratio, pixels,
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
    ) -> PyResult<RawPicture> {
        let nu = self.resolution.width();
        let nv = self.resolution.height();
        let mut array = NewArray::<PictureData>::empty(py, [nv, nu])?;
        let picture = array.as_slice_mut();

        geometry.ensure_stepper(py)?;
        let notifier = Notifier::from_arg(notify, picture.len(), "shooting geometry");

        let layers: Vec<_> = geometry.layers.iter().map(
            |layer| layer.bind_borrowed(py).borrow()
        ).collect();
        let data: Vec<_> = layers.iter().map(
            |layer| layer.get_data_ref(py)
        ).collect();

        let into_usize = |i: i32| -> usize {
            if i >= 0 { i as usize } else { usize::MAX }
        };

        let normalised = |mut v: [f64; 3]| -> [f64; 3] {
            let r2 = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
            if r2 > f64::EPSILON {
                let r = r2.sqrt();
                v[0] /= r;
                v[1] /= r;
                v[2] /= r;
                v
            } else {
                [0.0; 3]
            }
        };

        let r0 = self.position().to_ecef();
        for (i, direction) in self.iter().enumerate() {
            const WHY: &str = "while shooting geometry";
            if (i % 100) == 0 { error::check_ctrlc(WHY)? }

            geometry.reset_stepper();

            // XXX Check ray start location (air, or not / inverse normal accordingly).
            let (intersection, index) = geometry.trace(self.position(), direction)?;
            let layer = intersection.after;
            let (direction, normal) = if (layer as usize) < layers.len() {
                let position = GeographicCoordinates {
                    latitude: intersection.latitude,
                    longitude: intersection.longitude,
                    altitude: intersection.altitude,
                };
                let ri = position.to_ecef();
                let direction = normalised([
                    ri[0] - r0[0],
                    ri[1] - r0[1],
                    ri[2] - r0[2],
                ]);
                let normal = match data.get(into_usize(layer)) {
                    Some(data) => match data.get(into_usize(index)) {
                        Some(data) => normalised(data.gradient(
                            intersection.latitude,
                            intersection.longitude,
                            intersection.altitude,
                        )),
                        None => [0.0; 3],
                    }
                    None => [0.0; 3],
                };
                (direction, normal)
            } else {
                let direction = [0.0; 3];
                let normal = [0.0; 3];
                (direction, normal)
            };
            picture[i] = PictureData { layer, direction, normal };
            notifier.tic();
        }
        let pixels = array.into_bound().unbind();

        let materials: Vec<_> = geometry.layers.iter()
            .map(|layer| layer.bind(py).borrow().material.clone())
            .collect();

        let latitude = self.latitude;
        let longitude = self.longitude;
        let altitude = self.altitude;

        let picture = RawPicture { latitude, longitude, altitude, materials, pixels };
        Ok(picture)
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

static PICTURE_DTYPE: GILOnceCell<PyObject> = GILOnceCell::new();

impl Dtype for PictureData {
    fn dtype<'py>(py: Python<'py>) -> PyResult<&'py Bound<'py, PyAny>> {
        let ob = PICTURE_DTYPE.get_or_try_init(py, || -> PyResult<_> {
            let ob = PyModule::import(py, "numpy")?
                .getattr("dtype")?
                .call1(([
                        ("layer",     "i4"),
                        ("direction", "3f8"),
                        ("normal",    "3f8"),
                    ],
                    true,
                ))?
                .unbind();
            Ok(ob)
        })?
        .bind(py);
        Ok(ob)
    }
}

#[pymethods]
impl AmbientLight {
    #[new]
    #[pyo3(signature=(intensity=None))]
    fn new(intensity: Option<f64>) -> Self {
        let intensity = intensity.unwrap_or_else(|| Self::DEFAULT_INTENSITY);
        Self { intensity }
    }
}

impl AmbientLight {
    const DEFAULT_INTENSITY: f64 = 0.3;
}

impl Default for AmbientLight {
    fn default() -> Self {
        Self { intensity: Self::DEFAULT_INTENSITY }
    }
}

#[pymethods]
impl DirectionalLight {
    #[new]
    #[pyo3(signature=(azimuth, elevation, *, intensity=None))]
    fn new(azimuth: f64, elevation: f64, intensity: Option<f64>) -> Self {
        let intensity = intensity.unwrap_or_else(|| 1.0 - AmbientLight::DEFAULT_INTENSITY);
        Self { azimuth, elevation, intensity }
    }
}

impl DirectionalLight {
    #[inline]
    fn direction(&self) -> HorizontalCoordinates {
        HorizontalCoordinates { azimuth: self.azimuth, elevation: self.elevation }
    }

    fn resolve(&self, position: &GeographicCoordinates) -> ResolvedLight {
        let direction = self.direction().to_ecef(&position);
        ResolvedLight { direction, intensity: self.intensity }
    }
}

#[pymethods]
impl SunLight {
    #[new]
    #[pyo3(signature=(/, *, datetime=None, intensity=None))]
    fn new(datetime: Option<DateTimeArg>, intensity: Option<f64>) -> PyResult<Self> {
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
        Ok(Self { datetime, intensity })
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
    fn to_directional(&self, latitude: f64) -> PyResult<DirectionalLight> {
        let datetime = self.datetime.and_utc();
        let position = spa::solar_position::<spa::StdFloatOps>(
            datetime, latitude, 0.0,
        ).unwrap(); 
        let elevation = 90.0 - position.zenith_angle;
        let azimuth = position.azimuth;
        let intensity = self.intensity;
        Ok(DirectionalLight { azimuth, elevation, intensity })
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

#[pymethods]
impl RawPicture {
    #[new]
    fn new(py: Python) -> PyResult<Self> {
        let latitude = 0.0;
        let longitude = 0.0;
        let altitude = 0.0;
        let materials = Vec::new();
        let pixels = NewArray::zeros(py, [])?.into_bound().unbind();
        Ok(Self { latitude, longitude, altitude, materials, pixels })
    }

    #[getter]
    fn get_materials<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        PyTuple::new(
            py,
            self.materials.iter().map(|material| material.clone()),
        )
    }

    fn __getstate__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        // This ensures that no field is omitted.
        let Self { latitude, longitude, altitude, materials, pixels } = self;

        let state = PyDict::new(py);
        state.set_item("latitude", latitude)?;
        state.set_item("longitude", longitude)?;
        state.set_item("altitude", altitude)?;
        state.set_item("materials", materials)?;
        state.set_item("pixels", pixels)?;
        Ok(state)
    }

    fn __setstate__(&mut self, state: Bound<PyDict>) -> PyResult<()> {
        *self = Self { // This ensures that no field is omitted.
            latitude: state.get_item("latitude")?.unwrap().extract()?,
            longitude: state.get_item("longitude")?.unwrap().extract()?,
            altitude: state.get_item("altitude")?.unwrap().extract()?,
            materials: state.get_item("materials")?.unwrap().extract()?,
            pixels: state.get_item("pixels")?.unwrap().extract()?,
        };
        Ok(())
    }

    #[pyo3(signature=(/, *, lights=None, palette=None, notify=None))]
    fn develop<'py>(
        &mut self,
        py: Python<'py>,
        lights: Option<Vec<Light>>,
        palette: Option<HashMap<String, Colour>>,
        notify: Option<NotifyArg>,
    ) -> PyResult<NewArray<'py, f32>> {
        // Resolve lights.
        let lights = match lights {
            Some(lights) => lights,
            None => Self::default_lights(py)?.extract()?,
        };
        let (ambient, directionals) = {
            let mut ambient = 0.0;
            let mut directionals = Vec::<ResolvedLight>::new();
            for light in lights {
                match light {
                    Light::Ambient(light) => ambient += light.intensity,
                    Light::Directional(light) => {
                        directionals.push(light.resolve(&self.position()))
                    },
                    Light::Sun(light) => {
                        directionals.push(
                            light
                                .to_directional(self.latitude)?
                                .resolve(&self.position())
                        )
                    },
                }
            }
            (ambient, directionals)
        };

        // Resolve colours.
        let palette = match palette {
            Some(palette) => palette,
            None => Self::default_palette(py)?.extract()?,
        };
        let colours = {
            let mut colours = Vec::new();
            for material in self.materials.iter() {
                let colour = palette
                    .get(material)
                    .ok_or_else(|| {
                        let why = format!("undefined colour for material '{}'", material);
                        Error::new(ValueError).what("palette").why(&why).to_err()
                    })?;
                colours.push(colour);
            }
            colours
        };

        // Loop over pixels.
        let data = self.pixels.bind(py);
        let mut shape = data.shape();
        shape.push(3);
        let mut array = NewArray::empty(py, shape)?;
        let pixels = array.as_slice_mut();

        let notifier = Notifier::from_arg(notify, data.size(), "developing picture");
        for i in 0..data.size() {
            let PictureData { layer, direction, normal } = data.get_item(i)?;
            let rgb = if (layer as usize) < colours.len() {
                let Colour { rgb, specularity } = colours
                    .get(layer as usize)
                    .ok_or_else(|| {
                        let why = format!(
                            "expected a value in [0, {}], found '{}'",
                            colours.len(),
                            layer,
                        );
                        Error::new(ValueError).what("layer index").why(&why).to_err()
                    })?;
                let mut intensity = ambient;
                let mut delta = 0.0;
                for light in &directionals {
                    let diff = normal.dot(&light.direction);
                    if diff > 0.0 {
                        intensity += light.intensity * diff;
                        let spec = normal.mul(2.0 * diff).sub(&light.direction).dot(&direction);
                        if spec > 0.0 {
                            delta +=
                                light.intensity * specularity * spec.powi(Self::SPECULARITY_ALPHA);
                        }
                    }
                }
                [
                    (rgb.0 as f64 / 255.0 * intensity + delta).min(1.0),
                    (rgb.1 as f64 / 255.0 * intensity + delta).min(1.0),
                    (rgb.2 as f64 / 255.0 * intensity + delta).min(1.0),
                ]
            } else {
                [0.0; 3]
            };

            for j in 0..3 {
                pixels[3 * i + j] = rgb[j] as f32;
            }
            notifier.tic();
        }

        Ok(array)
    }
}

impl RawPicture {
    const SPECULARITY_ALPHA: i32 = 3;

    #[inline]
    fn default_lights(py: Python) -> PyResult<Bound<PyAny>> {
        RawPicture::type_object(py).getattr("lights")
    }

    #[inline]
    fn default_palette(py: Python) -> PyResult<Bound<PyAny>> {
        RawPicture::type_object(py).getattr("palette")
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
