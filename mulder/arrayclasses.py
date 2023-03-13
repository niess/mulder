"""Encapsulation of structured numpy.ndarrays
"""

import numpy
from .wrapper import ffi, lib


def arrayclass(cls):
    """Decorator for array classes, dynamically setting properties"""

    def add_property(name, tp, description):
        """Adds a property with proper index scoping"""

        if isinstance(tp, str):
            def get_property(self):
                return self._data[name]

            def set_property(self, v):
                self._data[name] = v
        else:
            altname = f"_{name}"

            def get_property(self):
                try:
                    return getattr(self, altname)
                except AttributeError:
                    view = tp.__new__(tp)
                    view._data = self._data[name]
                    view._size = self._size
                    setattr(self, altname, view)
                    return view

            def set_property(self, v):
                if isinstance(v, tp): v = v._data
                self._data[name] = v

        setattr(
            cls,
            name,
            property(get_property, set_property, None, description)
        )

    dtype = []
    for name, tp, description in cls.properties:
        add_property(name, tp, description)
        if not isinstance(tp, str):
            tp = tp.dtype
        dtype.append((name, tp))

    cls.dtype = numpy.dtype(dtype, align=True)

    return type(cls.__name__, (Array,), dict(cls.__dict__))


def broadcast(*args):
    """Return broadcasted arrays"""

    size = Array._get_size(*args)
    if size is not None:
        args = [arg.repeat(size) if arg.size != size else arg \
                for arg in args]
    return (*args, size)


class Array:
    """Base class wrapping a structured numpy.ndarray"""

    @classmethod
    def empty(cls, size):
        """Create an empty Array instance"""
        obj = super().__new__(cls)
        obj._init_array(numpy.empty, size)
        return obj

    @classmethod
    def zeros(cls, size):
        """Create a zeroed Array instance"""
        obj = super().__new__(cls)
        obj._init_array(numpy.empty, size)
        return obj

    @property
    def size(self):
        """Number of entries"""
        return self._size

    @property
    def cffi_ptr(self):
        """Raw cffi pointer"""
        return ffi.cast(self.ctype, self._data.ctypes.data)

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
            size = self._get_size(*args)

            # Create the array
            self._init_array(numpy.zeros, size)

            # Initialise the array
            for i, arg in enumerate(args):
                self._data[self.properties[i][0]] = arg

        elif kwargs:
            # Initialise from keyword arguments. First, let us compute the
            # array size
            size = self._get_size(*kwargs.values())

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

    @staticmethod
    def _get_size(*args):
        """Compute (common) array size for given arguments"""
        size = None
        for arg in args:
            try:
                s = len(arg)
            except:
                continue
            else:
                if size is None:
                    size = s
                elif (s != size) and (s != 1):
                    raise ValueError("incompatible size(s)")
        return size

    def __len__(self):
        return self._size

    def __repr__(self):
        return repr(self._data)

    def __str__(self):
        return str(self._data)

    def _init_array(self, method, size):
        self._data = method(size, dtype=self.dtype)
        self._size = size

    def copy(self):
        """Return a copy"""
        obj = self.empty(self._size)
        obj._data[:] = self.data
        return obj

    def repeat(self, repeats):
        """Return a repeated instance"""
        if repeats <= 1:
            return self.copy()
        elif self._size is None:
            obj = self.empty(repeats)
            obj._data[:] = self._data
            return obj
        else:
            size = self._size * repeats
            obj = self.empty(size)
            obj._data[:] = numpy.repeat(self._data, repeats, axis=0)
            return obj
