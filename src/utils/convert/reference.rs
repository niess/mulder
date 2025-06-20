use crate::utils::convert::Convert;
use enum_variants_strings::EnumVariantsStrings;
use pyo3::prelude::*;


#[derive(Clone, Copy, Default, EnumVariantsStrings, PartialEq)]
#[enum_variants_strings_transform(transform="none")]
pub enum ParametricModel {
    #[default]
    GCCLY15,
    Gaisser90,
}

impl Convert for ParametricModel {
    #[inline]
    fn what() -> &'static str {
        "model"
    }
}

impl<'py> FromPyObject<'py> for ParametricModel {
    fn extract_bound(any: &Bound<'py, PyAny>) -> PyResult<Self> {
        Self::from_any(any)
    }
}

impl<'py> IntoPyObject<'py> for ParametricModel {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = std::convert::Infallible;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        self.into_bound(py)
    }
}

impl From<ParametricModel> for &'static str {
    fn from(value: ParametricModel) -> Self {
        value.to_str()
    }
}
