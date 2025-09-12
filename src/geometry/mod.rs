use crate::materials::set::MaterialsSet;
use crate::simulation::materials::Materials;
use crate::utils::io::PathString;
use pyo3::prelude::*;

pub mod earth;
pub mod external;

use earth::EarthGeometry;
use external::ExternalGeometry;


#[derive(FromPyObject, IntoPyObject)]
pub enum Geometry {
    Earth(Py<EarthGeometry>),
    External(Py<ExternalGeometry>),
}

#[derive(Clone, Copy)]
pub enum BoundGeometry<'a, 'py> {
    Earth(&'a Bound<'py, EarthGeometry>),
    External(&'a Bound<'py, ExternalGeometry>),
}

pub enum GeometryRef<'py> {
    Earth(PyRef<'py, EarthGeometry>),
    External(PyRef<'py, ExternalGeometry>),
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
            Self::External(geometry) => BoundGeometry::External(geometry.bind(py)),
        }
    }

    pub fn borrow<'py>(&self, py: Python<'py>) -> GeometryRef<'py> {
        match self {
            Self::Earth(geometry) => GeometryRef::Earth(geometry.bind(py).borrow()),
            Self::External(geometry) => GeometryRef::External(geometry.bind(py).borrow()),
        }
    }

    pub fn subscribe(&self, py: Python, set: &MaterialsSet) {
        match self {
            Self::Earth(geometry) => geometry.bind(py).borrow_mut().subscribe(py, set),
            Self::External(geometry) => geometry.bind(py).borrow_mut().subscribe(py, set),
        }
    }

    pub fn unsubscribe(&self, py: Python, set: &MaterialsSet) {
        match self {
            Self::Earth(geometry) => geometry.bind(py).borrow_mut().unsubscribe(py, set),
            Self::External(geometry) => geometry.bind(py).borrow_mut().unsubscribe(py, set),
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
            Self::External(geometry) => match other {
                Self::External(other) => geometry.is(other),
                _ => false,
            },
        }
    }
}

impl<'py> GeometryRef<'py> {
    pub fn materials(&self) -> &Materials {
        match self {
            Self::Earth(geometry) => &geometry.materials,
            Self::External(geometry) => &geometry.materials,
        }
    }
}

impl GeometryArg {
    pub fn into_geometry(self, py: Python) -> PyResult<Geometry> {
        let geometry = match self {
            Self::Object(geometry) => geometry,
            Self::Path(path) => {
                let geometry = unsafe { ExternalGeometry::new(py, path, None)? };
                Geometry::External(Py::new(py, geometry)?)
            },
        };
        Ok(geometry)
    }
}
