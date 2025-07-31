use crate::utils::coordinates::GeographicCoordinates;
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::ValueError;
use crate::utils::notify::{Notifier, NotifyArg};
use crate::utils::numpy::{ArrayMethods, Dtype, NewArray, PyArray};
use crate::utils::traits::Vector3;
use pyo3::prelude::*;
use pyo3::type_object::PyTypeInfo;
use pyo3::types::{PyDict, PyTuple};
use pyo3::sync::GILOnceCell;
use std::collections::HashMap;

mod colours;
mod lights;

pub use colours::Colour;
pub use lights::{AmbientLight, DirectionalLight, SunLight};


pub fn initialise(py: Python) -> PyResult<()> {
    let raw_picture = RawPicture::type_object(py);
    raw_picture.setattr("lights", lights::default_lights(py)?)?;
    raw_picture.setattr("palette", colours::default_palette(py)?)?;
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

    #[pyo3(signature=(/, *, lights=None, palette=None, notify=None))]
    fn develop<'py>(
        &mut self,
        py: Python<'py>,
        lights: Option<Vec<lights::Light>>,
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
