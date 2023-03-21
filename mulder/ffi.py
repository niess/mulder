"""FFI related utilities.
"""

from .wrapper import ffi, lib


class LibraryError(Exception):
    """Mulder C-library error."""

    def __init__(self):
        msg = lib.mulder_error_get()
        if msg != ffi.NULL:
            self.args = (ffi.string(msg).decode(),)


# Type conversions between cffi and numpy
todouble = lambda x: ffi.cast("double *", x.ctypes.data)

toint = lambda x: ffi.cast("int *", x.ctypes.data)

tostr = lambda x: ffi.NULL if x is None else \
                  ffi.new("const char[]", x.encode())
