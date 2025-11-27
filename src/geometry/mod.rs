use crate::materials::set::MaterialsSet;
use crate::utils::io::PathString;
use pyo3::prelude::*;

pub mod earth;
pub mod local;

pub use earth::EarthGeometry;
pub use local::LocalGeometry;


#[derive(FromPyObject, IntoPyObject)]
pub enum Geometry {
    Earth(Py<EarthGeometry>),
    Local(Py<LocalGeometry>),
}

#[derive(Clone, Copy)]
pub enum BoundGeometry<'a, 'py> {
    Earth(&'a Bound<'py, EarthGeometry>),
    Local(&'a Bound<'py, LocalGeometry>),
}

pub enum GeometryRef<'py> {
    Earth(PyRef<'py, EarthGeometry>),
    Local(PyRef<'py, LocalGeometry>),
}

#[derive(FromPyObject)]
pub enum GeometryArg {
    Object(Geometry),
    Path(PathString),
}

impl Geometry {
    pub fn bind<'a, 'py>(&'a self, py: Python<'py>) -> BoundGeometry<'a, 'py> {
        match self {
            Self::Earth(geometry) => BoundGeometry::Earth(geometry.bind(py)),
            Self::Local(geometry) => BoundGeometry::Local(geometry.bind(py)),
        }
    }

    pub fn borrow<'py>(&self, py: Python<'py>) -> GeometryRef<'py> {
        match self {
            Self::Earth(geometry) => GeometryRef::Earth(geometry.bind(py).borrow()),
            Self::Local(geometry) => GeometryRef::Local(geometry.bind(py).borrow()),
        }
    }

    pub fn subscribe(&self, py: Python, set: &MaterialsSet) {
        match self {
            Self::Earth(geometry) => geometry.bind(py).borrow_mut().subscribe(py, set),
            Self::Local(geometry) => geometry.bind(py).borrow_mut().subscribe(py, set),
        }
    }

    pub fn unsubscribe(&self, py: Python, set: &MaterialsSet) {
        match self {
            Self::Earth(geometry) => geometry.bind(py).borrow_mut().unsubscribe(py, set),
            Self::Local(geometry) => geometry.bind(py).borrow_mut().unsubscribe(py, set),
        }
    }
}

impl<'a, 'py> BoundGeometry<'a, 'py> {
    pub fn is(self, other: Self) -> bool {
        match self {
            Self::Earth(geometry) => match other {
                Self::Earth(other) => geometry.is(other),
                _ => false,
            },
            Self::Local(geometry) => match other {
                Self::Local(other) => geometry.is(other),
                _ => false,
            },
        }
    }
}

impl GeometryArg {
    pub fn into_geometry(self, py: Python) -> PyResult<Geometry> {
        let geometry = match self {
            Self::Object(geometry) => geometry,
            Self::Path(path) => {
                let geometry = LocalGeometry::new(py, path, None)?;
                Geometry::Local(Py::new(py, geometry)?)
            },
        };
        Ok(geometry)
    }
}
