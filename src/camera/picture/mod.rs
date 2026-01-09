use crate::simulation::coordinates::{
    GeographicCoordinates, HorizontalCoordinates, LocalFrame, LocalTransformer,
};
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::ValueError;
use crate::utils::notify::{Notifier, NotifyArg};
use crate::utils::numpy::{ArrayMethods, NewArray, impl_dtype, PyArray};
use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use super::Transform;

mod atmosphere;
mod colours;
mod lights;
mod materials;
mod pbr;
mod vec3;

pub use atmosphere::SkyProperties;
pub use colours::ColourMap;
pub use lights::{AmbientLight, DirectionalLight, SunLight};
pub use materials::{default_materials, OpticalProperties};


const DEFAULT_EXPOSURE: f64 = std::f64::consts::PI;

#[pyclass(module="mulder.picture", name="Picture")]
pub struct RawPicture {
    pub(super) transform: Transform,
    pub medium: i32,
    pub pixels: Py<PyArray<PictureData>>,

    /// The materials mapping.
    #[pyo3(set)]
    pub materials: Vec<String>, // XXX List / simplify interface?
}

#[repr(C)]
#[derive(Clone)]
pub struct PictureData {
    pub medium: i32,
    pub altitude: f32,
    pub distance: f32,
    pub normal: [f32; 2],
}

impl_dtype!(
    PictureData,
    [
        ("medium",   "i4"),
        ("altitude", "f4"),
        ("distance", "f4"),
        ("normal",   "2f4"),
    ]
);

#[pymethods]
impl RawPicture {
    #[new]
    fn new(py: Python) -> PyResult<Self> {
        let transform = Default::default();
        let medium = 0;
        let materials = Vec::new();
        let pixels = NewArray::zeros(py, [])?.into_bound().unbind();
        Ok(Self { transform, medium, materials, pixels })
    }

    /// The altitude at intersections.
    #[getter]
    fn get_altitude<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.pixels.bind(py).as_any().get_item("altitude")
    }

    /// The distance to intersections.
    #[getter]
    fn get_distance<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.pixels.bind(py).as_any().get_item("distance")
    }

    /// The picture reference frame.
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
        let Self { transform, medium, materials, pixels } = self;
        let Transform { frame, ratio, f } = transform;

        let state = PyDict::new(py);
        state.set_item("frame", frame.clone())?;
        state.set_item("ratio", ratio)?;
        state.set_item("f", f)?;
        state.set_item("medium", medium)?;
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
            medium: state.get_item("medium")?.unwrap().extract()?,
            materials: state.get_item("materials")?.unwrap().extract()?,
            pixels: state.get_item("pixels")?.unwrap().extract()?,
        };
        Ok(())
    }

    /// Renders the picture as a colour image.
    #[pyo3(signature=(/, *, atmosphere=true, exposure=None, lights=None, materials=None, notify=None))]
    fn render<'py>(
        &self,
        py: Python<'py>,
        atmosphere: Option<bool>,
        exposure: Option<f64>,
        lights: Option<lights::Lights>,
        materials: Option<IndexMap<String, OpticalProperties>>,
        notify: Option<NotifyArg>,
    ) -> PyResult<NewArray<'py, f32>> {
        let atmosphere = atmosphere.unwrap_or(true);

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

        // Resolve lights.
        let lights = lights
            .unwrap_or_else(|| if self.medium as usize == materials.len() {
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
            atmosphere &&
            (self.medium as usize == materials.len()) && // XXX local case?
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
        let data = self.pixels.bind(py);
        let mut shape = data.shape();
        let (nv, nu) = (shape[0], shape[1]);
        shape.push(3);
        let mut array = NewArray::empty(py, shape)?;
        let pixels = array.as_slice_mut();

        let notifier = Notifier::from_arg(notify, data.size(), "developing picture");
        for i in 0..data.size() {
            let PictureData { medium, normal, altitude, distance } = data.get_item(i)?;
            let unpack = |v: [f32; 2]| {
                HorizontalCoordinates { azimuth: v[0] as f64, elevation: v[1] as f64 }
            };
            let normal = unpack(normal);
            let normal_ecef = normal
                .to_ecef(self.position());
            let u = Transform::uv(i % nu, nu);
            let v = Transform::uv(i / nu, nv);
            let direction = self.transform.direction(u, v);
            let view = direction
                .to_ecef(self.position());
            let view = core::array::from_fn(|i| -view[i]);
            let hdr = if medium < 0 {
                vec3::Vec3::ZERO
            } else if (medium as usize) < materials.len() {
                let material = materials
                    .get(medium as usize)
                    .ok_or_else(|| {
                        let why = format!(
                            "expected a value in [0, {}], found '{}'",
                            materials.len(),
                            medium,
                        );
                        Error::new(ValueError).what("medium index").why(&why).to_err()
                    })?;
                pbr::illuminate(
                    u, v, altitude as f64, distance as f64, normal_ecef, normal, view,
                    ambient, &directionals, material, atmosphere.as_ref(),
                )
            } else {
                match &atmosphere {
                    Some(atmosphere) => {
                        let sky = atmosphere.sky_view(&direction);
                        let sun = atmosphere.sun_view(direction.elevation, &view);
                        (sky + sun) * DEFAULT_EXPOSURE
                    },
                    None => vec3::Vec3::ZERO,
                }
            };
            let srgb = colours::StandardRgb::from(hdr * exposure_compensation);

            pixels[3 * i + 0] = srgb.red() as f32;
            pixels[3 * i + 1] = srgb.green() as f32;
            pixels[3 * i + 2] = srgb.blue() as f32;

            notifier.tic();
        }

        Ok(array)
    }

    /// Returns the pixels' normal directions.
    // XXX Geocentric case?
    #[pyo3(signature=(frame=None,))]
    fn normal<'py>(
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
        shape.push(3);
        let mut normal_array = NewArray::empty(py, shape)?;
        let normal = normal_array.as_slice_mut();

        const DEG: f64 = std::f64::consts::PI / 180.0;
        for i in 0..data.size() {
            let di = data.get_item(i)?;
            let theta = (90.0 - (di.normal[1] as f64)) * DEG;
            let phi = (90.0 - (di.normal[0] as f64)) * DEG;
            let (st, ct) = theta.sin_cos();
            let (sp, cp) = phi.sin_cos();
            let mut n = [ st * cp, st * sp, ct ];
            if let Some(transformer) = &transformer {
                n = transformer.transform_vector(&n);
            }
            for j in 0..3 {
                normal[3 * i + j] = n[j] as f32;
            }
        }

        Ok(normal_array)
    }

    /// Returns the pixels' view directions.
    // XXX Geocentric case?
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
    fn default_materials(py: Python) -> PyResult<Bound<PyAny>> {
        py.import("mulder.picture")?.getattr("materials")
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
