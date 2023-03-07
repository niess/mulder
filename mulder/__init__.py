import os
from pathlib import Path
import shutil
import weakref

import numpy

from .version import git_revision, version
from .wrapper import ffi, lib


"""Package / C-library installation prefix"""
PREFIX=str(Path(__file__).parent.resolve())


class LibraryError(Exception):
    """Mulder C-library error"""

    def __init__(self):
        msg = lib.mulder_error_get()
        if msg != ffi.NULL:
            self.args = (ffi.string(msg).decode(),)


# Numpy broadcasting
_asarray = lambda x: numpy.ascontiguousarray(x, dtype="f8")

_fromarray = lambda x: float(x) if x.size == 1 else x

def _asarray2(x, y):
    x, y = [_asarray(a) for a in numpy.broadcast_arrays(x, y)]
    return (x, y, x.size)

def _asarray3(x, y, z):
    x, y, z = [_asarray(a) for a in numpy.broadcast_arrays(x, y, z)]
    return (x, y, z, x.size)

def _fromarray2(x, y):
    return (float(x[0]), float(y[0])) if x.size == 1 else (x, y)

def _asmatrix(x, y):
    x, y = _asarray(x), _asarray(y)
    return (x, y, x.size * y.size)

def _frommatrix(nx, ny, z):
    if nx == 1:
        return float(z[0]) if ny == 1 else z
    elif ny == 1:
        return z
    else:
        return numpy.reshape(z, (ny, nx))


# Type conversions between cffi and numpy
_todouble = lambda x: ffi.cast("double *", x.ctypes.data)

_toint = lambda x: ffi.cast("int *", x.ctypes.data)

_tostr = lambda x: ffi.NULL if x is None else \
                   ffi.new("const char[]", x.encode())


def _is_regular(a):
    """Check if a 1d array has a regular stepping"""
    d = numpy.diff(a)
    dmin, dmax = min(d), max(d)
    amax = max(numpy.absolute(a))
    return dmax - dmin <= 1E-15 * amax


class Layer:
    """Topographic layer"""

    @property
    def material(self):
        """Constituant material"""
        return ffi.string(self._layer[0].material).decode()

    @property
    def model(self):
        """Topographic model"""
        v =  self._layer[0].model
        return None if v == ffi.NULL else ffi.string(v).decode()

    @property
    def density(self):
        """Material density"""
        v = float(self._layer[0].density)
        return None if v == 0 else v

    @density.setter
    def density(self, value):
        self._layer[0].density = 0 if value is None else value

    @property
    def offset(self):
        """Elevation offset"""
        v = float(self._layer[0].offset)
        return None if v == 0 else v

    @property
    def encoding(self):
        """Map encoding format"""
        v =  self._layer[0].encoding
        return None if v == ffi.NULL else ffi.string(v).decode()

    @property
    def projection(self):
        """Map cartographic projection"""
        v =  self._layer[0].projection
        return None if v == ffi.NULL else ffi.string(v).decode()

    @property
    def nx(self):
        """Map size along x-axis"""
        return int(self._layer[0].nx)

    @property
    def ny(self):
        """Map size along y-axis"""
        return int(self._layer[0].ny)

    @property
    def xmin(self):
        """Map minimum value along x-axis"""
        return float(self._layer[0].xmin)

    @property
    def xmax(self):
        """Map maximum value along x-axis"""
        return float(self._layer[0].xmax)

    @property
    def ymin(self):
        """Map minimum value along y-axis"""
        return float(self._layer[0].ymin)

    @property
    def ymax(self):
        """Map maximum value along y-axis"""
        return float(self._layer[0].ymax)

    @property
    def zmin(self):
        """Map minimum value along z-axis"""
        return float(self._layer[0].zmin)

    @property
    def zmax(self):
        """Map maximum value along z-axis"""
        return float(self._layer[0].zmax)

    def __init__(self, material, model=None, density=None, offset=None):
        layer = ffi.new("struct mulder_layer *[1]")
        layer[0] = lib.mulder_layer_create(_tostr(material), _tostr(model),
                                           0 if offset is None else offset)
        if layer[0] == ffi.NULL:
            raise LibraryError()

        weakref.finalize(self, lib.mulder_layer_destroy, layer)

        self._layer = layer
        self.density = density

    def height(self, x, y):
        """Layer height (including offset)"""

        x, y, size = _asmatrix(x, y)
        z = numpy.empty(size)

        rc = lib.mulder_layer_height_v(self._layer[0], x.size, y.size,
                                       _todouble(x), _todouble(y), _todouble(z))
        if rc != lib.MULDER_SUCCESS:
            raise LibraryError()

        return _frommatrix(x.size, y.size, z)

    def gradient(self, x, y):
        """Layer gradient"""

        x, y, size = _asarray2(x, y)
        gx = numpy.empty(size)
        gy = numpy.empty(size)

        rc = lib.mulder_layer_gradient_v(self._layer[0], size, _todouble(x),
                                         _todouble(y), _todouble(gx),
                                         _todouble(gy))
        if rc != lib.MULDER_SUCCESS:
            raise LibraryError()

        return _fromarray2(gx, gy)

    def geodetic(self, x, y):
        """Geodetic coordinates from map ones"""

        x, y, size = _asarray2(x, y)
        latitude = numpy.empty(size)
        longitude = numpy.empty(size)

        rc = lib.mulder_layer_geodetic_v(self._layer[0], size, _todouble(x),
                                        _todouble(y), _todouble(latitude),
                                        _todouble(longitude))
        if rc == lib.MULDER_FAILURE:
            raise LibraryError()

        return _fromarray2(latitude, longitude)

    def coordinates(self, latitude, longitude):
        """Map coordinates from geodetic ones"""

        latitude, longitude, size = _asarray2(latitude, longitude)
        x = numpy.empty(size)
        y = numpy.empty(size)

        rc = lib.mulder_layer_coordinates_v(self._layer[0], size,
                                           _todouble(latitude),
                                           _todouble(longitude), _todouble(x),
                                           _todouble(y))
        if rc != lib.MULDER_SUCCESS:
            raise LibraryError()

        return _fromarray2(x, y)


class Reference:
    """Reference (opensky) muon flux"""

    @property
    def energy_min(self):
        return float(self._reference[0].energy_min)

    @energy_min.setter
    def energy_min(self, value):
        self._reference[0].energy_min = value

    @property
    def energy_max(self):
        return float(self._reference[0].energy_max)

    @energy_max.setter
    def energy_max(self, value):
        self._reference[0].energy_max = value

    @property
    def height_min(self):
        return float(self._reference[0].height_min)

    @height_min.setter
    def height_min(self, value):
        self._reference[0].height_min = value

    @property
    def height_max(self):
        return float(self._reference[0].height_max)

    @height_max.setter
    def height_max(self, value):
        self._reference[0].height_max = value

    def __init__(self, path=None):
        if path is None:
            self._reference = ffi.new("struct mulder_reference *[1]",
                (lib.mulder_reference_default(),))
        else:
            # Load a tabulated reference flux from a file
            reference = ffi.gc(
                ffi.new("struct mulder_reference *[1]", (ffi.NULL,)),
                lib.mulder_reference_destroy_table
            )
            reference[0] = lib.mulder_reference_load_table(_tostr(path))
            self._reference = reference

    def flux(self, elevation, energy, height=None):
        """Get reference flux model, defined at reference height(s)"""

        if height is None:
            height = 0.5 * (self.height_min + self.height_max)

        energy = _asarray(energy)
        flux = numpy.empty(energy.size)

        if self._reference is None:
            flux[:] = 0
        else:
            rc = lib.mulder_reference_flux_v(self._reference[0], height,
                elevation, energy.size, _todouble(energy), _todouble(flux))
            if rc != lib.MULDER_SUCCESS:
                raise LibraryError()

        return _fromarray(flux)


class Prng:
    """Pseudo random numbers generator"""

    @property
    def fluxmeter(self):
        return self._fluxmeter()

    @property
    def seed(self):
        fluxmeter = self._fluxmeter()
        if fluxmeter is None:
            raise RuntimeError("dead fluxmeter ref")
        else:
            prng = fluxmeter._fluxmeter[0].prng
            return int(prng.get_seed(prng))

    @seed.setter
    def seed(self, value):
        fluxmeter = self._fluxmeter()
        if fluxmeter is None:
            raise RuntimeError("dead fluxmeter ref")
        else:
            value = ffi.NULL if value is None else \
                    ffi.new("unsigned long [1]", (value,))
            prng = fluxmeter._fluxmeter[0].prng
            prng.set_seed(prng, value)

    def __init__(self, fluxmeter):
        assert(isinstance(fluxmeter, Fluxmeter))
        self._fluxmeter = weakref.ref(fluxmeter)

    def __call__(self, n=None):
        """Get numbers pseudo-uniformly distributed overs (0, 1)"""

        fluxmeter = self._fluxmeter()
        if fluxmeter is None:
            raise RuntimeError("dead fluxmeter ref")
        else:
            prng = fluxmeter._fluxmeter[0].prng
            if n is None: n = 1
            values = numpy.empty(n, dtype="f8")
            lib.mulder_prng_uniform01_v(prng, n, _todouble(values))
            return _fromarray(values)


class Fluxmeter:
    """Muon flux calculator"""

    @property
    def mode(self):
        """Muons transport mode"""
        mode = self._fluxmeter[0].mode
        if mode == lib.MULDER_CSDA:
            return "csda"
        elif mode == lib.MULDER_MIXED:
            return "mixed"
        else:
            return "detailed"

    @mode.setter
    def mode(self, v):
        try:
            mode = getattr(lib, f"MULDER_{v.upper()}")
        except AttributeError:
            raise ValueError(f"bad mode (v)")
        else:
            self._fluxmeter[0].mode = mode

    @property
    def size(self):
        return int(self._fluxmeter[0].size)

    @property
    def physics(self):
        """Physics tabulations (stopping power etc.)"""
        return ffi.string(self._fluxmeter[0].physics).decode()

    @property
    def prng(self):
        """Pseudo random numbers generator"""
        return self._prng

    @property
    def reference(self):
        """Reference (opensky) flux model"""
        if self._reference is None:
            self._reference = Reference()
            self._reference._reference = ffi.new(
                "struct mulder_reference *[1]", (self._fluxmeter[0].reference,))
        return self._reference

    @reference.setter
    def reference(self, v):
        if not isinstance(v, Reference):
            raise TypeError("bad type (expected a mulder.Reference)")
        else:
            self._fluxmeter[0].reference = v._reference[0]
            self._reference = v

    def __init__(self, *layers, physics=None):

        if physics is None:
            physics = f"{PREFIX}/data/materials.pumas"

        fluxmeter = ffi.new("struct mulder_fluxmeter *[1]")
        fluxmeter[0] = lib.mulder_fluxmeter_create(
            _tostr(physics), len(layers), [l._layer[0] for l in layers])
        if fluxmeter[0] == ffi.NULL:
            raise LibraryError()

        weakref.finalize(self, lib.mulder_fluxmeter_destroy, fluxmeter)
        self._fluxmeter = fluxmeter
        self._reference = None
        self._prng = Prng(self)

    def flux(self, latitude, longitude, height, azimuth, elevation, energy):
        """Calculate muon flux"""

        energy = _asarray(energy)
        flux = numpy.empty(energy.size)

        rc = lib.mulder_fluxmeter_flux_v(self._fluxmeter[0], latitude,
            longitude, height, azimuth, elevation, energy.size,
            _todouble(energy), _todouble(flux))
        if rc != lib.MULDER_SUCCESS:
            raise LibraryError()

        return _fromarray(flux)

    def intersect(self, latitude, longitude, height, azimuth, elevation):
        """Compute first intersection with topographic layer(s)"""

        azimuth, elevation, size = _asarray2(azimuth, elevation)
        i = numpy.empty(size, dtype="i4")
        x = numpy.empty(size)
        y = numpy.empty(size)
        z = numpy.empty(size)

        rc = lib.mulder_fluxmeter_intersect_v(self._fluxmeter[0], latitude,
            longitude, height, size, _todouble(azimuth), _todouble(elevation),
            _toint(i), _todouble(x), _todouble(y), _todouble(z))
        if rc != lib.MULDER_SUCCESS:
            raise LibraryError()

        i = _fromarray(i)
        x = _fromarray(x)
        y = _fromarray(y)
        z = _fromarray(z)

        return i, x, y, z

    def grammage(self, latitude, longitude, height, azimuth, elevation):
        """Compute grammage(s) (a.k.a. column depth) along line(s) of sight"""

        azimuth, elevation, size = _asarray2(azimuth, elevation)
        m = self.size + 1
        grammage = numpy.empty(size * m)

        rc = lib.mulder_fluxmeter_grammage_v(self._fluxmeter[0], latitude,
            longitude, height, size, _todouble(azimuth), _todouble(elevation),
            _todouble(grammage))
        if rc != lib.MULDER_SUCCESS:
            raise LibraryError()

        grammage = _frommatrix(m, size, grammage)

        return grammage

    def whereami(self, latitude, longitude, height):
        """Get geometric layer indice(s) for given location(s)"""

        latitude, longitude, height, size = _asarray3(
            latitude, longitude, height)
        i = numpy.empty(size, dtype="i4")

        rc = lib.mulder_fluxmeter_whereami_v(self._fluxmeter[0], size,
            _todouble(latitude), _todouble(longitude), _todouble(height),
            _toint(i))
        if rc != lib.MULDER_SUCCESS:
            raise LibraryError()

        i = _fromarray(i)

        return i


def create_map(path, projection, x, y, data):
    """Create a Turtle map from a numpy array"""

    assert(len(x) > 1)
    assert(_is_regular(x))
    assert(len(y) > 1)
    assert(_is_regular(y))

    data = _asarray(data)

    assert(data.ndim == 2)
    assert(data.shape[0] == len(y))
    assert(data.shape[1] == len(x))

    rc = lib.mulder_map_create(_tostr(path), _tostr(projection), len(x),
        len(y), x[0], x[-1], y[0], y[-1], _todouble(data))
    if rc != lib.MULDER_SUCCESS:
        raise LibraryError()


def create_reference_table(path, height, cos_theta, energy, data):
    """Create a reference flux table from a numpy array"""

    # Check inputs
    assert(len(cos_theta) > 1)
    assert(_is_regular(cos_theta))
    assert(len(energy) > 1)
    assert(_is_regular(numpy.log(energy)))

    data = numpy.ascontiguousarray(data, dtype="f4")

    if isinstance(height, Number):
        assert(data.ndim == 3)
        assert(data.shape[0] == len(cos_theta))
        assert(data.shape[1] == len(energy))
        assert(data.shape[2] == 2)
        height = _asarray(height)
    else:
        assert(_is_regular(height))
        assert(data.ndim == 4)
        assert(data.shape[0] == len(height))
        assert(data.shape[1] == len(cos_theta))
        assert(data.shape[2] == len(energy))
        assert(data.shape[3] == 2)

    # Generate binary table file
    with open(path, "wb") as f:
        dims = numpy.array((len(energy), len(cos_theta), len(height)),
                           dtype="i8")
        dims.astype("<i8").tofile(f)

        lims = numpy.array((energy[0], energy[-1], cos_theta[0],
                            cos_theta[-1], altitude[0], altitude[-1]),
                           dtype="f8")
        lims.astype("<f8").tofile(f)

        data.flatten().astype("<f4").tofile(f)


def generate_physics(path, destination=None):
    """Generate physics tables for Pumas"""

    pathdir = str(Path(path).parent)
    if destination is None:
        destination = pathdir

    if not os.path.exists(destination):
        os.makedirs(destination)

    dump = str(Path(destination) / Path(path).with_suffix(".pumas").name)

    rc = lib.mulder_generate_physics(
        _tostr(path), _tostr(destination), _tostr(dump))
    if rc != lib.MULDER_SUCCESS:
        raise LibraryError()

    if pathdir != destination:
        shutil.copy(path, destination)
