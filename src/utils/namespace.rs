use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};


#[derive(IntoPyObject)]
pub struct Namespace<'py> (Bound<'py, PyAny>);

impl<'py> Namespace<'py> {
    pub fn new<T, U>(
        py: Python<'py>,
        kwargs: impl IntoIterator<Item = (&'static str, T), IntoIter = U>,
    ) -> PyResult<Self>
    where
        T: IntoPyObject<'py>,
        U: ExactSizeIterator<Item = (&'static str, T)>,
    {
        let kwargs = PyTuple::new(py, kwargs)?;
        let kwargs = PyDict::from_sequence(kwargs.as_any())?;
        let namespace = py.import("types")
            .and_then(|x| x.getattr("SimpleNamespace"))
            .and_then(|x| x.call((), Some(&kwargs)))?;
        Ok(Self(namespace))
    }
}
