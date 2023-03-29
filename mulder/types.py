"""Basic container like types.
"""

from typing import NamedTuple

import numpy

from .arrays import Algebraic, arrayclass


@arrayclass
class Atmosphere:
    """Container for atmosphere local properties."""

    ctype = "struct mulder_atmosphere *"

    properties = (
        ("density",  "f8", "Local density, in kg / m^3."),
        ("gradient", "f8", "Density gradient, in kg / m^4.")
    )


@arrayclass
class Direction(Algebraic):
    """Observation direction, using Horizontal angular coordinates."""

    ctype = "struct mulder_direction *"

    properties = (
        ("azimuth",   "f8", "Azimuth angle, in deg, (clockwise from North)."),
        ("elevation", "f8", "Elevation angle, in deg, (w.r.t. horizontal).")
    )


@arrayclass
class Enu(Algebraic):
    """East, North, Upward (ENU) local coordinates."""

    ctype = "struct mulder_enu *"

    properties = (
        ("east",   "f8", "Local east-ward coordinate."),
        ("north",  "f8", "Local north-ward coordinate."),
        ("upward", "f8", "Local upward coordinate.")
    )


@arrayclass
class Flux:
    """Container for muon flux data."""

    ctype = "struct mulder_flux *"

    properties = (
        ("value",     "f8", "The actual flux value, per GeV m^2 s sr."),
        ("asymmetry", "f8", "The corresponding charge asymmetry.")
    )

    def __add__(self, other):
        """Combine two fluxes with proper computation of the resulting
        asymmetry.
        """

        if isinstance(other, ReducedFlux):
            return ReducedFlux.__add__(other, self)
        else:
            assert(isinstance(other, Flux))
            value = self.value + other.value
            asymmetry = (
                    self.asymmetry * self.value +
                    other.asymmetry * other.value
                ) / \
                value
            return Flux(value, asymmetry)

    def reduce(self, weight=None):
        """Return a reduced flux estimate."""
        return ReducedFlux.reduce(self, weight=weight)


class FluxStatistics(NamedTuple):
    """Statictics container summarising a set of flux data."""

    events: int
    zeros: int

    max_value: float = 0
    min_value: float = 0

    sum_value: float = 0
    sum_value2: float = 0
    weighted_sum_asymmetry: float = 0
    weighted_sum_asymmetry2: float = 0


class ReducedFlux(Flux):
    """Container for reduced muon flux data."""

    @classmethod
    def reduce(cls, flux, weight=None):
        """Return a reduced flux estimate."""
        if isinstance(flux, cls):
            obj = flux.copy()
            obj._statistics = flux._statistics.copy()
            if weight is not None:
                obj.value *= weight
            return obj
        else:
            assert(isinstance(flux, Flux))
            events = flux._size or 1
            values = flux.value
            if weight is not None:
                values *= weight
            sel = values > 0
            zeros = events - sum(sel)
            values = values[sel]
            asymmetries = flux.asymmetry[sel]
            return cls._build_stats(
                events,
                zeros,
                values,
                values**2,
                values * asymmetries,
                values * asymmetries**2,
            )

    @classmethod
    def _build_stats(cls, events, zeros, v, v2, wa, wa2):
        obj = cls.empty(None)
        if zeros == events:
            statistics = FluxStatistics(events, zeros)
        else:
            statistics = FluxStatistics(
                events = events,
                zeros = zeros,
                max_value = max(v),
                min_value = min(v),
                sum_value = sum(v),
                sum_value2 = sum(v2),
                weighted_sum_asymmetry = sum(wa),
                weighted_sum_asymmetry2 = sum(wa2)
            )
            obj.value = statistics.sum_value / events
            obj.asymmetry = statistics.weighted_sum_asymmetry / \
                            statistics.sum_value
        obj._statistics = statistics
        return obj

    @property
    def asymmetry_error(self):
        """Error estimate on asymmetry value."""
        statistics = self._statistics
        n = statistics.events - 1
        if (n == 0) or (statistics.zeros == statistics.events):
            return 0
        tmp = statistics.weighted_sum_asymmetry2 - \
              statistics.weighted_sum_asymmetry**2
        if tmp <= 0:
            return 0
        else:
            return numpy.sqrt(tmp / (n * statistics.sum_value))

    @property
    def events(self):
        return self._statistics.events

    @property
    def value_error(self):
        """Error estimate on flux value."""
        statistics = self._statistics
        n = statistics.events - 1
        if (n == 0) or (statistics.zeros == statistics.events):
            return 0
        tmp = statistics.sum_value2 / statistics.events - self.value**2
        if tmp <= 0:
            return 0
        else:
            return numpy.sqrt(tmp / n)

    @property
    def value_max(self):
        """Maximum flux value."""
        return self._statistics.max_value

    @property
    def value_min(self):
        """Minimum (strictly positive) flux value."""
        return self._statistics.min_value

    @property
    def zeros(self):
        """Number of null flux values."""
        return self._statistics.zeros

    def __add__(self, other):
        """Add to a ReducedFlux with propagation of statistics."""

        if isinstance(other, ReducedFlux):
            s_other = other._statistics
        elif isinstance(other, Flux):
            s_other = other.reduce()._statistics
        else:
            return NotImplemented()

        s_self = self._statistics

        return self._build_stats(
            s_self.events + s_other.events,
            s_self.zeros + s_other.zeros,
            numpy.array((s_self.sum_value, s_other.sum_value)),
            numpy.array((s_self.sum_value2, s_other.sum_value2)),
            numpy.array((
                s_self.weighted_sum_asymmetry,
                s_other.weighted_sum_asymmetry
            )),
            numpy.array((
                s_self.weighted_sum_asymmetry2,
                s_other.weighted_sum_asymmetry2
            ))
        )

    def __radd__(self, other):
        return other.__add__(self)


@arrayclass
class Position(Algebraic):
    """Observation position, using geographic coordinates (GPS like)."""

    ctype = "struct mulder_position *"

    properties = (
        ("latitude",  "f8", "Geographic latitude, in deg."),
        ("longitude", "f8", "Geographic longitude, in deg."),
        ("height",    "f8", "Geographic height w.r.t. WGS84 ellipsoid, in m.")
    )


@arrayclass
class Projection(Algebraic):
    """Projected (map) local coordinates."""

    ctype = "struct mulder_projection *"

    properties = (
        ("x", "f8", "Local x-coordinate."),
        ("y", "f8", "Local y-coordinate.")
    )


@arrayclass
class Intersection:
    """Container for geometry intersection."""

    ctype = "struct mulder_intersection *"

    properties = (
        ("layer",    "i4",     "Intersected layer index."),
        ("position", Position, "Intersection position.")
    )


class MapLocation(NamedTuple):
    """Container for representing a map location."""

    """Location geographic coordinates"""
    position: Position

    """Location projected coordinates"""
    projection: Projection

    @property
    def latitude(self):
        """Location latitude coordinate, in deg."""
        return self.position.latitude

    @property
    def longitude(self):
        """Location longitude coordinate, in deg."""
        return self.position.longitude

    @property
    def height(self):
        """Location height coordinate, in m."""
        return self.position.height

    @property
    def x(self):
        """Location x coordinate."""
        return self.projection.x

    @property
    def y(self):
        """Location y coordinate."""
        return self.projection.y

    def copy(self):
        return MapLocation(
            self.position.copy(),
            self.projection.copy()
        )
