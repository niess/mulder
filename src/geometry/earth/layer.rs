use crate::bindings::turtle;
use crate::simulation::coordinates::LocalFrame;
use crate::utils::error::{self, Error};
use crate::utils::error::ErrorKind::ValueError;
use crate::utils::notify::{Notifier, NotifyArg};
use crate::utils::numpy::{AnyArray, ArrayMethods, NewArray};
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use super::grid::{self, Grid, GridLike};
use std::ptr::{null, null_mut};
use super::{EarthGeometry, grid::{get_shape, parse_xy}};


/// A layer (or strate) of an Earth geometry.
#[pyclass(module="mulder")]
pub struct Layer {
    /// The layer bulk density, in kg/m3.
    #[pyo3(get)]
    pub density: Option<f64>,

    /// The layer constitutive material.
    #[pyo3(get)]
    pub material: String,

    /// An optional description.
    #[pyo3(get, set)]
    pub description: Option<String>,

    pub zlim: (f64, f64),
    pub data: Vec<Data>,
    pub stepper: *mut turtle::Stepper,
    pub geometry: Option<Py<EarthGeometry>>,
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
    #[pyo3(signature=(*data, density=None, description=None, material=None))]
    #[new]
    pub fn py_new(
        data: &Bound<PyTuple>,
        density: Option<f64>,
        description: Option<String>,
        material: Option<String>
    ) -> PyResult<Self> {
        let py = data.py();
        let data = if data.len() == 0 {
            vec![Data::Flat(0.0)]
        } else {
            let mut v = Vec::with_capacity(data.len());
            for d in data.iter() {
                let d: DataLike = d.extract()?;
                v.push(d.into_data(py)?);
            }
            v
        };
        Self::new(py, data, density, description, material)
    }

    /// The layer top elevation data.
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

    #[setter]
    fn set_material(&mut self, py: Python, value: &str) {
        if value != self.material {
            if let Some(geometry) = self.geometry.as_ref() {
                let mut geometry = geometry.bind(py).borrow_mut();
                geometry.subscribers.retain(|subscriber|
                    subscriber.replace(self.material.as_str(), value)
                )
            }
            self.material = value.to_owned();
        }
    }

    /// Computes the layer top altitude(s) at coordinate(s).
    #[pyo3(name="altitude", signature=(latitude_or_latlon, longitude=None, /, *, notify=None))]
    fn py_altitude<'py>(
        &mut self,
        latitude_or_latlon: AnyArray<'py, f64>,
        longitude: Option<AnyArray<'py, f64>>,
        notify: Option<NotifyArg>,
    ) -> PyResult<NewArray<'py, f64>> {
        let py = latitude_or_latlon.py();
        self.ensure_stepper(py)?;
        let z = match longitude {
            Some(longitude) => {
                let latitude = latitude_or_latlon;
                let (nx, ny, shape) = get_shape(&longitude, &latitude);
                let mut array = NewArray::<f64>::empty(py, shape)?;
                let z = array.as_slice_mut();
                let notifier = Notifier::from_arg(notify, z.len(), "computing altitude(s)");
                for iy in 0..ny {
                    let lat = latitude.get_item(iy)?;
                    for ix in 0..nx {
                        const WHY: &str = "while computing altitude(s)";
                        let index = iy * nx + ix;
                        if (index % 100) == 0 { error::check_ctrlc(WHY)? }

                        let lon = longitude.get_item(ix)?;
                        z[iy * nx + ix] = self.altitude(lat, lon)?.0;
                        notifier.tic();
                    }
                }
                array
            },
            None => {
                let latlon = latitude_or_latlon;
                let mut shape = parse_xy(&latlon)?;
                shape.pop();
                let mut array = NewArray::<f64>::empty(py, shape)?;
                let z = array.as_slice_mut();
                let notifier = Notifier::from_arg(notify, z.len(), "computing altitude(s)");
                for i in 0..z.len() {
                    const WHY: &str = "while computing altitudes(s)";
                    if (i % 100) == 0 { error::check_ctrlc(WHY)? }

                    let lat = latlon.get_item(2 * i)?;
                    let lon = latlon.get_item(2 * i + 1)?;
                    z[i] = self.altitude(lat, lon)?.0;
                    notifier.tic();
                }
                array
            },
        };
        Ok(z)
    }

    /// Computes the layer top normal(s) at coordinate(s).
    #[pyo3(name="normal", signature=(latitude_or_latlon, longitude=None, /, *, frame=None, notify=None))]
    fn py_normal<'py>(
        &mut self,
        latitude_or_latlon: AnyArray<'py, f64>,
        longitude: Option<AnyArray<'py, f64>>,
        frame: Option<LocalFrame>,
        notify: Option<NotifyArg>,
    ) -> PyResult<NewArray<'py, f64>> {
        let py = latitude_or_latlon.py();
        self.ensure_stepper(py)?;
        let data = self.get_data_ref(py);
        let normal = match longitude {
            Some(longitude) => {
                let latitude = latitude_or_latlon;
                let (nx, ny, mut shape) = get_shape(&longitude, &latitude);
                shape.push(3);
                let mut array = NewArray::<f64>::empty(py, shape)?;
                let n = array.as_slice_mut();
                let notifier = Notifier::from_arg(notify, nx * ny, "computing normal(s)");
                for iy in 0..ny {
                    let lat = latitude.get_item(iy)?;
                    for ix in 0..nx {
                        const WHY: &str = "while computing normal(s)";
                        let index = iy * nx + ix;
                        if (index % 100) == 0 { error::check_ctrlc(WHY)? }

                        let lon = longitude.get_item(ix)?;
                        let nij = self.normal(lat, lon, &data, frame.as_ref())?;
                        let index = iy * nx + ix;
                        for k in 0..3 {
                            n[3 * index + k] = nij[k];
                        }
                        notifier.tic();
                    }
                }
                array
            },
            None => {
                let latlon = latitude_or_latlon;
                let mut shape = parse_xy(&latlon)?;
                shape.pop();
                shape.push(3);
                let mut array = NewArray::<f64>::empty(py, shape)?;
                let n = array.as_slice_mut();
                let size = n.len() / 3;
                let notifier = Notifier::from_arg(notify, size, "computing normal(s)");
                for i in 0..size {
                    const WHY: &str = "while computing normal(s)";
                    if (i % 100) == 0 { error::check_ctrlc(WHY)? }

                    let lat = latlon.get_item(2 * i)?;
                    let lon = latlon.get_item(2 * i + 1)?;
                    let ni = self.normal(lat, lon, &data, frame.as_ref())?;
                    for k in 0..3 {
                        n[3 * i + k] = ni[k];
                    }
                    notifier.tic();
                }
                array
            },
        };
        Ok(normal)
    }
}

impl Layer {
    const DEFAULT_MATERIAL: &str = "Rock";
    const WHAT: Option<&str> = Some("layer");

    fn altitude(&self, lat: f64, lon: f64) -> PyResult<(f64, usize)> {
        unsafe {
            turtle::stepper_reset(self.stepper);
        }

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
        let z = match index[0] {
            0 => elevation[1],
            1 => elevation[0],
            _ => f64::NAN,
        };
        Ok((z, index[1] as usize))
    }

    fn normal(
        &self,
        lat: f64,
        lon: f64,
        data: &[DataRef<'_>],
        frame: Option<&LocalFrame>,
    ) -> PyResult<[f64; 3]> {
        let (z, index) = self.altitude(lat, lon)?;
        let mut v = if index >= data.len() {
            [0.0; 3]
        } else {
            data[index].gradient(lat, lon, z)
        };
        if let Some(frame) = frame {
            v = frame.from_ecef_direction(&v);
        }
        let r2 = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
        let normal = if r2 > f64::EPSILON {
            let r = r2.sqrt();
            v[0] /= r;
            v[1] /= r;
            v[2] /= r;
            v
        } else {
            [0.0; 3]
        };
        Ok(normal)
    }

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
        for data in self.data.iter().rev() {
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
        description: Option<String>,
        material: Option<String>
    ) -> PyResult<Self> {
        let zlim = {
            let mut zlim = (f64::INFINITY, -f64::INFINITY);
            for d in data.iter() {
                let dz = d.zlim(py);
                if dz.0 < zlim.0 { zlim.0 = dz.0; }
                if dz.1 > zlim.1 { zlim.1 = dz.1; }
            }
            zlim
        };
        let material = material.unwrap_or_else(|| Self::DEFAULT_MATERIAL.to_string());
        let stepper = null_mut();
        let geometry = None;
        let mut layer = Self {
            density: None, description, material, zlim, data, stepper, geometry
        };
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

    pub fn zlim(&self, py: Python) -> (f64, f64) {
        match self {
            Self::Flat(f) => (*f, *f),
            Self::Grid(g) => g.bind(py).borrow().zlim,
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
            Self::Grid(g) => match *g.data {
                grid::Data::Map(map) => {
                    let projection = unsafe { turtle::map_projection(map) };
                    if projection == null_mut() {
                        g.data.gradient(longitude, latitude)
                    } else {
                        // Compute the gradient in map coordinates.
                        let mut x0 = 0.0;
                        let mut y0 = 0.0;
                        unsafe {
                            turtle::projection_project(
                                projection, latitude, longitude, &mut x0, &mut y0
                            );
                        }
                        let [gx, gy] = g.data.gradient(x0, y0);

                        // Compute the Jacobian matrix of the map projection.
                        const EPSILON: f64 = 1E-09;
                        let mut x1 = 0.0;
                        let mut y1 = 0.0;
                        unsafe {
                            turtle::projection_project(
                                projection, latitude, longitude + EPSILON, &mut x1, &mut y1
                            );
                        }
                        let mut x2 = 0.0;
                        let mut y2 = 0.0;
                        unsafe {
                            turtle::projection_project(
                                projection, latitude + EPSILON, longitude, &mut x2, &mut y2
                            );
                        }
                        let jac = [
                            [ (x1 - x0) / EPSILON, (y1 - y0) / EPSILON ],
                            [ (x2 - x0) / EPSILON, (y2 - y0) / EPSILON ],
                        ];

                        // Transform the map gradient to geographic coordinates.
                        let glon = gx * jac[0][0] + gy * jac[0][1];
                        let glat = gx * jac[1][0] + gy * jac[1][1];
                        [glon, glat]
                    }
                },
                grid::Data::Stack(_) => g.data.gradient(longitude, latitude),
            },
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
