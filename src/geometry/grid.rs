use crate::bindings::turtle;
use crate::utils::error::{self, Error};
use crate::utils::error::ErrorKind::{NotImplementedError, TypeError};
use crate::utils::io::PathString;
use crate::utils::numpy::{AnyArray, ArrayMethods, NewArray};
use pyo3::prelude::*;
use ::std::ffi::{c_char, c_int, CStr, CString, OsStr};
use ::std::path::Path;
use ::std::ptr::{null, null_mut};


#[pyclass(module="mulder")]
pub struct Grid {
    #[pyo3(get)]
    pub z: (f64, f64),

    pub data: Data,
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
                let mut z = [ f64::INFINITY, -f64::INFINITY ];
                for iy in 0..ny {
                    for ix in 0..nx {
                        let zij = array.get_item(iy * nx + ix)?;
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
                let projection = projection.map(|projection| CString::new(projection).unwrap());
                let mut map: *mut turtle::Map = null_mut();
                let rc = unsafe {
                    turtle::map_create(
                        &mut map,
                        &info,
                        projection
                            .map(|p| p.as_c_str().as_ptr())
                            .unwrap_or_else(|| null()),
                    )
                };
                error::to_result(rc, Some("grid"))?;

                for iy in 0..ny {
                    for ix in 0..nx {
                        let zij = array.get_item(iy * nx + ix)?;
                        let rc = unsafe { turtle::map_fill(map, ix as c_int, iy as c_int, zij) };
                        error::to_result(rc, Some("grid"))?;
                    }
                }
                (Data::Map(map), z)
            },
            DataArg::Path(string) => {
                let path = Path::new(string.as_str());
                if path.is_file() {
                    match path.extension().and_then(OsStr::to_str) {
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
                            let z = unsafe { get_map_zlim(map) };
                            (Data::Map(map), z)
                        },
                        Some("tif") => {
                            // XXX implement GeoTIFF loader.
                            let err = Error::new(NotImplementedError)
                                .what("grid")
                                .why(".tif");
                            return Err(err.into())
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
                    }
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
                    let why = format!(
                        "{}: not a file or directory",
                        string.as_str(),
                    );
                    let err = Error::new(TypeError)
                        .what("grid")
                        .why(&why);
                    return Err(err.into())
                }
            },
        };
        let z = (z[0], z[1]);
        Ok(Self { data, z })
    }

    /// Grid coordinates projection.
    #[getter]
    fn get_projection(&self) -> Option<String> {
        match self.data {
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
        match self.data {
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
        match self.data {
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
                        z[iy * nx + ix] = self.data.z(xi, yi);
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
                    z[i] = self.data.z(xi, yi);
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

impl Drop for Grid {
    fn drop(&mut self) {
        match self.data {
            Data::Map(mut map) => unsafe {
                turtle::map_destroy(&mut map)
            },
            Data::Stack(mut stack) => unsafe {
                turtle::stack_destroy(&mut stack)
            },
        }
    }
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
