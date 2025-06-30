use crate::bindings::turtle;
use crate::utils::error::{self, Error};
use crate::utils::error::ErrorKind::{IOError, NotImplementedError, TypeError};
use crate::utils::io::PathString;
use crate::utils::numpy::{AnyArray, ArrayMethods, NewArray};
use geotiff::GeoTiff;
use geo_types::geometry::Coord;
use pyo3::prelude::*;
use ::std::ffi::{c_char, c_int, CStr, CString, OsStr};
use ::std::fs::File;
use ::std::path::Path;
use ::std::ptr::{null, null_mut};
use ::std::sync::Arc;


#[pyclass(frozen, module="mulder")]
pub struct Grid {
    /// Grid limits along the z-coordinates.
    #[pyo3(get)]
    pub z: (f64, f64),

    pub data: Arc<Data>,
    pub offset: f64,
}

pub enum Data {
    Map(*mut turtle::Map),
    Stack(*mut turtle::Stack),
}

unsafe impl Send for Data {}
unsafe impl Sync for Data {}

#[derive(FromPyObject)]
pub enum DataArg<'py> {
    Array(AnyArray<'py, f64>),
    Path(PathString),
}

#[derive(FromPyObject)]
pub enum GridLike<'py> {
    Bound(Bound<'py, Grid>),
    Path(PathString),
}

#[pymethods]
impl Grid {
    #[new]
    #[pyo3(signature=(data, /, *, x=None, y=None, projection=None))]
    fn new(
        data: DataArg,
        x: Option<[f64; 2]>,
        y: Option<[f64; 2]>,
        projection: Option<&str>,
    ) -> PyResult<Self> {
        let (data, z) = match data {
            DataArg::Array(array) => {
                let shape = array.shape();
                if shape.len() != 2 {
                    let why = format!("expected a 2d array, found {}d", shape.len());
                    let err = Error::new(TypeError)
                        .what("grid")
                        .why(&why)
                        .to_err();
                    return Err(err)
                }
                let ny = shape[0];
                let nx = shape[1];
                let x = x.unwrap_or_else(|| [0.0, 1.0]);
                let y = y.unwrap_or_else(|| [0.0, 1.0]);

                let converter = ArrayConverter { array: &array, nx };
                let (map, z) = converter.convert(nx, ny, x, y, projection)?;
                (Data::Map(map), z)
            },
            DataArg::Path(string) => {
                if x.is_some() || y.is_some() {
                    let coord = if x.is_some() { "x" } else { "y" };
                    let why = format!("cannot redefine {}-limits", coord);
                    let err = Error::new(TypeError)
                        .what("grid")
                        .why(&why)
                        .to_err();
                    return Err(err)
                }

                let path = Path::new(string.as_str());
                if path.is_file() {
                    let (map, z) = match path.extension().and_then(OsStr::to_str) {
                        Some("asc" | "grd" | "hgt") => {
                            let mut map: *mut turtle::Map = null_mut();
                            let path = CString::new(string.0).unwrap();
                            let rc = unsafe {
                                turtle::map_load(
                                    &mut map,
                                    path.as_c_str().as_ptr()
                                )
                            };
                            error::to_result(rc, Some("grid"))?;
                            (map, None)
                        },
                        Some("tif") => {
                            let geotiff = GeoTiff::read(File::open(path)?)
                                .map_err(|err| Error::new(IOError)
                                    .what("GeoTiff file")
                                    .why(&err.to_string())
                                    .to_err()
                                )?;
                            let projection = match geotiff.geo_key_directory.projected_type {
                                Some(crs) => if (crs >= 32601) && (crs <= 32660) {
                                    Some(format!("UTM {}N", crs - 32600))
                                } else if (crs >= 32701) && (crs <= 32760) {
                                    Some(format!("UTM {}S", crs - 32700))
                                } else {
                                    match crs {
                                        2154 => Some("Lambert 93".to_owned()),
                                        4326 => None,
                                        27571 => Some("Lambert I".to_owned()),
                                        27572 => Some("Lambert II".to_owned()),
                                        27573 => Some("Lambert III".to_owned()),
                                        27574 => Some("Lambert IV".to_owned()),
                                        _ => {
                                            let why = format!("EPSG:{}", crs);
                                            let err = Error::new(NotImplementedError)
                                                .what("crs")
                                                .why(&why)
                                                .to_err();
                                            return Err(err)
                                        },
                                    }
                                },
                                None => None,
                            };

                            let nx = geotiff.raster_width;
                            let ny = geotiff.raster_height;
                            let (x, y) = {
                                let extent = geotiff.model_extent();
                                let min = extent.min();
                                let max = extent.max();
                                let x = [ min.x, max.x ];
                                let y = [ min.y, max.y ];
                                (x, y)
                            };
                            let converter = GeotiffConverter::new(nx, ny, &x, &y, &geotiff);
                            let (map, z) = converter.convert(nx, ny, x, y, projection.as_deref())?;
                            (map, Some(z))
                        },
                        Some(ext) => {
                            let why = format!(
                                "{}: unsupported data format (.{})",
                                string.as_str(),
                                ext,
                            );
                            let err = Error::new(TypeError)
                                .what("grid")
                                .why(&why);
                            return Err(err.into())
                        },
                        None => {
                            let why = format!(
                                "{}: missing data format extension",
                                string.as_str(),
                            );
                            let err = Error::new(TypeError)
                                .what("grid")
                                .why(&why);
                            return Err(err.into())
                        },
                    };
                    if let Some(name) = projection {
                        let projection = unsafe { turtle::map_projection(map) };
                        if projection != null() {
                            let err = Error::new(TypeError)
                                .what("grid")
                                .why("cannot redefine projection");
                            return Err(err.into())
                        }
                        let name = CString::new(name).unwrap();
                        let rc = unsafe {
                            turtle::projection_configure(
                                &mut (*map).meta.projection,
                                name.as_c_str().as_ptr(),
                            )
                        };
                        error::to_result(rc, Some("projection"))?;
                    }
                    let z = z.unwrap_or_else(|| unsafe { get_map_zlim(map) });
                    (Data::Map(map), z)
                } else if path.is_dir() {
                    let mut stack: *mut turtle::Stack = null_mut();
                    let path = CString::new(string.as_str()).unwrap();
                    let rc = unsafe {
                        turtle::stack_create(
                            &mut stack,
                            path.as_c_str().as_ptr(),
                            -1,
                            None,
                            None,
                        )
                    };
                    error::to_result(rc, Some("grid"))?;
                    let shape = unsafe {
                        let mut shape: [c_int; 2] = [0, 0];
                        turtle::stack_info(
                            stack,
                            &mut shape as *mut c_int,
                            null_mut(),
                            null_mut()
                        );
                        shape
                    };
                    if (shape[0] == 0) || (shape[1] == 0) {
                        let why = format!(
                            "{}: could not find any data tile",
                            string.as_str(),
                        );
                        let err = Error::new(TypeError)
                            .what("grid")
                            .why(&why);
                        return Err(err.into())
                    }
                    let rc = unsafe {
                        turtle::stack_load(stack)
                    };
                    error::to_result(rc, Some("grid"))?;
                    let z = unsafe { get_stack_zlim(stack) };
                    (Data::Stack(stack), z)
                } else {
                    let why = if path.exists() {
                        format!("{}: not a file or directory", string.as_str())
                    } else {
                        format!("{}: no such file or directory", string.as_str())
                    };
                    let err = Error::new(TypeError)
                        .what("grid")
                        .why(&why);
                    return Err(err.into())
                }
            },
        };
        Ok(Self { data: Arc::new(data), z: (z[0], z[1]), offset: 0.0 })
    }

    /// Grid coordinates projection.
    #[getter]
    fn get_projection(&self) -> Option<String> {
        match *self.data {
            Data::Map(map) => {
                let mut projection: *const c_char = null_mut();
                unsafe {
                    turtle::map_meta(map, null_mut(), &mut projection);
                }
                if projection == null_mut() {
                    None
                } else {
                    let projection = unsafe { CStr::from_ptr(projection) };
                    Some(
                        projection
                            .to_str()
                            .unwrap()
                            .to_string()
                    )
                }
            },
            Data::Stack(_) => None,
        }
    }

    /// Grid limits along the x-coordinates.
    #[getter]
    fn get_x(&self) -> (f64, f64) {
        match *self.data {
            Data::Map(map) => {
                let mut info = turtle::MapInfo::default();
                unsafe {
                    turtle::map_meta(map, &mut info, null_mut());
                }
                (info.x[0], info.x[1])
            },
            Data::Stack(stack) => {
                let mut x = [ f64::NAN; 2 ];
                unsafe {
                    turtle::stack_info(stack, null_mut(), null_mut(), x.as_mut_ptr());
                }
                (x[0], x[1])
            },
        }
    }

    /// Grid limits along the y-coordinates.
    #[getter]
    fn get_y(&self) -> (f64, f64) {
        match *self.data {
            Data::Map(map) => {
                let mut info = turtle::MapInfo::default();
                unsafe {
                    turtle::map_meta(map, &mut info, null_mut());
                }
                (info.y[0], info.y[1])
            },
            Data::Stack(stack) => {
                let mut y = [ f64::NAN; 2 ];
                unsafe {
                    turtle::stack_info(stack, null_mut(), null_mut(), y.as_mut_ptr());
                }
                (y[0], y[1])
            },
        }
    }

    fn __add__(&self, offset: f64) -> Self {
        let data = Arc::clone(&self.data);
        let z = (self.z.0 + offset, self.z.1 + offset);
        let offset = self.offset  + offset;
        Self { z, data, offset }
    }

    fn __radd__(&self, offset: f64) -> Self {
        self.__add__(offset)
    }

    fn __sub__(&self, offset: f64) -> Self {
        self.__add__(-offset)
    }

    /// Computes the elevation value at grid point(s).
    #[pyo3(signature=(xy, y=None, /))]
    fn __call__<'py>(
        &self,
        xy: AnyArray<'py, f64>,
        y: Option<AnyArray<'py, f64>>,
    ) -> PyResult<NewArray<'py, f64>> {
        let py = xy.py();
        let z = match y {
            Some(y) => {
                let x = xy;
                let (nx, ny, shape) = get_shape(&x, &y);
                let mut array = NewArray::<f64>::empty(py, shape)?;
                let z = array.as_slice_mut();
                for iy in 0..ny {
                    let yi = y.get_item(iy)?;
                    for ix in 0..nx {
                        let xi = x.get_item(ix)?;
                        z[iy * nx + ix] = self.data.z(xi, yi) + self.offset;
                    }
                }
                array
            },
            None => {
                let mut shape = parse_xy(&xy)?;
                shape.pop();
                let mut array = NewArray::<f64>::empty(py, shape)?;
                let z = array.as_slice_mut();
                for i in 0..z.len() {
                    let xi = xy.get_item(2 * i)?;
                    let yi = xy.get_item(2 * i + 1)?;
                    z[i] = self.data.z(xi, yi) + self.offset;
                }
                array
            },
        };
        Ok(z)
    }

    /// Computes the elevation gradient at grid point(s).
    #[pyo3(signature=(xy, y=None, /))]
    fn gradient<'py>(
        &self,
        xy: AnyArray<'py, f64>,
        y: Option<AnyArray<'py, f64>>,
    ) -> PyResult<NewArray<'py, f64>> {
        let py = xy.py();
        let gradient = match y {
            Some(y) => {
                let x = xy;
                let (nx, ny, shape) = get_shape(&x, &y);
                let mut array = NewArray::<f64>::empty(py, shape)?;
                let gradient = array.as_slice_mut();
                for iy in 0..ny {
                    let yi = y.get_item(iy)?;
                    for ix in 0..nx {
                        let xi = x.get_item(ix)?;
                        let [gx, gy] = self.data.gradient(xi, yi);
                        let i = iy * nx + ix;
                        gradient[2 * i] = gx;
                        gradient[2 * i + 1] = gy;
                    }
                }
                array
            },
            None => {
                let shape = parse_xy(&xy)?;
                let mut array = NewArray::<f64>::empty(py, shape)?;
                let gradient = array.as_slice_mut();
                for i in 0..xy.size() {
                    let xi = xy.get_item(2 * i)?;
                    let yi = xy.get_item(2 * i + 1)?;
                    let [gx, gy] = self.data.gradient(xi, yi);
                    gradient[2 * i] = gx;
                    gradient[2 * i + 1] = gy;
                }
                array
            },
        };
        Ok(gradient)
    }
}

fn parse_xy(xy: &AnyArray<f64>) -> PyResult<Vec<usize>> {
    let shape = xy.shape();
    if shape.len() == 1 {
        if shape[0] != 2 {
            let why = format!("expected a size 2 array, found size {}", shape[0]);
            let err = Error::new(TypeError)
                .what("xy")
                .why(&why)
                .to_err();
            return Err(err)
        }
    } else if shape.len() == 2 {
        if shape[1] != 2 {
            let why = format!("expected an Nx2 array, found Nx{}", shape[1]);
            let err = Error::new(TypeError)
                .what("xy")
                .why(&why)
                .to_err();
            return Err(err)
        }
    } else {
        let why = format!("expected a 1d or 2d array, found {}d", shape.len());
        let err = Error::new(TypeError)
            .what("xy")
            .why(&why)
            .to_err();
        return Err(err)
    }
    Ok(shape)
}

fn get_shape(x: &AnyArray<f64>, y: &AnyArray<f64>) -> (usize, usize, Vec<usize>) {
    let nx = x.size();
    let ny = y.size();
    let mut shape = Vec::new();
    if y.ndim() > 0 {
        shape.push(ny)
    }
    if x.ndim() > 0 {
        shape.push(nx)
    }
    (nx, ny, shape)
}

unsafe fn get_map_zlim(map: *const turtle::Map) -> [f64; 2] {
    let mut info = turtle::MapInfo::default();
    turtle::map_meta(map, &mut info, null_mut());
    let mut z = [f64::INFINITY, -f64::INFINITY];
    for iy in 0..info.ny {
        for ix in 0..info.nx {
            let mut zi = f64::NAN;
            turtle::map_node(map, ix, iy, null_mut(), null_mut(), &mut zi);
            if zi < z[0] { z[0] = zi }
            if zi > z[1] { z[1] = zi }
        }
    }
    z
}

unsafe fn get_stack_zlim(stack: *const turtle::Stack) -> [f64; 2] {
    let mut z = [f64::INFINITY, -f64::INFINITY];
    let mut map = (*stack).list.head as *const turtle::Map;
    while map != null() {
        let zi = get_map_zlim(map);
        if zi[0] < z[0] { z[0] = zi[0] }
        if zi[1] > z[1] { z[1] = zi[1] }
        map = (*map).element.next as *const turtle::Map;
    }
    z
}

impl Data {
    fn gradient(&self, x: f64, y: f64) -> [f64; 2] {
        match self {
            Self::Map(map) => {
                let mut gx = f64::NAN;
                let mut gy = f64::NAN;
                let mut inside: c_int = 0;
                unsafe { turtle::map_gradient(*map, x, y, &mut gx, &mut gy, &mut inside); }
                [gx, gy]
            },
            Self::Stack(stack) => {
                let mut gx = f64::NAN;
                let mut gy = f64::NAN;
                let mut inside: c_int = 0;
                // XXX (latitude, longitude) or reverse?
                unsafe { turtle::stack_gradient(*stack, y, x, &mut gy, &mut gx, &mut inside); }
                [gx, gy]
            },
        }
    }

    fn z(&self, x: f64, y: f64) -> f64 {
        match self {
            Self::Map(map) => {
                let mut z = f64::NAN;
                let mut inside: c_int = 0;
                unsafe { turtle::map_elevation(*map, x, y, &mut z, &mut inside); }
                z
            },
            Self::Stack(stack) => {
                let mut z = f64::NAN;
                let mut inside: c_int = 0;
                // XXX (latitude, longitude) or reverse?
                unsafe { turtle::stack_elevation(*stack, y, x, &mut z, &mut inside); }
                z
            },
        }
    }
}

impl<'py> GridLike<'py> {
    pub fn into_grid(self, py: Python<'py>) -> PyResult<Bound<'py, Grid>> {
        let grid = match self {
            Self::Bound(bound) => bound,
            Self::Path(path) => {
                let grid = Grid::new(DataArg::Path(path), None, None, None)?;
                Bound::new(py, grid)?
            },
        };
        Ok(grid)
    }
}

impl Drop for Data {
    fn drop(&mut self) {
        match self {
            Data::Map(map) => unsafe {
                turtle::map_destroy(map)
            },
            Data::Stack(stack) => unsafe {
                turtle::stack_destroy(stack)
            },
        }
    }
}

trait Convert {
    fn get_z(&self, ix: usize, iy: usize) -> PyResult<f64>;

    fn convert(
        &self,
        nx: usize,
        ny: usize,
        x: [f64; 2],
        y: [f64; 2],
        projection: Option<&str>,
    ) -> PyResult<(*mut turtle::Map, [f64; 2])> {
        let mut z = [ f64::INFINITY, -f64::INFINITY ];
        for iy in 0..ny {
            for ix in 0..nx {
                let zij = self.get_z(ix, iy)?;
                if zij < z[0] { z[0] = zij }
                if zij > z[1] { z[1] = zij }
            }
        }
        let info = turtle::MapInfo {
            nx: nx as c_int,
            ny: ny as c_int,
            x,
            y,
            z,
            encoding: null(),
        };
        let binding;
        let projection = match projection {
            Some(projection) => {
                binding = CString::new(projection).unwrap();
                binding.as_c_str().as_ptr()
            },
            None => null_mut(),
        };
        let mut map: *mut turtle::Map = null_mut();
        let rc = unsafe {
            turtle::map_create(&mut map, &info, projection)
        };
        error::to_result(rc, Some("grid"))?;

        for iy in 0..ny {
            for ix in 0..nx {
                let zij = self.get_z(ix, iy)?;
                let rc = unsafe { turtle::map_fill(map, ix as c_int, iy as c_int, zij) };
                error::to_result(rc, Some("grid"))?;
            }
        }
        Ok((map, z))
    }
}

struct ArrayConverter<'a, 'py> {
    nx: usize,
    array: &'a AnyArray<'py, f64>,
}

impl<'a, 'py> Convert for ArrayConverter<'a, 'py> {
    fn get_z(&self, ix: usize, iy: usize) -> PyResult<f64> {
        self.array.get_item(iy * self.nx + ix)
    }
}

struct GeotiffConverter<'a> {
    nx: usize,
    ny: usize,
    dx: f64,
    dy: f64,
    x: &'a [f64; 2],
    y: &'a [f64; 2],
    geotiff: &'a GeoTiff,
}

impl<'a> GeotiffConverter<'a> {
    fn new(
        nx: usize,
        ny: usize,
        x: &'a [f64; 2],
        y: &'a [f64; 2],
        geotiff: &'a GeoTiff,
    ) -> Self {
        let dx = (x[1] - x[0]) / ((nx - 1) as f64);
        let dy = (y[1] - y[0]) / ((ny - 1) as f64);
        Self { nx, ny, dx, dy, x, y, geotiff }
    }
}

impl<'a> Convert for GeotiffConverter<'a> {
    fn get_z(&self, ix: usize, iy: usize) -> PyResult<f64> {
        let x = if ix == self.nx - 1 { self.x[1] } else { self.x[0] + ix as f64 * self.dx };
        let y = if iy == self.ny - 1 { self.y[1] } else { self.y[0] + iy as f64 * self.dy };
        let z = self.geotiff.get_value_at(&Coord { x, y }, 0)
            .unwrap_or_else(|| 0.0); // XXX Use NO_DATA value?
        Ok(z)
    }
}
