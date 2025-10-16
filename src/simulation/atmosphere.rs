use crate::materials::{MaterialsSet, MaterialsSubscriber};
use crate::utils::convert::AtmosphericModel;
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::{ValueError, TypeError};
use crate::utils::numpy::{AnyArray, ArrayMethods, NewArray};
use enum_variants_strings::EnumVariantsStrings;
use pyo3::prelude::*;


#[pyclass(module="mulder")]
pub struct Atmosphere {
    lambda: Vec<f64>,
    rho: Vec<f64>,
    z: Vec<f64>,
    model: ModelAttribute,

    #[pyo3(get)]
    /// The constitutive material.
    pub material: String,

    pub subscribers: Vec<MaterialsSubscriber>,
}

#[derive(FromPyObject)]
pub enum AtmosphereArg<'py> {
    Model(AtmosphereLike<'py>),
    Object(Py<Atmosphere>),
}

#[derive(FromPyObject)]
pub enum AtmosphereLike<'py> {
    Model(AtmosphericModel),
    Data(AnyArray<'py, f64>),
}

#[derive(Clone, Copy)]
pub struct Density {
    pub value: f64,
    pub lambda: f64,
}

enum ModelAttribute {
    Model(AtmosphericModel),
    Data(Option<PyObject>),
}

#[pymethods]
impl Atmosphere {
    /// Predefined atmospheric models.
    #[classattr]
    fn models() -> Vec<String> {
        match AtmosphericModel::from_str("") {
            Err(models) => models
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            Ok(_) => unreachable!(),
        }
    }

    #[pyo3(signature=(model=None, /, *, material=None))]
    #[new]
    pub fn new(model: Option<AtmosphereLike>, material: Option<String>) -> PyResult<Self> {
        let model = model
            .unwrap_or_else(|| AtmosphereLike::Model(AtmosphericModel::default()));

        const WHAT: &str = "model";
        let (z, rho, model) = match model {
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
                    AtmosphericModel::MidlatitudeSummer => vec![
                        1.21427E+00, 1.09462E+00, 9.85896E-01, 8.89523E-01, 8.03013E-01,
                        7.23736E-01, 6.50253E-01, 5.83173E-01, 5.22424E-01, 4.67324E-01,
                        4.16297E-01, 3.70189E-01, 3.27664E-01, 2.89095E-01, 2.47211E-01,
                        2.10040E-01, 1.79361E-01, 1.53491E-01, 1.30554E-01, 1.11175E-01,
                        9.45857E-02, 8.06408E-02, 6.87154E-02, 5.88097E-02, 5.01061E-02,
                        4.28883E-02, 2.90924E-02, 1.96867E-02, 1.35605E-02, 9.26634E-03,
                        6.43402E-03, 4.50717E-03, 3.18526E-03, 2.27257E-03, 1.63398E-03,
                        1.20216E-03, 6.66476E-04, 3.68725E-04, 2.01770E-04, 1.07088E-04,
                        5.33275E-05, 2.40237E-05, 9.36433E-06, 3.38794E-06, 1.18024E-06,
                        4.44537E-07, 1.68205E-07, 7.22922E-08, 3.35673E-08, 1.72495E-08,
                    ],
                    AtmosphericModel::MidlatitudeWinter => vec![
                        1.30712E+00, 1.16618E+00, 1.03950E+00, 9.25416E-01, 8.29664E-01,
                        7.41867E-01, 6.61874E-01, 5.89139E-01, 5.22250E-01, 4.62219E-01,
                        4.07393E-01, 3.49636E-01, 2.99914E-01, 2.57310E-01, 2.20620E-01,
                        1.89027E-01, 1.61955E-01, 1.38777E-01, 1.18917E-01, 1.01703E-01,
                        8.69883E-02, 7.41974E-02, 6.33299E-02, 5.40973E-02, 4.63217E-02,
                        3.95175E-02, 2.66207E-02, 1.77969E-02, 1.19544E-02, 7.91988E-03,
                        5.32801E-03, 3.62574E-03, 2.50147E-03, 1.73929E-03, 1.23582E-03,
                        8.95849E-04, 4.84229E-04, 2.61252E-04, 1.37430E-04, 7.10233E-05,
                        3.51077E-05, 1.70850E-05, 7.87425E-06, 3.38325E-06, 1.41760E-06,
                        6.11832E-07, 2.69501E-07, 1.26426E-07, 6.09529E-08, 3.12165E-08,
                    ],
                    AtmosphericModel::SubarticSummer => vec![
                        1.23479E+00, 1.11438E+00, 1.00439E+00, 9.03353E-01, 8.10508E-01,
                        7.26139E-01, 6.53068E-01, 5.85016E-01, 5.23306E-01, 4.66521E-01,
                        4.14320E-01, 3.55936E-01, 3.05973E-01, 2.63080E-01, 2.25957E-01,
                        1.94990E-01, 1.67148E-01, 1.43634E-01, 1.23486E-01, 1.06175E-01,
                        9.13159E-02, 7.84770E-02, 6.74652E-02, 5.80403E-02, 4.96733E-02,
                        4.24796E-02, 2.90154E-02, 1.98646E-02, 1.36518E-02, 9.31921E-03,
                        6.46286E-03, 4.52111E-03, 3.20737E-03, 2.31825E-03, 1.69072E-03,
                        1.24111E-03, 6.83306E-04, 3.82093E-04, 2.13743E-04, 1.14253E-04,
                        5.76071E-05, 2.55384E-05, 9.62618E-06, 3.39594E-06, 1.15377E-06,
                        4.27555E-07, 1.59742E-07, 6.89517E-08, 3.27678E-08, 1.71738E-08,
                    ],
                    AtmosphericModel::SubarticWinter => vec![
                        1.37406E+00, 1.19566E+00, 1.05980E+00, 9.38361E-01, 8.35184E-01,
                        7.46496E-01, 6.65128E-01, 5.91032E-01, 5.22706E-01, 4.53941E-01,
                        3.88011E-01, 3.31700E-01, 2.83372E-01, 2.42307E-01, 2.07156E-01,
                        1.77006E-01, 1.51760E-01, 1.30025E-01, 1.11368E-01, 9.53073E-02,
                        8.16027E-02, 6.97735E-02, 5.96753E-02, 5.10198E-02, 4.35904E-02,
                        3.72286E-02, 2.46876E-02, 1.64600E-02, 1.10215E-02, 7.37168E-03,
                        4.92888E-03, 3.33096E-03, 2.27257E-03, 1.57050E-03, 1.08723E-03,
                        7.68901E-04, 4.02194E-04, 2.15330E-04, 1.10839E-04, 5.67898E-05,
                        2.96980E-05, 1.50365E-05, 7.28868E-06, 3.40677E-06, 1.44733E-06,
                        6.35381E-07, 2.82636E-07, 1.32722E-07, 6.21069E-08, 3.11288E-08,
                    ],
                    AtmosphericModel::Tropical => vec![
                        1.19709E+00, 1.08582E+00, 9.84468E-01, 8.83233E-01, 7.98505E-01,
                        7.22312E-01, 6.51454E-01, 5.86158E-01, 5.26793E-01, 4.70834E-01,
                        4.20659E-01, 3.74127E-01, 3.31992E-01, 2.92317E-01, 2.58560E-01,
                        2.25861E-01, 1.96384E-01, 1.67628E-01, 1.38344E-01, 1.14493E-01,
                        9.52588E-02, 7.93904E-02, 6.64072E-02, 5.62130E-02, 4.77018E-02,
                        4.04552E-02, 2.70680E-02, 1.83067E-02, 1.24930E-02, 8.60275E-03,
                        5.97720E-03, 4.18499E-03, 2.95589E-03, 2.09273E-03, 1.49982E-03,
                        1.10166E-03, 6.03483E-04, 3.29102E-04, 1.78688E-04, 9.23254E-05,
                        4.49028E-05, 2.07443E-05, 8.57436E-06, 3.31224E-06, 1.25689E-06,
                        4.97255E-07, 1.95932E-07, 8.31279E-08, 3.58794E-08, 1.68312E-08,
                    ],
                    AtmosphericModel::UsStandard => vec![
                        1.23114E+00, 1.11643E+00, 1.00982E+00, 9.11106E-01, 8.20488E-01,
                        7.37320E-01, 6.60602E-01, 5.90707E-01, 5.26182E-01, 4.67394E-01,
                        4.13654E-01, 3.64933E-01, 3.12034E-01, 2.66687E-01, 2.27881E-01,
                        1.94749E-01, 1.66475E-01, 1.42335E-01, 1.21658E-01, 1.04011E-01,
                        8.89117E-02, 7.57361E-02, 6.45320E-02, 5.50109E-02, 4.69565E-02,
                        4.00898E-02, 2.71209E-02, 1.84172E-02, 1.21371E-02, 8.46808E-03,
                        5.95314E-03, 3.99601E-03, 2.79047E-03, 1.96674E-03, 1.40413E-03,
                        1.02713E-03, 5.67899E-04, 3.09002E-04, 1.62820E-04, 8.28524E-05,
                        4.01375E-05, 1.84266E-05, 8.14560E-06, 3.35597E-06, 1.35814E-06,
                        5.38466E-07, 2.21851E-07, 9.18219E-08, 3.99264E-08, 2.03728E-08,
                    ],
                };
                (z, rho, ModelAttribute::Model(model))
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
                    z.push(zi);
                    rho.push(ri);
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
                (z, rho, ModelAttribute::Data(None))
            },
        };

        let n = z.len();
        let mut lambda = Vec::with_capacity(n - 1);
        for i in 0..(n - 1) {
            lambda.push((z[i + 1] - z[i]) / (rho[i + 1] / rho[i]).ln())
        }

        let material = material
            .unwrap_or_else(|| Self::DEFAULT_MATERIAL.to_owned());
        let subscribers = Vec::new();

        Ok(Atmosphere { lambda, z, rho, model, material, subscribers })
    }

    #[setter]
    fn set_material(&mut self, value: &str) {
        if value != self.material.as_str() {
            self.subscribers.retain(|subscriber| {
                subscriber.replace(self.material.as_str(), value)
            });
            self.material = value.to_owned();
        }
    }

    /// The density model.
    #[getter]
    fn get_model<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let model = match &mut self.model {
            ModelAttribute::Model(model) => model.into_pyobject(py)?.into_any(),
            ModelAttribute::Data(ref mut data) => match data {
                Some(ob) => ob.clone_ref(py).into_bound(py),
                None => {
                    let n = self.z.len();
                    let mut array = NewArray::<f64>::empty(py, [n, 2])?
                        .readonly();
                    let values = array.as_slice_mut();
                    for i in 0..n {
                        values[2 * i] = self.z[i];
                        values[2 * i + 1] = self.rho[i];
                    }
                    let array = array.into_bound().into_any();
                    data.replace(array.clone().unbind());
                    array
                },
            },
        };
        Ok(model)
    }

    /// Computes the density value(s) at the specified altitude(s).
    #[pyo3(signature=(altitude, /))]
    fn density<'py>(&self, altitude: AnyArray<'py, f64>) -> PyResult<NewArray<'py, f64>> {
        let py = altitude.py();
        let mut array = NewArray::<f64>::empty(py, altitude.shape())?;
        let rho = array.as_slice_mut();
        for i in 0..altitude.size() {
            let zi = altitude.get_item(i)?;
            rho[i] = self.compute_density(zi).value;
        }
        Ok(array)
    }
}

impl Atmosphere {
    pub const DEFAULT_MATERIAL: &str = "Air";

    pub fn compute_density(&self, z: f64) -> Density {
        if z < self.z[0] {
            return Density { value: self.rho[0], lambda: self.lambda[0] }
        } else {
            let n = self.z.len();
            for i in 1..n {
                if z < self.z[i] {
                    let lbd = self.lambda[i - 1];
                    let u = (z - self.z[i - 1]) / lbd;
                    return Density { value: self.rho[i - 1] * u.exp(), lambda: lbd }
                }
            }
            return Density { value: self.rho[n - 1], lambda: self.lambda[n - 2] }
        }
    }

    pub fn subscribe(&mut self, set: &MaterialsSet) {
        set.add(self.material.as_str());
        self.subscribers.push(set.subscribe());
        self.subscribers.retain(|s| s.is_alive());
    }

    pub fn unsubscribe(&mut self, set: &MaterialsSet) {
        set.remove(self.material.as_str());
        self.subscribers.retain(|s| s.is_alive() && !s.is_subscribed(set));
    }
}

impl Default for Atmosphere {
    fn default() -> Self {
        Self::new(None, None).unwrap()
    }
}

impl<'py> AtmosphereArg<'py> {
    pub fn into_atmosphere(self, py: Python<'py>) -> PyResult<Py<Atmosphere>> {
        match self {
            Self::Model(model) => Py::new(py, Atmosphere::new(Some(model), None)?),
            Self::Object(atmosphere) => Ok(atmosphere),
        }
    }
}
