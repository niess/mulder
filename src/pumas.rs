use libc::FILE;
use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use std::ffi::{CString, c_char, c_void};
use std::path::{Path, PathBuf};


//================================================================================================
// Physics interface.
//================================================================================================

#[pyclass]
pub struct Physics (*mut c_void);

unsafe impl Send for Physics {}

#[pymethods]
impl Physics {
    #[staticmethod]
    pub fn build(path: PathBuf) -> PyResult<Self> {
        let dedx_path = new_cstring(&path)?;
        let mdf_path = new_cstring(&path.join("materials.xml"))?;
        let mut ptr: *mut c_void = std::ptr::null_mut();
        unsafe {
            pumas_physics_create(
                &mut ptr,
                Particle::Muon,
                mdf_path.as_ptr() as *const c_char,
                dedx_path.as_ptr() as *const c_char,
                std::ptr::null(),
            );
        }
        Ok(Self(ptr))
    }

    pub fn dump(&self, path: PathBuf) -> PyResult<()> {
        let path = new_cstring(&path)?;
        unsafe {
            let file = libc::fopen(
                path.as_ptr() as *const c_char,
                "wb+".as_ptr() as *const c_char,
            ); // XXX Check result.
            pumas_physics_dump(
                self.0 as *const c_void,
                file,
            );
            libc::fclose(file);
        }
        Ok(())
    }
}

fn new_cstring(path: &Path) -> PyResult<CString> {
    let path: Vec<u8> = match path.to_str() {
        None => return Err(PyValueError::new_err("bad path")), // XXX more explicit msg
        Some(path) => path.as_bytes().into(),
    };
    let path = CString::new(path)
        .unwrap(); // XXX Forward error.
    Ok(path)
}

impl Drop for Physics {
    fn drop(&mut self) {
        unsafe {
            pumas_physics_destroy(&mut self.0);
        }
    }
}

#[link(name = "c-libs", kind = "static")]
extern "C" {
    fn pumas_physics_create(
        physics: *mut *mut c_void,
        particle: Particle,
        mdf_path: *const c_char,
        dedx_path: *const c_char,
        settings: *const c_void,
    ) -> i32; // XXX enum as return type.

    fn pumas_physics_destroy(physics: *mut *mut c_void) -> i32;

    fn pumas_physics_dump(
        physics: *const c_void,
        stream: *mut FILE,
    ) -> i32;
}

#[repr(C)]
enum Particle {
    Muon = 0,
}


//================================================================================================
// Unit tests.
//================================================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physics() -> PyResult<()> {
        let physics = Physics::build(&"share/mulder/materials")?;
        physics.dump(&"share/mulder/materials/materials.pumas")?;
        Ok(())
    }
}
