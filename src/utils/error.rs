use crate::bindings::{gull, pumas, turtle};
use paste::paste;
use pyo3::prelude::*;
use pyo3::create_exception;
use pyo3::exceptions::{
    PyAttributeError, PyException, PyFileNotFoundError, PyIndexError, PyIOError,
    PyKeyboardInterrupt, PyKeyError, PyMemoryError, PyNotImplementedError, PySystemError,
    PyTypeError, PyValueError
};
use pyo3::ffi::PyErr_CheckSignals;
use ::std::ffi::{c_char, c_uint, CStr};
use ::std::sync::RwLock;


// ===============================================================================================
//
// Normalised errors.
//
// ===============================================================================================

#[derive(Debug, Default)]
pub struct Error<'a> {
    pub kind: Option<ErrorKind>,
    pub what: Option<&'a str>,
    pub why: Option<&'a str>,
}

#[derive(Clone, Copy, Debug)]
pub enum ErrorKind {
    AttributeError,
    CLibraryException,
    Exception,
    FileNotFoundError,
    IndexError,
    IOError,
    KeyboardInterrupt,
    KeyError,
    MemoryError,
    NotImplementedError,
    SystemError,
    TypeError,
    ValueError,
}

impl<'a> Error<'a> {
    pub fn maybe_what(mut self, what: Option<&'a str>) -> Self {
        self.what = what;
        self
    }

    pub fn maybe_why(mut self, why: Option<&'a str>) -> Self {
        self.why = why;
        self
    }

    pub fn new(kind: ErrorKind) -> Self {
        Self {
            kind: Some(kind),
            what: None,
            why: None,
        }
    }

    pub fn to_err(&self) -> PyErr {
        self.into()
    }

    pub fn to_string(&self) -> String {
        self.into()
    }

    pub fn what(mut self, what: &'a str) -> Self {
        self.what = Some(what);
        self
    }

    pub fn why(mut self, why: &'a str) -> Self {
        self.why = Some(why);
        self
    }
}

impl<'a> From<&Error<'a>> for String {
    fn from(value: &Error<'a>) -> Self {
        let Error { what, why, .. } = value;
        match what {
            None => match why {
                None => "something bad happened".to_string(),
                Some(why) => why.to_string(),
            }
            Some(what) => match why {
                None => format!("bad {what}"),
                Some(why) => format!("bad {what} ({why})"),
            },
        }
    }
}

impl<'a> From<Error<'a>> for String {
    fn from(value: Error<'a>) -> Self {
        (&value).into()
    }
}

impl<'a> From<&Error<'a>> for PyErr {
    fn from(value: &Error<'a>) -> Self {
        let msg: String = value.into();
        let kind = value.kind
            .unwrap_or(ErrorKind::Exception);
        match kind {
            ErrorKind::AttributeError => PyErr::new::<PyAttributeError, _>(msg),
            ErrorKind::CLibraryException => PyErr::new::<CLibraryException, _>(msg),
            ErrorKind::Exception => PyErr::new::<PyException, _>(msg),
            ErrorKind::FileNotFoundError => PyErr::new::<PyFileNotFoundError, _>(msg),
            ErrorKind::IndexError => PyErr::new::<PyIndexError, _>(msg),
            ErrorKind::IOError => PyErr::new::<PyIOError, _>(msg),
            ErrorKind::KeyboardInterrupt => PyErr::new::<PyKeyboardInterrupt, _>(msg),
            ErrorKind::KeyError => PyErr::new::<PyKeyError, _>(msg),
            ErrorKind::MemoryError => PyErr::new::<PyMemoryError, _>(msg),
            ErrorKind::NotImplementedError => PyErr::new::<PyNotImplementedError, _>(msg),
            ErrorKind::SystemError => PyErr::new::<PySystemError, _>(msg),
            ErrorKind::TypeError => PyErr::new::<PyTypeError, _>(msg),
            ErrorKind::ValueError => PyErr::new::<PyValueError, _>(msg),
        }
    }
}

impl<'a> From<Error<'a>> for PyErr {
    fn from(value: Error<'a>) -> Self {
        (&value).into()
    }
}


// ===============================================================================================
//
// Variants explainers.
//
// ===============================================================================================

pub fn variant_explain(value: &str, options: &[&str]) -> String {
    let n = options.len();
    let options = match n {
        0 => unimplemented!(),
        1 => format!("'{}'", options[0]),
        2 => format!("'{}' or '{}'", options[0], options[1]),
        _ => {
            let options: Vec<_> = options
                .iter()
                .map(|e| format!("'{}'", e))
                .collect();
            format!(
                "{} or {}",
                options[0..(n - 1)].join(", "),
                options[n - 1],
            )
        },
    };
    format!(
        "expected one of {}, found '{}'",
        options,
        value
    )
}


// ===============================================================================================
//
// C library exceptions.
//
// ===============================================================================================

pub fn initialise() {
    unsafe {
        gull::error_handler_set(Some(gull_error));
        pumas::error_handler_set(Some(pumas_error));
        turtle::error_handler_set(Some(turtle_error));
    }
}

macro_rules! define_error_handler {
    ($lib:ident) => {
        paste! {
            #[no_mangle]
            unsafe extern "C" fn [<$lib _error>](
                code: c_uint,
                _function: $lib::Function,
                message: *const c_char,
            ) {
                if code != $lib::SUCCESS {
                    let message = CStr::from_ptr(message)
                        .to_string_lossy()
                        .into_owned();
                    let err = CError { message };
                    ERROR_BUFFER.write().unwrap().push(err);
                }
            }
        }
    }
}

define_error_handler!(gull);
define_error_handler!(pumas);
define_error_handler!(turtle);

static ERROR_BUFFER: RwLock<Vec<CError>> = RwLock::new(Vec::new());

struct CError {
    message: String,
}

create_exception!(mulder, CLibraryException, PyException, "A C-library exception.");

pub fn clear() {
    ERROR_BUFFER.write().unwrap().clear();
}

pub fn to_result<T: AsRef<str>>(rc: c_uint, what: Option<T>) -> Result<(), PyErr> {
    if rc == pumas::SUCCESS {
        Ok(())
    } else {
        let err = ERROR_BUFFER.write().unwrap().pop().unwrap();
        let mut err = Error::new(ErrorKind::CLibraryException).why(&err.message);
        if let Some(what) = what.as_ref() {
            err = err.what(what.as_ref());
        }
        Err(err.to_err())
    }
}


// ===============================================================================================
//
// Keyboard interupts (catched by Python runtime).
//
// ===============================================================================================

pub fn ctrlc_catched() -> bool {
    if unsafe { PyErr_CheckSignals() } == -1 { true } else { false }
}

pub fn check_ctrlc(why: &str) -> PyResult<()> {
    if ctrlc_catched() {
        clear();
        let err = Error::new(ErrorKind::KeyboardInterrupt).why(why);
        Err(err.to_err())
    } else {
        Ok(())
    }
}
