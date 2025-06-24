use crate::bindings::turtle;
use crate::utils::coordinates::{GeographicCoordinates, HorizontalCoordinates};
use crate::utils::error::{self, Error};
use crate::utils::error::ErrorKind::IndexError;
use crate::utils::extract::Extractor;
use crate::utils::numpy::{Dtype, NewArray};
use crate::utils::traits::MinMax;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use pyo3::sync::GILOnceCell;
use std::ffi::c_int;
use std::ptr::{null, null_mut};

pub mod atmosphere;
pub mod grid;
pub mod layer;

use atmosphere::{Atmosphere, AtmosphereLike};
use layer::{DataLike, Layer};


#[pyclass(module="mulder")]
pub struct Geometry {
    /// The geometry atmosphere.
    #[pyo3(get)]
    pub atmosphere: Py<Atmosphere>,

    /// Geometry limits along the z-coordinates.
    #[pyo3(get)]
    pub z: (f64, f64),

    pub layers: Vec<Py<Layer>>,
    pub stepper: *mut turtle::Stepper,
}

unsafe impl Send for Geometry {}
unsafe impl Sync for Geometry {}

#[derive(FromPyObject)]
pub enum AtmosphereArg<'py> {
    Model(AtmosphereLike<'py>),
    Object(Py<Atmosphere>),
}

#[derive(FromPyObject)]
enum LayerLike<'py> {
    Layer(Py<Layer>),
    OneData(DataLike<'py>),
    ManyData(Vec<DataLike<'py>>),
}

#[repr(C)]
struct Intersection {
    before: i32,
    after: i32,
    latitude: f64,
    longitude: f64,
    altitude: f64,
    distance: f64,
}

pub struct GeometryStepper {
    pub stepper: *mut turtle::Stepper,
    pub zlim: f64,
}

#[derive(Clone, Copy, Default)]
pub struct Doublet<T> {
    pub layers: T,
    pub opensky: T,
}

#[pymethods]
impl Geometry {
    #[pyo3(signature=(*layers, atmosphere=None))]
    #[new]
    pub fn new(layers: &Bound<PyTuple>, atmosphere: Option<AtmosphereArg>) -> PyResult<Self> {
        let py = layers.py();
        let (layers, z) = {
            let mut z = (f64::INFINITY, -f64::INFINITY);
            let mut v = Vec::with_capacity(layers.len());
            for layer in layers.iter() {
                let layer: LayerLike = layer.extract()?;
                let layer = match layer {
                    LayerLike::Layer(layer) => layer,
                    LayerLike::OneData(data) => {
                        let data = vec![data.into_data(py)?];
                        let layer = Layer::new(py, data, None, None)?;
                        Py::new(py, layer)?
                    },
                    LayerLike::ManyData(data) => {
                        let data: PyResult<Vec<_>> = data.into_iter()
                            .map(|data| data.into_data(py))
                            .collect();
                        let layer = Layer::new(py, data?, None, None)?;
                        Py::new(py, layer)?
                    },
                };
                let lz = layer.bind(py).borrow().z;
                if lz.min() < z.min() { *z.mut_min() = lz.min(); }
                if lz.max() > z.max() { *z.mut_max() = lz.max(); }
                v.push(layer)
            }
            (v, z)
        };

        let atmosphere = match atmosphere {
            Some(atmosphere) => match atmosphere {
                AtmosphereArg::Model(model) => {
                    let atmosphere = Atmosphere::new(Some(model))?;
                    Py::new(py, atmosphere)?
                },
                AtmosphereArg::Object(atmosphere) => atmosphere,
            },
            None => {
                let atmosphere = Atmosphere::new(None)?;
                Py::new(py, atmosphere)?
            },
        };

        let stepper = null_mut();

        Ok(Self { layers, z, atmosphere, stepper })
    }

    /// The geometry layers.
    #[getter]
    fn get_layers<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        let elements = self.layers
            .iter()
            .map(|data| data.clone_ref(py));
        PyTuple::new(py, elements)
    }

    fn __getitem__(&self, py: Python, index: usize) -> PyResult<Py<Layer>> {
        self.layers
            .get(index)
            .map(|layer| layer.clone_ref(py))
            .ok_or_else(|| {
                let why = format!(
                    "expected a value in [0, {}], found '{}'",
                    self.layers.len() - 1,
                    index,
                );
                Error::new(IndexError)
                    .what("layer index")
                    .why(&why)
                    .to_err()
            })
    }

    #[pyo3(signature=(position=None, /, **kwargs))]
    fn locate<'py>(
        &mut self,
        py: Python<'py>,
        position: Option<&Bound<PyAny>>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<NewArray<'py, i32>> {
        let position = Extractor::from_args(
            ["latitude", "longitude", "altitude"],
            position,
            kwargs,
        )?;

        self.ensure_stepper(py)?;
        let mut array = NewArray::empty(py, position.shape())?;
        let layer = array.as_slice_mut();
        for i in 0..position.size() {
            let [ latitude, longitude, altitude ] = position.get(i)?;
            let geographic = GeographicCoordinates { latitude, longitude, altitude };
            let mut r = geographic.to_ecef();
            let mut index = [ -2; 2 ];
            error::to_result(
                unsafe {
                    turtle::stepper_step(
                        self.stepper,
                        r.as_mut_ptr(),
                        null(),
                        null_mut(),
                        null_mut(),
                        null_mut(),
                        null_mut(),
                        null_mut(),
                        index.as_mut_ptr(),
                    )
                },
                None::<&str>,
            )?;
            layer[i] = layer_index(index[0]);
        }
        Ok(array)
    }

    #[pyo3(signature=(coordinates=None, /, *, **kwargs))]
    fn scan<'py>(
        &self,
        py: Python<'py>,
        coordinates: Option<&Bound<PyAny>>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<NewArray<'py, f64>> {
        let coordinates = Extractor::from_args(
            ["latitude", "longitude", "altitude", "azimuth", "elevation"],
            coordinates,
            kwargs
        )?;
        let (size, shape, n) = {
            let size = coordinates.size();
            let mut shape = coordinates.shape();
            let n = self.layers.len();
            shape.push(n);
            (size, shape, n)
        };

        let mut array = NewArray::<f64>::zeros(py, shape)?;
        let distances = array.as_slice_mut();
        for i in 0..size {
            // Get the starting point.
            let [latitude, longitude, altitude, azimuth, elevation] = coordinates.get(i)?;
            let geographic = GeographicCoordinates { latitude, longitude, altitude };
            let mut r = geographic.to_ecef();
            let mut index = [ -2; 2 ];
            error::to_result(
                unsafe {
                    turtle::stepper_step(
                        self.stepper,
                        r.as_mut_ptr(),
                        null(),
                        null_mut(),
                        null_mut(),
                        null_mut(),
                        null_mut(),
                        null_mut(),
                        index.as_mut_ptr(),
                    )
                },
                None::<&str>,
            )?;

            // Iterate until the particle exits.
            let horizontal = HorizontalCoordinates { azimuth, elevation };
            let u = horizontal.to_ecef(&geographic);
            while (index[0] >= 1) && (index[0] as usize <= n + 1) {
                let current = index[0];
                let mut di = 0.0;
                while index[0] == current {
                    let mut step: f64 = 0.0;
                    error::to_result(
                        unsafe {
                            turtle::stepper_step(
                                self.stepper,
                                r.as_mut_ptr(),
                                u.as_ptr(),
                                null_mut(),
                                null_mut(),
                                null_mut(),
                                null_mut(),
                                &mut step,
                                index.as_mut_ptr(),
                            )
                        },
                        None::<&str>,
                    )?;
                    di += step;
                }

                let current = current as usize;
                if current <= n {
                    distances[i * n + current - 1]+= di;
                }

                // Push the particle through the boundary.
                const EPS: f64 = f32::EPSILON as f64;
                for i in 0..3 {
                    r[i] += EPS * u[i];
                }
            }
        }

        Ok(array)
    }

    #[pyo3(signature=(coordinates=None, /, **kwargs))]
    fn trace<'py>(
        &mut self,
        py: Python<'py>,
        coordinates: Option<&Bound<PyAny>>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<NewArray<'py, Intersection>> {
        let coordinates = Extractor::from_args(
            ["latitude", "longitude", "altitude", "azimuth", "elevation"],
            coordinates,
            kwargs
        )?;
        let size = coordinates.size();
        let shape = coordinates.shape();

        self.ensure_stepper(py)?;
        let mut array = NewArray::empty(py, shape)?;
        let intersections = array.as_slice_mut();
        for i in 0..size {
            let [latitude, longitude, altitude, azimuth, elevation] = coordinates.get(i)?;
            let geographic = GeographicCoordinates { latitude, longitude, altitude };
            let mut r = geographic.to_ecef();
            let mut index = [ -2; 2 ];
            error::to_result(
                unsafe {
                    turtle::stepper_step(
                        self.stepper,
                        r.as_mut_ptr(),
                        null(),
                        null_mut(),
                        null_mut(),
                        null_mut(),
                        null_mut(),
                        null_mut(),
                        index.as_mut_ptr(),
                    )
                },
                None::<&str>,
            )?;
            let start_layer = index[0];
            let mut di = 0.0;
            let position = if (start_layer >= 1) &&
                              (start_layer as usize <= self.layers.len() + 1) {

                // Iterate until a boundary is hit.
                let horizontal = HorizontalCoordinates { azimuth, elevation };
                let u = horizontal.to_ecef(&geographic);
                let mut step = 0.0_f64;
                while index[0] == start_layer {
                    error::to_result(
                        unsafe {
                            turtle::stepper_step(
                                self.stepper,
                                r.as_mut_ptr(),
                                u.as_ptr(),
                                null_mut(),
                                null_mut(),
                                null_mut(),
                                null_mut(),
                                &mut step,
                                index.as_mut_ptr(),
                            )
                        },
                        None::<&str>,
                    )?;
                    di += step;
                }

                // Push the particle through the boundary.
                const EPS: f64 = f32::EPSILON as f64;
                di += EPS;
                for i in 0..3 {
                    r[i] += EPS * u[i];
                }
                GeographicCoordinates::from_ecef(&r)
            } else {
                geographic.clone()
            };
            intersections[i] = Intersection {
                before: layer_index(start_layer),
                after: layer_index(index[0]),
                latitude: position.latitude,
                longitude: position.longitude,
                altitude: position.altitude,
                distance: di,
            };
        }
        Ok(array)
    }
}

#[inline]
fn layer_index(stepper_index: c_int) -> c_int {
    if stepper_index >= 1 {
        stepper_index - 1
    } else {
        stepper_index
    }
}

impl Geometry {
    // Height of the bottom layer, in m.
    const ZMIN: f64 = -11E+03;

    // Top most height, in m.
    const ZMAX: f64 = 120E+03;


    pub fn create_steppers(
        &self,
        py: Python,
        zref: Option<(f64, f64)>,
    ) -> PyResult<Doublet<GeometryStepper>> {
        let zlim = zref.map(|zref| {
            if self.z.max() <= zref.min() {
                Doublet { layers: zref.min(), opensky: zref.min() }
            } else if self.z.max() <= zref.max() {
                Doublet { layers: self.z.max(), opensky: self.z.max() }
            } else {
                Doublet { layers: self.z.max(), opensky: zref.max() }
            }
        });

        const WHAT: Option<&str> = Some("geometry");
        let mut stepper = null_mut();
        error::to_result(unsafe { turtle::stepper_create(&mut stepper) }, WHAT)?;
        error::to_result(unsafe { turtle::stepper_add_flat(stepper, Self::ZMIN) }, WHAT)?;
        for layer in self.layers.iter() {
            let layer = layer.bind(py).borrow();
            unsafe { layer.insert(py, stepper)?; }
        }
        if let Some(zlim) = zlim {
            error::to_result(unsafe { turtle::stepper_add_layer(stepper) }, WHAT)?;
            error::to_result(unsafe { turtle::stepper_add_flat(stepper, zlim.layers) }, WHAT)?;
        }
        error::to_result(unsafe { turtle::stepper_add_layer(stepper) }, WHAT)?;
        error::to_result(unsafe { turtle::stepper_add_flat(stepper, Self::ZMAX) }, WHAT)?;
        let stepper = match zlim {
            Some(zlim) => GeometryStepper { stepper, zlim: zlim.layers },
            None => GeometryStepper { stepper, zlim: 0.0 },
        };

        let opensky_stepper = match zlim {
            Some(zlim) => {
                let mut stepper = null_mut();
                error::to_result(unsafe { turtle::stepper_create(&mut stepper) }, WHAT)?;
                error::to_result(unsafe { turtle::stepper_add_flat(stepper, zlim.opensky) }, WHAT)?;

                error::to_result(unsafe { turtle::stepper_add_layer(stepper) }, WHAT)?;
                error::to_result(unsafe { turtle::stepper_add_flat(stepper, Self::ZMAX) }, WHAT)?;
                GeometryStepper { stepper, zlim: zlim.opensky }
            },
            None => GeometryStepper { stepper: null_mut(), zlim: 0.0 },
        };

        Ok(Doublet { layers: stepper, opensky: opensky_stepper })
    }

    fn ensure_stepper(&mut self, py: Python) -> PyResult<()> {
        if self.stepper == null_mut() {
            self.stepper = self.create_steppers(py, None)?.layers.stepper;
        }
        Ok(())
    }
}

impl Drop for Geometry {
    fn drop(&mut self) {
        unsafe {
            turtle::stepper_destroy(&mut self.stepper);
        }
    }
}

static INTERSECTION_DTYPE: GILOnceCell<PyObject> = GILOnceCell::new();

impl Dtype for Intersection {
    fn dtype<'py>(py: Python<'py>) -> PyResult<&'py Bound<'py, PyAny>> {
        let ob = INTERSECTION_DTYPE.get_or_try_init(py, || -> PyResult<_> {
            let ob = PyModule::import(py, "numpy")?
                .getattr("dtype")?
                .call1(([
                        ("before",    "i4"),
                        ("after",     "i4"),
                        ("latitude",  "f8"),
                        ("longitude", "f8"),
                        ("altitude",  "f8"),
                        ("distance",  "f8")
                    ],
                    true,
                ))?
                .unbind();
            Ok(ob)
        })?
        .bind(py);
        Ok(ob)
    }
}

impl Default for GeometryStepper {
    fn default() -> Self {
        Self { stepper: null_mut(), zlim: 0.0 }
    }
}
