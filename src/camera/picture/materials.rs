use pyo3::prelude::*;
use pyo3::types::PyDict;


#[inline]
pub fn default_materials(py: Python) -> PyResult<PyObject> {
    let materials = PyDict::new(py);
    materials.set_item("Rock", OpticalProperties {
        colour: Srgb::Triplet((101.0 / 255.0, 67.0 / 255.0, 33.0 / 255.0)),
        roughness: 0.5,
        ..Default::default()
    })?;
    materials.set_item("Water", OpticalProperties {
        colour: Srgb::WHITE,
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
    pub colour: Srgb,

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

#[derive(Copy, Clone, Debug, FromPyObject, IntoPyObject)]
pub enum Srgb {
    Triplet((f64, f64, f64)),
    Scalar(f64),
}

pub struct MaterialData {
    pub diffuse_colour: [f64; 3],
    pub f0: [f64; 3],
    pub roughness: f64,
    pub perceptual_roughness: f64,
}

pub struct LinearRgb (pub [f64; 3]);

impl Default for OpticalProperties {
    fn default() -> Self {
        Self {
            colour: Srgb::WHITE,
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
    #[inline]
    pub const fn red(&self) -> f64 {
        self.0[0]
    }

    #[inline]
    pub const fn green(&self) -> f64 {
        self.0[1]
    }

    #[inline]
    pub const fn blue(&self) -> f64 {
        self.0[2]
    }

    // Convert a standard value to a linear one.
    // Ref: https://en.wikipedia.org/wiki/Gamma_correction.
    pub fn to_linear(value: f64) -> f64 {
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    // Convert a linear value to a standard one.
    // Ref: https://en.wikipedia.org/wiki/Gamma_correction.
    pub fn to_standard(value: f64) -> f64 {
        if value <= 0.0 {
            0.0
        } else if value <= 0.0031308 {
            value * 12.92
        } else if value < 1.0 {
            1.055 * value.powf(1.0 / 2.4) - 0.055
        } else {
            1.0
        }
    }
}

impl From<LinearRgb> for Srgb {
    #[inline]
    fn from(value: LinearRgb) -> Self {
        Self::Triplet((
            LinearRgb::to_standard(value.red()),
            LinearRgb::to_standard(value.green()),
            LinearRgb::to_standard(value.blue()),
        ))
    }
}

impl Srgb {
    pub const WHITE: Self = Self::Triplet((1.0, 1.0, 1.0));

    #[inline]
    pub const fn red(&self) -> f64 {
        match self {
            Self::Triplet(value) => value.0,
            Self::Scalar(value) => *value,
        }
    }

    #[inline]
    pub const fn green(&self) -> f64 {
        match self {
            Self::Triplet(value) => value.1,
            Self::Scalar(value) => *value,
        }
    }

    #[inline]
    pub const fn blue(&self) -> f64 {
        match self {
            Self::Triplet(value) => value.2,
            Self::Scalar(value) => *value,
        }
    }
}

impl From<Srgb> for LinearRgb {
    #[inline]
    fn from(value: Srgb) -> Self {
        match value {
            Srgb::Triplet(value) => Self ([
                Self::to_linear(value.0),
                Self::to_linear(value.1),
                Self::to_linear(value.2),
            ]),
            Srgb::Scalar(value) => {
                let value = Self::to_linear(value);
                Self ([value, value, value])
            },
        }
    }
}
