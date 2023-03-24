"""Core functionalities of the mulder library.
"""

from numbers import Integral, Number
from pathlib import Path
import weakref

import numpy

from .arrays import arrayclass, commonsize, pack
from .ffi import ffi, lib, LibraryError, todouble, toint, tostr
from .grids import MapGrid
from .types import Atmosphere, Direction, Enu, Flux, Intersection, \
                   MapLocation, Position, Projection


"""Package / C-library installation prefix."""
PREFIX = str(Path(__file__).parent.resolve())


@arrayclass
class State:
    """Observation state(s)."""

    ctype = "struct mulder_state *"

    properties = (
        ("pid",       "i4",        "Particle identifier (PDG scheme)."),
        ("position",  Position,    "Observation position."),
        ("direction", Direction,   "Observation direction."),
        ("energy",    "f8",        "Kinetic energy, in GeV."),
        ("weight",    "f8",        "Transport weight.")
    )

    def flux(self, reference: "Reference") -> Flux:
        """Sample a reference flux."""

        assert(isinstance(reference, Reference))

        size = self._size or 1
        flux = Flux.empty(self._size)

        lib.mulder_state_flux_v(
            reference._reference[0],
            size,
            self.numpy_stride,
            self.cffi_pointer,
            flux.cffi_pointer
        )

        return flux


class Layer:
    """Topographic layer."""

    @property
    def material(self):
        """Constituant material."""
        return ffi.string(self._layer[0].material).decode()

    @property
    def model(self):
        """Topographic model."""
        v =  self._layer[0].model
        return None if v == ffi.NULL else ffi.string(v).decode()

    @property
    def density(self):
        """Material density."""
        v = float(self._layer[0].density)
        return None if v == 0 else v

    @density.setter
    def density(self, value):
        self._layer[0].density = 0 if value is None else value

    @property
    def offset(self):
        """Elevation offset."""
        v = float(self._layer[0].offset)
        return None if v == 0 else v

    @property
    def encoding(self):
        """Map encoding format."""
        v =  self._layer[0].encoding
        return None if v == ffi.NULL else ffi.string(v).decode()

    @property
    def projection(self):
        """Map cartographic projection."""
        v =  self._layer[0].projection
        return None if v == ffi.NULL else ffi.string(v).decode()

    @property
    def nx(self):
        """Map size along x-axis."""
        return int(self._layer[0].nx)

    @property
    def ny(self):
        """Map size along y-axis."""
        return int(self._layer[0].ny)

    @property
    def xmin(self):
        """Map minimum value along x-axis."""
        return float(self._layer[0].xmin)

    @property
    def xmax(self):
        """Map maximum value along x-axis."""
        return float(self._layer[0].xmax)

    @property
    def ymin(self):
        """Map minimum value along y-axis."""
        return float(self._layer[0].ymin)

    @property
    def ymax(self):
        """Map maximum value along y-axis."""
        return float(self._layer[0].ymax)

    @property
    def zmin(self):
        """Map minimum value along z-axis."""
        return float(self._layer[0].zmin)

    @property
    def zmax(self):
        """Map maximum value along z-axis."""
        return float(self._layer[0].zmax)

    @property
    def bottom_left(self):
        """Layer's bottom-left location."""
        return self._get_location(
            "_bottom_left",
            (1, 0, 1, 0)
        )

    @property
    def bottom_right(self):
        """Layer's bottom-right location."""
        return self._get_location(
            "_bottom_right",
            (0, 1, 1, 0)
        )

    @property
    def middle(self):
        """Layer's middle location."""
        return self._get_location(
            "_middle",
            (0.5, 0.5, 0.5, 0.5)
        )

    @property
    def top_left(self):
        """Layer's top-left location."""
        return self._get_location(
            "_top_left",
            (1, 0, 0, 1)
        )

    @property
    def top_right(self):
        """Layer's top-right location."""
        return self._get_location(
            "_top_right",
            (0, 1, 0, 1)
        )

    def __init__(self, material=None, model=None, density=None, offset=None):
        if material is None: material = "Rock"

        layer = ffi.new("struct mulder_layer *[1]")
        layer[0] = lib.mulder_layer_create(
            tostr(material),
            tostr(model),
            0 if offset is None else offset
        )
        if layer[0] == ffi.NULL:
            raise LibraryError()
        else:
            self._layer = ffi.gc(
                layer,
                lib.mulder_layer_destroy
            )

    def __repr__(self):
        args = [self.material]
        if self.model: args.append(self.model)
        if self.offset: args.append(f"{self.offset:+g}")
        args = ", ".join(args)
        return f"Layer({args})"

    def grid(self):
        """Return topography data as a MapGrid object"""

        if self.model is None:
            return None
        else:
            x = numpy.linspace(self.xmin, self.xmax, self.nx)
            y = numpy.linspace(self.ymin, self.ymax, self.ny)
            grid = MapGrid(x, y)
            grid.height[:] = self.height(**grid.nodes)
            return grid

    def gradient(self, *args, **kwargs) -> Projection:
        """Topography gradient (w.r.t. map coordinates)."""

        projection = Projection.parse(*args, **kwargs)

        size = projection._size or 1
        gradient = Projection.empty(projection._size)

        lib.mulder_layer_gradient_v(
            self._layer[0],
            size,
            projection.numpy_stride,
            projection.cffi_pointer,
            gradient.cffi_pointer
        )

        return gradient

    def height(self, *args, **kwargs) -> numpy.ndarray:
        """Topography height (including offset)."""

        projection = Projection.parse(*args, **kwargs)

        size = projection._size or 1
        height = numpy.empty(size)

        lib.mulder_layer_height_v(
            self._layer[0],
            size,
            projection.numpy_stride,
            projection.cffi_pointer,
            todouble(height)
        )

        return height if size > 1 else height[0]

    def position(self, *args, **kwargs) -> Position:
        """Get geographic position corresponding to map location."""

        projection = Projection.parse(*args, **kwargs)

        size = projection._size or 1
        position = Position.empty(projection.size)

        lib.mulder_layer_position_v(
            self._layer[0],
            size,
            projection.numpy_stride,
            projection.cffi_pointer,
            position.cffi_pointer
        )

        return position

    def project(self, *args, **kwargs) -> Projection:
        """Project geographic position onto map."""

        position = Position.parse(*args, **kwargs)

        size = position._size or 1
        projection = Projection.empty(position._size)

        lib.mulder_layer_project_v(
            self._layer[0],
            size,
            position.numpy_stride,
            position.cffi_pointer,
            projection.cffi_pointer
        )

        return projection

    def _get_location(self, name, signature):
        """Get a specific (signed) map location."""
        try:
            location = getattr(self, name)
        except AttributeError:
            projection = Projection(
                signature[0] * self.xmin + signature[1] * self.xmax,
                signature[2] * self.ymin + signature[3] * self.ymax
            )
            location = MapLocation(
                self.position(projection),
                projection
            )
            setattr(self, name, location)
        return location.copy()


class Geomagnet:
    """Earth magnetic field."""

    @property
    def model(self):
        """Geomagnetic model."""
        v =  self._geomagnet[0].model
        return None if v == ffi.NULL else ffi.string(v).decode()

    @property
    def day(self):
        """Calendar day."""
        return int(self._geomagnet[0].day)

    @property
    def month(self):
        """Calendar month."""
        return int(self._geomagnet[0].month)

    @property
    def year(self):
        """Calendar year."""
        return int(self._geomagnet[0].year)

    @property
    def order(self):
        """Model harmonics order."""
        return int(self._geomagnet[0].order)

    @property
    def height_min(self):
        """Maximum model height, in m."""
        return float(self._geomagnet[0].height_min)

    @property
    def height_max(self):
        """Minimum model height, in m."""
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
            tostr(model),
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

    def field(self, *args, **kwargs) -> Enu:
        """Geomagnetic field, in T.

        The magnetic field components are returned in East-North-Upward (ENU)
        coordinates.
        """

        position = Position.parse(*args, **kwargs)

        size = position._size or 1
        enu = Enu.empty(position._size)

        lib.mulder_geomagnet_field_v(
            self._geomagnet[0],
            size,
            position.numpy_stride,
            position.cffi_pointer,
            enu.cffi_pointer,
        )

        return enu


class Geometry:
    """Stratified Earth geometry."""

    @property
    def geomagnet(self):
        """Earth magnetic field."""
        return self._geomagnet

    @geomagnet.setter
    def geomagnet(self, v):
        if v is True: v = Geomagnet()
        if not v:
            self._geomagnet = None
            self._geometry[0].geomagnet = ffi.NULL
        elif isinstance(v, Geomagnet):
            self._geometry[0].geomagnet = v._geomagnet[0]
            self._geomagnet = v
        else:
            raise TypeError("bad type (expected a mulder.Geomagnet)")

    @property
    def layers(self):
        """Topographic layers."""
        return self._layers

    def __init__(self, *args, geomagnet=None, **kwargs):

        # Convert arguments to Layers.
        def fromarg(v):
            if isinstance(v, Layer):
                return v
            elif isinstance(v, dict):
                return Layer(**v)
            elif hasattr(v, "__iter__"):
                return Layer(*v)
            else:
                raise TypeError(f"bad Geometry argument ({type(v)})")

        def fromkwarg(k, v):
            # Note that Python version greater than 3.6 is implied. That is,
            # PEP 468, preserving the order of kwargs.

            if isinstance(v, str):
                return Layer(material = k, model = v)
            elif isinstance(v, Number):
                return Layer(material = k, offset = v)
            else:
                raise TypeError(f"bad Geometry argument ({type(v)})")

        layers = [fromarg(v) for v in args] + \
                 [fromkwarg(k, v) for k, v in kwargs.items()]

        # Create the geometry.
        geometry = ffi.new("struct mulder_geometry *[1]")
        geometry[0] = lib.mulder_geometry_create(
            len(layers),
            [layer._layer[0] for layer in layers]
        )
        if geometry[0] == ffi.NULL:
            raise LibraryError()
        else:
            self._geometry = ffi.gc(
                geometry,
                lib.mulder_geometry_destroy
            )

        self._layers = tuple(layers)
        self._geomagnet = None

        if geomagnet:
            self.geomagnet = geomagnet

    def atmosphere(self, height) -> Atmosphere:
        """Return atmosphere local properties, at given height.

        The magnetic field components are returned in East-North-Upward (ENU)
        coordinates.
        """

        height = numpy.asarray(height, dtype="f8")

        size = height.size
        stride = height.strides[-1] if height.strides else 0
        atmosphere = Atmosphere.empty(None if size <= 1 else size)

        lib.mulder_geometry_atmosphere_v(
            self._geometry[0],
            size,
            stride,
            todouble(height),
            atmosphere.cffi_pointer
        )

        return atmosphere


class Reference:
    """Reference (opensky) muon flux."""

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
                tostr(path)
            )
            if reference[0] == ffi.NULL:
                raise LibraryError()
            else:
                self._reference = ffi.gc(
                    reference,
                    lib.mulder_reference_destroy_table
                )

    def flux(self, elevation, energy, height=None):
        """Get reference flux model, defined at reference height(s)."""

        if height is None:
            height = 0.5 * (self.height_min + self.height_max)

        args = [numpy.asarray(a, dtype="f8") \
                for a in (height, elevation, energy)]
        size = commonsize(*args)
        strides = [a.strides[-1] if a.strides else 0 for a in args]
        height, elevation, energy = args

        flux = Flux.empty(size)

        lib.mulder_reference_flux_v(
            self._reference[0],
            size or 1,
            strides,
            todouble(height),
            todouble(elevation),
            todouble(energy),
            flux.cffi_pointer
        )

        return flux


class Prng:
    """Pseudo random numbers generator."""

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
        """Get numbers pseudo-uniformly distributed overs (0, 1)."""

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
                todouble(values)
            )

            return values if n > 1 else values[0]


class Fluxmeter:
    """Muon flux calculator."""

    @property
    def geometry(self):
        """Stratified Earth geometry."""
        return self._geometry

    @property
    def mode(self):
        """Muons transport mode."""
        mode = self._fluxmeter[0].mode
        if mode == lib.MULDER_CONTINUOUS:
            return "continuous"
        elif mode == lib.MULDER_MIXED:
            return "mixed"
        else:
            return "discrete"

    @mode.setter
    def mode(self, v):
        try:
            mode = getattr(lib, f"MULDER_{v.upper()}")
        except AttributeError:
            raise ValueError(f"bad mode ({v})")
        else:
            self._fluxmeter[0].mode = mode

    @property
    def physics(self):
        """Physics tabulations (stopping power etc.)."""
        return ffi.string(self._fluxmeter[0].physics).decode()

    @property
    def prng(self):
        """Pseudo random numbers generator."""
        return self._prng

    @property
    def reference(self):
        """Reference (opensky) flux model."""
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

    def __init__(self, *args, physics=None, **kwargs):

        geometry = pack(Geometry, *args, **kwargs)

        if physics is None:
            physics = f"{PREFIX}/data/materials.pumas"

        fluxmeter = ffi.new("struct mulder_fluxmeter *[1]")
        fluxmeter[0] = lib.mulder_fluxmeter_create(
            tostr(physics),
            geometry._geometry[0]
        )
        if fluxmeter[0] == ffi.NULL:
            raise LibraryError()
        else:
            self._fluxmeter = ffi.gc(
                fluxmeter,
                lib.mulder_fluxmeter_destroy
            )

        self._geometry = geometry
        self._reference = None
        self._prng = Prng(self)

    def flux(self, *args, **kwargs) -> Flux:
        """Calculate the muon flux for the given observation state."""

        state = State.parse(*args, **kwargs)

        size = state._size or 1
        flux = Flux.empty(state._size)

        rc = lib.mulder_fluxmeter_flux_v(
            self._fluxmeter[0],
            size,
            state.numpy_stride,
            state.cffi_pointer,
            flux.cffi_pointer
        )
        if rc != lib.MULDER_SUCCESS:
            raise LibraryError()

        return flux

    def transport(self, *args, events=None, **kwargs) -> State:
        """Transport observation state to the reference location."""

        state = State.parse(*args, **kwargs)

        size = state._size or 1
        if events is not None:
            assert(isinstance(events, Integral))
            assert(events > 0)
            result = State.empty(events * size)
        else:
            events = 1
            result = State.empty(state._size)

        rc = lib.mulder_fluxmeter_transport_v(
            self._fluxmeter[0],
            events,
            size,
            state.numpy_stride,
            state.cffi_pointer,
            result.cffi_pointer
        )
        if rc != lib.MULDER_SUCCESS:
            raise LibraryError()

        return result

    def intersect(self, position: Position,
                        direction: Direction) -> Intersection:
        """Compute first intersection with topographic layer(s)."""

        assert(isinstance(position, Position))
        assert(isinstance(direction, Direction))

        size = commonsize(position, direction)
        intersection = Intersection.empty(size)

        rc = lib.mulder_fluxmeter_intersect_v(
            self._fluxmeter[0],
            size or 1,
            (position.numpy_stride, direction.numpy_stride),
            position.cffi_pointer,
            direction.cffi_pointer,
            intersection.cffi_pointer
        )
        if rc != lib.MULDER_SUCCESS:
            raise LibraryError()

        return intersection

    def grammage(self, position: Position,
                       direction: Direction) -> numpy.ndarray:
        """Compute grammage(s) (a.k.a. column depth) along line(s) of sight."""

        assert(isinstance(position, Position))
        assert(isinstance(direction, Direction))

        size = commonsize(position, direction)
        m = len(self.geometry.layers) + 1
        if size is None:
            grammage = numpy.empty(m)
        else:
            grammage = numpy.empty((size, m))

        rc = lib.mulder_fluxmeter_grammage_v(
            self._fluxmeter[0],
            size or 1,
            (position.numpy_stride, direction.numpy_stride),
            position.cffi_pointer,
            direction.cffi_pointer,
            todouble(grammage)
        )
        if rc != lib.MULDER_SUCCESS:
            raise LibraryError()

        return grammage

    def whereami(self, *args, **kwargs) -> numpy.ndarray:
        """Get geometric layer indice(s) for given location(s)."""

        position = Position.parse(*args, **kwargs)

        size = position._size or 1
        i = numpy.empty(size, dtype="i4")

        rc = lib.mulder_fluxmeter_whereami_v(
            self._fluxmeter[0],
            size,
            position.numpy_stride,
            position.cffi_pointer,
            toint(i)
        )
        if rc != lib.MULDER_SUCCESS:
            raise LibraryError()

        return i if size > 1 else i[0]
