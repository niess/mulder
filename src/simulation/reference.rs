use crate::utils::error::Error;
use crate::utils::error::ErrorKind::{IOError, TypeError, ValueError};
use crate::utils::io::PathString;
use crate::utils::numpy::{AnyArray, ArrayMethods, Dtype, NewArray};
use pyo3::prelude::*;
use pyo3::sync::GILOnceCell;
use pyo3::types::PyDict;
use std::ffi::OsStr;
use std::path::Path;


#[pyclass(frozen, module="mulder")]
pub struct Reference {
    /// Altitude (range) of the reference flux.
    #[pyo3(get)]
    altitude: Altitude,

    /// Elevation range of the reference flux.
    #[pyo3(get)]
    elevation: (f64, f64),

    /// Energy range of the reference flux.
    #[pyo3(get)]
    energy: (f64, f64),

    model: Model,
}

#[derive(Clone, Copy, FromPyObject, IntoPyObject)]
pub enum Altitude {
    Scalar(f64),
    Range((f64, f64)),
}

#[repr(C)]
pub struct Flux {
    value: f64,
    asymmetry: f64,
}

enum Model {
    GCCLY,
    Table(Table),
}

struct Table {
    shape: [usize; 3],
    energy: [f64; 2],
    cos_theta: [f64; 2],
    altitude: [f64; 2],
    data: Vec<f32>,
}

#[derive(FromPyObject)]
pub enum ModelArg<'py> {
    Array(AnyArray<'py, f64>),
    Path(PathString),
}

#[pymethods]
impl Reference {
    #[new]
    #[pyo3(signature=(model, /, **kwargs))]
    fn new(
        model: ModelArg,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<Self> {
        let reference: Self = match model {
            ModelArg::Array(array) => {
                let missing_energy = || Error::new(TypeError)
                    .what("reference")
                    .why("missing energy range")
                    .to_err();
                let (energy, cos_theta, altitude) = match kwargs {
                    Some(kwargs) => {
                        let mut altitude: Option<Altitude> = None;
                        let mut cos_theta: Option<[f64; 2]> = None;
                        let mut energy: Option<[f64; 2]> = None;
                        for (key, value) in kwargs.iter() {
                            let key: String = key.extract()?;
                            match key.as_str() {
                                "altitude" => { altitude = Some(value.extract()?); },
                                "cos_theta" => { cos_theta = Some(value.extract()?); },
                                "energy" => { energy = Some(value.extract()?); },
                                key => {
                                    let why = format!("invalid keyword argument '{}'", key);
                                    let err = Error::new(TypeError)
                                        .what("kwargs")
                                        .why(&why);
                                    return Err(err.to_err())
                                },
                            }
                        }
                        match energy {
                            Some(energy) => (energy, cos_theta, altitude),
                            None => return Err(missing_energy()),
                        }
                    },
                    None => return Err(missing_energy()),
                };
                Table::from_array(array, energy, cos_theta, altitude)?.into()
            },
            ModelArg::Path(string) => {
                let path = Path::new(string.as_str());
                if path.is_file() {
                    match path.extension().and_then(OsStr::to_str) {
                        Some("table") => Table::from_file(path)?.into(),
                        Some(ext) => {
                            let why = format!(
                                "{}: unsupported format (.{})",
                                string.as_str(),
                                ext,
                            );
                            let err = Error::new(TypeError)
                                .what("model")
                                .why(&why);
                            return Err(err.into())
                        },
                        _ => {
                            let why = format!(
                                "{}: missing format",
                                string.as_str(),
                            );
                            let err = Error::new(TypeError)
                                .what("model")
                                .why(&why);
                            return Err(err.into())
                        },
                    }
                } else {
                    unimplemented!()
                }
            },
        };
        Ok(reference)
    }

    #[pyo3(signature=(state=None, /, *, **kwargs))]
    fn __call__<'py>(
        &self,
        py: Python<'py>,
        state: Option<&Bound<PyAny>>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<NewArray<'py, Flux>> {
        unimplemented!()
    }
}

impl Reference {
    const DEFAULT_ALTITUDE: f64 = 0.0;
}

impl From<Table> for Reference {
    fn from(value: Table) -> Self {
        let altitude = if value.altitude[0] == value.altitude[1] {
            Altitude::Scalar(value.altitude[0])
        } else {
            Altitude::Range((value.altitude[0], value.altitude[1]))
        };
        const DEG: f64 = 180.0 / std::f64::consts::PI;
        let elevation = (value.cos_theta[0].asin() * DEG, value.cos_theta[1].asin() * DEG);
        let energy = (value.energy[0], value.energy[1]);
        let model = Model::Table(value);
        Self { altitude, elevation, energy, model }
    }
}

impl Table {
    pub fn flux(&self, altitude: f64, elevation: f64, energy: f64) -> Option<Flux> {
        // Compute indices.
        #[inline]
        fn getindex(x: f64, xmin: f64, xmax: f64, nx: usize) -> Option<(usize, f64)> {
            let dlx = (xmax / xmin).ln() / ((nx - 1) as f64);
            let mut hx = (x / xmin).ln() / dlx;
            if (hx < 0.0) || (hx > (nx - 1) as f64) { return None }
            let ix = hx as usize;
            hx -= ix as f64;
            Some((ix, hx))
        }

        const DEG: f64 = std::f64::consts::PI / 180.0;
        let c = ((90.0 - elevation) * DEG).cos();
        let [ n_h, n_c, n_k ] = self.shape;
        let [ k_min, k_max ] = self.energy;
        let [ c_min, c_max ] = self.cos_theta;
        let [ h_min, h_max ] = self.altitude;

        let (ik, hk) = getindex(energy, k_min, k_max, n_k)?;
        let (ic, hc) = getindex(c, c_min, c_max, n_c)?;
        let (ih, hh) = if n_h > 1 {
            getindex(altitude, h_min, h_max, n_h)?
        } else {
            (0, 0.0)
        };

        let ik1 = if ik < n_k - 1 { ik + 1 } else { n_k - 1 };
        let ic1 = if ic < n_c - 1 { ic + 1 } else { n_c - 1 };
        let ih1 = if ih < n_h - 1 { ih + 1 } else { n_h - 1 };
        let i000 = 2 * ((ih * n_c + ic) * n_k + ik);
        let i010 = 2 * ((ih * n_c + ic1) * n_k + ik);
        let i100 = 2 * ((ih * n_c + ic) * n_k + ik1);
        let i110 = 2 * ((ih * n_c + ic1) * n_k + ik1);
        let i001 = 2 * ((ih1 * n_c + ic) * n_k + ik);
        let i011 = 2 * ((ih1 * n_c + ic1) * n_k + ik);
        let i101 = 2 * ((ih1 * n_c + ic) * n_k + ik1);
        let i111 = 2 * ((ih1 * n_c + ic1) * n_k + ik1);

        // Interpolate the flux.
        let f = |i: usize| -> f64 { self.data[i] as f64 };
        let mut flux = [0.0_f64; 2];
        for i in 0..2 {
            // Linear interpolation along cos(theta).
            let g00 = f(i000 + i) * (1.0 - hc) + f(i010 + i) * hc;
            let g10 = f(i100 + i) * (1.0 - hc) + f(i110 + i) * hc;
            let g01 = f(i001 + i) * (1.0 - hc) + f(i011 + i) * hc;
            let g11 = f(i101 + i) * (1.0 - hc) + f(i111 + i) * hc;

            // Log or linear interpolation along log(energy).
            let g0 = if (g00 <= 0.0) || (g10 <= 0.0) {
                g00 * (1.0 - hk) + g10 * hk
            } else {
                (g00.ln() * (1.0 - hk) + g10.ln() * hk).exp()
            };

            let g1 = if (g01 <= 0.0) || (g11 <= 0.0) {
                g01 * (1.0 - hk) + g11 * hk
            } else {
                (g01.ln() * (1.0 - hk) + g11.ln() * hk).exp()
            };

            // Log or linear interpolation along altitude.
            flux[i] = if (g0 <= 0.0) || (g1 <= 0.0) {
                g0 * (1.0 - hh) + g1 * hh
            } else {
                (g0.ln() * (1.0 - hh) + g1.ln() * hh).exp()
            };
        }

        let tmp = flux[0] + flux[1];
        let flux = if tmp > 0.0 {
            Flux { value: tmp, asymmetry: (flux[0] - flux[1]) / tmp }
        } else {
            Flux::ZERO
        };
        Some(flux)
    }

    fn from_array(
        array: AnyArray<f64>,
        energy: [f64; 2],
        cos_theta: Option<[f64; 2]>,
        altitude: Option<Altitude>,
    ) -> PyResult<Self> {
        let (ndim, altitude) = match altitude {
            Some(altitude) => match altitude {
                Altitude::Scalar(altitude) => (2, [ altitude, altitude ]),
                Altitude::Range(altitude) => (3, [ altitude.0, altitude.1 ]),
            },
            None => (2, [ Reference::DEFAULT_ALTITUDE, Reference::DEFAULT_ALTITUDE ]),
        };
        if array.ndim() != ndim {
            let why = format!("expected a {}d array, found {}d", ndim, array.ndim());
            let err = Error::new(TypeError)
                .what("grid")
                .why(&why)
                .to_err();
            return Err(err)
        }
        let shape = {
            let mut shape = array.shape();
            if ndim == 2 { [ 1, shape[0], shape[1] ] } else { shape.try_into().unwrap() }
        };
        let [ n_h, n_c, n_k ] = shape;
        let cos_theta = cos_theta.unwrap_or_else(|| [ 0.0, 1.0 ]);

        let n = 2 * n_k * n_c * n_h;
        let mut data = Vec::<f32>::with_capacity(n);
        for i in 0..n {
            let di = array.get_item(i)?;
            data.push(di as f32);
        }

        let table = Self { shape, energy, cos_theta, altitude, data };
        Ok(table)
    }

    fn from_file<P: AsRef<Path>>(path: P) -> PyResult<Self> {
        let path: &Path = path.as_ref();
        let bad_format = || {
            let why = format!("{}: bad table format)", path.display());
            Error::new(ValueError).why(&why).to_err()
        };

        let bytes = std::fs::read(path)
            .map_err(|err| {
                let why = format!("{}: {}", path.display(), err);
                Error::new(IOError).why(&why).to_err()
            })?;

        #[repr(C)]
        #[derive(Debug)]
        struct Header {
            n_k: i64,
            n_c: i64,
            n_h: i64,
            k_min: f64,
            k_max: f64,
            c_min: f64,
            c_max: f64,
            h_min: f64,
            h_max: f64,
            data: [u8; 0],
        }
        const HEADER_SIZE: usize = std::mem::size_of::<Header>();
        let header: [u8; HEADER_SIZE] = bytes.get(0..HEADER_SIZE)
            .ok_or_else(bad_format)?.try_into().unwrap();
        let header = unsafe { std::mem::transmute::<_, Header>(header) };
        let n_k: usize = header.n_k.try_into().or_else(|_| Err(bad_format()))?;
        let n_c: usize = header.n_c.try_into().or_else(|_| Err(bad_format()))?;
        let n_h: usize = header.n_h.try_into().or_else(|_| Err(bad_format()))?;

        let n = 2 * n_k * n_c * n_h;
        let bytes = bytes.get(HEADER_SIZE..(HEADER_SIZE + 4 * n))
            .ok_or_else(bad_format)?;

        let mut data = Vec::<f32>::with_capacity(n);
        let mut offset = 0;
        for _ in 0..n {
            let d = &bytes[offset..(offset + 4)];
            let v = f32::from_le_bytes(d.try_into().unwrap());
            data.push(v);
            offset += 4;
        }

        let Header { k_min, k_max, c_min, c_max, h_min, h_max, .. } = header;
        let shape = [ n_h, n_c, n_k ];
        let energy = [ k_min, k_max ];
        let cos_theta = [ c_min, c_max ];
        let altitude = [ h_min, h_max ];
        let table = Self { shape, energy, cos_theta, altitude, data };
        Ok(table)
    }
}

impl Flux {
    const ZERO: Self = Self { value: 0.0, asymmetry: 0.0 };
}

static FLUX_DTYPE: GILOnceCell<PyObject> = GILOnceCell::new();

impl Dtype for Flux {
    fn dtype<'py>(py: Python<'py>) -> PyResult<&'py Bound<'py, PyAny>> {
        let ob = FLUX_DTYPE.get_or_try_init(py, || -> PyResult<_> {
            let ob = PyModule::import(py, "numpy")?
                .getattr("dtype")?
                .call1(([
                        ("value",     "f8"),
                        ("asymmetry", "f8"),
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
