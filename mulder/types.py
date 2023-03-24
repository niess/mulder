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
        if isinstance(other, Flux):
            value = self.value + other.value
            asymmetry = (
                    self.asymmetry * self.value +
                    other.asymmetry * other.value
                ) / \
                value
            return Flux(value, asymmetry)
        else:
            return NotImplemented()


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
