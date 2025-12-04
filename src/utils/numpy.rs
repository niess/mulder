#![allow(unused)]

use pyo3::prelude::*;
use pyo3::exceptions::{PyIndexError, PyValueError};
use pyo3::{ffi, PyTypeInfo};
use pyo3::types::{DerefToPyAny, PyCapsule};
use pyo3::sync::GILOnceCell;
use std::ffi::{c_char, c_int, c_uchar, c_void};
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::null_mut;
use std::sync::OnceLock;

#[repr(transparent)]
pub struct PyArray<T> (PyAny, PhantomData<T>);

pub enum AnyArray<'py, T> {
    Borrowed(Bound<'py, PyArray<T>>),
    Owned(Bound<'py, PyArray<T>>),
}

pub struct NewArray<'py, T> {
    array: Bound<'py, PyArray<T>>,
    size: usize,
}

pub struct Iter<'a, T> {
    object: &'a PyArrayObject<T>,
    index: usize,
}

#[repr(C)]
struct PyArrayObject<T> {
    pub object: ffi::PyObject,
    pub data: *mut c_char,
    pub nd: c_int,
    pub dimensions: *mut npy_intp,
    pub strides: *mut npy_intp,
    pub base: *mut ffi::PyObject,
    pub descr: *mut ffi::PyObject,
    pub flags: c_int,
    target: PhantomData<T>,
}

pub struct Data<'a, T> {
    object: &'a PyArrayObject<T>,
}

#[derive(FromPyObject)]
pub enum ShapeArg {
    Scalar(usize),
    Array(Vec<usize>),
}

#[allow(non_camel_case_types)]
type npy_intp = ffi::Py_intptr_t;

#[allow(non_camel_case_types)]
struct PyArray_API (*const *const c_void);

static API: OnceLock<PyArray_API> = OnceLock::new();

unsafe impl Send for PyArray_API {}
unsafe impl Sync for PyArray_API {}

pub fn initialise(py: Python) -> PyResult<()> {
    if API.get().is_some() {
        return Ok(())
    }

    let api = PyArray_API::new(py)?;
    let _ = API.set(api);

    Ok(())
}

macro_rules! impl_attr {
    ($name:ident, $slot:literal, $ret:ty) => {
        #[inline]
        fn $name(&self) -> $ret {
            self.object($slot)
        }
    }
}

macro_rules! impl_meth {
    ($name:ident, $slot:literal, ($($arg:ident: $arg_type:ty,)*) -> $ret:ty) => {
        #[inline]
        fn $name(
            &self,
            $(
                $arg: $arg_type,
            )*
        ) -> $ret {
            type Signature = extern "C" fn(
                $(
                    $arg: $arg_type,
                )*
            ) -> $ret;

            let f = unsafe { *self.function::<Signature>($slot) };
            f($($arg,)*)
        }
    }
}

impl PyArray_API {
    fn new(py: Python) -> PyResult<Self> {
        let ptr = PyModule::import(py, "numpy.core.multiarray")?
            .getattr("_ARRAY_API")?
            .downcast::<PyCapsule>()?
            .pointer() as *const *const c_void;
        Ok(Self(ptr))
    }

    #[inline]
    fn function<T>(&self, offset: isize) -> *const T {
        unsafe {
            self.0.offset(offset) as *const T
        }
    }

    #[inline]
    fn object<T>(&self, offset: isize) -> *mut T {
        unsafe {
            let tmp = self.0.offset(offset) as *mut *mut T;
            *tmp
        }
    }

    impl_attr!(ndarray, 2, *mut ffi::PyTypeObject);

    impl_meth!(empty, 184, (
            nd: c_int,
            dims: *const npy_intp,
            dtype: *mut ffi::PyObject,
            fortran: c_int,
        ) -> *mut ffi::PyObject
    );

    impl_meth!(equiv_types, 182, (
            type1: *mut ffi::PyObject,
            type2: *mut ffi::PyObject,
        ) -> c_uchar
    );

    impl_meth!(new_from_descriptor, 94, (
            subtype: *mut ffi::PyTypeObject,
            descr: *mut ffi::PyObject,
            nd: c_int,
            dims: *const npy_intp,
            strides: *const npy_intp,
            data: *mut c_void,
            flags: c_int,
            obj: *mut ffi::PyObject,
        ) -> *mut ffi::PyObject
    );

    impl_meth!(set_base_object, 282, (
            arr: *mut ffi::PyObject,
            obj: *mut ffi::PyObject,
        ) -> c_int
    );

    impl_meth!(zeros, 183, (
            nd: c_int,
            dims: *const npy_intp,
            dtype: *mut ffi::PyObject,
            fortran: c_int,
        ) -> *mut ffi::PyObject
    );
}

impl<T> AsRef<PyAny> for PyArray<T> {
    fn as_ref(&self) -> &PyAny {
        &self.0
    }
}

impl<T> Deref for PyArray<T> {
    type Target = PyAny;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefToPyAny for PyArray<T> {}

unsafe impl<T: Dtype> PyTypeInfo for PyArray<T> {
    const NAME: &'static str = "PyArray<T>";
    const MODULE: Option<&'static str> = Some("numpy");

    fn type_object_raw(_py: Python) -> *mut ffi::PyTypeObject {
        API.get().unwrap().ndarray()
    }

    fn is_type_of(object: &Bound<PyAny>) -> bool {
        let api = API.get().unwrap();
        if unsafe { ffi::PyObject_TypeCheck(object.as_ptr(), api.ndarray()) == 0 } {
            return false
        }

        let dtype = T::dtype(object.py()).unwrap().as_ptr();
        let array = unsafe { PyArrayObject::<T>::cast(object) };
        let mut same = array.descr == dtype;
        if !same {
            same = api.equiv_types(array.descr, dtype) != 0;
        }
        same
    }
}

impl<'py, T> NewArray<'py, T> {
    const WRITEABLE: c_int = 0x0400;

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        let object = unsafe { &*(self.array.as_ptr() as *const PyArrayObject<T>) };
        unsafe { std::slice::from_raw_parts(object.data as *const T, self.size) }
    }

    #[inline]
    pub fn as_slice_mut(&mut self) -> &mut [T] {
        let object = unsafe { &*(self.array.as_ptr() as *mut PyArrayObject<T>) };
        unsafe { std::slice::from_raw_parts_mut(object.data as *mut T, self.size) }
    }

    pub fn empty<S>(py: Python<'py>, shape: S) -> PyResult<Self>
    where
        S: IntoIterator<Item=usize, IntoIter: ExactSizeIterator>,
        T: Dtype,
    {
        Self::new::<S>(
            py,
            shape,
            |api, nd, dims, dtype, fortran| api.empty(nd, dims, dtype, fortran),
        )
    }

    pub fn from_array<A>(
        py: Python<'py>,
        array: A
    ) -> PyResult<Self>
    where
        A: ArrayMethods<Target=T>,
        T: Clone + Dtype,
    {
        let mut new_array = Self::empty(py, array.shape())?;
        let data = new_array.as_slice_mut();
        for i in 0..array.size() {
            data[i] = array.get_item(i)?;
        }
        Ok(new_array)
    }

    pub fn from_data<S, D>(py: Python<'py>, shape: S, data: D) -> PyResult<Self>
    where
        S: IntoIterator<Item=usize, IntoIter: ExactSizeIterator>,
        D: IntoIterator<Item=T, IntoIter: ExactSizeIterator>,
        T: Dtype,
    {
        let mut array = Self::empty(py, shape)?;
        let data = data.into_iter();
        if data.len() != array.size {
            return Err(PyValueError::new_err(format!(
                "bad shape (expected size = {}, found {})",
                data.len(),
                array.size(),
            )))
        }
        for (di, ai) in data.zip(array.iter_mut()) {
            *ai = di;
        }
        Ok(array)
    }

    pub unsafe fn from_slice<S>(
        data: &[T],
        owner: &Bound<'py, PyAny>,
        shape: Option<S>,
    ) -> PyResult<Self>
    where
        S: IntoIterator<Item=usize, IntoIter: ExactSizeIterator>,
        T: Dtype,
    {
        let py = owner.py();
        let api = API.get().unwrap();
        let dtype = T::dtype(py)?;
        let (ndim, shape) = match shape {
            Some(shape) => {
                let (ndim, shape) = Self::try_shape(shape)?;
                let size = shape.iter().copied().reduce(|a, b| a * b).unwrap_or(1) as usize;
                assert!(size == data.len());
                (ndim, shape)
            },
            None => (1, vec![data.len() as npy_intp]),
        };
        let array = api.new_from_descriptor(
            api.ndarray(),
            dtype.as_ptr(),
            ndim,
            shape.as_ptr() as *const npy_intp,
            null_mut(),
            data.as_ptr() as *mut c_void,
            Self::WRITEABLE,
            null_mut(),
        );
        if PyErr::occurred(py) {
            match PyErr::take(py) {
                None => unreachable!(),
                Some(err) => return Err(err),
            }
        }

        unsafe { pyo3::ffi::Py_INCREF(dtype.as_ptr()); }
        let ptr = owner.as_ptr();
        api.set_base_object(array, ptr);
        unsafe { pyo3::ffi::Py_INCREF(ptr); }

        let array = unsafe { Py::<PyArray<T>>::from_owned_ptr_or_err(py, array)? };
        let array = array.into_bound(py);
        let size = unsafe { (&*(array.as_ptr() as *const PyArrayObject<T>)).size() as usize };
        Ok(Self { array, size })
    }

    #[inline]
    pub fn into_bound(self) -> Bound<'py, PyArray<T>> {
        self.array
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item=&'_ T> + ExactSizeIterator {
        self.as_slice().iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item=&'_ mut T> + ExactSizeIterator {
        self.as_slice_mut().iter_mut()
    }

    pub fn readonly(self) -> Self {
        let object = unsafe { &mut *(self.array.as_ptr() as *mut PyArrayObject<T>) };
        object.flags &= !Self::WRITEABLE;
        self
    }

    #[inline]
    pub fn size(&self) -> usize {
        self.size
    }

    pub fn zeros<S>(py: Python<'py>, shape: S) -> PyResult<Self>
    where
        S: IntoIterator<Item=usize, IntoIter: ExactSizeIterator>,
        T: Dtype,
    {
        Self::new::<S>(
            py,
            shape,
            |api, nd, dims, dtype, fortran| api.zeros(nd, dims, dtype, fortran),
        )
    }
}

impl<'py, T> NewArray<'py, T> {
    fn new<S>(py: Python<'py>, shape: S, func: NewArrayFn) -> PyResult<Self>
    where
        S: IntoIterator<Item=usize, IntoIter: ExactSizeIterator>,
        T: Dtype,
    {
        let api = API.get().unwrap();
        let dtype = T::dtype(py)?;
        let (ndim, shape) = Self::try_shape(shape)?;
        let array = func(
            api,
            ndim,
            shape.as_ptr() as *const npy_intp,
            dtype.as_ptr(),
            0,
        );
        if PyErr::occurred(py) {
            match PyErr::take(py) {
                None => unreachable!(),
                Some(err) => return Err(err),
            }
        }
        unsafe { pyo3::ffi::Py_INCREF(dtype.as_ptr()); }
        let array = unsafe { Py::<PyArray<T>>::from_owned_ptr_or_err(py, array)? };
        let array = array.into_bound(py);
        let size = unsafe { (&*(array.as_ptr() as *const PyArrayObject<T>)).size() as usize };
        Ok(Self { array, size })
    }

    #[inline]
    fn try_shape<S>(shape: S) -> PyResult<(i32, Vec<npy_intp>)>
    where
        S: IntoIterator<Item=usize, IntoIter: ExactSizeIterator>,
    {
        let shape = shape.into_iter();
        let ndim = i32::try_from(shape.len())
            .map_err(|err| PyValueError::new_err(format!(
                "bad i32 value ({}: {})",
                shape.len(),
                err,
            )))?;
        let raw_shape = shape
            .map(|v| Self::try_size(v))
            .collect::<PyResult<Vec<_>>>()?;
        Ok((ndim, raw_shape))
    }

    #[inline]
    fn try_size(size: usize) -> PyResult<npy_intp> {
        npy_intp::try_from(size)
            .map_err(|err| PyValueError::new_err(format!(
                "bad npy_intp value ({}: {})",
                size,
                err,
            )))
    }
}

type NewArrayFn = fn(
    api: &PyArray_API,
    nd: c_int,
    dims: *const npy_intp,
    dtype: *mut ffi::PyObject,
    fortran: c_int,
) -> *mut ffi::PyObject;

impl<'py, T> IntoPyObject<'py> for NewArray<'py, T> {
    type Target = PyArray<T>;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    #[inline]
    fn into_pyobject(self, _py: Python<'py>) -> PyResult<Self::Output> {
        Ok(self.into_bound())
    }
}

impl<'py, T> AnyArray<'py, T> {
    #[inline]
    pub fn into_bound(self) -> Bound<'py, PyArray<T>> {
        match self {
            Self::Borrowed(array) => array,
            Self::Owned(array) => array,
        }
    }
}

impl<'py, T> AsRef<Bound<'py, PyArray<T>>> for AnyArray<'py, T> {
    #[inline]
    fn as_ref(&self) -> &Bound<'py, PyArray<T>> {
        match self {
            Self::Borrowed(array) => array,
            Self::Owned(array) => array,
        }
    }
}

impl<'py, T> Deref for AnyArray<'py, T> {
    type Target = Bound<'py, PyArray<T>>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'py, T: Dtype> FromPyObject<'py> for AnyArray<'py, T> {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        let array = if <PyArray<T>>::is_type_of(ob) {
            unsafe { Self::Borrowed(ob.downcast_unchecked().clone()) }
        } else {
            let py = ob.py();
            let array = PyModule::import(py, "numpy")?
                .getattr("asarray")?
                .call1((ob, T::dtype(py)?))?;
            unsafe { Self::Owned(array.downcast_into_unchecked()) }
        };
        Ok(array)
    }
}

pub trait ArrayMethods {
    type Target;

    fn data(&self) -> Data<'_, Self::Target>;
    fn get_item(&self, index: usize) -> PyResult<Self::Target>;
    fn get_unchecked(&self, index: usize) -> Self::Target;
    fn ndim(&self) -> usize;
    fn iter(&self) -> Iter<'_, Self::Target>;
    fn set_item(&self, index: usize, value: Self::Target) -> PyResult<()>;
    fn set_unchecked(&self, index: usize, value: Self::Target);
    fn shape(&self) -> Vec<usize>;
    fn size(&self) -> usize;
}

macro_rules! index_error {
    ($slf:ident, $index:ident) => {
        PyIndexError::new_err(format!(
            "expected index < {}, found {}",
            $slf.size(),
            $index,
        ))
    }
}

macro_rules! impl_methods {
    ($type:ty) => {
        impl<T: Clone> ArrayMethods for $type {
            type Target = T;

            #[inline]
            fn data(&self) -> Data<'_, Self::Target> {
                Data { object: self.as_object() }
            }

            #[inline]
            fn get_item(&self, index: usize) -> PyResult<Self::Target> {
                let value = self.as_object()
                    .get(index)
                    .ok_or_else(|| index_error!(self, index))?
                    .clone();
                Ok(value)
            }

            #[inline]
            fn get_unchecked(&self, index: usize) -> Self::Target {
                self.as_object()
                    .get(index)
                    .unwrap()
                    .clone()
            }

            #[inline]
            fn iter(&self) -> Iter<'_, Self::Target> {
                Iter { object: self.as_object(), index: 0 }
            }

            #[inline]
            fn ndim(&self) -> usize {
                self.as_object().nd as usize
            }

            #[inline]
            fn set_item(&self, index: usize, value: Self::Target) -> PyResult<()> {
                let dst = self
                    .as_object()
                    .get_mut(index)
                    .ok_or_else(|| index_error!(self, index))?;
                *dst = value;
                Ok(())
            }

            #[inline]
            fn set_unchecked(&self, index: usize, value: Self::Target) {
                let dst = self
                    .as_object()
                    .get_mut(index)
                    .unwrap();
                *dst = value;
            }

            #[inline]
            fn shape(&self) -> Vec<usize> {
                self.as_object()
                    .shape()
                    .iter()
                    .map(|v| *v as usize)
                    .collect()
            }

            #[inline]
            fn size(&self) -> usize {
                self.as_object().size() as usize
            }
        }
    }
}

impl_methods!(AnyArray<'_, T>);
impl_methods!(Bound<'_, PyArray<T>>);
impl_methods!(NewArray<'_, T>);

trait AsObject<T> {
    fn as_object(&self) -> &PyArrayObject<T>;
}

impl<T> AsObject<T> for AnyArray<'_, T> {
    #[inline]
    fn as_object(&self) -> &PyArrayObject<T> {
        unsafe { &*(self.as_ref().as_ptr() as *const PyArrayObject<T>) }
    }
}

impl<T> AsObject<T> for Bound<'_, PyArray<T>> {
    #[inline]
    fn as_object(&self) -> &PyArrayObject<T> {
        unsafe { &*(self.as_ptr() as *const PyArrayObject<T>) }
    }
}

impl<T> AsObject<T> for NewArray<'_, T> {
    #[inline]
    fn as_object(&self) -> &PyArrayObject<T> {
        unsafe { &*(self.array.as_ptr() as *const PyArrayObject<T>) }
    }
}

impl<'a, T: Clone> Iterator for Iter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.object
            .get(self.index)
            .map(|item| {
                self.index += 1;
                item.clone()
            })
    }
}

impl<'a, T: Clone> ExactSizeIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        (self.object.size() as usize) - self.index
    }
}

impl<'a, T: Clone> std::iter::FusedIterator for Iter<'a, T> {}

static EMPTY_SLICE: [npy_intp; 0] = [];

impl<T> PyArrayObject<T> {
    unsafe fn cast<'a>(ob: &'a Bound<PyAny>) -> &'a Self {
        &*(ob.as_ptr() as *const PyArrayObject<T>)
    }

    fn get(&self, index: usize) -> Option<&T> {
        self.offset_of(index)
            .map(|offset| {
                let data = unsafe { self.data.offset(offset as isize) };
                unsafe { std::mem::transmute::<*mut c_char, &T>(data) }
            })
    }

    fn get_mut(&self, index: usize) -> Option<&mut T> {
        self.offset_of(index)
            .map(|offset| {
                let data = unsafe { self.data.offset(offset as isize) };
                unsafe { std::mem::transmute::<*mut c_char, &mut T>(data) }
            })
    }

    fn offset_of(&self, index: usize) -> Option<isize> {
        let shape = self.shape();
        let strides = self.strides();
        let n = shape.len();
        if n == 0 {
            Some(0)
        } else {
            let mut remainder = index;
            let mut offset = 0_isize;
            let mut size = 1_usize;
            for i in (0..n).rev() {
                let m = shape[i] as usize;
                size *= m;
                let j = remainder % m;
                remainder = (remainder - j) / m;
                offset += (j as isize) * strides[i];
            }
            if index >= size {
                None
            } else {
                Some(offset)
            }
        }
    }

    #[inline]
    fn shape(&self) -> &[npy_intp] {
        if self.nd == 0 {
            &EMPTY_SLICE
        } else {
            unsafe { std::slice::from_raw_parts(self.dimensions, self.nd as usize) }
        }
    }

    #[inline]
    fn size(&self) -> npy_intp {
        if self.nd == 0 {
            1
        } else {
            self.shape()
                .iter()
                .product::<npy_intp>()
        }
    }

    #[inline]
    fn strides(&self) -> &[npy_intp] {
        if self.nd == 0 {
            &EMPTY_SLICE
        } else {
            unsafe { std::slice::from_raw_parts(self.strides, self.nd as usize) }
        }
    }
}

impl<'a, T> Data<'a, T> {
    pub fn get(&self, index: usize) -> Option<&T> {
        self.object.get(index)
    }
}

unsafe impl<'a, T: Clone> Send for Data<'a, T> {}
unsafe impl<'a, T: Clone> Sync for Data<'a, T> {}

pub trait Dtype {
    fn dtype<'py>(py: Python<'py>) -> PyResult<&'py Bound<'py, PyAny>>;
}

macro_rules! impl_dtype {
    ($type:ty, $def:tt) => {
        paste::paste! {
            static [< $type:upper _DTYPE >]: pyo3::sync::GILOnceCell<PyObject> =
                pyo3::sync::GILOnceCell::new();

            impl crate::utils::numpy::Dtype for $type {
                fn dtype<'py>(py: Python<'py>) -> PyResult<&'py Bound<'py, PyAny>> {
                    let ob = [< $type:upper _DTYPE >].get_or_try_init(py, || -> PyResult<_> {
                        let ob = PyModule::import(py, "numpy")?
                            .getattr("dtype")?
                            .call1(($def, true /* C alignment */))?
                            .unbind();
                        Ok(ob)
                    })?
                    .bind(py);
                    Ok(ob)
                }
            }
        }
    }
}

pub(crate) use impl_dtype;

impl_dtype!(bool, "bool");
impl_dtype!(i8, "i1");
impl_dtype!(i16, "i2");
impl_dtype!(i32, "i4");
impl_dtype!(i64, "i8");
impl_dtype!(f32, "f4");
impl_dtype!(f64, "f8");
impl_dtype!(u8, "u1");
impl_dtype!(u16, "u2");
impl_dtype!(u32, "u4");
impl_dtype!(u64, "u8");

impl ShapeArg {
    #[inline]
    pub fn into_vec(self) -> Vec<usize> {
        self.into()
    }
}

impl From<ShapeArg> for Vec<usize> {
    #[inline]
    fn from(value: ShapeArg) -> Self {
        match value {
            ShapeArg::Scalar(s) => vec![s],
            ShapeArg::Array(a) => a,
        }
    }
}
