use crate::utils::convert::Convert;
use enum_variants_strings::EnumVariantsStrings;
use pyo3::prelude::*;
use ::std::ffi::CString;
use ::std::hash::Hash;


#[derive(Clone, Copy, Default, EnumVariantsStrings, Hash, PartialEq)]
#[enum_variants_strings_transform(transform="none")]
pub enum PairProduction {
    KKP68,
    #[default]
    SSR19,
}

impl PairProduction {
    pub fn as_pumas(&self) -> &str {
        let value = self.to_str();
        &value[0..value.len()-2]
    }
}

impl Convert for PairProduction {
    #[inline]
    fn what() -> &'static str {
        "pair-production model"
    }
}

impl<'py> FromPyObject<'py> for PairProduction {
    fn extract_bound(any: &Bound<'py, PyAny>) -> PyResult<Self> {
        Self::from_any(any)
    }
}

impl<'py> IntoPyObject<'py> for PairProduction {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = std::convert::Infallible;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        self.into_bound(py)
    }
}

impl From<PairProduction> for &'static str {
    fn from(value: PairProduction) -> Self {
        value.to_str()
    }
}

impl From<PairProduction> for CString {
    fn from(value: PairProduction) -> Self {
        CString::new(value.as_pumas()).unwrap()
    }
}
