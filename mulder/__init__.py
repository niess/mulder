from pathlib import Path
import weakref

import numpy

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
        if rc == lib.MULDER_FAILURE:
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
        if rc == lib.MULDER_FAILURE:
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
        if rc == lib.MULDER_FAILURE:
            raise LibraryError()

        return _fromarray2(x, y)


class Fluxmeter:
    """Muon flux calculator"""

    @property
    def size(self):
        return int(self._fluxmeter[0].size)

    @property
    def physics(self):
        """Physics tabulations (stopping power etc.)"""
        return ffi.string(self._fluxmeter[0].physics).decode()

    @property
    def reference_height(self):
        """Height of reference flux model"""
        return float(self._fluxmeter[0].reference_height)

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

    def flux(self, latitude, longitude, height, azimuth, elevation, energy):
        """Calculate muon flux"""

        energy = _asarray(energy)
        flux = numpy.empty(energy.size)

        rc = lib.mulder_fluxmeter_flux_v(self._fluxmeter[0], latitude,
            longitude, height, azimuth, elevation, energy.size,
            _todouble(energy), _todouble(flux))
        if rc == lib.MULDER_FAILURE:
            raise LibraryError()

        return _fromarray(flux)

    def reference_flux(self, elevation, energy):
        """Get reference flux model, defined at reference height"""

        energy = _asarray(energy)
        flux = numpy.empty(energy.size)

        rc = lib.mulder_fluxmeter_reference_flux_v(self._fluxmeter[0],
            elevation, energy.size, _todouble(energy), _todouble(flux))
        if rc == lib.MULDER_FAILURE:
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
        if rc == lib.MULDER_FAILURE:
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
        if rc == lib.MULDER_FAILURE:
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
        if rc == lib.MULDER_FAILURE:
            raise LibraryError()

        i = _fromarray(i)

        return i


def create_map(path, projection, xlim, ylim, data):
    """Create a Turtle map from a numpy array"""

    path = ffi.new("const char[]", path.encode())
    projection = ffi.new("const char[]", projection.encode())
    data = numpy.asarray(data, dtype="f8", order="C")

    todouble = lambda x: ffi.cast("double *", x.ctypes.data)
    rc = lib.mulder_map_create(path, projection, data.shape[1], data.shape[0],
        xlim[0], xlim[1], ylim[0], ylim[1], todouble(data))
    if rc == lib.MULDER_FAILURE:
        raise LibraryError()


# XXX Add materials builder
