"""Grid related utilities.
"""

from numbers import Number
from pathlib import Path

from collections import namedtuple
import numpy

from .types import Flux
from .ffi import ffi, lib, LibraryError, _todouble, _tostr


class Grid:
    """Structured grid, spanning over a set of base vectors."""

    @property
    def base(self):
        """Grid base vectors."""
        return self._base

    @property
    def ndim(self):
        """Grid dimension."""
        return len(self._shape)

    @property
    def nodes(self):
        """Nodes dictionary, organised by grid coordinates."""
        return self._nodes

    @property
    def size(self):
        """Grid size."""
        return self._size

    @property
    def shape(self):
        """Grid shape."""
        return self._shape

    @property
    def structured(self):
        """Structured view of nodes coordinates."""
        return self._structured

    def __init__(self, indexing=None, **kwargs):
        if indexing is None: indexing = "xy"
        arrays = numpy.meshgrid(*tuple(kwargs.values()), indexing=indexing)
        self._size = arrays[0].size
        self._shape = arrays[0].shape
        self._nodes = {
            k: arrays[i].flatten() for i, k in enumerate(kwargs.keys())
        }
        name = f"{self.__class__.__name__}_{'_'.join(kwargs.keys())}"
        tp = namedtuple(name, kwargs.keys())
        self._base = tp(*[numpy.asarray(v) for v in kwargs.values()])
        self._structured = tp(*[self.reshape(v) for v in self._nodes.values()])

    def __iter__(self):
        """Iterator over node coordinates (flatten), as a dict."""
        return iter(self._nodes)

    def __getattr__(self, k):
        try:
            return self.__dict__["_nodes"][k]
        except KeyError:
            try:
                return self.__dict__[k]
            except KeyError:
                raise AttributeError(
                    f"'{self.__class__.__name__}' has not attribute '{k}'"
                )

    def __setattr__(self, k, v):
        try:
            a = self.__dict__["_nodes"][k]
        except KeyError:
            self.__dict__[k] = v
        else:
            a[:] = v

    def empty(self):
        """Return an empty (flat) array spanning the grid."""
        return numpy.empty(self._size)

    def ones(self):
        """Return a flat array of ones spanning the grid."""
        return numpy.ones(self._size)

    def reshape(self, *args):
        """Structure (unflatten) a set of grid array(s)."""

        if len(args) == 1:
            return numpy.reshape(args[0], self._shape)
        else:
            return (numpy.reshape(arg, self._shape) for arg in args)


def _is_regular(a):
    """Check if a 1d array has a regular stepping."""
    d = numpy.diff(a)
    dmin, dmax = min(d), max(d)
    amax = max(numpy.absolute(a))
    return dmax - dmin <= 1E-15 * amax


class FluxGrid(Grid):
    """Specialised Grid, used for tabulating flux values."""

    @property
    def flux(self) -> Flux:
        """Flux at grid nodes"""
        return self._flux

    def __init__(self, energy, cos_theta, height=None):

        # Check inputs
        assert(len(energy) > 1)
        assert(len(cos_theta) > 1)

        if not _is_regular(numpy.log(energy)):
            raise ValueError("bad energy vector (not log-regular)")

        if not _is_regular(cos_theta):
            raise ValueError("bad cos(theta) vector (not regular)")

        if height is None:
            height = 0
        elif not isinstance(height, Number):
            if not _is_regular(height):
                raise ValueError("bad height vector (not regular)")

        super().__init__(
            height = height,
            cos_theta = cos_theta,
            energy = energy,
            indexing = "ij"
        )

        self._flux = Flux.zeros(self._size)

    def create_table(self, path):
        """Create a (reference) table from flux values at grid nodes."""

        # Format data.
        data = numpy.empty((self._size, 2), dtype="<f4")
        flux = self._flux
        data[:,0] = 0.5 * flux.value * (1 + flux.asymmetry)
        data[:,1] = 0.5 * flux.value * (1 - flux.asymmetry)

        # Prepare path.
        path = Path(path)
        path.parent.mkdir(parents=True, exist_ok=True)

        # Generate binary table file.
        with path.open("wb") as f:
            dims = numpy.array(
                self._shape[::-1],
                dtype="i8"
            )
            dims.astype("<i8").tofile(f)

            lims = numpy.array(
                (
                    self.energy[0],
                    self.energy[-1],
                    self.cos_theta[0],
                    self.cos_theta[-1],
                    self.height[0],
                    self.height[-1]
                ),
                dtype="f8"
            )
            lims.astype("<f8").tofile(f)

            data.flatten().tofile(f)


class MapGrid(Grid):
    """Specialised Grid, used for tabulating topography values."""

    @property
    def height(self) -> numpy.ndarray:
        """Height at grid nodes"""
        return self._height

    def __init__(self, x, y):

        # Check inputs
        assert(len(x) > 1)
        assert(len(y) > 1)

        if not _is_regular(x):
            raise ValueError("bad x-coordinate vector (not regular)")

        if not _is_regular(y):
            raise ValueError("bad y-coordinate vector (not regular)")

        super().__init__(
            x = x,
            y = y
        )

        self._height = numpy.zeros(self._size)

    def create_map(self, path, projection=None):
        """Create a Turtle map from a numpy array."""

        # Prepare path directory.
        directory = Path(path).parent
        directory.mkdir(parents=True, exist_ok=True)

        # Generate the map.
        rc = lib.mulder_map_create(
            _tostr(path),
            _tostr(projection),
            len(self.base.x),
            len(self.base.y),
            self.base.x[0],
            self.base.x[-1],
            self.base.y[0],
            self.base.y[-1],
            _todouble(self.height)
        )
        if rc != lib.MULDER_SUCCESS:
            raise LibraryError()
