use crate::utils::convert::Convert;
use enum_variants_strings::EnumVariantsStrings;
use pyo3::prelude::*;


#[derive(Clone, Copy, Default, EnumVariantsStrings, PartialEq)]
#[enum_variants_strings_transform(transform="lower_case")]
pub enum TransformMode {
    #[default]
    Point,
    Vector
}

impl Convert for TransformMode {
    #[inline]
    fn what() -> &'static str {
        "mode"
    }
}

impl<'py> FromPyObject<'py> for TransformMode {
    fn extract_bound(any: &Bound<'py, PyAny>) -> PyResult<Self> {
        Self::from_any(any)
    }
}

impl<'py> IntoPyObject<'py> for TransformMode {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = std::convert::Infallible;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        self.into_bound(py)
    }
}

impl From<TransformMode> for &'static str {
    fn from(value: TransformMode) -> Self {
        value.to_str()
    }
}
