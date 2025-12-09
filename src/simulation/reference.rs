use crate::geometry::EarthGeometry;
use crate::simulation::coordinates::LocalFrame;
use crate::simulation::states::{ExtractedState, StatesExtractor};
use crate::utils::convert::{Convert, ParametricModel};
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::{IOError, TypeError, ValueError};
use crate::utils::io::PathString;
use crate::utils::numpy::{AnyArray, ArrayMethods, impl_dtype, NewArray};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::ffi::OsStr;
use std::path::Path;


// XXX MCEq interface?
// XXX Replace table format with pkl? Or remove.

#[pyclass(frozen, module="mulder")]
pub struct Reference {
    /// Altitude (range) of the reference flux.
    #[pyo3(get)]
    pub altitude: Altitude,

    /// Elevation range of the reference flux.
    #[pyo3(get)]
    pub elevation: (f64, f64),

    /// Energy range of the reference flux.
    #[pyo3(get)]
    pub energy: (f64, f64),

    model: Model,
}

#[derive(Clone, Copy, FromPyObject, IntoPyObject)]
pub enum Altitude {
    Scalar(f64),
    Range((f64, f64)),
}

#[repr(C)]
#[derive(Debug)]
pub struct Flux {
    pub muon: f64,
    pub anti: f64,
}

enum Model {
    Flat(f64),
    Parametric(ParametricModel),
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
    Number(f64),
    Array(AnyArray<'py, f64>),
    Path(PathString),
}

#[pymethods]
impl Reference {
    #[new]
    #[pyo3(signature=(model=None, /, **kwargs))]
    pub fn new(
        model: Option<ModelArg>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<Self> {
        let model = match model {
            Some(model) => model,
            None => return  Ok(ParametricModel::default().into()),
        };
        let extract_kwargs = |varname: &'static str| -> PyResult<_> {
            match kwargs {
                Some(kwargs) => {
                    let mut altitude: Option<Altitude> = None;
                    let mut range: Option<[f64; 2]> = None;
                    let mut energy: Option<[f64; 2]> = None;
                    for (key, value) in kwargs.iter() {
                        let key: String = key.extract()?;
                        match key.as_str() {
                            "altitude" => { altitude = value.extract()?; },
                            "energy" => { energy = value.extract()?; },
                            key => if key == varname {
                                range = value.extract()?;
                            } else {
                                let why = format!("invalid keyword argument '{}'", key);
                                let err = Error::new(TypeError)
                                    .what("kwargs")
                                    .why(&why);
                                return Err(err.to_err())
                            },
                        }
                    }
                    Ok((energy, range, altitude))
                },

                None => Ok((None, None, None))
            }
        };
        let reference: Self = match model {
            ModelArg::Array(array) => {
                let (energy, cos_theta, altitude) = extract_kwargs("cos_theta")?;
                match energy {
                    Some(energy) => Table::from_array(array, energy, cos_theta, altitude)?.into(),
                    None => {
                        let err = Error::new(TypeError)
                            .what("reference")
                            .why("missing energy range")
                            .to_err();
                        return Err(err)
                    },
                }
            },
            ModelArg::Number(value) => {
                let (energy, elevation, altitude) = extract_kwargs("elevation")?;
                let model = Model::Flat(value);
                let energy = energy.unwrap_or([1E-03, 1E+12]).into();
                let elevation = elevation.unwrap_or([-90.0, 90.0]).into();
                let altitude = altitude
                    .unwrap_or(Altitude::Range((EarthGeometry::ZMIN, EarthGeometry::ZMAX)));
                Self { energy, elevation, altitude, model }
            },
            ModelArg::Path(string) => {
                if let Some(kwargs) = kwargs {
                    let key: String = kwargs.keys().get_item(0).unwrap().extract().unwrap();
                    let why = format!("unexpected '{}' named argument", key);
                    let err = Error::new(TypeError).what("reference").why(&why).to_err();
                    return Err(err)
                }
                let path = Path::new(string.as_str());
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
                        let model = ParametricModel::from_string(string.0)?;
                        model.into()
                    },
                }
            },
        };
        Ok(reference)
    }

    /// Computes the reference flux.
    #[pyo3(
        name="flux",
        signature=(states=None, /, *, frame=None, **kwargs),
        text_signature="(self, states=None, /, **kwargs)",
    )]
    fn py_flux<'py>(
        &self,
        py: Python<'py>,
        states: Option<&Bound<PyAny>>,
        frame: Option<LocalFrame>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<NewArray<'py, f64>> {
        // Extract states.
        let states = StatesExtractor::new(states, kwargs, frame.as_ref().into())?;
        let tagged = states.is_tagged();

        let mut array = NewArray::zeros(py, states.shape())?;
        let flux = array.as_slice_mut();

        for i in 0..states.size() {
            let (pid, energy, elevation, altitude) = match states.extract(i)? {
                ExtractedState::Geographic { state } => {
                    (state.pid, state.energy, state.elevation, state.altitude)
                },
                ExtractedState::Local { state, frame } => {
                    let (geographic, horizontal) = frame.to_geographic(
                        &state.position, &state.direction
                    );
                    (state.pid, state.energy, horizontal.elevation, geographic.altitude)
                }
            };
            let f = self.flux(energy, elevation, altitude);
            flux[i] = if tagged {
                if pid == 13 {
                    f.muon
                } else if pid == -13 {
                    f.anti
                } else {
                    let why = format!("expected '13' or '-13', found {}", pid);
                    let err = Error::new(ValueError).what("pid").why(&why).to_err();
                    return Err(err)
                }
            } else {
                f.muon + f.anti
            };
        }
        Ok(array)
    }
}

impl Reference {
    const DEFAULT_ALTITUDE: f64 = 0.0;
    const DEFAULT_RATIO: f64 = 1.2766;  // Ref: CMS (https://arxiv.org/abs/1005.5332).

    pub fn flux(&self, energy: f64, elevation: f64, altitude: f64) -> Flux {
        match self.model {
            Model::Flat(value) => Flux {
                muon: value / (1.0 + Self::DEFAULT_RATIO),
                anti: value * Self::DEFAULT_RATIO / (1.0 + Self::DEFAULT_RATIO),
            },
            Model::Table(ref table) => table.flux(energy, elevation, altitude)
                .unwrap_or_else(|| Flux::ZERO),
            Model::Parametric(ref model) => {
                const RAD: f64 = std::f64::consts::PI / 180.0;
                let cos_theta = ((90.0 - elevation) * RAD).cos();
                let value = match model {
                    ParametricModel::GCCLY15 => flux_gccly(cos_theta, energy),
                    ParametricModel::Gaisser90 => flux_gaisser(cos_theta, energy),
                };
                if value > 0.0 {
                    Flux {
                        muon: value / (1.0 + Self::DEFAULT_RATIO),
                        anti: value * Self::DEFAULT_RATIO / (1.0 + Self::DEFAULT_RATIO),
                    }
                } else {
                    Flux::ZERO
                }
            },
        }
    }
}

impl From<ParametricModel> for Reference {
    fn from(value: ParametricModel) -> Self {
        let model = Model::Parametric(value);
        let altitude = Altitude::Scalar(0.0);
        let energy = (1E-03, 1E+12);
        let elevation = (0.0, 90.0);
        Self { altitude, elevation, energy, model }
    }
}

fn to_elevation(cos_theta: [f64; 2]) -> (f64, f64) {
    const DEG: f64 = 180.0 / std::f64::consts::PI;
    (cos_theta[0].asin() * DEG, cos_theta[1].asin() * DEG)
}

impl From<Table> for Reference {
    fn from(value: Table) -> Self {
        let altitude = if value.altitude[0] == value.altitude[1] {
            Altitude::Scalar(value.altitude[0])
        } else {
            Altitude::Range((value.altitude[0], value.altitude[1]))
        };
        let elevation = to_elevation(value.cos_theta);
        let energy = (value.energy[0], value.energy[1]);
        let model = Model::Table(value);
        Self { altitude, elevation, energy, model }
    }
}

impl Table {
    fn check(energy: &[f64; 2], cos_theta: &[f64; 2]) -> PyResult<()> {
        let check_energy = |value: f64| -> PyResult<()> {
            if value <= 0.0 {
                let why = format!("expected a strictly positive value, found '{}'", value);
                Err(Error::new(ValueError).what("energy").why(&why).to_err())
            } else {
                Ok(())
            }
        };
        check_energy(energy[0])?;
        check_energy(energy[1])?;

        let check_cos_theta = |value: f64| -> PyResult<()> {
            if (value < -1.0) || (value > 1.0) {
                let why = format!("expected a value in [-1, 1], found '{}'", value);
                Err(Error::new(ValueError).what("cos(theta)").why(&why).to_err())
            } else {
                Ok(())
            }
        };
        check_cos_theta(cos_theta[0])?;
        check_cos_theta(cos_theta[1])?;

        Ok(())
    }

    fn flux(&self, energy: f64, elevation: f64, altitude: f64) -> Option<Flux> {
        // Compute indices.
        #[inline]
        fn getindex_ln(x: f64, xmin: f64, xmax: f64, nx: usize) -> Option<(usize, f64)> {
            let dlx = (xmax / xmin).ln() / ((nx - 1) as f64);
            let mut hx = (x / xmin).ln() / dlx;
            if (hx < 0.0) || (hx > (nx - 1) as f64) { return None }
            let ix = hx as usize;
            hx -= ix as f64;
            Some((ix, hx))
        }

        #[inline]
        fn getindex_li(x: f64, xmin: f64, xmax: f64, nx: usize) -> Option<(usize, f64)> {
            let dlx = (xmax - xmin) / ((nx - 1) as f64);
            let mut hx = (x - xmin) / dlx;
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

        let (ik, hk) = getindex_ln(energy, k_min, k_max, n_k)?;
        let (ic, hc) = getindex_li(c, c_min, c_max, n_c)?;
        let (ih, hh) = if n_h > 1 {
            getindex_li(altitude, h_min, h_max, n_h)?
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

        Some(Flux { muon: flux[0].max(0.0), anti: flux[1].max(0.0) })
    }

    fn from_array(
        array: AnyArray<f64>,
        energy: [f64; 2],
        cos_theta: Option<[f64; 2]>,
        altitude: Option<Altitude>,
    ) -> PyResult<Self> {
        let (ndim, altitude) = match altitude {
            Some(altitude) => match altitude {
                Altitude::Scalar(altitude) => (3, [ altitude, altitude ]),
                Altitude::Range(altitude) => (4, [ altitude.0, altitude.1 ]),
            },
            None => (3, [ Reference::DEFAULT_ALTITUDE, Reference::DEFAULT_ALTITUDE ]),
        };
        let (shape, is_tagged) = if array.ndim() == ndim - 1 {
            let shape = array.shape();
            (shape, false)
        } else if array.ndim() == ndim {
            let mut shape = array.shape();
            let n_p = shape.pop().unwrap();
            if n_p != 2 {
                let why = format!("expected a shape (.., 2) array, found (.., {})", n_p);
                let err = Error::new(TypeError)
                    .what("array")
                    .why(&why)
                    .to_err();
                return Err(err)
            }
            (shape, true)
        } else {
            let why = format!("expected a {}d array, found {}d", ndim, array.ndim());
            let err = Error::new(TypeError)
                .what("array")
                .why(&why)
                .to_err();
            return Err(err)
        };
        let shape = if ndim == 3 { [ 1, shape[0], shape[1] ] } else { shape.try_into().unwrap() };
        let [ n_h, n_c, n_k ] = shape;
        let cos_theta = cos_theta.unwrap_or_else(|| [ 0.0, 1.0 ]);

        let n = n_k * n_c * n_h;
        let (n_p, r) = if is_tagged {
            (2, 0.0)
        } else {
            (1, 1.0 / (1.0 + Reference::DEFAULT_RATIO))
        };
        let mut data = Vec::<f32>::with_capacity(2 * n);
        for i in 0..(n_p * n) {
            let di = array.get_item(i)?;
            if is_tagged {
                data.push(di as f32);
            } else {
                data.push((di * r) as f32);
                data.push((di * (1.0 - r)) as f32);
            }
        }

        Self::check(&energy, &cos_theta)?;
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
        Self::check(&energy, &cos_theta)?;
        let table = Self { shape, energy, cos_theta, altitude, data };
        Ok(table)
    }
}

impl Flux {
    const ZERO: Self = Self { muon: 0.0, anti: 0.0 };
}

impl_dtype!(
    Flux,
    [
        ("value", "f8"),
        ("ratio", "f8"),
    ]
);

impl Altitude {
    pub fn min(&self) -> f64 {
        match self {
            Self::Scalar(s) => *s,
            Self::Range(r) => r.0,
        }
    }

    pub fn max(&self) -> f64 {
        match self {
            Self::Scalar(s) => *s,
            Self::Range(r) => r.1,
        }
    }

    pub fn to_range(&self) -> (f64, f64) {
        match self {
            Self::Scalar(s) => (*s, *s),
            Self::Range(r) => *r,
        }
    }
}


// ===============================================================================================
//
// Gaisser's flux model (in GeV^-1 m^-2 s^-1 sr^-1).
// Ref: see e.g. the ch.30 of the PDG (https://pdglive.lbl.gov).
//
// ===============================================================================================

const MUON_MASS: f64 = 0.10566;

fn flux_gaisser(cos_theta: f64, energy: f64) -> f64 {
    if cos_theta < 0.0 {
        0.0
    } else {
        let emu = energy + MUON_MASS;
        let ec = 1.1 * emu * cos_theta;
        let rpi = 1.0 + ec / 115.0;
        let rk = 1.0 + ec / 850.0;
        1.4E+03 * emu.powf(-2.7) * (1.0 / rpi + 0.054 / rk)
    }
}


// ===============================================================================================
//
// Volkova's parameterization of cos(theta*).
//
// This is a correction for the Earth curvature, relevant for close to
// horizontal trajectories.
//
// ===============================================================================================

fn cos_theta_star(cos_theta: f64) -> f64 {
    const P: [f64; 5] = [ 0.102573, -0.068287, 0.958633, 0.0407253, 0.817285 ];
    let cs2 = (
            cos_theta * cos_theta +
            P[0] * P[0] +
            P[1] * cos_theta.powf(P[2]) +
            P[3] * cos_theta.powf(P[4])
        ) / (
            1.0 +
            P[0] * P[0] +
            P[1] +
            P[3]
        );
    if cs2 > 0.0 {
        cs2.sqrt()
    } else {
        0.0
    }
}


// ===============================================================================================
//
// Guan et al. parameterization of the sea level flux of atmospheric muons.
// Ref: https://arxiv.org/abs/1509.06176.
//
// ===============================================================================================

fn flux_gccly(cos_theta: f64, energy: f64) -> f64 {
    let emu = energy + MUON_MASS;
    let cs = cos_theta_star(cos_theta);
    (1.0 + 3.64 / (emu * cs.powf(1.29))).powf(-2.7) * flux_gaisser(cs, energy)
}
