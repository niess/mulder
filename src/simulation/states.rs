use crate::utils::coordinates::LocalFrame;
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::TypeError;
use crate::utils::extract::{Extractor, Field, Name};
use crate::utils::numpy::{Dtype, impl_dtype, NewArray, PyArray, ShapeArg};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyType};
use super::Particle;


#[pyclass(module="mulder", sequence)]
pub struct GeographicStates {
    pub array: GeographicStatesArray,
}

#[pyclass(module="mulder", sequence)]
pub struct LocalStates {
    /// The coordionates local frame.
    #[pyo3(get)]
    pub frame: LocalFrame,

    pub array: LocalStatesArray,
}

#[derive(Debug, FromPyObject, IntoPyObject)]
pub enum GeographicStatesArray {
    Flavoured(Py<PyArray<FlavouredGeographicState>>),
    Unflavoured(Py<PyArray<UnflavouredGeographicState>>),
}

#[repr(C)]
#[derive(Clone, Debug, Default)]
pub struct FlavouredGeographicState {
    pid: i32,
    energy: f64,
    latitude: f64,
    longitude: f64,
    altitude: f64,
    azimuth: f64,
    elevation: f64,
    weight: f64,
}

#[repr(C)]
#[derive(Clone, Debug, Default)]
pub struct UnflavouredGeographicState {
    energy: f64,
    latitude: f64,
    longitude: f64,
    altitude: f64,
    azimuth: f64,
    elevation: f64,
    weight: f64,
}

#[derive(Debug, FromPyObject, IntoPyObject)]
pub enum LocalStatesArray {
    Flavoured(Py<PyArray<FlavouredGeographicState>>),
    Unflavoured(Py<PyArray<UnflavouredGeographicState>>),
}

#[repr(C)]
#[derive(Clone, Debug, Default)]
pub struct FlavouredLocalState {
    pid: i32,
    energy: f64,
    position: [f64; 3],
    direction: [f64; 3],
    weight: f64,
}

#[repr(C)]
#[derive(Clone, Debug, Default)]
pub struct UnflavouredLocalState {
    energy: f64,
    position: [f64; 3],
    direction: [f64; 3],
    weight: f64,
}

impl_dtype!(
    FlavouredGeographicState,
    [
        ("pid",       "i4"),
        ("energy",    "f8"),
        ("latitude",  "f8"),
        ("longitude", "f8"),
        ("altitude",  "f8"),
        ("azimuth",   "f8"),
        ("elevation", "f8"),
        ("weight",    "f8"),
    ]
);

impl_dtype!(
    UnflavouredGeographicState,
    [
        ("energy",    "f8"),
        ("latitude",  "f8"),
        ("longitude", "f8"),
        ("altitude",  "f8"),
        ("azimuth",   "f8"),
        ("elevation", "f8"),
        ("weight",    "f8"),
    ]
);

impl_dtype!(
    FlavouredLocalState,
    [
        ("pid",       "i4"),
        ("energy",    "f8"),
        ("position",  "3f8"),
        ("direction", "3f8"),
        ("weight",    "f8"),
    ]
);

impl_dtype!(
    UnflavouredLocalState,
    [
        ("energy",    "f8"),
        ("position",  "3f8"),
        ("direction", "3f8"),
        ("weight",    "f8"),
    ]
);

macro_rules! new_array {
    ($name:ident, $func:ident, $cls:ident, $shape:ident, $with_pid:ident) => {
        {
            let py = $cls.py();
            let shape = $shape
                .map(|shape| shape.into_vec())
                .unwrap_or_else(|| Vec::new());
            let with_pid = $with_pid.unwrap_or(false);
            paste::paste! {
                let array = if with_pid {
                    [<  $name StatesArray >] ::Flavoured (
                        NewArray::< [< Flavoured $name State >] >::$func(py, shape)?
                            .into_bound().unbind()
                    )
                } else {
                    [< $name StatesArray >] ::Unflavoured (
                        NewArray::< [< Unflavoured $name State >] >::$func(py, shape)?
                            .into_bound().unbind()
                    )
                };
            }
            Ok(Self { array })
        }
    }
}

#[pymethods]
impl GeographicStates {
    #[pyo3(signature=(states=None, /, **kwargs))]
    #[new]
    fn new(
        py: Python,
        states: Option<&Bound<PyAny>>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<Self> {
        if states.is_none() && kwargs.is_none() {
            let array = NewArray::<UnflavouredGeographicState>::empty(py, [])?;
            let array = GeographicStatesArray::Unflavoured(array.into_bound().unbind());
            return Ok(Self { array })
        }

        let states = Self::extract_states(states, kwargs)?;
        let shape = states.shape();

        Self::from_extractor(py, shape, states)
    }

    fn __getitem__<'py>(&self, arg: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let py = arg.py();
        let result = self.array.getitem(arg)?;
        let maybe_array: Option<GeographicStatesArray> = result.extract().ok();
        let result = match maybe_array {
            Some(array) => Bound::new(py, Self { array })?.into_any(),
            None => result,
        };
        Ok(result)
    }

    fn __repr__(&self) -> String {
        match &self.array {
            GeographicStatesArray::Flavoured(array) => format!(
                "GeographicStates({})",
                array
            ),
            GeographicStatesArray::Unflavoured(array) => format!(
                "GeographicStates({})",
                array
            ),
        }
    }

    /// The underlying NumPy array.
    #[getter]
    fn get_array(&self, py: Python) -> PyObject {
        match &self.array {
            GeographicStatesArray::Flavoured(array) => array.clone_ref(py).into_any(),
            GeographicStatesArray::Unflavoured(array) => array.clone_ref(py).into_any(),
        }
    }

    #[getter]
    fn get_pid<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.array.getattr(py, "pid")
    }

    #[getter]
    fn get_energy<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.array.getattr(py, "energy")
    }

    #[getter]
    fn get_latitude<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.array.getattr(py, "latitude")
    }

    #[getter]
    fn get_longitude<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.array.getattr(py, "longitude")
    }

    #[getter]
    fn get_altitude<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.array.getattr(py, "altitude")
    }

    #[getter]
    fn get_azimuth<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.array.getattr(py, "azimuth")
    }

    #[getter]
    fn get_elevation<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.array.getattr(py, "elevation")
    }

    #[getter]
    fn get_weight<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.array.getattr(py, "weight")
    }

    #[classmethod]
    #[pyo3(signature=(shape=None, /, *, with_pid=false))]
    fn empty(
        cls: &Bound<PyType>,
        shape: Option<ShapeArg>,
        with_pid: Option<bool>,
    ) -> PyResult<Self> {
        new_array!(Geographic, empty, cls, shape, with_pid)
    }

    #[classmethod]
    #[pyo3(signature=(array, /))]
    fn from_array(_cls: &Bound<PyType>, array: GeographicStatesArray) -> Self {
        Self { array }
    }

    #[classmethod]
    #[pyo3(signature=(shape=None, /, fill_value=None, **kwargs))]
    fn full(
        cls: &Bound<PyType>,
        shape: Option<ShapeArg>,
        fill_value: Option<&Bound<PyAny>>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<Self> {
        let py = cls.py();

        let states = Self::extract_states(fill_value, kwargs)?;
        let size = states.size();
        if size != 1 {
            let why = format!("expected a scalar, found a size {} array", size);
            let err = Error::new(TypeError).what("fill_value").why(&why).to_err();
            return Err(err)
        }

        let shape = shape
            .map(|shape| shape.into_vec())
            .unwrap_or_else(|| Vec::new());

        Self::from_extractor(py, shape, states)
    }

    #[classmethod]
    #[pyo3(signature=(shape=None, /, *, with_pid=false))]
    fn zeros(
        cls: &Bound<PyType>,
        shape: Option<ShapeArg>,
        with_pid: Option<bool>,
    ) -> PyResult<Self> {
        new_array!(Geographic, zeros, cls, shape, with_pid)
    }
}

impl GeographicStates {
    fn extract_states<'py>(
        states: Option<&Bound<'py, PyAny>>,
        kwargs: Option<&Bound<'py, PyDict>>,
    ) -> PyResult<Extractor<'py, 8>> {
        Extractor::from_args(
            [
                Field::maybe_int(Name::Pid),
                Field::float(Name::Energy),
                Field::float(Name::Latitude),
                Field::float(Name::Longitude),
                Field::float(Name::Altitude),
                Field::float(Name::Azimuth),
                Field::float(Name::Elevation),
                Field::maybe_float(Name::Weight),
            ],
            states,
            kwargs,
        )
    }

    fn from_extractor(
        py: Python,
        shape: Vec<usize>,
        states: Extractor<8>,
    ) -> PyResult<GeographicStates> {
        let array = if states.contains(Name::Pid) {
            let mut array = NewArray::<FlavouredGeographicState>::empty(py, shape)?;
            let size = array.size();
            let data = array.as_slice_mut();
            for i in 0..size {
                data[i] = FlavouredGeographicState::from_extractor(&states, i)?;
            }
            GeographicStatesArray::Flavoured(array.into_bound().unbind())
        } else {
            let mut array = NewArray::<UnflavouredGeographicState>::empty(py, shape)?;
            let size = array.size();
            let data = array.as_slice_mut();
            for i in 0..size {
                data[i] = UnflavouredGeographicState::from_extractor(&states, i)?;
            }
            GeographicStatesArray::Unflavoured(array.into_bound().unbind())
        };
        Ok(Self { array })
    }
}

impl GeographicStatesArray {
    #[inline]
    fn getattr<'py>(&self, py: Python<'py>, field: &'static str) -> PyResult<Bound<'py, PyAny>> {
        match self {
            Self::Flavoured(array) => array.bind(py).get_item(field),
            Self::Unflavoured(array) => array.bind(py).get_item(field),
        }
    }

    #[inline]
    fn getitem<'py>(&self, arg: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        match self {
            Self::Flavoured(array) => array.bind(arg.py()).get_item(arg),
            Self::Unflavoured(array) => array.bind(arg.py()).get_item(arg),
        }
    }
}

impl FlavouredGeographicState {
    #[inline]
    fn from_extractor(states: &Extractor<8>, index: usize) -> PyResult<Self> {
        let state = Self {
            pid: states.get_i32_opt(Name::Pid, index)?.unwrap_or(Particle::Muon.pid()),
            energy: states.get_f64(Name::Energy, index)?,
            latitude: states.get_f64(Name::Latitude, index)?,
            longitude: states.get_f64(Name::Longitude, index)?,
            altitude: states.get_f64(Name::Altitude, index)?,
            azimuth: states.get_f64(Name::Azimuth, index)?,
            elevation: states.get_f64(Name::Elevation, index)?,
            weight: states.get_f64_opt(Name::Weight, index)?.unwrap_or(1.0),
        };
        Ok(state)
    }
}

impl UnflavouredGeographicState {
    #[inline]
    fn from_extractor(states: &Extractor<8>, index: usize) -> PyResult<Self> {
        let state = Self {
            energy: states.get_f64(Name::Energy, index)?,
            latitude: states.get_f64(Name::Latitude, index)?,
            longitude: states.get_f64(Name::Longitude, index)?,
            altitude: states.get_f64(Name::Altitude, index)?,
            azimuth: states.get_f64(Name::Azimuth, index)?,
            elevation: states.get_f64(Name::Elevation, index)?,
            weight: states.get_f64_opt(Name::Weight, index)?.unwrap_or(1.0),
        };
        Ok(state)
    }
}
