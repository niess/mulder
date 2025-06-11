use crate::utils::error::{Error, variant_explain};
use crate::utils::error::ErrorKind::ValueError;
use enum_variants_strings::EnumVariantsStrings;
use pyo3::prelude::*;
use std::convert::Infallible;

// XXX mod array;
mod materials;
mod mdf;
mod physics;
mod toml;

// XXX pub use array::Array;
pub use mdf::Mdf;
pub use physics::{Bremsstrahlung, PairProduction, Photonuclear};
pub use toml::ToToml;


trait Convert {
    fn what() -> &'static str;

    #[inline]
    fn from_any<'py>(any: &Bound<'py, PyAny>) -> PyResult<Self>
    where
        Self: EnumVariantsStrings,
    {
        let name: String = any.extract()?;
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
