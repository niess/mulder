use crate::utils::error::{Error, variant_explain};
use crate::utils::error::ErrorKind::ValueError;
use enum_variants_strings::EnumVariantsStrings;
use pyo3::prelude::*;
use std::convert::Infallible;

mod atmosphere;
mod light;
mod physics;
mod reference;
mod transform;
mod transport;

pub use atmosphere::AtmosphericModel;
pub use light::LightModel;
pub use physics::{Bremsstrahlung, PairProduction, Photonuclear};
pub use reference::ParametricModel;
pub use transform::TransformMode;
pub use transport::TransportMode;


pub trait Convert {
    fn what() -> &'static str;

    #[inline]
    fn from_any<'py>(any: &Bound<'py, PyAny>) -> PyResult<Self>
    where
        Self: EnumVariantsStrings,
    {
        let name: String = any.extract()?;
        Self::from_string(name)
    }

    #[inline]
    fn from_string(name: String) -> PyResult<Self>
    where
        Self: EnumVariantsStrings,
    {
        let value = Self::from_str(&name)
            .map_err(|options| {
                let why = variant_explain(&name, options);
                Error::new(ValueError).what(Self::what()).why(&why).to_err()
            })?;
        Ok(value)
    }

    #[inline]
    fn into_bound<'py>(self, py: Python<'py>) -> Result<Bound<'py, PyAny>, Infallible>
    where
        Self: EnumVariantsStrings,
    {
        self.to_str()
            .into_pyobject(py)
            .map(|obj| obj.into_any())
    }
}
