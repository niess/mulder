use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use super::colours::{LinearRgb, StandardRgb};
use super::vec3::Vec3;


#[inline]
pub fn default_materials(py: Python) -> PyResult<PyObject> {
    let materials = PyDict::new(py);
    materials.set_item("Rock", OpticalProperties {
        colour: StandardRgb(101.0 / 255.0, 67.0 / 255.0, 33.0 / 255.0),
        roughness: 0.5,
        ..Default::default()
    })?;
    materials.set_item("Water", OpticalProperties {
        colour: StandardRgb::WHITE,
        roughness: 0.2,
        metallic: Metallic::Bool(true),
        ..Default::default()
    })?;
    let materials = materials.into_any().unbind();
    Ok(materials)
}

/// Graphic properties of a material.
#[pyclass(module="mulder.picture", name="Material")]
#[derive(Clone)]
pub struct OpticalProperties {
    /// Perceived colour (albedo), in sRGB space.
    #[pyo3(get, set)]
    pub colour: StandardRgb,

    /// Dielectric (false) or conductor (true).
    #[pyo3(get, set)]
    pub metallic: Metallic,

    /// Specular intensity for non-metals, in [0, 1].
    #[pyo3(get, set)]
    pub reflectance: f64,

    /// Surface roughness, in [0, 1].
    #[pyo3(get, set)]
    pub roughness: f64,
}

#[derive(Clone, Copy, FromPyObject, IntoPyObject)]
pub enum Metallic {
    Float(f64),
    Bool(bool),
}

#[derive(Clone, FromPyObject, IntoPyObject)]
pub enum Material {
    Properties(OpticalProperties),
    Map(MaterialMap),
}

/// A Material map.
#[derive(Clone)]
#[pyclass(module="mulder.picture")]
pub struct MaterialMap {
    nodes: Vec<(f64, OpticalProperties)>,
}

#[derive(Clone)]
pub struct MaterialData {
    pub diffuse_colour: Vec3,
    pub f0: Vec3,
    pub roughness: f64,
}

#[pymethods]
impl OpticalProperties {
    #[new]
    #[pyo3(signature=(*, colour=None, metallic=None, reflectance=None, roughness=None))]
    fn new(
        colour: Option<StandardRgb>,
        metallic: Option<Metallic>,
        reflectance: Option<f64>,
        roughness: Option<f64>,
    ) -> Self {
        let colour = colour.unwrap_or(Self::DEFAULT_COLOUR);
        let metallic = metallic.unwrap_or(Self::DEFAULT_METALLIC);
        let reflectance = reflectance.unwrap_or(Self::DEFAULT_REFLECTANCE);
        let roughness = roughness.unwrap_or(Self::DEFAULT_ROUGHNESS);
        Self { colour, metallic, reflectance, roughness }
    }
}

impl OpticalProperties {
    const DEFAULT_COLOUR: StandardRgb = StandardRgb::WHITE;
    const DEFAULT_METALLIC: Metallic = Metallic::Bool(false);
    const DEFAULT_REFLECTANCE: f64 = 0.5;
    const DEFAULT_ROUGHNESS: f64 = 0.0;
}

impl Default for OpticalProperties {
    fn default() -> Self {
        Self {
            colour: Self::DEFAULT_COLOUR,
            metallic: Self::DEFAULT_METALLIC,
            reflectance: Self::DEFAULT_REFLECTANCE,
            roughness: Self::DEFAULT_ROUGHNESS,
        }
    }
}

impl MaterialData {
    const MIN_ROUGHNESS: f64 = 0.045;
}

impl From<&OpticalProperties> for MaterialData {
    fn from(value: &OpticalProperties) -> Self {
        let metallic: f64 = value.metallic.into();
        let roughness = value.roughness
            .clamp(Self::MIN_ROUGHNESS, 1.0)
            .powi(2);
        let reflectance = 0.16 * value.reflectance
            .clamp(0.0, 1.0)
            .powi(2);
        let colour = Vec3(LinearRgb::from(value.colour).0);
        let diffuse_colour = (1.0 - metallic) * colour;
        let f0 = (1.0 - metallic) * Vec3::splat(reflectance) + metallic * colour;
        Self { diffuse_colour, f0, roughness }
    }
}

impl From<Metallic> for f64 {
    fn from(value: Metallic) -> Self {
        match value {
            Metallic::Bool(metallic) => match metallic {
                true => 1.0,
                false => 0.0,
            },
            Metallic::Float(metallic) => metallic.clamp(0.0, 1.0),
        }
    }
}

impl From<f64> for Metallic {
    fn from(value: f64) -> Self {
        if value <= 0.0 {
            Metallic::Bool(false)
        } else if value < 1.0 {
            Metallic::Float(value)
        } else {
            Metallic::Bool(true)
        }
    }
}

#[pymethods]
impl MaterialMap {
    /// Creates a new material map.
    #[pyo3(signature=(nodes, /))]
    #[new]
    pub fn new(mut nodes: Vec<(f64, OpticalProperties)>) -> Self {
        nodes.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        nodes.dedup_by(|a, b| a.0 == b.0);
        Self { nodes }
    }

    /// The map nodes.
    #[getter]
    fn get_nodes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        PyTuple::new(py, self.nodes.clone())
    }

    /// Maps to a material.
    #[pyo3(name="material", signature=(alpha, /))]
    pub fn map(&self, alpha: f64) -> OpticalProperties {
        let n = self.nodes.len();
        if n == 0 {
            OpticalProperties::default()
        } else if (n == 1) || (alpha <= self.nodes[0].0) {
            self.nodes[0].1.clone()
        } else {
            let mut x0 = self.nodes[0].0;
            for i in 0..(n - 1) {
                let x1 = self.nodes[i + 1].0;
                if alpha < x1 {
                    let t = (alpha - x0) / (x1 - x0);
                    let m0 = &self.nodes[i].1;
                    let m1 = &self.nodes[i + 1].1;
                    let c0 = &m0.colour;
                    let c1 = &m1.colour;
                    let f0: f64 = m0.metallic.into();
                    let f1: f64 = m1.metallic.into();
                    return OpticalProperties {
                        colour: StandardRgb(
                            c0.0 * (1.0 - t) + c1.0 * t,
                            c0.1 * (1.0 - t) + c1.1 * t,
                            c0.2 * (1.0 - t) + c1.2 * t,
                        ),
                        metallic: (f0 * (1.0 - t) + f1 * t).into(),
                        roughness: m0.roughness * (1.0 - t) + m1.roughness * t,
                        reflectance: m0.reflectance * (1.0 - t) + m1.reflectance * t,
                    }
                } else {
                    x0 = x1;
                }
            }
            self.nodes[n - 1].1.clone()
        }
    }
}
