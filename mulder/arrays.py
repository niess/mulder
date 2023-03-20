"""Encapsulation of structured numpy.ndarrays.
"""

from collections import namedtuple
from collections.abc import Mapping
import numpy
from .wrapper import ffi, lib


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
    for name, tp, description in cls.properties:
        add_property(name, tp, description)
        if not isinstance(tp, str):
            tp = tp._dtype
        dtype.append((name, tp))

    cls._dtype = numpy.dtype(dtype, align=True)

    argnames = [name for (name, _, _) in cls.properties]
    for (_, tp, _) in cls.properties:
        if not isinstance(tp, str):
            argnames += [name for (name, _, _) in tp.properties]

    cls._parser = namedtuple( # For unpacking arguments
        f"{cls.__name__}Parser",
        argnames,
        defaults=len(argnames) * [None,],
        module = cls.__module__
    )

    return type(cls.__name__, (Array,), dict(cls.__dict__))


def commonsize(*args):
    """Return the common size of a set of arrays."""
    return Array._get_size(*args)


class Array:
    """Base class wrapping a structured numpy.ndarray."""

    @classmethod
    def empty(cls, size):
        """Create an empty Array instance."""
        obj = super().__new__(cls)
        obj._init_array(numpy.empty, size)
        return obj

    @classmethod
    def zeros(cls, size):
        """Create a zeroed Array instance."""
        obj = super().__new__(cls)
        obj._init_array(numpy.empty, size)
        return obj

    @classmethod
    def parse(cls, *args, **kwargs):
        """Create or forward an Array instance."""

        if args and isinstance(args[0], cls):
            if len(args) == 1 and not kwargs:
                return args[0]
        elif args or kwargs:
            try:
                return cls(*args, **kwargs)
            except:
                pass
        raise ValueError(f"bad arguments for {cls.__name__}")

    @property
    def size(self):
        """Number of array entries."""
        return self._size

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
                if properties:
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

    def __repr__(self):
        return repr(self._data)

    def __str__(self):
        return str(self._data)

    def _init_array(self, method, size):
        self._data = method(size, dtype=self._dtype)
        self._size = size

    def copy(self):
        """Return a copy of self."""
        obj = self.empty(self._size)
        obj._data[:] = self._data
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


class Flatgrid(Mapping):
    """Flat grid container, built from a set of base vectors."""

    @property
    def size(self):
        """Grid size"""
        return self._size

    @property
    def shape(self):
        """Grid shape"""
        return self._shape

    def __init__(self, **kwargs):
        arrays = numpy.meshgrid(*tuple(kwargs.values()))
        self._size = arrays[0].size
        self._shape = arrays[0].shape
        self._vectors = {
            k: arrays[i].flatten() for i, k in enumerate(kwargs.keys())
        }

    def __iter__(self):
        """Iterator over flattened grid vectors, as dict."""
        return iter(self._vectors)

    def __getitem__(self, k):
        return self._vectors[k]

    def __getattr__(self, k):
        return self._vectors[k]

    def __len__(self):
        return len(self._vectors)

    def unflatten(self, *args):
        """Unflatten arrays over the grid."""
        if len(args) == 1:
            return args[0].reshape(self._shape)
        else:
            return (arg.reshape(self._shape) for arg in args)
