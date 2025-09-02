use crate::utils::numpy::{NewArray, ShapeArg};
use pyo3::prelude::*;
use pyo3::exceptions::PySystemError;
use rand::Rng;
use rand::distr::Open01;
use rand::SeedableRng;
use rand_pcg::Pcg64Mcg;


// ===============================================================================================
//
// Generator interface.
//
// ===============================================================================================

/// A Pseudo-Random Numbers Generator (PRNG).
#[derive(Clone)]
#[pyclass(module = "mulder")]
pub struct Random {
    rng: Pcg64Mcg,
    /// Prng stream index.
    #[pyo3(get)]
    index: u128,
    /// Prng initial seed.
    #[pyo3(get)]
    seed: u128,
}

#[pymethods]
impl Random {
    #[new]
    #[pyo3(signature=(seed=None, *, index=None))]
    pub fn new(seed: Option<u128>, index: Option<Index>) -> PyResult<Self> {
        let rng = Pcg64Mcg::new(0xCAFEF00DD15EA5E5);
        let mut random = Self { rng, seed: 0, index: 0 };
        random.initialise(seed)?;
        if index.is_some() {
            random.set_index(index)?;
        }
        Ok(random)
    }

    #[setter]
    fn set_index(&mut self, index: Option<Index>) -> PyResult<()> {
        match index {
            None => self.initialise(Some(self.seed))?,
            Some(index) => {
                let index: u128 = index.into();
                let delta: u128 = index.wrapping_sub(self.index);
                self.rng.advance(delta);
                self.index = index;
            },
        }
        Ok(())
    }

    #[setter]
    fn set_seed(&mut self, seed: Option<u128>) -> PyResult<()> {
        self.initialise(seed)
    }

    /// Generate pseudo-random number(s) uniformly distributed over (0,1).
    fn uniform01(
        &mut self,
        py: Python,
        shape: Option<ShapeArg>,
    ) -> PyResult<PyObject> {
        match shape {
            None => {
                let value = self.open01();
                Ok(value.into_pyobject(py)?.into_any().unbind())
            },
            Some(shape) => {
                let shape = shape.into_vec();
                let n = shape.iter().product();
                let mut array = NewArray::<f64>::empty(py, shape)?;
                let u = array.as_slice_mut();
                for i in 0..n {
                    u[i] = self.open01();
                }
                Ok(array.into_bound().into_any().unbind())
            },
        }
    }
}

impl Random {
    fn initialise(&mut self, seed: Option<u128>) -> PyResult<()> {
        match seed {
            None => {
                let mut seed = [0_u8; 16];
                getrandom::fill(&mut seed)
                    .map_err(|_| PySystemError::new_err("could not seed random engine"))?;
                self.rng = Pcg64Mcg::from_seed(seed);
                self.seed = u128::from_ne_bytes(seed);
            },
            Some(seed) => {
                self.seed = seed;
                let seed = u128::to_ne_bytes(seed);
                self.rng = Pcg64Mcg::from_seed(seed);
            },
        }
        self.index = 0;
        Ok(())
    }

    #[inline]
    pub fn open01(&mut self) -> f64 {
        self.index += 1;
        self.rng.sample::<f64, Open01>(Open01)
    }
}

#[derive(FromPyObject)]
pub enum Index {
    #[pyo3(transparent, annotation = "[u64;2]")]
    Array([u64; 2]),
    #[pyo3(transparent, annotation = "u128")]
    Scalar(u128),
}

impl From<Index> for u128 {
    fn from(value: Index) -> Self {
        match value {
            Index::Array(value) => ((value[0] as u128) << 64) + (value[1] as u128),
            Index::Scalar(value) => value,
        }
    }
}
