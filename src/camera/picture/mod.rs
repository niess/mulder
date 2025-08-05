use crate::utils::coordinates::GeographicCoordinates;
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::ValueError;
use crate::utils::notify::{Notifier, NotifyArg};
use crate::utils::numpy::{ArrayMethods, Dtype, NewArray, PyArray};
use pyo3::prelude::*;
use pyo3::type_object::PyTypeInfo;
use pyo3::types::{PyDict, PyTuple};
use pyo3::sync::GILOnceCell;
use std::collections::HashMap;

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
    /// The picture latitude coordinate, in degrees.
    #[pyo3(get)]
    pub latitude: f64,

    /// The picture longitude coordinate, in degrees.
    #[pyo3(get)]
    pub longitude: f64,

    /// The picture altitude coordinate, in m.
    #[pyo3(get)]
    pub altitude: f64,

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
    pub direction: [f64; 3],
    pub normal: [f64; 3],
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

    #[pyo3(signature=(/, *, lights=None, materials=None, notify=None))]
    fn develop<'py>(
        &mut self,
        py: Python<'py>,
        lights: Option<Vec<lights::Light>>,
        materials: Option<HashMap<String, OpticalProperties>>,
        notify: Option<NotifyArg>,
    ) -> PyResult<NewArray<'py, f32>> {
        // Resolve lights.
        let lights = match lights {
            Some(lights) => lights,
            None => Self::default_lights(py)?.extract()?,
        };
        let (ambient, directionals) = {
            let mut ambient = 0.0;
            let mut directionals = Vec::<lights::ResolvedLight>::new();
            for light in lights {
                match light {
                    lights::Light::Ambient(light) => ambient += light.intensity,
                    lights::Light::Directional(light) => {
                        directionals.push(light.resolve(&self.position()))
                    },
                    lights::Light::Sun(light) => {
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

        // Loop over pixels.
        let data = self.pixels.bind(py);
        let mut shape = data.shape();
        shape.push(3);
        let mut array = NewArray::empty(py, shape)?;
        let pixels = array.as_slice_mut();

        let notifier = Notifier::from_arg(notify, data.size(), "developing picture");
        for i in 0..data.size() {
            let PictureData { layer, direction, normal } = data.get_item(i)?;
            let rgb = if (layer as usize) < materials.len() {
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
                pbr::illuminate(normal, direction, ambient, &directionals, material)
            } else {
                [0.0; 3]
            };
            let rgb: (u8, u8, u8) = materials::LinearRgb(rgb).into();

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
    fn position(&self) -> GeographicCoordinates {
        GeographicCoordinates {
            latitude: self.latitude,
            longitude: self.longitude,
            altitude: self.altitude,
        }
    }
}
