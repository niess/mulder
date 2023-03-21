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
_todouble = lambda x: ffi.cast("double *", ffi.from_buffer(x))

_toint = lambda x: ffi.cast("int *", ffi.from_buffer(x))

_tostr = lambda x: ffi.NULL if x is None else \
                   ffi.new("const char[]", x.encode())
