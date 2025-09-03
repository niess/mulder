use crate::utils::coordinates::{GeographicCoordinates, LocalFrame, HorizontalCoordinates};
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::TypeError;
use crate::utils::extract::{Extractor, Field, Name};
use crate::utils::numpy::{ArrayMethods, Dtype, impl_dtype, NewArray, PyArray, ShapeArg};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyType};
use super::Particle;


#[pyclass(module="mulder", sequence)]
pub struct GeographicStates {
    pub array: GeographicStatesArray,
}

#[pyclass(module="mulder", sequence)]
pub struct LocalStates {
    /// The coordinates local frame.
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
    Flavoured(Py<PyArray<FlavouredLocalState>>),
    Unflavoured(Py<PyArray<UnflavouredLocalState>>),
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

    fn __len__(&self, py: Python) -> PyResult<usize> {
        let shape = match &self.array {
            GeographicStatesArray::Flavoured(array) => array.bind(py).shape(),
            GeographicStatesArray::Unflavoured(array) => array.bind(py).shape(),
        };
        shape
            .get(0)
            .copied()
            .ok_or_else(|| Error::new(TypeError).why("len() of unsized object").to_err())
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

    /// The geographic states' array dimension.
    #[getter]
    fn get_ndim(&self, py: Python) -> usize {
        match &self.array {
            GeographicStatesArray::Flavoured(array) => array.bind(py).ndim(),
            GeographicStatesArray::Unflavoured(array) => array.bind(py).ndim(),
        }
    }

    /// The geographic states' array shape.
    #[getter]
    fn get_shape(&self, py: Python) -> Vec<usize> {
        match &self.array {
            GeographicStatesArray::Flavoured(array) => array.bind(py).shape(),
            GeographicStatesArray::Unflavoured(array) => array.bind(py).shape(),
        }
    }

    /// The total number of geographic states.
    #[getter]
    fn get_size(&self, py: Python) -> usize {
        match &self.array {
            GeographicStatesArray::Flavoured(array) => array.bind(py).size(),
            GeographicStatesArray::Unflavoured(array) => array.bind(py).size(),
        }
    }

    /// The PDG particle identifier.
    #[getter]
    fn get_pid<'py>(&self, py: Python<'py>) -> PyResult<PyObject> {
        let pid = match &self.array {
            GeographicStatesArray::Flavoured(array) => {
                array.bind(py).as_any().get_item("pid")?.unbind()
            },
            GeographicStatesArray::Unflavoured(_) => py.None(),
        };
        Ok(pid)
    }

    /// The kinetic energy, in GeV.
    #[getter]
    fn get_energy<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.array.getattr(py, "energy")
    }

    /// The latitude coordinate, in deg.
    #[getter]
    fn get_latitude<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.array.getattr(py, "latitude")
    }

    /// The longitude coordinate, in deg.
    #[getter]
    fn get_longitude<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.array.getattr(py, "longitude")
    }

    /// The altitude coordinate, in m.
    #[getter]
    fn get_altitude<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.array.getattr(py, "altitude")
    }

    /// The azimuth angle of observation, in deg.
    #[getter]
    fn get_azimuth<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.array.getattr(py, "azimuth")
    }

    /// The elevation angle of observation, in deg.
    #[getter]
    fn get_elevation<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.array.getattr(py, "elevation")
    }

    /// The Monte Carlo weight.
    #[getter]
    fn get_weight<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.array.getattr(py, "weight")
    }

    /// Returns uninitialised geographic states.
    #[classmethod]
    #[pyo3(signature=(shape=None, /, *, with_pid=false))]
    fn empty(
        cls: &Bound<PyType>,
        shape: Option<ShapeArg>,
        with_pid: Option<bool>,
    ) -> PyResult<Self> {
        new_array!(Geographic, empty, cls, shape, with_pid)
    }

    /// Creates geographic states from a Numpy array.
    #[classmethod]
    #[pyo3(signature=(array, /))]
    fn from_array(_cls: &Bound<PyType>, array: GeographicStatesArray) -> Self {
        Self { array }
    }

    /// Returns a collection of identical geographic states.
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

    /// Creates geographic states from local ones.
    #[classmethod]
    #[pyo3(name="from_local", signature=(states, /))]
    fn py_from_local(cls: &Bound<PyType>, states: &LocalStates) -> PyResult<Self> {
        Self::from_local(cls.py(), states)
    }

    /// Returns zeroed geographic states.
    #[classmethod]
    #[pyo3(signature=(shape=None, /, *, with_pid=false))]
    fn zeros(
        cls: &Bound<PyType>,
        shape: Option<ShapeArg>,
        with_pid: Option<bool>,
    ) -> PyResult<Self> {
        new_array!(Geographic, zeros, cls, shape, with_pid)
    }

    /// Converts the geographic states to local ones.
    #[pyo3(signature=(frame=None, /))]
    fn to_local(&self, py: Python, frame: Option<LocalFrame>) -> PyResult<LocalStates> {
        LocalStates::from_geographic(py, self, frame)
    }
}

macro_rules! convert_local {
    ($flav:ident, $py:ident, $local:ident, $frame:ident) => {
        {
            paste::paste! {
                let local = $local.bind($py);
                let mut array = NewArray::< [< $flav GeographicState >] >::empty(
                    $py, local.shape()
                )?;
                let size = array.size();
                let data = array.as_slice_mut();
                for i in 0..size {
                    data[i] = [< $flav GeographicState >] ::from_local(
                        local.get_item(i)?,
                        $frame,
                    );
                }
                GeographicStatesArray::$flav(array.into_bound().unbind())
            }
        }
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
                Field::maybe_float(Name::Energy),
                Field::maybe_float(Name::Latitude),
                Field::maybe_float(Name::Longitude),
                Field::maybe_float(Name::Altitude),
                Field::maybe_float(Name::Azimuth),
                Field::maybe_float(Name::Elevation),
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

    fn from_local(py: Python, local: &LocalStates) -> PyResult<Self> {
        let frame = &local.frame;
        let array = match &local.array {
            LocalStatesArray::Flavoured(local) => {
                convert_local!(Flavoured, py, local, frame)
            },
            LocalStatesArray::Unflavoured(local) => {
                convert_local!(Unflavoured, py, local, frame)
            },
        };
        Ok(Self { array })
    }
}

impl GeographicStatesArray {
    #[inline]
    fn getattr<'py>(&self, py: Python<'py>, field: &'static str) -> PyResult<Bound<'py, PyAny>> {
        match self {
            Self::Flavoured(array) => array.bind(py).as_any().get_item(field),
            Self::Unflavoured(array) => array.bind(py).as_any().get_item(field),
        }
    }

    #[inline]
    fn getitem<'py>(&self, arg: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        match self {
            Self::Flavoured(array) => array.bind(arg.py()).as_any().get_item(arg),
            Self::Unflavoured(array) => array.bind(arg.py()).as_any().get_item(arg),
        }
    }
}

impl FlavouredGeographicState {
    #[inline]
    fn direction(&self) -> HorizontalCoordinates {
        HorizontalCoordinates {
            azimuth: self.azimuth,
            elevation: self.elevation,
        }
    }

    #[inline]
    fn from_extractor(states: &Extractor<8>, index: usize) -> PyResult<Self> {
        let state = Self {
            pid: states.get_i32_opt(Name::Pid, index)?.unwrap_or(Particle::Muon.pid()),
            energy: states.get_f64_opt(Name::Energy, index)?.unwrap_or(1.0),
            latitude: states.get_f64_opt(Name::Latitude, index)?
                .unwrap_or(LocalFrame::DEFAULT_LATITUDE),
            longitude: states.get_f64_opt(Name::Longitude, index)?
                .unwrap_or(LocalFrame::DEFAULT_LONGITUDE),
            altitude: states.get_f64_opt(Name::Altitude, index)?.unwrap_or(0.0),
            azimuth: states.get_f64_opt(Name::Azimuth, index)?.unwrap_or(0.0),
            elevation: states.get_f64_opt(Name::Elevation, index)?.unwrap_or(0.0),
            weight: states.get_f64_opt(Name::Weight, index)?.unwrap_or(1.0),
        };
        Ok(state)
    }

    #[inline]
    fn from_local(state: FlavouredLocalState, frame: &LocalFrame) -> Self {
        let (position, direction) = frame.to_geographic(&state.position, &state.direction);
        Self {
            pid: state.pid,
            energy: state.energy,
            latitude: position.latitude,
            longitude: position.longitude,
            altitude: position.altitude,
            azimuth: direction.azimuth,
            elevation: direction.elevation,
            weight: state.weight,
        }
    }

    #[inline]
    fn position(&self) -> GeographicCoordinates {
        GeographicCoordinates {
            latitude: self.latitude,
            longitude: self.longitude,
            altitude: self.altitude,
        }
    }
}

impl UnflavouredGeographicState {
    #[inline]
    fn direction(&self) -> HorizontalCoordinates {
        HorizontalCoordinates {
            azimuth: self.azimuth,
            elevation: self.elevation,
        }
    }

    #[inline]
    fn from_extractor(states: &Extractor<8>, index: usize) -> PyResult<Self> {
        let state = Self {
            energy: states.get_f64_opt(Name::Energy, index)?.unwrap_or(1.0),
            latitude: states.get_f64_opt(Name::Latitude, index)?
                .unwrap_or(LocalFrame::DEFAULT_LATITUDE),
            longitude: states.get_f64_opt(Name::Longitude, index)?
                .unwrap_or(LocalFrame::DEFAULT_LONGITUDE),
            altitude: states.get_f64_opt(Name::Altitude, index)?.unwrap_or(0.0),
            azimuth: states.get_f64_opt(Name::Azimuth, index)?.unwrap_or(0.0),
            elevation: states.get_f64_opt(Name::Elevation, index)?.unwrap_or(0.0),
            weight: states.get_f64_opt(Name::Weight, index)?.unwrap_or(1.0),
        };
        Ok(state)
    }

    #[inline]
    fn from_local(state: UnflavouredLocalState, frame: &LocalFrame) -> Self {
        let (position, direction) = frame.to_geographic(&state.position, &state.direction);
        Self {
            energy: state.energy,
            latitude: position.latitude,
            longitude: position.longitude,
            altitude: position.altitude,
            azimuth: direction.azimuth,
            elevation: direction.elevation,
            weight: state.weight,
        }
    }

    #[inline]
    fn position(&self) -> GeographicCoordinates {
        GeographicCoordinates {
            latitude: self.latitude,
            longitude: self.longitude,
            altitude: self.altitude,
        }
    }
}

macro_rules! new_array {
    ($name:ident, $func:ident, $cls:ident, $shape:ident, $frame:ident, $with_pid:ident) => {
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
            let frame = $frame.unwrap_or_else(|| LocalFrame::default());
            Ok(Self { array, frame })
        }
    }
}

#[pymethods]
impl LocalStates {
    #[pyo3(signature=(states=None, /, *, frame=None, **kwargs))]
    #[new]
    fn new(
        py: Python,
        states: Option<&Bound<PyAny>>,
        frame: Option<LocalFrame>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<Self> {
        let states = Self::extract_states(states, kwargs)?;
        let shape = states.shape();
        Self::from_extractor(py, shape, frame, states)
    }

    fn __getitem__<'py>(&self, arg: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let py = arg.py();

        let maybe_index: Option<ShapeArg> = arg.extract().ok();
        if let Some(index) = maybe_index {
            let index = index.into_vec();
            let n = index.len();
            let shape = self.get_shape(py);
            if shape.len() == n {
                let index = if n == 0 {
                    0
                } else if n == 1 {
                    index[0]
                } else {
                    let mut i = index[n - 1];
                    for j in 0..(n - 2) {
                        i = i * shape[n - 1 - j] + index[n - 1 - j];
                    }
                    i
                };
                return Ok(Bound::new(py, self.getitem(py, index)?)?.into_any())
            }
        }

        let result = self.array.getitem(arg)?;
        let maybe_array: Option<LocalStatesArray> = result.extract().ok();
        let result = match maybe_array {
            Some(array) => Bound::new(py, Self { array, frame: self.frame.clone() })?.into_any(),
            None => result,
        };
        Ok(result)
    }

    fn __len__(&self, py: Python) -> PyResult<usize> {
        let shape = match &self.array {
            LocalStatesArray::Flavoured(array) => array.bind(py).shape(),
            LocalStatesArray::Unflavoured(array) => array.bind(py).shape(),
        };
        shape
            .get(0)
            .copied()
            .ok_or_else(|| Error::new(TypeError).why("len() of unsized object").to_err())
    }

    fn __repr__(&self) -> String {
        match &self.array {
            LocalStatesArray::Flavoured(array) => format!(
                "LocalStates({})",
                array,
            ),
            LocalStatesArray::Unflavoured(array) => format!(
                "LocalStates({})",
                array,
            ),
        }
    }

    /// The underlying NumPy array.
    #[getter]
    fn get_array(&self, py: Python) -> PyObject {
        match &self.array {
            LocalStatesArray::Flavoured(array) => array.clone_ref(py).into_any(),
            LocalStatesArray::Unflavoured(array) => array.clone_ref(py).into_any(),
        }
    }

    /// The local states' array dimension.
    #[getter]
    fn get_ndim(&self, py: Python) -> usize {
        match &self.array {
            LocalStatesArray::Flavoured(array) => array.bind(py).ndim(),
            LocalStatesArray::Unflavoured(array) => array.bind(py).ndim(),
        }
    }

    /// The local states' array shape.
    #[getter]
    fn get_shape(&self, py: Python) -> Vec<usize> {
        match &self.array {
            LocalStatesArray::Flavoured(array) => array.bind(py).shape(),
            LocalStatesArray::Unflavoured(array) => array.bind(py).shape(),
        }
    }

    /// The total number of local states.
    #[getter]
    fn get_size(&self, py: Python) -> usize {
        match &self.array {
            LocalStatesArray::Flavoured(array) => array.bind(py).size(),
            LocalStatesArray::Unflavoured(array) => array.bind(py).size(),
        }
    }

    /// The PDG particle identifier.
    #[getter]
    fn get_pid<'py>(&self, py: Python<'py>) -> PyResult<PyObject> {
        let pid = match &self.array {
            LocalStatesArray::Flavoured(array) => {
                array.bind(py).as_any().get_item("pid")?.unbind()
            },
            LocalStatesArray::Unflavoured(_) => py.None(),
        };
        Ok(pid)
    }

    /// The kinetic energy, in GeV.
    #[getter]
    fn get_energy<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.array.getattr(py, "energy")
    }

    /// The local position, in m.
    #[getter]
    fn get_position<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.array.getattr(py, "position")
    }

    /// The observation's local direction.
    #[getter]
    fn get_direction<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.array.getattr(py, "direction")
    }

    /// The Monte Carlo weight.
    #[getter]
    fn get_weight<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.array.getattr(py, "weight")
    }

    /// Returns uninitialised local states.
    #[classmethod]
    #[pyo3(signature=(shape=None, /, *, frame=None, with_pid=false))]
    fn empty(
        cls: &Bound<PyType>,
        shape: Option<ShapeArg>,
        frame: Option<LocalFrame>,
        with_pid: Option<bool>,
    ) -> PyResult<Self> {
        new_array!(Local, empty, cls, shape, frame, with_pid)
    }

    /// Creates local states from a Numpy array.
    #[classmethod]
    #[pyo3(signature=(array, /, *, frame=None))]
    fn from_array(
        _cls: &Bound<PyType>,
        array: LocalStatesArray,
        frame: Option<LocalFrame>,
    ) -> Self {
        let frame = frame.unwrap_or_else(|| LocalFrame::default());
        Self { array, frame }
    }

    /// Creates local states from geographic ones.
    #[classmethod]
    #[pyo3(name="from_geographic", signature=(states, /, *, frame=None))]
    fn py_from_geographic(
        cls: &Bound<PyType>,
        states: &GeographicStates,
        frame: Option<LocalFrame>,
    ) -> PyResult<Self> {
        Self::from_geographic(cls.py(), states, frame)
    }

    /// Returns a collection of identical local states.
    #[classmethod]
    #[pyo3(signature=(shape=None, /, fill_value=None, *, frame=None, **kwargs))]
    fn full(
        cls: &Bound<PyType>,
        shape: Option<ShapeArg>,
        fill_value: Option<&Bound<PyAny>>,
        frame: Option<LocalFrame>,
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

        Self::from_extractor(py, shape, frame, states)
    }

    /// Returns zeroed local states.
    #[classmethod]
    #[pyo3(signature=(shape=None, /, *, frame=None, with_pid=false))]
    fn zeros(
        cls: &Bound<PyType>,
        shape: Option<ShapeArg>,
        frame: Option<LocalFrame>,
        with_pid: Option<bool>,
    ) -> PyResult<Self> {
        new_array!(Local, zeros, cls, shape, frame, with_pid)
    }

    /// Converts the local states to geographic ones.
    fn to_geographic(&self, py: Python) -> PyResult<GeographicStates> {
        GeographicStates::from_local(py, self)
    }
}

macro_rules! convert_geographic {
    ($flav:ident, $py:ident, $geo:ident, $frame:ident) => {
        {
            paste::paste! {
                let geographic = $geo.bind($py);
                let mut array = NewArray::< [< $flav LocalState >] >::empty(
                    $py, geographic.shape()
                )?;
                let size = array.size();
                let data = array.as_slice_mut();
                for i in 0..size {
                    data[i] = [< $flav LocalState >] ::from_geographic(
                        geographic.get_item(i)?,
                        & $frame,
                    );
                }
                LocalStatesArray::$flav(array.into_bound().unbind())
            }
        }
    }
}

impl LocalStates {
    fn extract_states<'py>(
        states: Option<&Bound<'py, PyAny>>,
        kwargs: Option<&Bound<'py, PyDict>>,
    ) -> PyResult<Extractor<'py, 5>> {
        Extractor::from_args(
            [
                Field::maybe_int(Name::Pid),
                Field::maybe_float(Name::Energy),
                Field::maybe_vec3(Name::Position),
                Field::maybe_vec3(Name::Direction),
                Field::maybe_float(Name::Weight),
            ],
            states,
            kwargs,
        )
    }

    fn from_extractor(
        py: Python,
        shape: Vec<usize>,
        frame: Option<LocalFrame>,
        states: Extractor<5>,
    ) -> PyResult<LocalStates> {
        let array = if states.contains(Name::Pid) {
            let mut array = NewArray::<FlavouredLocalState>::empty(py, shape)?;
            let size = array.size();
            let data = array.as_slice_mut();
            for i in 0..size {
                data[i] = FlavouredLocalState::from_extractor(&states, i)?;
            }
            LocalStatesArray::Flavoured(array.into_bound().unbind())
        } else {
            let mut array = NewArray::<UnflavouredLocalState>::empty(py, shape)?;
            let size = array.size();
            let data = array.as_slice_mut();
            for i in 0..size {
                data[i] = UnflavouredLocalState::from_extractor(&states, i)?;
            }
            LocalStatesArray::Unflavoured(array.into_bound().unbind())
        };
        let frame = frame.unwrap_or_else(|| LocalFrame::default());
        Ok(Self { array, frame})
    }

    fn from_geographic(
        py: Python,
        geographic: &GeographicStates,
        frame: Option<LocalFrame>,
    ) -> PyResult<Self> {
        let frame = frame.unwrap_or_else(|| LocalFrame::default());
        let array = match &geographic.array {
            GeographicStatesArray::Flavoured(geographic) => {
                convert_geographic!(Flavoured, py, geographic, frame)
            },
            GeographicStatesArray::Unflavoured(geographic) => {
                convert_geographic!(Unflavoured, py, geographic, frame)
            },
        };
        Ok(Self { array, frame })
    }

    fn getitem( // XXX implement for geographic states as well, & iterators.
        &self,
        py: Python,
        index: usize,
    ) -> PyResult<Self> {
        let array = match &self.array {
            LocalStatesArray::Flavoured(array) => {
                let state = array.bind(py).get_item(index)?;
                let mut array = NewArray::<FlavouredLocalState>::empty(py, [])?;
                array.as_slice_mut()[0] = state;
                LocalStatesArray::Flavoured(array.into_bound().unbind())
            },
            LocalStatesArray::Unflavoured(array) => {
                let state = array.bind(py).get_item(index)?;
                let mut array = NewArray::<UnflavouredLocalState>::empty(py, [])?;
                array.as_slice_mut()[0] = state;
                LocalStatesArray::Unflavoured(array.into_bound().unbind())
            },
        };
        Ok(Self { array, frame: self.frame.clone() })
    }
}

impl LocalStatesArray {
    #[inline]
    fn getattr<'py>(&self, py: Python<'py>, field: &'static str) -> PyResult<Bound<'py, PyAny>> {
        match self {
            Self::Flavoured(array) => array.bind(py).as_any().get_item(field),
            Self::Unflavoured(array) => array.bind(py).as_any().get_item(field),
        }
    }

    #[inline]
    fn getitem<'py>(&self, arg: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        match self {
            Self::Flavoured(array) => array.bind(arg.py()).as_any().get_item(arg),
            Self::Unflavoured(array) => array.bind(arg.py()).as_any().get_item(arg),
        }
    }
}

impl FlavouredLocalState {
    #[inline]
    fn from_extractor(states: &Extractor<5>, index: usize) -> PyResult<Self> {
        let state = Self {
            pid: states.get_i32_opt(Name::Pid, index)?.unwrap_or(Particle::Muon.pid()),
            energy: states.get_f64_opt(Name::Energy, index)?.unwrap_or(1.0),
            position: states.get_vec3_opt(Name::Position, index)?
                .unwrap_or_else(|| [0.0; 3]),
            direction: states.get_vec3_opt(Name::Direction, index)?
                .unwrap_or_else(|| [0.0, 0.0, 1.0]),
            weight: states.get_f64_opt(Name::Weight, index)?.unwrap_or(1.0),
        };
        Ok(state)
    }

    #[inline]
    fn from_geographic(state: FlavouredGeographicState, frame: &LocalFrame) -> Self {
        let (position, direction) = frame.from_geographic(state.position(), state.direction());
        Self {
            pid: state.pid,
            energy: state.energy,
            position,
            direction,
            weight: state.weight,
        }
    }
}

impl UnflavouredLocalState {
    #[inline]
    fn from_extractor(states: &Extractor<5>, index: usize) -> PyResult<Self> {
        let state = Self {
            energy: states.get_f64_opt(Name::Energy, index)?.unwrap_or(1.0),
            position: states.get_vec3_opt(Name::Position, index)?
                .unwrap_or_else(|| [0.0; 3]),
            direction: states.get_vec3_opt(Name::Direction, index)?
                .unwrap_or_else(|| [0.0, 0.0, 1.0]),
            weight: states.get_f64_opt(Name::Weight, index)?.unwrap_or(1.0),
        };
        Ok(state)
    }

    #[inline]
    fn from_geographic(state: UnflavouredGeographicState, frame: &LocalFrame) -> Self {
        let (position, direction) = frame.from_geographic(state.position(), state.direction());
        Self {
            energy: state.energy,
            position,
            direction,
            weight: state.weight,
        }
    }
}
