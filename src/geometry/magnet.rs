use crate::bindings::gull;
use crate::utils::error::{self, Error};
use crate::utils::error::ErrorKind::ValueError;
use crate::utils::extract::{Field, Extractor, Name};
use crate::utils::io::PathString;
use crate::utils::numpy::NewArray;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::ffi::{c_int, CString, c_void, OsStr};
use std::path::Path;
use std::ptr::null_mut;


#[pyclass(module="mulder")]
pub struct Magnet {
    /// The calendar day.
    #[pyo3(get)] // XXX Use date object instead?
    day: usize,

    /// The calendar month.
    #[pyo3(get)]
    month: usize,

    /// The calendar year.
    #[pyo3(get)]
    year: usize,

    /// The model limits along the z-coordinates.
    #[pyo3(get)]  // XXX Change to zlim (everywhere)?
    altitude: (f64, f64),

    snapshot: *mut gull::Snapshot,
    workspace: *mut f64,
}

unsafe impl Send for Magnet {}
unsafe impl Sync for Magnet {}

#[pymethods]
impl Magnet {
    #[pyo3(signature=(model=None, /, *, day=None, month=None, year=None))]
    #[new]
    pub fn new(
        py: Python,
        model: Option<PathString>, // XXX Accept a uniform value?
        day: Option<usize>,
        month: Option<usize>,
        year: Option<usize>,
    ) -> PyResult<Self> {
        let model = match model {
            None => {
                Path::new(crate::PREFIX.get(py).unwrap())
                    .join(format!("data/magnet/{}", Self::DEFAULT_MODEL))
                    .into_os_string()
                    .into_string()
                    .unwrap()
            },
            Some(model) => {
                const WHAT: &str = "model";
                let path = Path::new(model.as_str());
                if !path.is_file() {
                    let why = if !path.exists() { // XXX Generic check (as trait?)
                        format!("no such file '{}'", path.display())
                    } else {
                        format!("not a file '{}'", path.display())
                    };
                    let err = Error::new(ValueError).what(WHAT).why(&why);
                    return Err(err.to_err())
                }
                match path.extension().and_then(OsStr::to_str) {
                    Some("COF" | "cof") => model.0,
                    _ => {
                        let why = format!("invalid file format '{}'", path.display());
                        let err = Error::new(ValueError).what(WHAT).why(&why);
                        return Err(err.to_err())
                    },
                }
            },
        };

        let day = day.unwrap_or_else(|| 21);
        let month = month.unwrap_or_else(|| 6);
        let year = year.unwrap_or_else(|| 2025);

        let workspace: *mut f64 = null_mut();
        let mut snapshot: *mut gull::Snapshot = null_mut();
        let model = CString::new(model.as_str()).unwrap();
        let rc = unsafe {
            gull::snapshot_create(
                &mut snapshot,
                model.as_c_str().as_ptr(),
                day as c_int,
                month as c_int,
                year as c_int,
            )
        };
        error::to_result(rc, Some("magnet"))?;

        let altitude = {
            let mut zmin = 0.0;
            let mut zmax = 0.0;
            unsafe {
                gull::snapshot_info(
                    snapshot,
                    null_mut(),
                    &mut zmin,
                    &mut zmax,
                );
            }
            (zmin, zmax)
        };

        Ok(Self { day, month, year, altitude, snapshot, workspace })
    }

    #[pyo3(signature=(position=None, /, **kwargs))]
    fn __call__<'py>(
        &mut self,
        py: Python<'py>,
        position: Option<&Bound<PyAny>>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<NewArray<'py, f64>> {
        let position = Extractor::from_args(
            [
                Field::float(Name::Latitude),
                Field::float(Name::Longitude),
                Field::maybe_float(Name::Altitude),
            ],
            position,
            kwargs,
        )?;

        let shape = {
            let mut shape = position.shape();
            shape.push(3);
            shape
        };
        let mut array = NewArray::empty(py, shape)?;
        let fields = array.as_slice_mut();
        for i in 0..position.size() {
            let fi = self.field(
                position.get_f64(Name::Latitude, i)?,
                position.get_f64(Name::Longitude, i)?,
                position.get_f64_opt(Name::Altitude, i)?
                    .unwrap_or_else(|| 0.0),
            )?;
            for j in 0..3 {
                fields[3 * i + j] = fi[j];
            }
        }
        Ok(array)
    }
}

impl Magnet {
    const DEFAULT_MODEL: &str = "IGRF14.COF";

    pub fn field(&mut self, latitude: f64, longitude: f64, altitude: f64) -> PyResult<[f64; 3]> {
        let mut field = [ 0.0_f64; 3 ];
        let rc = unsafe {
            gull::snapshot_field(
                self.snapshot,
                latitude,
                longitude,
                altitude,
                field.as_mut_ptr(),
                &mut self.workspace
            )
        };
        error::to_result(rc, Some("field"))?;
        Ok(field)
    }
}

impl Drop for Magnet {
    fn drop(&mut self) {
        unsafe {
            gull::snapshot_destroy(&mut self.snapshot);
            libc::free(self.workspace as *mut c_void);
        }
        self.workspace = null_mut();
    }
}
