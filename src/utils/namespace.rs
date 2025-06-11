use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};


pub struct Namespace;

impl Namespace {
    pub fn new<'py, T, U>(
        py: Python<'py>,
        kwargs: impl IntoIterator<Item = (&'static str, T), IntoIter = U>,
    ) -> PyResult<Bound<'py, PyAny>>
    where
        T: IntoPyObject<'py>,
        U: ExactSizeIterator<Item = (&'static str, T)>,
    {
        let kwargs = PyTuple::new(py, kwargs)?;
        let kwargs = PyDict::from_sequence(kwargs.as_any())?;
        let namespace = py.import("types")
            .and_then(|x| x.getattr("SimpleNamespace"))
            .and_then(|x| x.call((), Some(&kwargs)))?;
        Ok(namespace)
    }
}
