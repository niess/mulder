"""Generic type exposing numpy.ndarrays as structured data
"""

import numpy
from .wrapper import ffi, lib


def arrayclass(cls):
    """Decorator for array classes, dynamically setting properties"""

    def add_property(i, name, description):
        """Adds a property with proper index scoping"""

        def get_property(self):
            return self._view[i]

        def set_property(self, v):
            self._view[i] = v

        setattr(
            cls,
            name,
            property(get_property, set_property, None, description)
        )

    for i, (name, description) in enumerate(cls.properties):
        add_property(i, name, description)

    return type(str(cls), (Array,), dict(cls.__dict__))


class Array:
    """Base class wrapping a C-compliant numpy.ndarray"""

    @property
    def size(self):
        """Number of entries"""
        return self._size

    @property
    def cffi_ptr(self):
        """Raw cffi pointer"""
        return ffi.cast("double *", self._data.ctypes.data)

    @classmethod
    def empty(cls, size=None):
        """Create an empty Array instance"""
        obj = super().__new__(cls)
        obj._init_array(numpy.empty, size)
        return obj

    @classmethod
    def zeros(cls, size=None):
        """Create a zeroed Array instance"""
        obj = super().__new__(cls)
        obj._init_array(numpy.empty, size)
        return obj

    def __init__(self, *args, **kwargs):
        if args:
            # Initialise from an argument list. First let us check input
            # consistency
            if kwargs:
                raise NotImplementedError()

            if len(args) > len(self.properties):
                raise ValueError(
                    f"too many arguments (expected {self._depth}, "
                    f"got {len(args)})")

            # Compute the array size
            size = None
            for arg in args:
                try:
                    s = len(arg)
                except:
                    pass
                else:
                    if size is None: size = s
                    elif (s != size) and (s != 1):
                        raise ValueError("incompatible size(s)")

            # Create the array
            self._init_array(numpy.zeros, size)

            # Initialise the array
            for i, arg in enumerate(args):
                self._view[i] = arg

        elif kwargs:
            # Initialise from keyword arguments. First, let us compute the
            # array size
            size = None
            for v in kwargs.values():
                try:
                    s = len(v)
                except:
                    pass
                else:
                    if size is None: size = s
                    elif (s != size) and (s != 1):
                        raise ValueError("incompatible size(s)")

            # Create the array
            self._init_array(numpy.zeros, size)

            # Initialise the array
            for k, v in kwargs.items():
                try:
                    setattr(self, k, v)
                except AttributeError:
                    raise ValueError(
                        f"unknown property for {self.__class__} ({k})")

        else:
            raise NotImplementedError()

    def __repr__(self):
        return repr(self._data)

    def __str__(self):
        return str(self._data)

    def _init_array(self, method, size=None):
        if size is None:
            self._data = method(len(self.properties))
            self._view = self._data
        else:
            self._data = method((size, len(self.properties)))
            self._view = self._data.T
        self._size = size
