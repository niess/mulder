use crate::simulation::coordinates::{GeographicCoordinates, LocalFrame, LocalTransformer};
use crate::utils::numpy::{impl_dtype, NewArray};
use pyo3::prelude::*;


#[repr(C)]
#[derive(Debug)]
pub struct GeographicIntersection {
    pub before: i32,
    pub after: i32,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub distance: f64,
}

#[repr(C)]
#[derive(Debug)]
pub struct LocalIntersection {
    pub before: i32,
    pub after: i32,
    pub position: [f64; 3],
    pub distance: f64,
}

#[derive(IntoPyObject)]
pub enum IntersectionsArray<'py> {
    Geographic(NewArray<'py, GeographicIntersection>),
    Local(NewArray<'py, LocalIntersection>),
}

pub enum IntersectionsSlice<'a> {
    Geographic(&'a mut [GeographicIntersection]),
    Local(&'a mut [LocalIntersection]),
}

impl_dtype!(
    GeographicIntersection,
    [
        ("before",    "i4"),
        ("after",     "i4"),
        ("latitude",  "f8"),
        ("longitude", "f8"),
        ("altitude",  "f8"),
        ("distance",  "f8"),
    ]
);

impl_dtype!(
    LocalIntersection,
    [
        ("before",    "i4"),
        ("after",     "i4"),
        ("position",  "3f8"),
        ("distance",  "f8")
    ]
);

impl<'py> IntersectionsArray<'py> {
    pub fn as_slice_mut<'a>(&'a mut self) -> IntersectionsSlice<'a> {
        match self {
            Self::Geographic(array) => IntersectionsSlice::Geographic(array.as_slice_mut()),
            Self::Local(array) => IntersectionsSlice::Local(array.as_slice_mut()),
        }
    }
}

impl<'a> IntersectionsSlice<'a> {
    pub fn set_geographic(
        &mut self,
        index: usize,
        value: GeographicIntersection,
        frame: Option<&LocalFrame>,
    ) {
        match self {
            Self::Geographic(slice) => slice[index] = value,
            Self::Local(slice) => {
                let frame = frame.unwrap();
                let GeographicIntersection {
                    before, after, latitude, longitude, altitude, distance
                } = value;
                let position = GeographicCoordinates { latitude, longitude, altitude };
                let position = frame.from_ecef_position(position.to_ecef());
                slice[index] = LocalIntersection { before, after, position, distance };
            },
        }
    }

    pub fn set_local(
        &mut self,
        index: usize,
        mut value: LocalIntersection,
        frame: Option<&LocalFrame>,
        transformer: Option<&LocalTransformer>,
    ) {
        match self {
            Self::Geographic(slice) => {
                let frame = frame.unwrap();
                let LocalIntersection { before, after, position, distance } = value;
                let GeographicCoordinates { latitude, longitude, altitude } =
                    frame.to_geographic_position(&position);
                slice[index] = GeographicIntersection {
                    before, after, latitude, longitude, altitude, distance
                };
            },
            Self::Local(slice) => {
                if let Some(transformer) = transformer {
                    value.position = transformer.inverse_transform_point(&value.position);
                }
                slice[index] = value;
            },
        }
    }
}
