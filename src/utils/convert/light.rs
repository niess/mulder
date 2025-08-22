use crate::utils::convert::Convert;
use enum_variants_strings::EnumVariantsStrings;
use pyo3::prelude::*;


#[derive(Clone, Copy, Debug, Default, EnumVariantsStrings, PartialEq)]
#[enum_variants_strings_transform(transform="lower_case")]
pub enum LightModel {
    #[default]
    Ambient,
    Directional,
    Sun,
}

impl Convert for LightModel {
    #[inline]
    fn what() -> &'static str {
        "light model"
    }
}

impl<'py> FromPyObject<'py> for LightModel {
    fn extract_bound(any: &Bound<'py, PyAny>) -> PyResult<Self> {
        Self::from_any(any)
    }
}

impl<'py> IntoPyObject<'py> for LightModel {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = std::convert::Infallible;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        self.into_bound(py)
    }
}

impl From<LightModel> for &'static str {
    fn from(value: LightModel) -> Self {
        value.to_str()
    }
}
