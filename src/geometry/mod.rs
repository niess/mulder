use crate::bindings::turtle;
use crate::simulation::materials::{Materials, MaterialsArg};
use crate::utils::coordinates::{GeographicCoordinates, HorizontalCoordinates};
use crate::utils::error::{self, Error};
use crate::utils::error::ErrorKind::IndexError;
use crate::utils::extract::{Field, Extractor, Name};
use crate::utils::io::PathString;
use crate::utils::notify::{Notifier, NotifyArg};
use crate::utils::numpy::{Dtype, impl_dtype, NewArray};
use crate::utils::ptr::{Destroy, OwnedPtr};
use crate::utils::traits::MinMax;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use std::ffi::c_int;
use std::ptr::{NonNull, null, null_mut};

pub mod atmosphere;
pub mod external;
pub mod grid;
pub mod layer;
pub mod magnet;

use atmosphere::{Atmosphere, AtmosphereLike};
use external::ExternalGeometry;
use layer::{DataLike, Layer};
use magnet::Magnet;


#[derive(FromPyObject, IntoPyObject)]
pub enum Geometry {
    Earth(Py<EarthGeometry>),
    External(Py<ExternalGeometry>),
}

#[derive(Clone, Copy)]
pub enum BoundGeometry<'a, 'py> {
    Earth(&'a Bound<'py, EarthGeometry>),
    External(&'a Bound<'py, ExternalGeometry>),
}

pub enum GeometryRefMut<'py> {
    Earth(PyRefMut<'py, EarthGeometry>),
    External(PyRefMut<'py, ExternalGeometry>),
}

#[derive(FromPyObject)]
pub enum GeometryArg {
    Object(Geometry),
    Path(PathString),
}

// XXX Allow for any geoid?

#[pyclass(module="mulder")]
pub struct EarthGeometry {
    /// The Earth atmosphere.
    #[pyo3(get)]
    pub atmosphere: Py<Atmosphere>,

    /// The geomagnetic field.
    #[pyo3(get)]
    pub magnet: Option<Py<Magnet>>,

    /// The geometry materials.
    #[pyo3(get)]
    pub materials: Materials,

    /// Geometry limits along the z-coordinates.
    #[pyo3(get)]
    pub z: (f64, f64),

    pub layers: Vec<Py<Layer>>,
}

unsafe impl Send for EarthGeometry {}
unsafe impl Sync for EarthGeometry {}

#[derive(FromPyObject)]
pub enum AtmosphereArg<'py> {
    Model(AtmosphereLike<'py>),
    Object(Py<Atmosphere>),
}

#[derive(FromPyObject)]
pub enum MagnetArg {
    Flag(bool),
    Model(PathString),
    Object(Py<Magnet>),
}

#[derive(FromPyObject)]
enum LayerLike<'py> {
    Layer(Py<Layer>),
    OneData(DataLike<'py>),
    ManyData(Vec<DataLike<'py>>),
}

#[repr(C)]
#[derive(Debug)]
pub struct Intersection {
    pub before: i32,
    pub after: i32,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub distance: f64,
}

pub struct EarthGeometryStepper {
    ptr: OwnedPtr<turtle::Stepper>,
    pub layers: usize,
}

#[pymethods]
impl EarthGeometry {
    #[pyo3(signature=(*layers, atmosphere=None, magnet=None, materials=None))]
    #[new]
    pub fn new(
        layers: &Bound<PyTuple>,
        atmosphere: Option<AtmosphereArg>, // XXX hide in kwargs?
        magnet: Option<MagnetArg>,
        materials: Option<MaterialsArg>,
    ) -> PyResult<Self> {
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
            Some(atmosphere) => atmosphere.into_atmosphere(py)?,
            None => Py::new(py, Atmosphere::new(None)?)?,
        };

        let magnet = magnet.and_then(|magnet| magnet.into_magnet(py)).transpose()?;
        let materials = Materials::from_arg(py, materials)?;

        Ok(Self { layers, z, atmosphere, magnet, materials })
    }

    /// The geometry layers.
    #[getter]
    fn get_layers<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        let elements = self.layers
            .iter()
            .map(|data| data.clone_ref(py));
        PyTuple::new(py, elements)
    }

    #[setter]
    fn set_atmosphere(&mut self, py: Python, value: Option<AtmosphereArg>) -> PyResult<()> {
        self.atmosphere = match value {
            Some(atmosphere) => atmosphere.into_atmosphere(py)?,
            None => Py::new(py, Atmosphere::new(None)?)?,
        };
        Ok(())
    }

    #[setter]
    fn set_magnet(&mut self, py: Python, value: Option<MagnetArg>) -> PyResult<()> {
        self.magnet = value.and_then(|magnet| magnet.into_magnet(py)).transpose()?;
        Ok(())
    }

    #[setter]
    fn set_materials(&mut self, py: Python, value: Option<MaterialsArg>) -> PyResult<()> {
        self.materials = Materials::from_arg(py, value)?;
        Ok(())
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

    #[pyo3(name="locate", signature=(position=None, /, *, notify=None, **kwargs))]
    fn py_locate<'py>(
        &mut self,
        py: Python<'py>,
        position: Option<&Bound<PyAny>>,
        notify: Option<NotifyArg>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<NewArray<'py, i32>> {
        let position = Extractor::from_args(
            [
                Field::float(Name::Latitude),
                Field::float(Name::Longitude),
                Field::float(Name::Altitude),
            ],
            position,
            kwargs,
        )?;

        let mut stepper = self.stepper(py)?;
        let notifier = Notifier::from_arg(notify, position.size(), "locating position(s)");

        let mut array = NewArray::empty(py, position.shape())?;
        let layer = array.as_slice_mut();
        for i in 0..position.size() {
            const WHY: &str = "while locating position(s)";
            if (i % 100) == 0 { error::check_ctrlc(WHY)? }

            stepper.reset();

            let geographic = GeographicCoordinates {
                latitude: position.get_f64(Name::Latitude, i)?,
                longitude: position.get_f64(Name::Longitude, i)?,
                altitude: position.get_f64(Name::Altitude, i)?,
            };
            layer[i] = stepper.locate(geographic)?;
            notifier.tic();
        }
        Ok(array)
    }

    #[pyo3(signature=(coordinates=None, /, *, notify=None, **kwargs))]
    fn scan<'py>(
        &mut self,
        py: Python<'py>,
        coordinates: Option<&Bound<PyAny>>,
        notify: Option<NotifyArg>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<NewArray<'py, f64>> {
        let coordinates = Extractor::from_args(
            [
                Field::float(Name::Latitude),
                Field::float(Name::Longitude),
                Field::float(Name::Altitude),
                Field::float(Name::Azimuth),
                Field::float(Name::Elevation),
            ],
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

        let mut stepper = self.stepper(py)?;
        let notifier = Notifier::from_arg(notify, size, "scanning geometry");

        let mut array = NewArray::<f64>::zeros(py, shape)?;
        let distances = array.as_slice_mut();
        for i in 0..size {
            const WHY: &str = "while scanning geometry";
            if (i % 100) == 0 { error::check_ctrlc(WHY)? }

            stepper.reset();

            // Get the starting point.
            let geographic = GeographicCoordinates {
                latitude: coordinates.get_f64(Name::Latitude, i)?,
                longitude: coordinates.get_f64(Name::Longitude, i)?,
                altitude: coordinates.get_f64(Name::Altitude, i)?,
            };
            let mut r = geographic.to_ecef();
            let mut index = [ -2; 2 ];
            error::to_result(
                unsafe {
                    turtle::stepper_step(
                        stepper.as_ptr(),
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
            let horizontal = HorizontalCoordinates {
                azimuth: coordinates.get_f64(Name::Azimuth, i)?,
                elevation: coordinates.get_f64(Name::Elevation, i)?,
            };
            let u = horizontal.to_ecef(&geographic);
            while (index[0] >= 1) && (index[0] as usize <= n + 1) {
                let current = index[0];
                let mut di = 0.0;
                while index[0] == current {
                    let mut step: f64 = 0.0;
                    error::to_result(
                        unsafe {
                            turtle::stepper_step(
                                stepper.as_ptr(),
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

            notifier.tic();
        }

        Ok(array)
    }

    #[pyo3(name="trace", signature=(coordinates=None, /, *, notify=None, **kwargs))]
    fn py_trace<'py>(
        &mut self,
        py: Python<'py>,
        coordinates: Option<&Bound<PyAny>>,
        notify: Option<NotifyArg>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<NewArray<'py, Intersection>> {
        let coordinates = Extractor::from_args(
            [
                Field::float(Name::Latitude),
                Field::float(Name::Longitude),
                Field::float(Name::Altitude),
                Field::float(Name::Azimuth),
                Field::float(Name::Elevation),
            ],
            coordinates,
            kwargs
        )?;
        let size = coordinates.size();
        let shape = coordinates.shape();

        let mut stepper = self.stepper(py)?;
        let notifier = Notifier::from_arg(notify, size, "tracing geometry");

        let mut array = NewArray::empty(py, shape)?;
        let intersections = array.as_slice_mut();
        for i in 0..size {
            const WHY: &str = "while tracing geometry";
            if (i % 100) == 0 { error::check_ctrlc(WHY)? }

            stepper.reset();

            let position = GeographicCoordinates {
                latitude: coordinates.get_f64(Name::Latitude, i)?,
                longitude: coordinates.get_f64(Name::Longitude, i)?,
                altitude: coordinates.get_f64(Name::Altitude, i)?,
            };
            let direction = HorizontalCoordinates {
                azimuth: coordinates.get_f64(Name::Azimuth, i)?,
                elevation: coordinates.get_f64(Name::Elevation, i)?,
            };
            intersections[i] = stepper.trace(position, direction)?.0;
            notifier.tic();
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

impl EarthGeometry {
    // Height of the bottom layer, in m.
    pub const ZMIN: f64 = -11E+03;

    // Height of the atmosphere layer, in m.
    pub const ZMAX: f64 = 120E+03;

    pub fn stepper(&self, py: Python) -> PyResult<EarthGeometryStepper> {
        const WHAT: Option<&str> = Some("geometry");
        let mut ptr = null_mut();
        error::to_result(unsafe { turtle::stepper_create(&mut ptr) }, WHAT)?;
        error::to_result(unsafe { turtle::stepper_add_flat(ptr, Self::ZMIN) }, WHAT)?;
        for layer in self.layers.iter() {
            let layer = layer.bind(py).borrow();
            unsafe { layer.insert(py, ptr)?; }
        }
        error::to_result(unsafe { turtle::stepper_add_layer(ptr) }, WHAT)?;
        error::to_result(unsafe { turtle::stepper_add_flat(ptr, Self::ZMAX) }, WHAT)?;
        let ptr = OwnedPtr::new(ptr)?;
        let layers = self.layers.len();
        let stepper = EarthGeometryStepper { ptr, layers };
        Ok(stepper)
    }
}

impl<'py> AtmosphereArg<'py> {
    fn into_atmosphere(self, py: Python<'py>) -> PyResult<Py<Atmosphere>> {
        match self {
            Self::Model(model) => Py::new(py, Atmosphere::new(Some(model))?),
            Self::Object(atmosphere) => Ok(atmosphere),
        }
    }
}

impl MagnetArg {
    fn into_magnet(self, py: Python) -> Option<PyResult<Py<Magnet>>> {
        match self {
            Self::Flag(b) => if b {
                Some(Magnet::new(py, None, None, None, None)
                    .and_then(|magnet| Py::new(py, magnet)))
            } else {
                None
            },
            Self::Model(model) => {
                Some(Magnet::new(py, Some(model), None, None, None)
                    .and_then(|magnet| Py::new(py, magnet)))
            },
            Self::Object(ob) => Some(Ok(ob.clone_ref(py))),
        }
    }
}

impl_dtype!(
    Intersection,
    [
        ("before",    "i4"),
        ("after",     "i4"),
        ("latitude",  "f8"),
        ("longitude", "f8"),
        ("altitude",  "f8"),
        ("distance",  "f8"),
    ]
);

impl EarthGeometryStepper {
    pub fn step(
        &mut self,
        position: &mut [f64; 3],
        geographic: &mut GeographicCoordinates
    ) -> (f64, usize) {
        let mut step = 0.0;
        let mut index = [ -1; 2 ];
        unsafe {
            turtle::stepper_step(
                self.as_ptr(),
                position.as_mut_ptr(),
                null(),
                &mut geographic.latitude,
                &mut geographic.longitude,
                &mut geographic.altitude,
                null_mut(),
                &mut step,
                index.as_mut_ptr(),
            );
        }
        (step, index[0] as usize)
    }

    #[inline]
    pub fn reset(&mut self) {
        unsafe {
            turtle::stepper_reset(self.as_ptr());
        }
    }

    #[inline]
    fn as_ptr(&self) -> *mut turtle::Stepper {
        self.ptr.0.as_ptr()
    }

    pub fn locate(&mut self, position: GeographicCoordinates) -> PyResult<i32> {
        let mut r = position.to_ecef();
        let mut index = [ -2; 2 ];
        error::to_result(
            unsafe {
                turtle::stepper_step(
                    self.as_ptr(),
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
        Ok(layer_index(index[0]))
    }

    pub fn trace(
        &mut self,
        position: GeographicCoordinates,
        direction: HorizontalCoordinates
    ) -> PyResult<(Intersection, i32)> {
        let mut r = position.to_ecef();
        let mut index = [ -2; 2 ];
        error::to_result(
            unsafe {
                turtle::stepper_step(
                    self.as_ptr(),
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
                          (start_layer as usize <= self.layers + 1) {

            // Iterate until a boundary is hit.
            let u = direction.to_ecef(&position);
            let mut step = 0.0_f64;
            while index[0] == start_layer {
                error::to_result(
                    unsafe {
                        turtle::stepper_step(
                            self.as_ptr(),
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
            position
        };
        Ok((
            Intersection {
                before: layer_index(start_layer),
                after: layer_index(index[0]),
                latitude: position.latitude,
                longitude: position.longitude,
                altitude: position.altitude,
                distance: di,
            },
            index[1],
        ))
    }
}

impl Destroy for NonNull<turtle::Stepper> {
    fn destroy(self) {
        unsafe { turtle::stepper_destroy(&mut self.as_ptr()); }
    }
}

impl Geometry {
    pub fn bind<'a, 'py>(&'a self, py: Python<'py>) -> BoundGeometry<'a, 'py> {
        match self {
            Self::Earth(geometry) => BoundGeometry::Earth(geometry.bind(py)),
            Self::External(geometry) => BoundGeometry::External(geometry.bind(py)),
        }
    }

    pub fn borrow_mut<'py>(&self, py: Python<'py>) -> GeometryRefMut<'py> {
        match self {
            Self::Earth(geometry) => GeometryRefMut::Earth(geometry.bind(py).borrow_mut()),
            Self::External(geometry) => GeometryRefMut::External(geometry.bind(py).borrow_mut()),
        }
    }
}

impl<'a, 'py> BoundGeometry<'a, 'py> {
    pub fn is(self, other: Self) -> bool {
        match self {
            Self::Earth(geometry) => match other {
                Self::Earth(other) => geometry.is(other),
                _ => false,
            },
            Self::External(geometry) => match other {
                Self::External(other) => geometry.is(other),
                _ => false,
            },
        }
    }
}

impl<'py> GeometryRefMut<'py> {
    pub fn materials(&self) -> &Materials {
        match self {
            Self::Earth(geometry) => &geometry.materials,
            Self::External(geometry) => &geometry.materials,
        }
    }
}

impl GeometryArg {
    pub fn into_geometry(self, py: Python) -> PyResult<Geometry> {
        let geometry = match self {
            Self::Object(geometry) => geometry,
            Self::Path(path) => {
                let geometry = unsafe { ExternalGeometry::new(py, path, None)? };
                Geometry::External(Py::new(py, geometry)?)
            },
        };
        Ok(geometry)
    }
}
