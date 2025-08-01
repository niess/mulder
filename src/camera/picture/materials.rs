use pyo3::prelude::*;
use pyo3::types::PyDict;


#[inline]
pub fn default_materials(py: Python) -> PyResult<PyObject> {
    let materials = PyDict::new(py);
    materials.set_item("Rock", OpticalProperties {
        colour: (139, 69, 19),
        ..Default::default()
    })?;
    materials.set_item("Water", OpticalProperties {
        colour: (212, 241, 249),
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
    pub colour: (u8, u8, u8),

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
    pub perceptual_roughness: f64,
}

pub struct LinearRgb (pub [f64; 3]);

impl OpticalProperties {
    const WHITE: (u8, u8, u8) = (255, 255, 255);
}

impl Default for OpticalProperties {
    fn default() -> Self {
        Self {
            colour: Self::WHITE,
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
        Self { diffuse_colour, f0, roughness, perceptual_roughness }
    }
}

impl LinearRgb {
    // Convert a standard value to a linear one.
    // Ref: https://en.wikipedia.org/wiki/Gamma_correction.
    pub fn to_linear(value: u8) -> f64 {
        let value = value as f64 / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    // Convert a linear value to a standard one.
    // Ref: https://en.wikipedia.org/wiki/Gamma_correction.
    pub fn to_standard(value: f64) -> u8 {
        let value = if value <= 0.0 {
            0.0
        } else if value <= 0.0031308 {
            value * 12.92
        } else if value < 1.0 {
            1.055 * value.powf(1.0 / 2.4) - 0.055
        } else {
            1.0
        };
        (value * 255.0) as u8
    }
}

impl From<LinearRgb> for (u8, u8, u8) {
    #[inline]
    fn from(value: LinearRgb) -> Self {
        (
            LinearRgb::to_standard(value.0[0]),
            LinearRgb::to_standard(value.0[1]),
            LinearRgb::to_standard(value.0[2]),
        )
    }
}

impl From<(u8, u8, u8)> for LinearRgb {
    #[inline]
    fn from(value: (u8, u8, u8)) -> Self {
        Self ([
            Self::to_linear(value.0),
            Self::to_linear(value.1),
            Self::to_linear(value.2),
        ])
    }
}
