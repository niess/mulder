"""Encapsulation of structured numpy.ndarrays.
"""

from collections import namedtuple
from numbers import Number

import numpy
from numpy.lib.recfunctions import structured_to_unstructured, \
                                   unstructured_to_structured

from .ffi import ffi, lib


def arrayclass(cls):
    """Decorator for array classes, dynamically setting properties."""

    def add_property(name, tp, description):
        """Add a property with proper index scoping."""

        def add_nickname(base, name):
            """Add a nickname for composite properties."""

            def get_property(self):
                return self._data[base][name]

            def set_property(self, v):
                self._data[base][name] = v

            description = f"Nickname for {base}.{name}"

            setattr(
                cls,
                name,
                property(get_property, set_property, None, description)
            )

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

            for (nickname, _, _) in tp.properties:
                add_nickname(name, nickname)

        setattr(
            cls,
            name,
            property(get_property, set_property, None, description)
        )

    dtype = []
    for name, tp, description, *_ in cls.properties:
        add_property(name, tp, description)
        if not isinstance(tp, str):
            tp = tp._dtype
        dtype.append((name, tp))

    cls._dtype = numpy.dtype(dtype, align=True)

    defaults = [None if len(v) < 4 else v[3] for v in cls.properties]

    argnames = [name for (name, *_) in cls.properties]
    for (_, tp, *_) in cls.properties:
        if not isinstance(tp, str):
            argnames += [name for (name, *_) in tp.properties]
            defaults += [None if len(v) < 4 else v[3] for v in tp.properties]

    cls._parser = namedtuple( # For unpacking arguments
        f"{cls.__name__}Parser",
        argnames,
        defaults = defaults,
        module = cls.__module__
    )

    return type(cls.__name__, (Array, *cls.__bases__), dict(cls.__dict__))


def commonsize(*args):
    """Return the common size of a set of arrays."""
    return Array._get_size(*args)


def pack(cls, *args, **kwargs):
    """Implicit argument(s) packing"""

    if args and isinstance(args[0], cls):
        if len(args) == 1 and not kwargs:
            return args[0]
    elif args or kwargs:
        return cls(*args, **kwargs)
        try:
            return cls(*args, **kwargs)
        except:
            pass
    raise ValueError(f"bad arguments for {cls.__name__}")


class Array:
    """Base class wrapping a structured numpy.ndarray."""

    @classmethod
    def empty(cls, size=None):
        """Create an empty Array instance."""
        obj = super().__new__(cls)
        obj._init_array(numpy.empty, size)
        return obj

    @classmethod
    def new(cls, size=None):
        """Create a new Array instance initialised with default values."""
        obj = cls.zeros(size)
        for v in cls.properties:
            if len(v) >= 4:
                setattr(obj, v[0], v[3])
        return obj

    @classmethod
    def zeros(cls, size=None):
        """Create a zeroed Array instance."""
        obj = super().__new__(cls)
        obj._init_array(numpy.zeros, size)
        return obj

    @classmethod
    def from_structured(cls, data, copy=True):
        """Create an Array instance from a numpy structured array."""
        assert(data.dtype == cls.dtype)
        obj = super().__new__(cls)
        if copy:
            obj._init_array(numpy.empty, data.size)
            obj._data[:] = data
        else:
            obj._size = data.size
            obj._data = data
        return obj

    @classmethod
    def parse(cls, *args, **kwargs):
        """Create or forward an Array instance."""
        return pack(cls, *args, **kwargs)

    @property
    def size(self):
        """Number of array entries."""
        return self._size

    @property
    def shape(self):
        """Numpy like shape, for compatibility."""
        return (self._size,)

    @property
    def cffi_pointer(self):
        """cffi pointer."""
        return ffi.cast(self.ctype, self._data.ctypes.data)

    @classmethod
    @property
    def numpy_dtype(cls):
        """Numpy data type."""
        return cls._dtype

    @property
    def numpy_array(self):
        """Numpy structured array."""
        return self._data

    @property
    def numpy_stride(self):
        """Numpy array stride."""
        strides = self._data.strides
        return strides[0] if strides else 0

    def __init__(self, *args, **kwargs):

        if len(args) > len(self.properties):
            raise TypeError(
                f"{self.__class__.__name__} takes at most "
                f"{len(self.properties)} arguments "
                f"({len(args)} given)"
            )
        else:
            args = self._parser(*args, **kwargs)
            size = self._get_size(*args, properties=self.properties)
            self._init_array(numpy.zeros, size)
            for arg, field in zip(args, self._parser._fields):
                if arg is not None:
                    setattr(self, field, arg)

    @staticmethod
    def _get_size(*args, properties=None):
        """Compute (common) array size for given arguments."""
        size = None
        for i, arg in enumerate(args):
            if isinstance(arg, Array):
                s = arg.size
                if (s is None) or (s == 1):
                    continue
            else:
                try:
                    shape = numpy.shape(arg)
                except:
                    raise ValueError(
                        "bad shape (expected a scalar or vector, "
                        "got an undefined shape)"
                    )
                ndim = len(shape)
                if ndim == 0:
                    continue
                else:
                    if properties and (i < len(properties)):
                        tp = properties[i][1]
                        if not isinstance(tp, str):
                            ndim -= 1
                            if ndim == 0:
                                continue
                    if ndim > 1:
                        raise ValueError(
                            f"bad shape (expected a scalar or vector, "
                            f"got a {ndim}d array)"
                        )
                    s = shape[0]

            if size is None:
                size = s
            elif (s != size) and (s != 1):
                raise ValueError("incompatible size(s)")
        return size

    def __eq__(self, other):
        return (self.__class__ == other.__class__) and \
               numpy.array_equal(self._data, other._data)

    def __iter__(self):
        """Iterate over properties (i.e. column wise)."""
        return (self._data[key] for (key, _, _) in self.properties)

    def __len__(self):
        """Get the number of array entries."""
        return self._size

    def __getitem__(self, i):
        """Get a sub-array of self."""
        data = self._data[i]
        try:
            size = len(data)
        except TypeError:
            size = None
        else:
            if size == 1: size = None

        obj = self.__new__(self.__class__)
        obj._size = size
        obj._data = data
        return obj

    def __setitem__(self, i, v):
        """Set elements from a sub-array"""
        assert(isinstance(v, self.__class__))
        self._data[i] = v._data

    def __repr__(self):
        return repr(self._data)

    def __str__(self):
        return str(self._data)

    def _init_array(self, method, size):
        self._data = method(size, dtype=self._dtype)
        self._size = size

    def copy(self):
        """Return a copy of self."""
        obj = super().__new__(self.__class__)
        obj._data = self._data.copy()
        obj._size = self._size
        return obj

    def repeat(self, repeats):
        """Return a repeated instance of self."""
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


class Algebraic:
    """Algebraic properties for Array types"""

    def __add__(self, other):
        other = self._format(other)
        data = self.unstructured() + other
        return self.from_unstructured(data)

    def __radd__(self, other):
        return self.__add__(other)

    def __sub__(self, other):
        other = self._format(other)
        data = self.unstructured() - other
        return self.from_unstructured(data)

    def __rsub__(self, other):
        other = self._format(other)
        data = other - self.unstructured()
        return self.from_unstructured(data)

    def __mul__(self, other):
        other = self._format(other)
        data = self.unstructured() * other
        return self.from_unstructured(data)

    def __rmul__(self, other):
        return self.__mul__(other)

    def __truediv__(self, other):
        other = self._format(other)
        data = self.unstructured() / other
        return self.from_unstructured(data)

    def __rtruediv__(self, other):
        other = self._format(other)
        data = other / self.unstructured()
        return self.from_unstructured(data)

    def __pow__(self, exponent):
        if isinstance(exponent, Number):
            data = self.unstructured()**exponent
            return self.from_unstructured(data)
        else:
            return NotImplemented()

    def _format(self, other):
        if isinstance(other, Number):
            size = self._size
        elif isinstance(other, self.__class__):
            size = self._size if other._size is None else other._size
            other = other.unstructured()
        else:
            return NotImplemented()
        return other

    @classmethod
    def from_unstructured(cls, data, copy=False):
        """Create an Array instance from an unstuctured numpy array"""
        obj = super().__new__(cls)
        data = unstructured_to_structured(data, cls.numpy_dtype, copy=copy)
        obj._size = data.size if data.size > 1 else None
        obj._data = data
        return obj

    def unstructured(self, copy=False):
        """Return an unstructured numpy view of self"""
        return structured_to_unstructured(self._data, copy=copy)

    def norm(self):
        """Return L^2 norm"""
        if self._size is None:
            return numpy.linalg.norm(self.unstructured())
        else:
            return numpy.linalg.norm(self.unstructured(), axis=1)
