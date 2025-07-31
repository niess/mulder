use pyo3::prelude::*;
use pyo3::types::PyDict;


#[inline]
pub fn default_palette(py: Python) -> PyResult<PyObject> {
    let palette = PyDict::new(py);
    palette.set_item("Rock", Colour {
        rgb: (139, 69, 19),
        specularity: 0.5,
    })?;
    palette.set_item("Water", Colour {
        rgb: (212, 241, 249),
        specularity: 0.8,
    })?;
    let palette = palette.into_any().unbind();
    Ok(palette)
}

#[pyclass(module="mulder")]
#[derive(Clone)]
pub struct Colour {
    #[pyo3(get, set)]
    pub rgb: (u8, u8, u8),

    #[pyo3(get)]
    pub specularity: f64,
}
