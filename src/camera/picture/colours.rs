use crate::utils::error::Error;
use crate::utils::error::ErrorKind::ValueError;
use pyo3::prelude::*;
use super::vec3::Vec3;


#[derive(Clone, Debug, IntoPyObject, FromPyObject)]
pub enum MaterialColour {
    Map(ColourMap),
    Standard(StandardRgb),
}

#[derive(Clone, Debug)]
#[pyclass(module="mulder.picture")]
pub struct ColourMap {
    data: Vec<(f64, StandardRgb)>,
}

pub struct LinearRgb (pub [f64; 3]);

#[derive(Copy, Clone, Debug, IntoPyObject)]
pub struct StandardRgb (pub f64, pub f64, pub f64);

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

impl<'py> FromPyObject<'py> for StandardRgb {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        let py = ob.py();
        let value: [f64; 3] = if ob.extract::<String>().is_ok() {
            let to_rgb = py
                .import("matplotlib.colors")?
                .getattr("to_rgb")?;
            let rgb = to_rgb.call1((ob,))?;
            rgb.extract()?
        } else {
            let value: [f64; 3] = ob.extract()?;
            for i in 0..3 {
                let vi = value[i];
                if (vi < 0.0) || (vi > 1.0) {
                    let why = format!(
                        "#{}: expected a value in [0,1], found '{}'",
                        i,
                        vi,
                    );
                    let err = Error::new(ValueError)
                        .what("colour")
                        .why(&why);
                    return Err(err.to_err())
                }
            }
            value
        };

        Ok(Self (value[0], value[1], value[2]))
    }
}

impl From<LinearRgb> for StandardRgb {
    #[inline]
    fn from(value: LinearRgb) -> Self {
        Self(
            LinearRgb::to_standard(value.red()),
            LinearRgb::to_standard(value.green()),
            LinearRgb::to_standard(value.blue()),
        )
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
    pub const WHITE: Self = Self (1.0, 1.0, 1.0);

    #[inline]
    pub const fn red(&self) -> f64 {
        self.0
    }

    #[inline]
    pub const fn green(&self) -> f64 {
        self.1
    }

    #[inline]
    pub const fn blue(&self) -> f64 {
        self.2
    }
}

impl From<StandardRgb> for LinearRgb {
    #[inline]
    fn from(value: StandardRgb) -> Self {
        Self ([
            Self::to_linear(value.red()),
            Self::to_linear(value.green()),
            Self::to_linear(value.blue()),
        ])
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

impl MaterialColour {
    pub const WHITE: Self = Self::Standard(StandardRgb::WHITE);

    pub fn standard(red: f64, green: f64, blue: f64) -> Self {
        Self::Standard(StandardRgb(red, green, blue))
    }

    pub fn to_linear(&self, altitude: f64) -> LinearRgb {
        let srgb = match self {
            Self::Map(map) => map.map(altitude),
            Self::Standard(srgb) => *srgb,
        };
        srgb.into()
    }
}

#[pymethods]
impl ColourMap {
    #[pyo3(signature=(data, /))]
    #[new]
    pub fn new(mut data: Vec<(f64, StandardRgb)>) -> Self {
        data.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        data.dedup_by(|a, b| a.0 == b.0);
        Self { data }
    }
}

impl ColourMap {
    pub fn map(&self, value: f64) -> StandardRgb {
        let n = self.data.len();
        if n == 0 {
            StandardRgb::WHITE
        } else {
            let (mut x0, mut c0) = &self.data[0];
            for (x1, c1) in self.data.iter() {
                if value < *x1 {
                    let t = (value - x0) / (*x1 - x0);
                    return StandardRgb (
                        c0.0 * (1.0 - t) + c1.0 * t,
                        c0.1 * (1.0 - t) + c1.1 * t,
                        c0.2 * (1.0 - t) + c1.2 * t,
                    )
                } else {
                    x0 = *x1;
                    c0 = *c1;
                }
            }
            self.data[n - 1].1
        }
    }
}
