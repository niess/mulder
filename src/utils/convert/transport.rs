use crate::utils::convert::Convert;
use crate::bindings::pumas;
use enum_variants_strings::EnumVariantsStrings;
use pyo3::prelude::*;
use std::ffi::c_int;


#[derive(Clone, Copy, Default, EnumVariantsStrings, PartialEq)]
#[enum_variants_strings_transform(transform="lower_case")]
pub enum TransportMode {
    #[default]
    Continuous,
    Discrete,
    Mixed,
}

impl Convert for TransportMode {
    #[inline]
    fn what() -> &'static str {
        "mode"
    }
}

impl<'py> FromPyObject<'py> for TransportMode {
    fn extract_bound(any: &Bound<'py, PyAny>) -> PyResult<Self> {
        Self::from_any(any)
    }
}

impl<'py> IntoPyObject<'py> for TransportMode {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = std::convert::Infallible;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        self.into_bound(py)
    }
}

impl TransportMode {
    pub fn to_pumas_mode(self) -> c_int {
        match self {
            Self::Continuous => pumas::MODE_CSDA,
            Self::Discrete => pumas::MODE_STRAGGLED,
            Self::Mixed => pumas::MODE_MIXED,
        }
    }
}
