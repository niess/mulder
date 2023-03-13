import os
from pathlib import Path
import shutil
from typing import NamedTuple, Optional
import weakref

import numpy

from .arrayclasses import arrayclass, commonsize
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


# Type conversions between cffi and numpy
_todouble = lambda x: ffi.cast("double *", x.ctypes.data)

_toint = lambda x: ffi.cast("int *", x.ctypes.data)

_tostr = lambda x: ffi.NULL if x is None else \
                   ffi.new("const char[]", x.encode())


# Decorated array types
@arrayclass
class Position:
    """Observation position, using geographic coordinates (GPS like)"""

    ctype = "struct mulder_position *"

    properties = (
        ("latitude",  "f8", "Geographic latitude, in deg"),
        ("longitude", "f8", "Geographic longitude, in deg"),
        ("height",    "f8", "Geographic height w.r.t. WGS84 ellipsoid, in m")
    )


@arrayclass
class Direction:
    """Observation direction, using Horizontal angular coordinates"""

    ctype = "struct mulder_direction *"

    properties = (
        ("azimuth",   "f8", "Azimuth angle, in deg, (clockwise from North)"),
        ("elevation", "f8", "Elevation angle, in deg, (w.r.t. horizontal)")
    )


@arrayclass
class Enu:
    """East, North, Upward (ENU) local coordinates"""

    ctype = "struct mulder_enu *"

    properties = (
        ("east",   "f8", "Local east-ward coordinate"),
        ("north",  "f8", "Local north-ward coordinate"),
        ("upward", "f8", "Local upward coordinate")
    )


@arrayclass
class Projection:
    """Projected (map) local coordinates"""

    ctype = "struct mulder_projection *"

    properties = (
        ("x", "f8", "Local x-coordinate"),
        ("y", "f8", "Local y-coordinate")
    )


@arrayclass
class Flux:
    """Container for muon flux data"""

    ctype = "struct mulder_flux *"

    properties = (
        ("value",     "f8", "The actual flux value, per GeV m^2 s sr"),
        ("asymmetry", "f8", "The corresponding charge asymmetry")
    )


@arrayclass
class Intersection:
    """Container for geometry intersection"""

    ctype = "struct mulder_intersection *"

    properties = (
        ("layer",    "i4",        "Intersected layer index"),
        ("position", Position, "Intersection position")
    )


@arrayclass
class State:
    """Observation state(s)"""

    ctype = "struct mulder_state *"

    properties = (
        ("pid",       "i4",        "Particle identifier (PDG scheme)"),
        ("position",  Position, "Observation position"),
        ("direction", Direction,   "Observation direction"),
        ("energy",    "f8",        "Kinetic energy, in GeV"),
        ("weight",    "f8",        "Transport weight")
    )

    def flux(self, reference: "Reference") -> Flux:
        """Sample a reference flux"""

        assert(isinstance(reference, Reference))

        size = self._size or 1
        flux = Flux(self._size)

        lib.mulder_state_flux_v(
            reference._reference[0],
            size,
            self.stride,
            self.cffi_ptr,
            flux.cffi_ptr
        )

        return flux


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
        layer[0] = lib.mulder_layer_create(
            _tostr(material),
            _tostr(model),
            0 if offset is None else offset
        )
        if layer[0] == ffi.NULL:
            raise LibraryError()
        else:
            self._layer = ffi.gc(
                layer,
                lib.mulder_layer_destroy
            )

    def height(self, projection: Projection):
        """Topography height (including offset)"""

        assert(isinstance(projection, Projection))

        size = projection._size or 1
        height = numpy.empty(size)

        lib.mulder_layer_height_v(
            self._layer[0],
            size,
            projection.stride,
            projection.cffi_ptr,
            _todouble(height)
        )

        return height if size > 1 else height[0]

    def gradient(self, projection: Projection) -> Projection:
        """Topography gradient (w.r.t. map coordinates)"""

        assert(isinstance(projection, Projection))

        size = projection._size or 1
        gradient = Projection.empty(projection._size)

        lib.mulder_layer_gradient_v(
            self._layer[0],
            size,
            projection.stride,
            projection.cffi_ptr,
            gradient.cffi_ptr
        )

        return gradient

    def position(self, projection: Projection) -> Position:
        """Get geographic position corresponding to map location"""

        assert(isinstance(projection, Projection))

        size = projection._size or 1
        position = Position.empty(projection.size)

        lib.mulder_layer_position_v(
            self._layer[0],
            size,
            projection.stride,
            projection.cffi_ptr,
            position.cffi_ptr
        )

        return position

    def project(self, position: Position) -> Projection:
        """Project geographic position onto map"""

        assert(isinstance(position, Position))

        size = position._size or 1
        projection = Projection.empty(position._size)

        lib.mulder_layer_project_v(
            self._layer[0],
            size,
            position.stride,
            position.cffi_ptr,
            projection.cffi_ptr
        )

        return projection


class Geomagnet:
    """Earth magnetic field"""

    @property
    def model(self):
        """Geomagnetic model"""
        v =  self._geomagnet[0].model
        return None if v == ffi.NULL else ffi.string(v).decode()

    @property
    def day(self):
        """Calendar day"""
        return int(self._geomagnet[0].day)

    @property
    def month(self):
        """Calendar month"""
        return int(self._geomagnet[0].month)

    @property
    def year(self):
        """Calendar year"""
        return int(self._geomagnet[0].year)

    @property
    def order(self):
        """Model harmonics order"""
        return int(self._geomagnet[0].order)

    @property
    def height_min(self):
        """Maximum model height, in m"""
        return float(self._geomagnet[0].height_min)

    @property
    def height_max(self):
        """Minimum model height, in m"""
        return float(self._geomagnet[0].height_max)


    def __init__(self, model=None, day=None, month=None, year=None):
        # Set default arguments
        if model is None: model = f"{PREFIX}/data/IGRF13.COF"
        if day is None: day = 1
        if month is None: month = 1
        if year is None: year = 2020

        # Create the C object
        geomagnet = ffi.new("struct mulder_geomagnet *[1]")
        geomagnet[0] = lib.mulder_geomagnet_create(
            _tostr(model),
            day,
            month,
            year
        )
        if geomagnet[0] == ffi.NULL:
            raise LibraryError()
        else:
            self._geomagnet = ffi.gc(
                geomagnet,
                lib.mulder_geomagnet_destroy
            )

    def field(self, position: Position) -> Enu:
        """Geomagnetic field, in T

        The magnetic field components are returned in East-North-Upward (ENU)
        coordinates.
        """

        assert(isinstance(position, Position))

        size = position._size or 1
        enu = Enu.empty(position._size)

        lib.mulder_geomagnet_field_v(
            self._geomagnet[0],
            size,
            position.stride,
            position.cffi_ptr,
            enu.cffi_ptr,
        )

        return enu


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
            # Map default flux
            self._reference = ffi.new(
                "struct mulder_reference *[1]",
                (lib.mulder_reference_default(),)
            )
        else:
            # Load a tabulated reference flux from a file
            reference = ffi.new("struct mulder_reference *[1]")
            reference[0] = lib.mulder_reference_load_table(
                _tostr(path)
            )
            if reference[0] == ffi.NULL:
                raise LibraryError()
            else:
                self._reference = ffi.gc(
                    reference,
                    lib.mulder_reference_destroy_table
                )

    def flux(self, elevation, energy, height=None):
        """Get reference flux model, defined at reference height(s)"""

        if height is None:
            height = 0.5 * (self.height_min + self.height_max)

        args = [numpy.asarray(a) for a in (height, elevation, energy)]
        size = commonsize(*args)
        strides = [a.strides[-1] if a.strides else 0 for a in args]
        height, elevation, energy = args

        flux = Flux.empty(size if size > 1 else None)

        lib.mulder_reference_flux_v(
            self._reference[0],
            size,
            strides,
            _todouble(height),
            _todouble(elevation),
            _todouble(energy),
            flux.cffi_ptr
        )

        return flux


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

    def __init__(self, fluxmeter: "Fluxmeter"):
        assert(isinstance(fluxmeter, Fluxmeter))
        self._fluxmeter = weakref.ref(fluxmeter)

    def __call__(self, n=None):
        """Get numbers pseudo-uniformly distributed overs (0, 1)"""

        fluxmeter = self._fluxmeter()
        if fluxmeter is None:
            raise RuntimeError("dead fluxmeter ref")
        else:
            if n is None: n = 1
            values = numpy.empty(n)

            prng = fluxmeter._fluxmeter[0].prng
            lib.mulder_prng_uniform01_v(
                prng,
                n,
                _todouble(values)
            )

            return values if n > 1 else values[0]


class Fluxmeter:
    """Muon flux calculator"""

    @property
    def geomagnet(self):
        """Earth magnetic field"""
        return self._geomagnet

    @geomagnet.setter
    def geomagnet(self, v):
        if v is None:
            self._geomagnet = None
            self._fluxmeter[0].geomagnet = ffi.NULL
        elif isinstance(v, Geomagnet):
            self._fluxmeter[0].geomagnet = v._geomagnet[0]
            self._geomagnet = v
        else:
            raise TypeError("bad type (expected a mulder.Geomagnet)")

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
            raise ValueError(f"bad mode ({v})")
        else:
            self._fluxmeter[0].mode = mode

    @property
    def selection(self):
        """Particle(s) selection"""
        sel = self._fluxmeter[0].selection
        if sel == lib.MULDER_ALL:
            return "all"
        elif sel == lib.MULDER_MUON:
            return "muon"
        else:
            return "antimuon"

    @selection.setter
    def selection(self, v):
        try:
            sel = getattr(lib, f"MULDER_{v.upper()}")
        except AttributeError:
            raise ValueError(f"bad selection ({v})")
        else:
            self._fluxmeter[0].selection = sel

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
            _tostr(physics),
            len(layers),
            [l._layer[0] for l in layers]
        )
        if fluxmeter[0] == ffi.NULL:
            raise LibraryError()
        else:
            self._fluxmeter = ffi.gc(
                fluxmeter,
                lib.mulder_fluxmeter_destroy
            )

        self._geomagnet = None
        self._reference = None
        self._prng = Prng(self)

    def flux(self, state: State) -> Flux:
        """Calculate the muon flux for the given observation state"""

        assert(isinstance(state, State))

        size = state._size or 1
        flux = Flux.empty(state._size)

        rc = lib.mulder_fluxmeter_flux_v(
            self._fluxmeter[0],
            size,
            state.stride,
            state.cffi_ptr,
            flux.cffi_ptr
        )
        if rc != lib.MULDER_SUCCESS:
            raise LibraryError()

        return flux

    def transport(self, state: State) -> State:
        """Transport observation state to the reference location"""

        assert(isinstance(state, State))

        size = state._size or 1
        result = State.empty(state._size)

        rc = lib.mulder_fluxmeter_transport_v(
            self._fluxmeter[0],
            size,
            state.stride,
            state.cffi_ptr,
            result.cffi_ptr
        )
        if rc != lib.MULDER_SUCCESS:
            raise LibraryError()

        return result

    def intersect(self, position: Position,
                        direction: Direction) -> Intersection:
        """Compute first intersection with topographic layer(s)"""

        assert(isinstance(position, Position))
        assert(isinstance(direction, Direction))

        size = commonsize(position, direction)
        intersection = Intersection.empty(size)

        rc = lib.mulder_fluxmeter_intersect_v(
            self._fluxmeter[0],
            size or 1,
            (position.stride, direction.stride),
            position.cffi_ptr,
            direction.cffi_ptr,
            intersection.cffi_ptr
        )
        if rc != lib.MULDER_SUCCESS:
            raise LibraryError()

        return intersection

    def grammage(self, position: Position,
                       direction: Direction) -> numpy.ndarray:
        """Compute grammage(s) (a.k.a. column depth) along line(s) of sight"""

        assert(isinstance(position, Position))
        assert(isinstance(direction, Direction))

        size = commonsize(position, direction)
        m = self.size + 1
        if size is None:
            grammage = numpy.empty(m)
        else:
            grammage = numpy.empty((size, m))

        rc = lib.mulder_fluxmeter_grammage_v(
            self._fluxmeter[0],
            size or 1,
            (position.stride, direction.stride),
            position.cffi_ptr,
            direction.cffi_ptr,
            _todouble(grammage)
        )
        if rc != lib.MULDER_SUCCESS:
            raise LibraryError()

        return grammage

    def whereami(self, position: Position) -> numpy.ndarray:
        """Get geometric layer indice(s) for given location(s)"""

        assert(isinstance(position, Position))

        size = position._size or 1
        i = numpy.empty(size, dtype="i4")

        rc = lib.mulder_fluxmeter_whereami_v(
            self._fluxmeter[0],
            size,
            position.stride,
            position.cffi_ptr,
            _toint(i)
        )
        if rc != lib.MULDER_SUCCESS:
            raise LibraryError()

        return i if size > 1 else i[0]


def create_map(path, projection, x, y, data):
    """Create a Turtle map from a numpy array"""

    assert(len(x) > 1)
    assert(_is_regular(x))
    assert(len(y) > 1)
    assert(_is_regular(y))

    data = ascontiguousarray(data, "d8")

    assert(data.ndim == 2)
    assert(data.shape[0] == len(y))
    assert(data.shape[1] == len(x))

    rc = lib.mulder_map_create(
        _tostr(path),
        _tostr(projection),
        len(x),
        len(y),
        x[0],
        x[-1],
        y[0],
        y[-1],
        _todouble(data)
    )
    if rc != lib.MULDER_SUCCESS:
        raise LibraryError()


def create_reference_table(path, height, cos_theta, energy, data):
    """Create a reference flux table from a numpy array"""

    def is_regular(a):
        """Check if a 1d array has a regular stepping"""
        d = numpy.diff(a)
        dmin, dmax = min(d), max(d)
        amax = max(numpy.absolute(a))
        return dmax - dmin <= 1E-15 * amax

    # Check inputs
    assert(len(cos_theta) > 1)
    assert(is_regular(cos_theta))
    assert(len(energy) > 1)
    assert(is_regular(numpy.log(energy)))

    data = numpy.ascontiguousarray(data, dtype="<f4")

    if isinstance(height, Number):
        assert(data.ndim == 3)
        assert(data.shape[0] == len(cos_theta))
        assert(data.shape[1] == len(energy))
        assert(data.shape[2] == 2)
    else:
        assert(is_regular(height))
        assert(data.ndim == 4)
        assert(data.shape[0] == len(height))
        assert(data.shape[1] == len(cos_theta))
        assert(data.shape[2] == len(energy))
        assert(data.shape[3] == 2)

    # Generate binary table file
    with open(path, "wb") as f:
        dims = numpy.array(
            (
                len(energy),
                len(cos_theta),
                len(height)
            ),
            dtype="i8"
        )
        dims.astype("<i8").tofile(f)

        lims = numpy.array(
            (
                energy[0],
                energy[-1],
                cos_theta[0],
                cos_theta[-1],
                height[0],
                height[-1]
            ),
            dtype="f8"
        )
        lims.astype("<f8").tofile(f)

        data.flatten().tofile(f)


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
