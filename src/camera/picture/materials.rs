use pyo3::prelude::*;
use pyo3::types::PyDict;
use super::colours::{LinearRgb, StandardRgb};


#[inline]
pub fn default_materials(py: Python) -> PyResult<PyObject> {
    let materials = PyDict::new(py);
    materials.set_item("Rock", OpticalProperties {
        colour: StandardRgb::Triplet((101.0 / 255.0, 67.0 / 255.0, 33.0 / 255.0)),
        roughness: 0.5,
        ..Default::default()
    })?;
    materials.set_item("Water", OpticalProperties {
        colour: StandardRgb::WHITE,
        roughness: 0.2,
        metallic: true,
        ..Default::default()
    })?;
    let materials = materials.into_any().unbind();
    Ok(materials)
}

#[pyclass(module="mulder")]
#[derive(Clone)]
pub struct OpticalProperties {
    /// Perceived colour (albedo), in sRGB space.
    #[pyo3(get, set)]
    pub colour: StandardRgb,

    /// Dielectric (false) or conductor (true).
    #[pyo3(get, set)]
    pub metallic: bool,

    /// Surface roughness, in [0, 1].
    #[pyo3(get, set)]
    pub roughness: f64,

    /// Specular intensity for non-metals, in [0, 1].
    #[pyo3(get, set)]
    pub reflectance: f64,
}

pub struct MaterialData {
    pub diffuse_colour: [f64; 3],
    pub f0: [f64; 3],
    pub roughness: f64,
}

impl Default for OpticalProperties {
    fn default() -> Self {
        Self {
            colour: StandardRgb::WHITE,
            metallic: false,
            roughness: 0.0,
            reflectance: 0.5,
        }
    }
}

impl MaterialData {
    const MIN_ROUGHNESS: f64 = 0.045;
}

impl From<&OpticalProperties> for MaterialData {
    fn from(value: &OpticalProperties) -> Self {
        let colour = LinearRgb::from(value.colour).0;
        let (diffuse_colour, f0) = if value.metallic {
            ([0.0; 3], colour)
        } else {
            let r = 0.16 * value.reflectance
                .clamp(0.0, 1.0)
                .powi(2);
            (colour, [r; 3])
        };
        let perceptual_roughness = value.roughness
            .clamp(Self::MIN_ROUGHNESS, 1.0);
        let roughness = perceptual_roughness.powi(2);
        Self { diffuse_colour, f0, roughness }
    }
}
