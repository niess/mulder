use pyo3::prelude::*;
use super::vec3::Vec3;


pub struct LinearRgb (pub [f64; 3]);

#[derive(Copy, Clone, Debug, FromPyObject, IntoPyObject)]
pub enum StandardRgb {
    Triplet((f64, f64, f64)),
    Scalar(f64),
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

impl From<LinearRgb> for StandardRgb {
    #[inline]
    fn from(value: LinearRgb) -> Self {
        Self::Triplet((
            LinearRgb::to_standard(value.red()),
            LinearRgb::to_standard(value.green()),
            LinearRgb::to_standard(value.blue()),
        ))
    }
}

impl From<Vec3> for StandardRgb {
    #[inline]
    fn from(value: Vec3) -> Self {
        let linear: LinearRgb = value.into();
        linear.into()
    }
}

impl StandardRgb {
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

impl From<StandardRgb> for LinearRgb {
    #[inline]
    fn from(value: StandardRgb) -> Self {
        match value {
            StandardRgb::Triplet(value) => Self ([
                Self::to_linear(value.0),
                Self::to_linear(value.1),
                Self::to_linear(value.2),
            ]),
            StandardRgb::Scalar(value) => {
                let value = Self::to_linear(value);
                Self ([value, value, value])
            },
        }
    }
}

impl From<Vec3> for LinearRgb {
    #[inline]
    fn from(value: Vec3) -> Self {
        let ldr = ToneMapping::map(value);
        Self(ldr.0)
    }
}

struct ToneMapping;

impl ToneMapping {
    // Extended Reinhard tone mapping.
    // Ref: https://64.github.io/tonemapping/
    #[inline]
    fn map(c: Vec3) -> Vec3 {
        const BASE: Vec3 = Vec3([0.2126, 0.7152, 0.0722]);
        let c = c / (1.0 + Vec3::dot(&c, &BASE));
        c.clamp(0.0, 1.0)
    }
}
