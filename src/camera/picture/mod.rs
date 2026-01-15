use crate::simulation::coordinates::{
    GeographicCoordinates, HorizontalCoordinates, LocalFrame, LocalTransformer,
};
use crate::utils::error::{Error, ErrorKind::TypeError};
use crate::utils::notify::{Notifier, NotifyArg};
use crate::utils::numpy::{AnyArray, ArrayMethods, NewArray, impl_dtype, PyArray};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use std::collections::HashMap;
use super::Transform;

mod atmosphere;
mod colours;
mod lights;
mod materials;
mod pbr;
mod vec3;

pub use atmosphere::SkyProperties;
pub use lights::{AmbientLight, DirectionalLight, SunLight};
pub use materials::{default_materials, MaterialMap, OpticalProperties};
use materials::{Material, MaterialData};


const DEFAULT_EXPOSURE: f64 = std::f64::consts::PI;

/// A geometry projection.
#[pyclass(module="mulder.picture", name="Projection")]
pub struct RawPicture {
    pub(super) transform: Transform,
    pub atmosphere_medium: i32,
    pub camera_medium: i32,
    pub pixels: Py<PyArray<PictureData>>,

    /// The materials mapping.
    #[pyo3(set)]
    pub materials: Vec<String>,
}

#[derive(FromPyObject)]
enum AtmosphereArg {
    Flag(bool),
    Index(i32),
}

#[repr(C)]
#[derive(Clone)]
pub struct PictureData {
    pub medium: i32,
    pub altitude: f32,
    pub distance: f32,
    pub normal: [f32; 3],
}

impl_dtype!(
    PictureData,
    [
        ("medium",   "i4"),
        ("altitude", "f4"),
        ("distance", "f4"),
        ("normal",   "3f4"),
    ]
);

#[pymethods]
impl RawPicture {
    #[new]
    fn new(py: Python) -> PyResult<Self> {
        let transform = Default::default();
        let atmosphere_medium = 0;
        let camera_medium = 0;
        let materials = Vec::new();
        let pixels = NewArray::zeros(py, [])?.into_bound().unbind();
        Ok(Self { transform, atmosphere_medium, camera_medium, materials, pixels })
    }

    /// The altitude at intersections, in meters.
    #[getter]
    fn get_altitude<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.pixels.bind(py).as_any().get_item("altitude")
    }

    /// The distance to intersections, in meters.
    #[getter]
    fn get_distance<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.pixels.bind(py).as_any().get_item("distance")
    }

    /// The camera reference frame.
    #[getter]
    fn get_frame(&self) -> LocalFrame {
        self.transform.frame.clone()
    }

    /// The visible media indices.
    #[getter]
    fn get_medium<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.pixels.bind(py).as_any().get_item("medium")
    }

    /// The materials mapping.
    #[getter]
    fn get_materials<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        PyTuple::new(
            py,
            self.materials.iter().map(|material| material.clone()),
        )
    }

    fn __getstate__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        // This ensures that no field is omitted.
        let Self { transform, atmosphere_medium, camera_medium, materials, pixels } = self;
        let Transform { frame, ratio, f } = transform;

        let state = PyDict::new(py);
        state.set_item("frame", frame.clone())?;
        state.set_item("ratio", ratio)?;
        state.set_item("f", f)?;
        state.set_item("atmosphere_medium", atmosphere_medium)?;
        state.set_item("camera_medium", camera_medium)?;
        state.set_item("materials", materials)?;
        state.set_item("pixels", pixels)?;
        Ok(state)
    }

    fn __setstate__(&mut self, state: Bound<PyDict>) -> PyResult<()> {
        let transform = Transform { // This ensures that no field is omitted.
            frame: state.get_item("frame")?.unwrap().extract()?,
            ratio: state.get_item("ratio")?.unwrap().extract()?,
            f: state.get_item("f")?.unwrap().extract()?,
        };
        *self = Self { // This ensures that no field is omitted.
            transform,
            atmosphere_medium: state.get_item("atmosphere_medium")?.unwrap().extract()?,
            camera_medium: state.get_item("camera_medium")?.unwrap().extract()?,
            materials: state.get_item("materials")?.unwrap().extract()?,
            pixels: state.get_item("pixels")?.unwrap().extract()?,
        };
        Ok(())
    }

    /// Renders the projection as an image.
    #[pyo3(signature=(/, *, atmosphere=None, data=None, exposure=None, lights=None, notify=None))]
    fn render<'py>(
        &self,
        py: Python<'py>,
        atmosphere: Option<AtmosphereArg>,
        data: Option<HashMap::<String, AnyArray<'py, f64>>>,
        exposure: Option<f64>,
        lights: Option<lights::Lights>,
        notify: Option<NotifyArg>,
    ) -> PyResult<NewArray<'py, f32>> {
        // Resolve materials.
        enum Properties {
            Data(MaterialData),
            Map(MaterialMap)
        }
        let materials = Self::materials(py)?;
        let materials = {
            let mut properties = Vec::new();
            for material in self.materials.iter() {
                let property = materials
                    .get_item(material)?
                    .map(|material| {
                        let material: Material = material.extract()?;
                        let material = match material {
                            Material::Map(map) => Properties::Map(map),
                            Material::Properties(material) => {
                                Properties::Data(MaterialData::from(&material))
                            },
                        };
                        Ok::<_, PyErr>(material)
                    })
                    .transpose()?
                    .unwrap_or_else(|| Properties::Data(MaterialData::from(
                        &OpticalProperties::default()
                    )));
                properties.push(property);
            }
            properties
        };

        // Resolve lights.
        let atmosphere_medium = match atmosphere {
            Some(atmosphere) => match atmosphere {
                AtmosphereArg::Flag(atmosphere) => match atmosphere {
                    true => self.atmosphere_medium,
                    false => -2,
                },
                AtmosphereArg::Index(atmosphere) => atmosphere,
            },
            None => -2,
        };
        let lights = lights
            .unwrap_or_else(|| if self.camera_medium == atmosphere_medium {
                lights::Lights::SUN
            } else {
                lights::Lights::DIRECTIONAL
            })
            .into_vec(self.direction());

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

        // Instanciate the atmosphere.
        let atmosphere = if
            (atmosphere_medium >= 0) &&
            (self.camera_medium == atmosphere_medium) &&
            (directionals.len() > 0)
        {
            Some(atmosphere::Atmosphere::new(self, &directionals))
        } else {
            None
        };

        // Exposure compensation (in stops).
        let exposure_compensation = match exposure {
            Some(exposure) => 2.0_f64.powf(exposure),
            None => 1.0,
        };

        // Loop over pixels.
        let pixels = self.pixels.bind(py);
        let mut shape = pixels.shape();
        let (nv, nu) = (shape[0], shape[1]);
        shape.push(3);
        let mut array = NewArray::empty(py, shape)?;
        let image = array.as_slice_mut();

        // Flatten any data.
        let data = match data {
            Some(mut data) => {
                let data: Result<Vec<Option<AnyArray<'py, f64>>>, PyErr> = self.materials
                    .iter()
                    .map(|material| {
                        data
                            .remove(material)
                            .map(|datum| {
                                if (datum.ndim() == 0) || (datum.size() == pixels.size()) {
                                    Ok(datum)
                                } else {
                                    let why = format!(
                                        "{}: expected a size {} array, found size {}",
                                        material,
                                        pixels.size(),
                                        datum.size(),
                                    );
                                    let err = Error::new(TypeError)
                                        .what("data")
                                        .why(&why)
                                        .to_err();
                                    Err(err)
                                }
                            })
                            .transpose()
                    })
                    .collect();
                Some(data?)
            },
            None => None,
        };

        // Loop over pixels.
        let notifier = Notifier::from_arg(notify, pixels.size(), "rendering projection");
        for i in 0..pixels.size() {
            let PictureData { medium, normal, altitude, distance } = pixels.get_item(i)?;
            let normal = std::array::from_fn(|i| normal[i] as f64);
            let u = Transform::uv(i % nu, nu);
            let v = Transform::uv(i / nu, nv);
            let direction = self.transform.direction(u, v);
            let view = direction
                .to_ecef(self.position());
            let view = core::array::from_fn(|i| -view[i]);
            let hdr = if atmosphere.is_some() && medium == atmosphere_medium {
                match &atmosphere {
                    Some(atmosphere) => {
                        let sky = atmosphere.sky_view(&direction);
                        let sun = atmosphere.sun_view(direction.elevation, &view);
                        (sky + sun) * DEFAULT_EXPOSURE
                    },
                    None => unreachable!(),
                }
            } else if (medium >= 0) && (medium < materials.len() as i32) {
                let material = materials.get(medium as usize).unwrap();
                let material: MaterialData = match material {
                    Properties::Data(material) => material.clone(),
                    Properties::Map(material) => {
                        let alpha = match &data {
                            Some(data) => match data.get(medium as usize).unwrap() {
                                Some(datum) => datum.get_item(i)?,
                                None => 0.0,
                            },
                            None => 0.0,
                        };
                        (&material.map(alpha)).into()
                    },
                };
                pbr::illuminate(
                    u, v, altitude as f64, distance as f64, normal, view, self.position(),
                    ambient, &directionals, &material, atmosphere.as_ref(),
                )
            } else {
                vec3::Vec3::ZERO
            };
            let srgb = colours::StandardRgb::from(hdr * exposure_compensation);

            image[3 * i + 0] = srgb.red() as f32;
            image[3 * i + 1] = srgb.green() as f32;
            image[3 * i + 2] = srgb.blue() as f32;

            notifier.tic();
        }

        Ok(array)
    }

    /// Returns the surface normal at intersections.
    #[pyo3(signature=(frame=None,))]
    fn normal<'py>(
        &self,
        py: Python<'py>,
        frame: Option<LocalFrame>,
    ) -> PyResult<NewArray<'py, f32>> {
        let frame = frame.as_ref().unwrap_or_else(|| &self.transform.frame);
        let data = self.pixels.bind(py);
        let mut shape = data.shape();
        shape.push(3);
        let mut normal_array = NewArray::empty(py, shape)?;
        let normal = normal_array.as_slice_mut();

        for i in 0..data.size() {
            let di = data.get_item(i)?;
            let n = std::array::from_fn(|i| di.normal[i] as f64);
            let n = frame.from_ecef_direction(&n);
            for j in 0..3 {
                normal[3 * i + j] = n[j] as f32;
            }
        }

        Ok(normal_array)
    }

    /// Returns the view directions.
    #[pyo3(signature=(frame=None,))]
    fn view<'py>(
        &self,
        py: Python<'py>,
        frame: Option<LocalFrame>,
    ) -> PyResult<NewArray<'py, f32>> {
        let frame = frame.as_ref().unwrap_or_else(|| &self.transform.frame);
        let transformer = if frame.eq(&self.transform.frame) {
            None
        } else {
            Some(LocalTransformer::new(&self.transform.frame, frame))
        };

        let data = self.pixels.bind(py);
        let mut shape = data.shape();
        let (nv, nu) = (shape[0], shape[1]);
        shape.push(3);
        let mut view_array = NewArray::empty(py, shape)?;
        let view = view_array.as_slice_mut();

        const DEG: f64 = std::f64::consts::PI / 180.0;
        for i in 0..data.size() {
            let u = Transform::uv(i % nu, nu);
            let v = Transform::uv(i / nu, nv);
            let direction = self.transform.direction(u, v);
            let theta = (90.0 - direction.elevation) * DEG;
            let phi = (90.0 - direction.azimuth) * DEG;
            let (st, ct) = theta.sin_cos();
            let (sp, cp) = phi.sin_cos();
            let mut v = [ st * cp, st * sp, ct ];
            if let Some(transformer) = &transformer {
                v = transformer.transform_vector(&v);
            }
            for j in 0..3 {
                view[3 * i + j] = v[j] as f32;
            }
        }

        Ok(view_array)
    }
}

impl RawPicture {
    #[inline]
    fn materials(py: Python) -> PyResult<Bound<PyDict>> {
        let materials = py.import("mulder.picture")?
            .getattr("MATERIALS")?
            .downcast_into::<PyDict>()?;
        Ok(materials)
    }

    #[inline]
    fn direction(&self) -> HorizontalCoordinates {
        self.transform.direction(0.5, 0.5)
    }

    #[inline]
    fn position(&self) -> &GeographicCoordinates {
        &self.transform.frame.origin
    }
}
