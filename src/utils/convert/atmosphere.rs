use crate::utils::convert::Convert;
use enum_variants_strings::EnumVariantsStrings;
use pyo3::prelude::*;


#[derive(Clone, Copy, Default, EnumVariantsStrings, PartialEq)]
#[enum_variants_strings_transform(transform="kebab_case")]
pub enum AtmosphericModel {
    MidlatitudeSummer,
    MidlatitudeWinter,
    SubarticSummer,
    SubarticWinter,
    Tropical,
    #[default]
    UsStandard,
}

impl Convert for AtmosphericModel {
    #[inline]
    fn what() -> &'static str {
        "atmospheric model"
    }
}

impl<'py> FromPyObject<'py> for AtmosphericModel {
    fn extract_bound(any: &Bound<'py, PyAny>) -> PyResult<Self> {
        Self::from_any(any)
    }
}

impl<'py> IntoPyObject<'py> for AtmosphericModel {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = std::convert::Infallible;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        self.into_bound(py)
    }
}

impl From<AtmosphericModel> for &'static str {
    fn from(value: AtmosphericModel) -> Self {
        value.to_str()
    }
}
