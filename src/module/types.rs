#![allow(non_snake_case)]

use crate::utils::error::Error;
use crate::utils::error::ErrorKind::ValueError;
use crate::materials::{Component, Element, Material, MaterialsBroker, Mixture};
use crate::utils::ptr::{Destroy, null_pointer_err, OwnedPtr};
use paste::paste;
use pyo3::prelude::*;
use std::collections::HashSet;
use std::ffi::{c_char, c_double, c_int, CStr, CString};
use std::ptr::NonNull;


#[repr(C)]
pub struct CModule {
    element: Option<
        extern "C" fn(symbol: *const c_char) -> *mut CElement
    >,
    geometry: Option<
        extern "C" fn() -> *mut CGeometry
    >,
    material: Option<
        extern "C" fn(name: *const c_char) -> *mut CMaterial
    >,
}

#[repr(C)]
pub struct CGeometry {
    destroy: Option<
        extern "C" fn(*mut CGeometry)
    >,
    media_len: Option<
        extern "C" fn(*const CGeometry) -> usize
    >,
    medium: Option<
        extern "C" fn(*const CGeometry, index: usize) -> *mut CMedium
    >,
    tracer: Option<
        extern "C" fn(*const CGeometry) -> *mut CTracer
    >,
}

#[repr(C)]
pub struct CMedium {
    destroy: Option<
        extern "C" fn(*mut CMedium)
    >,
    material: Option<extern "C" fn(
        *const CMedium) -> *const c_char
    >,
    density: Option<extern "C" fn(
        *const CMedium) -> c_double
    >,
    description: Option<extern "C" fn(
        *const CMedium) -> *const c_char
    >,
}

#[repr(C)]
pub struct CMaterial {
    destroy: Option<
        extern "C" fn(*mut CMaterial)
    >,
    component: Option<
        extern "C" fn(*const CMaterial, usize) -> *mut CComponent
    >,
    components_len: Option<
        extern "C" fn(*const CMaterial) -> usize
    >,
    density: Option<extern "C" fn(
        *const CMaterial) -> c_double
    >,
    I: Option<extern "C" fn(
        *const CMaterial) -> c_double
    >,
}

#[repr(C)]
pub struct CComponent {
    destroy: Option<
        extern "C" fn(*mut CComponent)
    >,
    symbol: Option<extern "C" fn(
        *const CComponent) -> *const c_char
    >,
    weight: Option<extern "C" fn(
        *const CComponent) -> c_double
    >,
}

#[repr(C)]
pub struct CElement {
    destroy: Option<
        extern "C" fn(*mut CElement)
    >,
    A: Option<extern "C" fn(
        *const CElement) -> c_double
    >,
    I: Option<extern "C" fn(
        *const CElement) -> c_double
    >,
    Z: Option<extern "C" fn(
        *const CElement) -> c_int
    >,
}

#[repr(C)]
pub struct CTracer {
    destroy: Option<
        extern "C" fn(*mut CTracer)
    >,
    pub locate: Option<
        extern "C" fn(*mut CTracer, position: CVec3) -> usize
    >,
    pub reset: Option<
        extern "C" fn(*mut CTracer, position: CVec3, direction: CVec3)
    >,
    pub trace: Option<
        extern "C" fn(*mut CTracer, max_length: c_double) -> c_double
    >,
    pub move_: Option<
        extern "C" fn(*mut CTracer, length: c_double)
    >,
    pub turn: Option<
        extern "C" fn(*mut CTracer, direction: CVec3)
    >,
    pub medium: Option<
        extern "C" fn(*mut CTracer) -> usize
    >,
    pub position: Option<
        extern "C" fn(*mut CTracer) -> CVec3
    >,
}

#[repr(C)]
pub struct CVec3 {
    pub x: c_double,
    pub y: c_double,
    pub z: c_double,
}

// ===============================================================================================
// C structs wrappers.
// ===============================================================================================

macro_rules! impl_destroy {
    ($($type:ty),+) => {
        $(
            impl Destroy for NonNull<$type> {
                fn destroy(mut self) {
                    if let Some(destroy) = unsafe { self.as_mut().destroy } {
                        destroy(self.as_ptr());
                    }
                }
            }
        )+
    }
}

impl_destroy! { CComponent, CElement, CGeometry, CMaterial, CMedium, CTracer }

macro_rules! null_pointer_fmt {
    ($($arg:tt)*) => {
        {
            let what = format!($($arg)*);
            Error::new(ValueError).what(&what).why("null pointer").to_err()
        }
    }
}

impl CModule {
    pub fn element(&self, symbol: &str) -> PyResult<Option<Element>> {
        self.element
            .and_then(|func| {
                let binding = CString::new(symbol).unwrap();
                let element = func(binding.as_c_str().as_ptr());
                if element.is_null() {
                    None
                } else {
                    Some(OwnedPtr::new(element))
                }
            })
            .transpose()?
            .map(|element| Ok(Element {
                Z: element.Z()? as u32,
                A: element.A()?,
                I: element.I()?,
            }))
            .transpose()
    }

    pub fn geometry(&self) -> PyResult<OwnedPtr<CGeometry>> {
        match self.geometry {
            Some(func) => OwnedPtr::new(func()),
            None => Err(null_pointer_fmt!("CModule::geometry")),
        }
    }

    pub fn material(&self, name: &str, broker: &MaterialsBroker) -> PyResult<Option<Material>> {
        self.material
            .and_then(|func| {
                let binding = CString::new(name).unwrap();
                let material = func(binding.as_c_str().as_ptr());
                if material.is_null() {
                    None
                } else {
                    Some(OwnedPtr::new(material))
                }
            })
            .transpose()?
            .map(|material| {
                let n = material.components_len()
                    .map_err(|_| null_pointer_fmt!("elements_len for {} material", name))?;

                let mut symbols = HashSet::new();
                let mut composition = Vec::with_capacity(n);
                for i in 0..n {
                    let component = material.component(i)
                        .map_err(|_| null_pointer_fmt!("component#{} for {} material", i, name))?;
                    let symbol = component.symbol()?;
                    let weight = component.weight()?;
                    symbols.insert(symbol.clone());
                    composition.push(Component { name: symbol, weight });
                }
                let density = material.density()
                    .map_err(|_| null_pointer_fmt!("density for {} material", name))?;
                let mee = material.I();
                let mixture = Mixture::from_elements(density, &composition, mee, broker)
                    .map_err(|(kind, why)| {
                        let what = format!("{} material", name);
                        Error::new(kind).what(&what).why(&why).to_err()
                    })?;
                Ok(Material::Mixture(mixture))
            })
            .transpose()
    }
}

macro_rules! impl_get_attr {
    ($name:tt, $output:ty) => {
        pub fn $name(&self) -> PyResult<$output> {
            match unsafe { self.0.as_ref().$name } {
                Some(func) => Ok(func(self.0.as_ptr())),
                None => Err(null_pointer_err()),
            }
        }
    }
}

macro_rules! impl_get_str_attr {
    ($name:tt) => {
        pub fn $name(&self) -> PyResult<String> {
            match unsafe { self.0.as_ref().$name } {
                Some(func) => {
                    let cstr = unsafe { CStr::from_ptr(func(self.0.as_ptr())) };
                    Ok(cstr.to_str()?.to_owned())
                },
                None => Err(null_pointer_err()),
            }
        }
    }
}

macro_rules! impl_get_item {
    ($name:tt, $output:ty) => {
        pub fn $name(&self, index: usize) -> PyResult<OwnedPtr<$output>> {
            match unsafe { self.0.as_ref().$name } {
                Some(func) => OwnedPtr::new(func(self.0.as_ptr(), index)),
                None => Err(null_pointer_err()),
            }
        }
    }
}

macro_rules! impl_is_none {
    ($name:tt) => {
        paste! {
            pub fn [< is_none_ $name >](&self) -> bool {
                unsafe { self.0.as_ref().$name }
                    .is_none()
            }
        }
    }
}

macro_rules! impl_get_opt_attr {
    ($name:tt, $output:ty) => {
        pub fn $name(&self) -> Option<$output> {
            unsafe { self.0.as_ref().$name }
                .map(|func| func(self.0.as_ptr()))
        }
    }
}

macro_rules! impl_get_opt_str {
    ($name:tt) => {
        pub fn $name(&self) -> PyResult<Option<String>> {
            unsafe { self.0.as_ref().$name }
                .map(|func| {
                    let cstr = unsafe { CStr::from_ptr(func(self.0.as_ptr())) };
                    Ok(cstr.to_str()?.to_owned())
                })
                .transpose()
        }
    }
}

impl OwnedPtr<CGeometry> {
    impl_get_attr!(media_len, usize);
    impl_get_item!(medium, CMedium);

    pub fn tracer(&self) -> PyResult<OwnedPtr<CTracer>> {
        match unsafe { self.0.as_ref() }.tracer {
            Some(func) => OwnedPtr::new(func(self.0.as_ptr())),
            None => Err(null_pointer_fmt!("CModule::tracer")),
        }
    }
}

impl OwnedPtr<CMaterial> {
    impl_get_item!(component, CComponent);
    impl_get_attr!(components_len, usize);
    impl_get_attr!(density, c_double);
    impl_get_opt_attr!(I, c_double);

    impl_is_none!(I);
}

impl OwnedPtr<CComponent> {
    impl_get_str_attr!(symbol);
    impl_get_attr!(weight, c_double);
}

impl OwnedPtr<CElement> {
    impl_get_attr!(A, c_double);
    impl_get_attr!(I, c_double);
    impl_get_attr!(Z, c_int);
}

impl OwnedPtr<CMedium> {
    impl_get_str_attr!(material);
    impl_get_opt_attr!(density, c_double);
    impl_get_opt_str!(description);
}

impl OwnedPtr<CTracer> {
    impl_is_none!(locate);
    impl_is_none!(reset);
    impl_is_none!(trace);
    impl_is_none!(move_);
    impl_is_none!(turn);
    impl_is_none!(medium);
    impl_is_none!(position);
}
