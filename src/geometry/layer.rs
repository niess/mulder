use crate::bindings::turtle;
use crate::utils::error::{self, Error};
use crate::utils::error::ErrorKind::{IndexError, TypeError, ValueError};
use crate::utils::notify::{Notifier, NotifyArg};
use crate::utils::numpy::{AnyArray, ArrayMethods, NewArray};
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use super::grid::{self, Grid, GridLike};
use std::ptr::{null, null_mut};


#[pyclass(module="mulder")]
pub struct Layer {
    /// The layer bulk density.
    #[pyo3(get)]
    pub density: Option<f64>,

    /// The layer constitutive material.
    #[pyo3(get, set)]
    pub material: String,

    /// The layer limits along the z-coordinates.
    #[pyo3(get)]
    pub z: (f64, f64),

    pub data: Vec<Data>,
    pub stepper: *mut turtle::Stepper,
}

unsafe impl Send for Layer {}
unsafe impl Sync for Layer {}

#[derive(IntoPyObject)]
pub enum Data {
    Flat(f64),
    Grid(Py<Grid>),
}

#[derive(FromPyObject)]
pub enum DataLike<'py> {
    Flat(f64),
    Grid(GridLike<'py>),
}

pub enum DataRef<'a> {
    Flat(f64),
    Grid(&'a Grid),
}

#[pymethods]
impl Layer {
    #[pyo3(signature=(*data, density=None, material=None))]
    #[new]
    pub fn py_new(
        data: &Bound<PyTuple>,
        density: Option<f64>,
        material: Option<String>
    ) -> PyResult<Self> {
        let py = data.py();
        let data = if data.len() == 0 {
            vec![Data::Flat(0.0)]
        } else {
            let mut v = Vec::with_capacity(data.len());
            for d in data.iter().rev() {
                let d: DataLike = d.extract()?;
                v.push(d.into_data(py)?);
            }
            v
        };
        Self::new(py, data, density, material)
    }

    /// The layer elevation data.
    #[getter]
    fn get_data<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        let elements = self.data
            .iter()
            .map(|data| data.clone_ref(py));
        PyTuple::new(py, elements)
    }

    #[setter]
    fn set_density(&mut self, value: Option<f64>) -> PyResult<()> {
        match value {
            Some(density) => if density <= 0.0 {
                let why = format!(
                    "expected a strictly positive value or 'None', found '{}'",
                    density,
                );
                let err = Error::new(ValueError)
                    .what("density")
                    .why(&why);
                return Err(err.into())
            } else {
                self.density = Some(density);
            }
            None => self.density = None,
        }
        Ok(())
    }

    fn __getitem__(&self, py: Python, index: usize) -> PyResult<Data> {
        self.data
            .get(index)
            .map(|data| data.clone_ref(py))
            .ok_or_else(|| {
                let why = format!(
                    "expected a value in [0, {}], found '{}'",
                    self.data.len() - 1,
                    index,
                );
                Error::new(IndexError)
                    .what("data index")
                    .why(&why)
                    .to_err()
            })
    }

    #[pyo3(signature=(/, *, latitude, longitude, notify=None))]
    fn altitude<'py>(
        &mut self,
        latitude: AnyArray<'py, f64>,
        longitude: AnyArray<'py, f64>,
        notify: Option<NotifyArg>,
    ) -> PyResult<NewArray<'py, f64>> {
        let shape = if latitude.ndim() == 0 {
            longitude.shape()
        } else if longitude.ndim() == 0 {
            latitude.shape()
        } else {
            if latitude.size() != longitude.size() {
                let why = format!(
                    "inconsistent latitude ({}) and longitude ({}) array sizes",
                    latitude.size(),
                    longitude.size(),
                );
                let err = Error::new(TypeError)
                    .what("coordinates")
                    .why(&why)
                    .to_err();
                return Err(err);
            }
            latitude.shape()
        };

        let py = latitude.py();
        self.ensure_stepper(py)?;

        let mut array = NewArray::empty(py, shape)?;
        let z = array.as_slice_mut();
        let notifier = Notifier::from_arg(notify, z.len(), "computing altitude(s)");

        for i in 0..z.len() {
            const WHY: &str = "while computing altitude(s)";
            if (i % 100) == 0 { error::check_ctrlc(WHY)? }

            unsafe {
                turtle::stepper_reset(self.stepper);
            }

            let lat = latitude.get_item(i)?;
            let lon = longitude.get_item(i)?;
            let mut r = [ 0.0_f64; 3 ];
            unsafe { turtle::ecef_from_geodetic(lat, lon, 0.0, r.as_mut_ptr()); }
            let mut elevation = [f64::NAN; 2];
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
                        elevation.as_mut_ptr(),
                        null_mut(),
                        index.as_mut_ptr(),
                    )
                },
                None::<&str>,
            )?;
            z[i] = match index[0] {
                0 => elevation[1],
                1 => elevation[0],
                _ => f64::NAN,
            };
            notifier.tic();
        }
        Ok(array)
    }
}

impl Layer {
    const DEFAULT_MATERIAL: &str = "Rock";
    const WHAT: Option<&str> = Some("layer");

    fn ensure_stepper(&mut self, py: Python) -> PyResult<()> {
        if self.stepper == null_mut() {
            unsafe {
                error::to_result(turtle::stepper_create(&mut self.stepper), None::<&str>)?;
                self.insert(py, self.stepper)?
            }
        }
        Ok(())
    }

    pub fn get_data_ref<'a, 'py:'a>(&'a self, py: Python<'py>) -> Vec<DataRef<'a>> {
        let data: Vec<_> = self.data.iter().map(|data| data.get(py)).collect();
        data
    }

    pub unsafe fn insert(&self, py: Python, stepper: *mut turtle::Stepper) -> PyResult<()> {
        if !self.data.is_empty() {
            error::to_result(turtle::stepper_add_layer(stepper), Self::WHAT)?;
        }
        for data in &self.data {
            match data {
                Data::Flat(f) => error::to_result(
                    turtle::stepper_add_flat(stepper, *f),
                    Self::WHAT,
                )?,
                Data::Grid(g) => {
                    let g = g.bind(py).borrow();
                    match *g.data {
                        grid::Data::Map(m) => error::to_result(
                            turtle::stepper_add_map(stepper, m, g.offset),
                            Self::WHAT,
                        )?,
                        grid::Data::Stack(s) => error::to_result(
                            turtle::stepper_add_stack(stepper, s, g.offset),
                            Self::WHAT,
                        )?,
                    }
                },
            }
        }
        Ok(())
    }
}

impl Layer {
    pub fn new(
        py: Python,
        data: Vec<Data>,
        density: Option<f64>,
        material: Option<String>
    ) -> PyResult<Self> {
        let z = {
            let mut z = (f64::INFINITY, -f64::INFINITY);
            for d in data.iter() {
                let dz = d.z(py);
                if dz.0 < z.0 { z.0 = dz.0; }
                if dz.1 > z.1 { z.1 = dz.1; }
            }
            z
        };
        let material = material.unwrap_or_else(|| Self::DEFAULT_MATERIAL.to_string());
        let stepper = null_mut();
        let mut layer = Self { density: None, material, z, data, stepper };
        if density.is_some() {
            layer.set_density(density)?;
        }
        Ok(layer)
    }
}

impl Drop for Layer {
    fn drop(&mut self) {
        unsafe {
            turtle::stepper_destroy(&mut self.stepper);
        }
    }
}

impl Data {
    fn clone_ref(&self, py: Python) -> Self {
        match self {
            Data::Flat(f) => Data::Flat(*f),
            Data::Grid(g) => Data::Grid(g.clone_ref(py)),
        }
    }

    fn get<'a, 'py:'a>(&'a self, py: Python<'py>) -> DataRef<'a> {
        match self {
            Self::Flat(f) => DataRef::Flat(*f),
            Self::Grid(g) => DataRef::Grid(g.bind(py).get()),
        }
    }

    pub fn z(&self, py: Python) -> (f64, f64) {
        match self {
            Self::Flat(f) => (*f, *f),
            Self::Grid(g) => g.bind(py).borrow().z,
        }
    }
}

impl<'py> DataLike<'py> {
    pub fn into_data(self, py: Python<'py>) -> PyResult<Data> {
        let data = match self {
            Self::Flat(f) => Data::Flat(f),
            Self::Grid(g) => {
                let g = g
                    .into_grid(py)?
                    .unbind();
                Data::Grid(g)
            },
        };
        Ok(data)
    }
}

impl<'a> DataRef<'a> {
    pub fn gradient(
        &self,
        latitude: f64,
        longitude: f64,
        altitude: f64,
    ) -> [f64; 3] {
        let [glon, glat] = match self {
            Self::Flat(_) => [ 0.0; 2 ],
            Self::Grid(g) => g.data.gradient(longitude, latitude),
        };
        const DEG: f64 = std::f64::consts::PI / 180.0;
        const RT: f64 = 0.5 * (6378137.0 + 6356752.314245);  // WGS84 average.
        let theta = (90.0 - latitude) * DEG;
        let phi = longitude * DEG;
        let st = theta.sin();
        let ct = theta.cos();
        let sp = phi.sin();
        let cp = phi.cos();
        let r_inv = 1.0 / (RT + altitude);
        let rho_inv = if st.abs() <= (f32::EPSILON as f64) { 0.0 } else { r_inv / st };
        let gt = -glat * r_inv / DEG;
        let gp = glon * rho_inv / DEG;
        [
            st * cp - ct * cp * gt + sp * gp,
            st * sp - ct * sp * gt - cp * gp,
            ct      + st      * gt,
        ]
    }
}
