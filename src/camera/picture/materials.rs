use pyo3::prelude::*;
use pyo3::types::PyDict;
use super::colours::MaterialColour;
use super::vec3::Vec3;


#[inline]
pub fn default_materials(py: Python) -> PyResult<PyObject> {
    let materials = PyDict::new(py);
    materials.set_item("Rock", OpticalProperties {
        colour: MaterialColour::standard(101.0 / 255.0, 67.0 / 255.0, 33.0 / 255.0),
        roughness: 0.5,
        ..Default::default()
    })?;
    materials.set_item("Water", OpticalProperties {
        colour: MaterialColour::WHITE,
        roughness: 0.2,
        metallic: true,
        ..Default::default()
    })?;
    let materials = materials.into_any().unbind();
    Ok(materials)
}

#[pyclass(module="mulder.picture", name="Material")]
#[derive(Clone)]
pub struct OpticalProperties {
    /// Perceived colour (albedo), in sRGB space.
    #[pyo3(get, set)]
    pub colour: MaterialColour,

    /// Dielectric (false) or conductor (true).
    #[pyo3(get, set)]
    pub metallic: bool,

    /// Specular intensity for non-metals, in [0, 1].
    #[pyo3(get, set)]
    pub reflectance: f64,

    /// Surface roughness, in [0, 1].
    #[pyo3(get, set)]
    pub roughness: f64,
}

pub struct MaterialData {
    colour: MaterialColour,
    metallic: bool,

    pub roughness: f64,
    pub reflectance: f64,
}

#[pymethods]
impl OpticalProperties {
    #[new]
    #[pyo3(signature=(*, colour=None, metallic=None, reflectance=None, roughness=None))]
    fn new(
        colour: Option<MaterialColour>,
        metallic: Option<bool>,
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
    const DEFAULT_COLOUR: MaterialColour = MaterialColour::WHITE;
    const DEFAULT_METALLIC: bool = false;
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

    pub fn resolve_colour(&self, value: f64) -> (Vec3, Vec3) {
        let colour = Vec3(self.colour.to_linear(value).0);
        if self.metallic {
            (Vec3::ZERO, colour)
        } else {
            (colour, Vec3::splat(self.reflectance))
        }
    }
}

impl From<&OpticalProperties> for MaterialData {
    fn from(value: &OpticalProperties) -> Self {
        let colour = value.colour.clone();
        let roughness = value.roughness
            .clamp(Self::MIN_ROUGHNESS, 1.0)
            .powi(2);
        let reflectance = 0.16 * value.reflectance
            .clamp(0.0, 1.0)
            .powi(2);
        Self { colour, metallic: value.metallic, roughness, reflectance }
    }
}
