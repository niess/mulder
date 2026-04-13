use crate::utils::convert::Convert;
use enum_variants_strings::EnumVariantsStrings;
use pyo3::prelude::*;


#[derive(Clone, Copy, Default, EnumVariantsStrings, PartialEq)]
#[enum_variants_strings_transform(transform="snake_case")]
pub enum ScanOutput {
    Grammage,
    Intersections,
    #[default]
    Thickness,
}

impl Convert for ScanOutput {
    #[inline]
    fn what() -> &'static str {
        "output"
    }
}

impl<'py> FromPyObject<'py> for ScanOutput {
    fn extract_bound(any: &Bound<'py, PyAny>) -> PyResult<Self> {
        Self::from_any(any)
    }
}

impl<'py> IntoPyObject<'py> for ScanOutput {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = std::convert::Infallible;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        self.into_bound(py)
    }
}

impl From<ScanOutput> for &'static str {
    fn from(value: ScanOutput) -> Self {
        value.to_str()
    }
}
