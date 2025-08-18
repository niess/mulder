use crate::utils::coordinates::{GeographicCoordinates, HorizontalCoordinates, LocalFrame};
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::ValueError;
use crate::utils::notify::{Notifier, NotifyArg};
use crate::utils::numpy::{ArrayMethods, Dtype, NewArray, PyArray};
use pyo3::prelude::*;
use pyo3::type_object::PyTypeInfo;
use pyo3::types::{PyDict, PyTuple};
use pyo3::sync::GILOnceCell;
use std::collections::HashMap;
use super::Transform;

mod atmosphere;
mod lights;
mod materials;
mod pbr;
mod vec3;

pub use atmosphere::SkyProperties;
pub use lights::{AmbientLight, DirectionalLight, SunLight};
pub use materials::OpticalProperties;


pub fn initialise(py: Python) -> PyResult<()> {
    let raw_picture = RawPicture::type_object(py);
    raw_picture.setattr("lights", lights::default_lights(py)?)?;
    raw_picture.setattr("materials", materials::default_materials(py)?)?;
    Ok(())
}

#[pyclass(module="mulder")]
pub struct RawPicture {
    pub(super) transform: Transform,

    /// The picture exposure value.
    #[pyo3(get, set)]
    pub exposure_value: f64,

    /// The layers' materials.
    #[pyo3(set)]
    pub materials: Vec<String>,

    /// The pixels data.
    #[pyo3(get)]
    pub pixels: Py<PyArray<PictureData>>,
}

#[repr(C)]
#[derive(Clone)]
pub struct PictureData {
    pub layer: i32,
    pub altitude: f32,
    pub distance: f32,
    pub normal: [f32; 2],
}

static PICTURE_DTYPE: GILOnceCell<PyObject> = GILOnceCell::new();

impl Dtype for PictureData {
    fn dtype<'py>(py: Python<'py>) -> PyResult<&'py Bound<'py, PyAny>> {
        let ob = PICTURE_DTYPE.get_or_try_init(py, || -> PyResult<_> {
            let ob = PyModule::import(py, "numpy")?
                .getattr("dtype")?
                .call1(([
                        ("layer",    "i4"),
                        ("altitude", "f4"),
                        ("distance", "f4"),
                        ("normal",   "2f4"),
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
impl RawPicture {
    #[new]
    fn new(py: Python) -> PyResult<Self> {
        let transform = Default::default();
        let exposure_value = 0.0;
        let materials = Vec::new();
        let pixels = NewArray::zeros(py, [])?.into_bound().unbind();
        Ok(Self { transform, exposure_value, materials, pixels })
    }

    /// The picture latitude coordinate, in degrees.
    #[getter]
    pub fn get_latitude(&self) -> f64 {
        self.position().latitude
    }

    /// The picture longitude coordinate, in degrees.
    #[getter]
    pub fn get_longitude(&self) -> f64 {
        self.position().longitude
    }

    /// The picture altitude coordinate, in m.
    #[getter]
    pub fn get_altitude(&self) -> f64 {
        self.position().altitude
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
        let Self { transform, exposure_value, materials, pixels } = self;
        let Transform { frame, ratio, f } = transform;
        let LocalFrame { origin, rotation, .. } = frame;
        let GeographicCoordinates { latitude, longitude, altitude } = origin;

        let state = PyDict::new(py);
        state.set_item("latitude", latitude)?;
        state.set_item("longitude", longitude)?;
        state.set_item("altitude", altitude)?;
        state.set_item("rotation", rotation)?;
        state.set_item("ratio", ratio)?;
        state.set_item("f", f)?;
        state.set_item("exposure_value", exposure_value)?;
        state.set_item("materials", materials)?;
        state.set_item("pixels", pixels)?;
        Ok(state)
    }

    fn __setstate__(&mut self, state: Bound<PyDict>) -> PyResult<()> {
        let origin = GeographicCoordinates {
            latitude: state.get_item("latitude")?.unwrap().extract()?,
            longitude: state.get_item("longitude")?.unwrap().extract()?,
            altitude: state.get_item("altitude")?.unwrap().extract()?,
        };
        let frame = LocalFrame {
            origin,
            rotation: state.get_item("rotation")?.unwrap().extract()?,
            translation: [0.0; 3],
        };
        let transform = Transform { // This ensures that no field is omitted.
            frame,
            ratio: state.get_item("ratio")?.unwrap().extract()?,
            f: state.get_item("f")?.unwrap().extract()?,
        };
        *self = Self { // This ensures that no field is omitted.
            transform,
            exposure_value: state.get_item("exposure_value")?.unwrap().extract()?,
            materials: state.get_item("materials")?.unwrap().extract()?,
            pixels: state.get_item("pixels")?.unwrap().extract()?,
        };
        Ok(())
    }

    #[pyo3(signature=(/, *, atmosphere=true, lights=None, materials=None, notify=None))]
    fn develop<'py>(
        &mut self,
        py: Python<'py>,
        atmosphere: Option<bool>,
        lights: Option<Vec<lights::Light>>,
        materials: Option<HashMap<String, OpticalProperties>>,
        notify: Option<NotifyArg>,
    ) -> PyResult<NewArray<'py, f32>> {
        let atmosphere = atmosphere.unwrap_or(true);

        // Resolve lights.
        let lights = match lights {
            Some(lights) => lights,
            None => Self::default_lights(py)?.extract()?,
        };
        let (ambient, directionals) = {
            let mut ambient = vec3::Vec3::ZERO;
            let mut directionals = Vec::<lights::ResolvedLight>::new();
            for light in lights {
                match light {
                    lights::Light::Ambient(light) => ambient += light.luminance(),
                    lights::Light::Directional(light) => {
                        directionals.push(light.resolve(self.position()))
                    },
                    lights::Light::Sun(light) => {
                        directionals.push(
                            light
                                .to_directional(self.position().latitude)?
                                .resolve(self.position())
                        )
                    },
                }
            }
            (ambient, directionals)
        };

        // Resolve materials.
        let materials = match materials {
            Some(materials) => materials,
            None => Self::default_materials(py)?.extract()?,
        };
        let materials = {
            let mut properties = Vec::new();
            for material in self.materials.iter() {
                let property = materials
                    .get(material)
                    .map(|material| materials::MaterialData::from(material))
                    .unwrap_or_else(|| materials::MaterialData::from(
                        &OpticalProperties::default()
                    ));
                properties.push(property);
            }
            properties
        };

        // Instanciate the atmosphere.
        let atmosphere = if atmosphere {
            Some(atmosphere::Atmosphere::new(self, &directionals))
        } else {
            None
        };

        // Compute the exposure.
        let exposure = 2.0_f64.powf(-self.exposure_value) / 120.0;

        // Loop over pixels.
        let data = self.pixels.bind(py);
        let mut shape = data.shape();
        let (nv, nu) = (shape[0], shape[1]);
        shape.push(3);
        let mut array = NewArray::empty(py, shape)?;
        let pixels = array.as_slice_mut();

        let notifier = Notifier::from_arg(notify, data.size(), "developing picture");
        for i in 0..data.size() {
            let PictureData { layer, normal, altitude, distance } = data.get_item(i)?;
            let unpack = |v: [f32; 2]| {
                HorizontalCoordinates { azimuth: v[0] as f64, elevation: v[1] as f64 }
            };
            let normal = unpack(normal)
                .to_ecef(self.position());
            let u = Transform::uv(i % nu, nu);
            let v = Transform::uv(i / nu, nv);
            let direction = self.transform.direction(u, v);
            let view = direction
                .to_ecef(self.position());
            let view = core::array::from_fn(|i| -view[i]);
            let hdr = if (layer as usize) < materials.len() {
                let material = materials
                    .get(layer as usize)
                    .ok_or_else(|| {
                        let why = format!(
                            "expected a value in [0, {}], found '{}'",
                            materials.len(),
                            layer,
                        );
                        Error::new(ValueError).what("layer index").why(&why).to_err()
                    })?;
                pbr::illuminate(
                    u, v, altitude as f64, distance as f64, normal, view, ambient, &directionals,
                    material, atmosphere.as_ref(),
                )
            } else {
                match &atmosphere {
                    Some(atmosphere) => atmosphere.sky_view(&direction),
                    None => vec3::Vec3::ZERO,
                }
            };
            let hdr = hdr * exposure;
            let ldr = ToneMapping::map(hdr);
            let rgb: (u8, u8, u8) = materials::LinearRgb(ldr.0).into();

            pixels[3 * i + 0] = (rgb.0 as f32) / 255.0;
            pixels[3 * i + 1] = (rgb.1 as f32) / 255.0;
            pixels[3 * i + 2] = (rgb.2 as f32) / 255.0;

            notifier.tic();
        }

        Ok(array)
    }
}

impl RawPicture {
    #[inline]
    fn default_lights(py: Python) -> PyResult<Bound<PyAny>> {
        RawPicture::type_object(py).getattr("lights")
    }

    #[inline]
    fn default_materials(py: Python) -> PyResult<Bound<PyAny>> {
        RawPicture::type_object(py).getattr("materials")
    }

    #[inline]
    fn position(&self) -> &GeographicCoordinates {
        &self.transform.frame.origin
    }
}

struct ToneMapping;

impl ToneMapping {
    // Extended Reinhard tone mapping.
    // Ref: https://64.github.io/tonemapping/
    fn map(c: vec3::Vec3) -> vec3::Vec3 {
        const BASE: vec3::Vec3 = vec3::Vec3([0.2126, 0.7152, 0.0722]);
        c / (1.0 + vec3::Vec3::dot(&c, &BASE))
    }
}
