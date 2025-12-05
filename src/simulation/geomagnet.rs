use chrono::{Datelike, NaiveDate};
use crate::bindings::gull;
use crate::simulation::coordinates::{LocalFrame, PositionExtractor};
use crate::utils::error::{self, Error};
use crate::utils::error::ErrorKind::ValueError;
use crate::utils::io::PathString;
use crate::utils::notify::{Notifier, NotifyArg};
use crate::utils::numpy::NewArray;
use crate::utils::traits::EnsureFile;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::ffi::{c_int, CString, c_void, OsStr};
use std::path::Path;
use std::ptr::null_mut;


#[pyclass(module="mulder")]
pub struct EarthMagnet {
    /// The snapshot date.
    #[pyo3(get)]
    date: NaiveDate,

    /// The model name.
    #[pyo3(get)]
    model: Option<String>,

    /// The model altitude limits, in m.
    #[pyo3(get)]
    zlim: (f64, f64),

    snapshot: *mut gull::Snapshot,
    workspace: *mut f64,

    // XXX Accept a uniform field?
    // XXX add a density threshold field.
}

#[derive(FromPyObject)]
pub enum EarthMagnetArg {
    Flag(bool),
    Model(PathString),
    Object(Py<EarthMagnet>),
}

#[derive(FromPyObject)]
pub enum DateArg {
    String(String),
    Date(NaiveDate),
}

unsafe impl Send for EarthMagnet {}
unsafe impl Sync for EarthMagnet {}

#[pymethods]
impl EarthMagnet {
    #[pyo3(signature=(model=None, /, *, date=None))]
    #[new]
    pub fn new(
        py: Python,
        model: Option<PathString>,
        date: Option<DateArg>,
    ) -> PyResult<Self> {
        let (path, model) = match model {
            None => {
                let path = Path::new(crate::PREFIX.get(py).unwrap())
                    .join(format!("data/magnet/{}.COF", Self::DEFAULT_MODEL))
                    .into_os_string()
                    .into_string()
                    .unwrap();
                (path, Some(Self::DEFAULT_MODEL.to_string()))
            },
            Some(model) => {
                const WHAT: &str = "model";
                let path = Path::new(model.as_str()).ensure_file(WHAT)?;
                let stem = path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(|stem| stem.to_string());
                let path = match path.extension().and_then(OsStr::to_str) {
                    Some("COF" | "cof") => model.0,
                    _ => {
                        let why = format!("invalid file format '{}'", path.display());
                        let err = Error::new(ValueError).what(WHAT).why(&why);
                        return Err(err.to_err())
                    },
                };
                (path, stem)
            },
        };

        let date = match date {
            Some(date) => match date {
                DateArg::String(date) => NaiveDate::parse_from_str(date.as_str(), "%Y-%m-%d")
                    .map_err(|why| Error::new(ValueError)
                        .what("date")
                        .why(&why.to_string())
                        .to_err()
                    )?,
                DateArg::Date(date) => date,
            },
            None => NaiveDate::from_ymd_opt(2025, 6, 21).unwrap(),
        };

        let workspace: *mut f64 = null_mut();
        let mut snapshot: *mut gull::Snapshot = null_mut();
        let path = CString::new(path.as_str()).unwrap();
        let rc = unsafe {
            gull::snapshot_create(
                &mut snapshot,
                path.as_c_str().as_ptr(),
                date.day() as c_int,
                date.month() as c_int,
                date.year() as c_int,
            )
        };
        error::to_result(rc, Some("magnet"))?;

        let zlim = {
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

        Ok(Self { date, model, zlim, snapshot, workspace })
    }

    /// Computes the geomagnetic field value(s) at the specified position(s).
    #[pyo3(
        name="field",
        signature=(position=None, /, *, notify=None, frame=None, **kwargs),
        text_signature="(self, position=None, /, *, notify=None, **kwargs)",
    )]
    fn py_field<'py>(
        &mut self,
        py: Python<'py>,
        position: Option<&Bound<PyAny>>,
        notify: Option<NotifyArg>,
        frame: Option<LocalFrame>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<NewArray<'py, f64>> {
        let position = PositionExtractor::new(py, position, kwargs, frame.as_ref().into(), None)?;
        let shape = {
            let mut shape = position.shape();
            shape.push(3);
            shape
        };
        let size = position.size();
        let mut array = NewArray::empty(py, shape)?;
        let fields = array.as_slice_mut();
        let notifier = Notifier::from_arg(notify, size, "computing field");
        for i in 0..size {
            const WHY: &str = "while computing field";
            if (i % 1000) == 0 { error::check_ctrlc(WHY)? }

            let ri = position.extract(i)?
                .into_geographic();
            let mut fi = self.field(
                ri.latitude,
                ri.longitude,
                ri.altitude,
            )?;
            if let PositionExtractor::Local { frame, .. } = &position {
                let field_frame = LocalFrame::new(ri, 0.0, 0.0);
                let ecef = field_frame.to_ecef_direction(&fi);
                fi = frame.from_ecef_direction(&ecef);
            }
            for j in 0..3 {
                fields[3 * i + j] = fi[j];
            }
            notifier.tic();
        }
        Ok(array)
    }
}

impl EarthMagnet {
    const DEFAULT_MODEL: &str = "IGRF14";

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

impl Drop for EarthMagnet {
    fn drop(&mut self) {
        unsafe {
            gull::snapshot_destroy(&mut self.snapshot);
            libc::free(self.workspace as *mut c_void);
        }
        self.workspace = null_mut();
    }
}

impl EarthMagnetArg {
    pub fn into_geomagnet(self, py: Python) -> PyResult<Option<Py<EarthMagnet>>> {
        let geomagnet = match self {
            Self::Flag(b) => if b {
                Some(EarthMagnet::new(py, None, None)
                    .and_then(|magnet| Py::new(py, magnet)))
            } else {
                None
            },
            Self::Model(model) => {
                Some(EarthMagnet::new(py, Some(model), None)
                    .and_then(|magnet| Py::new(py, magnet)))
            },
            Self::Object(ob) => Some(Ok(ob.clone_ref(py))),
        };
        Ok(geomagnet.transpose()?)
    }
}
