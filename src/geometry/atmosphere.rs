use crate::utils::convert::AtmosphericModel;
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::{ValueError, TypeError};
use crate::utils::numpy::{AnyArray, ArrayMethods};
use pyo3::prelude::*;


#[pyclass(frozen, module="mulder")]
pub struct Atmosphere {
    lambda: Vec<f64>,
    rho: Vec<f64>,
    z: Vec<f64>,
}

#[derive(FromPyObject)]
pub enum AtmosphereLike<'py> {
    Model(AtmosphericModel),
    Data(AnyArray<'py, f64>),
}

#[pymethods]
impl Atmosphere {
    #[pyo3(signature=(model, /))]
    #[new]
    fn new(model: AtmosphereLike) -> PyResult<Self> {
        const WHAT: &str = "model";
        let (z, rho) = match model {
            AtmosphereLike::Model(model) => {
                let z = vec![
                      0.00E+03,   1.00E+03,   2.00E+03,   3.00E+03,   4.00E+03,   5.00E+03,
                      6.00E+03,   7.00E+03,   8.00E+03,   9.00E+03,  10.00E+03,  11.00E+03,
                     12.00E+03,  13.00E+03,  14.00E+03,  15.00E+03,  16.00E+03,  17.00E+03,
                     18.00E+03,  19.00E+03,  20.00E+03,  21.00E+03,  22.00E+03,  23.00E+03,
                     24.00E+03,  25.00E+03,  27.50E+03,  30.00E+03,  32.50E+03,  35.00E+03,
                     37.50E+03,  40.00E+03,  42.50E+03,  45.00E+03,  47.50E+03,  50.00E+03,
                     55.00E+03,  60.00E+03,  65.00E+03,  70.00E+03,  75.00E+03,  80.00E+03,
                     85.00E+03,  90.00E+03,  95.00E+03, 100.00E+03, 105.00E+03, 110.00E+03,
                    115.00E+03, 120.00E+03,
                ];
                let rho = match model {
                    AtmosphericModel::USStandard => vec![],
                };
                (z, rho)
            },
            AtmosphereLike::Data(data) => {
                if data.ndim() != 2 {
                    let why = format!("expected a 2d array, found {}d", data.ndim(),);
                    return Err(Error::new(TypeError).what(WHAT).why(&why).to_err())
                }
                let shape = data.shape();
                if shape[1] != 2 {
                    let why = format!("expected an Nx2 array, found Nx{}", shape[1]);
                    return Err(Error::new(TypeError).what(WHAT).why(&why).to_err())
                }
                let n = shape[0];
                if n < 2 {
                    let why = format!("expected a 2 or more length array, found {}", n);
                    return Err(Error::new(TypeError).what(WHAT).why(&why).to_err())
                }
                let mut rho = Vec::with_capacity(n);
                let mut z = Vec::with_capacity(n);
                for i in 0..n {
                    let zi = data.get_item(2 * i)?;
                    let ri = data.get_item(2 * i + 1)?;
                    if ri <= 0.0 {
                        let why = format!("expected a strictly positive value, found {}", ri);
                        return Err(Error::new(ValueError).what("density").why(&why).to_err())
                    }
                    z[i] = zi;
                    rho[i] = ri;
                    if i > 0 {
                        let i0 = i - 1;
                        let z0 = z[i0];
                        if z0 >= zi {
                            let why = format!(
                                "expected strictly increasing values, found {}, {}", z0, zi
                            );
                            return Err(Error::new(ValueError).what("height").why(&why).to_err())
                        }
                    }
                }
                (z, rho)
            },
        };

        let n = z.len();
        let mut lambda = Vec::with_capacity(n - 1);
        for i in 0..(n - 1) {
            lambda[i] = (z[i + 1] - z[i]) / (rho[i + 1] / rho[i]).ln()
        }

        Ok(Atmosphere { lambda, z, rho })
    }
}
