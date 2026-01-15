use crate::geometry::{Geometry, GeometryArg};
use crate::geometry::earth::{EarthGeometry, Layer};
use crate::geometry::local::{LocalGeometry, Medium};
use crate::simulation::coordinates::{GeographicCoordinates, HorizontalCoordinates, LocalFrame};
use crate::utils::error::{self, Error};
use crate::utils::error::ErrorKind::{TypeError, ValueError};
use crate::utils::notify::{Notifier, NotifyArg};
use crate::utils::namespace::Namespace;
use crate::utils::numpy::{NewArray, PyArray};
use pyo3::prelude::*;

pub mod picture;

/// A camera model.
#[pyclass(module="mulder.picture")]
pub struct Camera {
    /// The camera reference frame.
    #[pyo3(get)]
    frame: LocalFrame,

    /// The camera horizontal field of view (FOV), in degrees.
    #[pyo3(get)]
    fov: f64,

    /// The camera screen ratio (width / height).
    #[pyo3(get)]
    ratio: f64,

    /// The camera screen resolution (height, width), in pixels.
    #[pyo3(get)]
    resolution: (usize, usize),

    pixels: Option<Py<PixelsCoordinates>>,
}

/// A set of camera pixels.
#[pyclass(module="mulder.picture", name="Pixels")]
pub struct PixelsCoordinates {
    origin: GeographicCoordinates,

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
}

struct Iter {
    transform: Transform,
    nu: usize,
    nv: usize,
    index: usize,
}

#[derive(Default)]
struct Transform {
    frame: LocalFrame,
    ratio: f64,
    f: f64,
}

#[pymethods]
impl Camera {
    /// The camera focal length.
    #[getter]
    fn get_focal(&self) -> f64 {
        self.focal_length()
    }

    /// The camera pixels.
    #[getter]
    fn get_pixels<'py>(&mut self, py: Python<'py>) -> PyResult<Py<PixelsCoordinates>> {
        if self.pixels.is_none() {
            let pixels = PixelsCoordinates::new(py, self)?;
            self.pixels = Some(Py::new(py, pixels)?);
        };
        Ok(self.pixels.as_ref().unwrap().clone_ref(py))
    }

    /// Projects a geometry.
    #[pyo3(signature=(geometry, /, *, notify=None))]
    fn project<'py>(
        &mut self,
        py: Python<'py>,
        geometry: GeometryArg<'py>,
        notify: Option<NotifyArg>,
    ) -> PyResult<picture::RawPicture> {
        let geometry = geometry.into_geometry(py)?;
        match geometry {
            Geometry::Earth(geometry) => {
                let mut geometry = geometry.bind(py).borrow_mut();
                self.shoot_earth(py, &mut geometry, notify)
            },
            Geometry::Local(geometry) => {
                let mut geometry = geometry.bind(py).borrow_mut();
                self.shoot_local(py, &mut geometry, notify)
            },
        }
    }
}

impl Camera {
    pub fn new(
        frame: &LocalFrame,
        resolution: Option<[usize; 2]>,
        focal: Option<f64>,
        fov: Option<f64>,
        ratio: Option<f64>,
    ) -> PyResult<Self> {
        let resolution = resolution.unwrap_or_else(|| [90, 120]);
        let resolution = Self::checked_resolution(resolution)?;
        let ratio = ratio.unwrap_or_else(||
            (resolution.width() as f64) / (resolution.height() as f64)
        );
        if fov.is_some() && focal.is_some() {
            let err = Error::new(TypeError)
                .what("arguments")
                .why("cannot set 'focal' and 'fov'")
                .to_err();
            return Err(err)
        }
        let fov = fov
            .or_else(|| focal.map(|focal| Self::compute_fov(focal)))
            .unwrap_or_else(|| 60.0);
        let pixels = None;
        let frame = frame.clone();

        Ok(Self { frame, resolution, fov, ratio, pixels })
    }

    fn shoot_earth<'py>(
        &mut self,
        py: Python<'py>,
        geometry: &mut EarthGeometry,
        notify: Option<NotifyArg>,
    ) -> PyResult<picture::RawPicture> {
        let nu = self.resolution.width();
        let nv = self.resolution.height();
        let mut array = NewArray::<picture::PictureData>::empty(py, [nv, nu])?;
        let picture = array.as_slice_mut();

        let notifier = Notifier::from_arg(notify, picture.len(), "projecting geometry");

        let mut stepper = geometry.stepper(py)?;
        let layers: Vec<_> = geometry.layers.bind(py).iter().map(
            |layer| layer.downcast::<Layer>().unwrap().borrow()
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

        let camera_medium = stepper.locate(self.position())?;
        let atmosphere_medium = stepper.layers as i32;

        for (i, direction) in self.iter().enumerate() {
            const WHY: &str = "while projecting geometry";
            if (i % 100) == 0 { error::check_ctrlc(WHY)? }

            stepper.reset();
            let (intersection, data_index) = stepper.trace(self.position(), direction)?;
            let (backface, layer) = if intersection.after >= camera_medium {
                (true, camera_medium)
            } else {
                (false, intersection.after)
            };
            let altitude = intersection.altitude as f32;
            let distance = intersection.distance as f32;
            let normal = if (layer < atmosphere_medium) && (layer >= 0) {
                let normal = match data.get(into_usize(layer)) {
                    Some(data) => match data.get(into_usize(data_index)) {
                        Some(data) => {
                            let n = normalised(data.gradient(
                                intersection.latitude,
                                intersection.longitude,
                                intersection.altitude,
                            ));
                            if backface { [ -n[0], -n[1], -n[2] ] } else { n }
                        },
                        None => [0.0; 3],
                    }
                    None => [0.0; 3],
                };
                normal
            } else {
                [0.0; 3]
            };
            let normal = std::array::from_fn(|i| normal[i] as f32);
            picture[i] = picture::PictureData { medium: layer, altitude, distance, normal };
            notifier.tic();
        }
        let pixels = array.into_bound().unbind();

        let materials: Vec<_> = geometry.layers.bind(py).iter()
            .map(|layer| layer.downcast::<Layer>().unwrap().borrow().material.clone())
            .collect();

        let transform = self.transform();

        let picture = picture::RawPicture {
            transform, atmosphere_medium, camera_medium, materials, pixels,
        };
        Ok(picture)
    }

    fn shoot_local<'py>(
        &mut self,
        py: Python<'py>,
        geometry: &mut LocalGeometry,
        notify: Option<NotifyArg>,
    ) -> PyResult<picture::RawPicture> {
        let nu = self.resolution.width();
        let nv = self.resolution.height();
        let mut array = NewArray::<picture::PictureData>::empty(py, [nv, nu])?;
        let picture = array.as_slice_mut();

        let notifier = Notifier::from_arg(notify, picture.len(), "projecting geometry");

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

        let locator = geometry.locator()?;
        let frame = &geometry.frame;
        let camera_position = self.position();
        let r0 = frame.from_ecef_position(camera_position.to_ecef());
        let camera_medium = locator.locate(r0);
        drop(locator);

        let media: Vec<_> = geometry.media.bind(py).iter().map(
            |medium| medium.downcast::<Medium>().unwrap().borrow()
        ).collect();
        let tracer = geometry.tracer()?;

        for (i, direction) in self.iter().enumerate() {
            const WHY: &str = "while projecting geometry";
            if (i % 100) == 0 { error::check_ctrlc(WHY)? }

            let outer_medium = media.len();
            let ui = frame.from_ecef_direction(&direction.to_ecef(&camera_position));
            tracer.reset(r0, ui);
            let distance = tracer.trace();
            tracer.move_(distance);
            let after = tracer.medium();
            let ri = tracer.position();
            let GeographicCoordinates { altitude, .. } = frame.to_geographic_position(&ri);

            let (backface, medium) = if (after == outer_medium) || (after <= camera_medium) {
                (true, camera_medium)
            } else {
                (false, after)
            };
            let altitude = altitude as f32;
            let distance = distance as f32;
            let normal = if medium < outer_medium {
                let normal = match media.get(medium) {
                    Some(medium) => match medium.normal(ri) {
                        Some(normal) => {
                            let normal = frame.to_ecef_direction(&normal);
                            let n = normalised(normal);
                            if backface { [ -n[0], -n[1], -n[2] ] } else { n }
                        },
                        None => [0.0; 3],
                    }
                    None => [0.0; 3],
                };
                normal
            } else {
                [0.0; 3]
            };
            let normal = std::array::from_fn(|i| normal[i] as f32);
            let medium = medium as i32;
            picture[i] = picture::PictureData { medium, altitude, distance, normal };
            notifier.tic();
        }
        let pixels = array.into_bound().unbind();

        let materials: Vec<_> = media.iter()
            .map(|medium| medium.material.clone())
            .collect();

        let transform = self.transform();

        let atmosphere_medium = 0;
        let camera_medium = camera_medium as i32;
        let picture = picture::RawPicture {
            transform, atmosphere_medium, camera_medium, materials, pixels,
        };
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

    fn compute_fov(f: f64) -> f64 {
        2.0 * (1.0 / f).atan() / Self::DEG
    }

    fn focal_length(&self) -> f64 {
        1.0 / (0.5 * self.fov * Self::DEG).tan()
    }

    fn iter(&self) -> Iter {
        Iter {
            transform: self.transform(),
            nu: self.resolution.width(),
            nv: self.resolution.height(),
            index: 0,
        }
    }

    #[inline]
    fn position(&self) -> GeographicCoordinates {
        GeographicCoordinates {
            latitude: self.frame.origin.latitude,
            longitude: self.frame.origin.longitude,
            altitude: self.frame.origin.altitude,
        }
    }

    fn transform(&self) -> Transform {
        Transform {
            frame: self.frame.clone(),
            ratio: self.ratio,
            f: self.focal_length(),
        }
    }
}

#[pymethods]
impl PixelsCoordinates {
    /// The pixels geographic coordinates.
    #[getter]
    fn get_coordinates<'py>(&self, py: Python<'py>) -> PyResult<Namespace<'py>> {
        Namespace::new(py, [
            ("latitude", self.origin.latitude.into_pyobject(py)?.into_any()),
            ("longitude", self.origin.latitude.into_pyobject(py)?.into_any()),
            ("altitude", self.origin.latitude.into_pyobject(py)?.into_any()),
            ("azimuth", self.azimuth.clone_ref(py).into_bound(py).into_any()),
            ("elevation", self.elevation.clone_ref(py).into_bound(py).into_any()),
        ])
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

        for i in 0..nv {
            v[i] = Transform::uv(i, nv);
        }
        for j in 0..nu {
            u[j] = Transform::uv(j, nu);
        }
        for (i, direction) in camera.iter().enumerate() {
            azimuth[i] = direction.azimuth;
            elevation[i] = direction.elevation;
        }

        let origin = camera.frame.origin.clone();
        let azimuth = az_array.into_bound().unbind();
        let elevation = el_array.into_bound().unbind();
        let u = u_array.into_bound().unbind();
        let v = v_array.into_bound().unbind();

        Ok(Self { origin, azimuth, elevation, u, v })
    }
}

impl Iterator for Iter {
    type Item = HorizontalCoordinates;

    fn next(&mut self) -> Option<Self::Item> {
        let i = self.index / self.nu;
        let j = self.index % self.nu;
        self.index += 1;

        if (i < self.nv) && (j < self.nu) {
            let uj = Transform::uv(j, self.nu);
            let vi = Transform::uv(i, self.nv);
            let horizontal = self.transform.direction(uj, vi);
            Some(horizontal)
        } else {
            None
        }
    }
}

impl Transform {
    #[inline]
    fn direction(&self, u: f64, v: f64) -> HorizontalCoordinates {
        self.frame.to_horizontal(&[u - 0.5, self.f, (v - 0.5) / self.ratio])
    }

    #[inline]
    fn uv(i: usize, n: usize) -> f64 {
        if n == 1 { 0.5 } else { (i as f64) / ((n - 1) as f64) }
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
