#! /usr/bin/env python3
"""This example illustrates the usage of mulder.Geomagnet objects.

Geomagnets represent a snapshot of the Earth magnetic field at a given date.
They can be attached to Geometry objects in order to magnetize the atmosphere
(see the geometry.py example).
"""

import matplotlib.pyplot as plot
from mulder import Geomagnet
import numpy


# =============================================================================
# The example below creates a snapshot of the geomagnetic field at the given
# date. Note that all arguments are optionnal.

geomagnet = Geomagnet(
    day = 1,
    month = 1,
    year = 2020
)

# Let us print the geomagnet metadata, for illustrative purpose.

print(f"""\
Geomagnet metadata:
- model: {geomagnet.model}
- order: {geomagnet.order}
- day:   {geomagnet.day}
- month: {geomagnet.month}
- year:  {geomagnet.year}
""")

# Note that the Geomagnet uses IGRF parametrisation as defaul model (.COF file).
# A different .COF file can be provided using the *model* argument when creating
# the Geomagnet. The *order* property refers to the parametrisation order, based
# on spherical harmonics.

# Geomagnet snapshots are immutable. For example, the following raises an error

try:
    geomagnet.day = 2
except AttributeError:
    pass


# =============================================================================
# The Geomagnet.field method returns the geomagnetic field at a given Earth
# location. The field components are returned in a local East, North, Upward
# (ENU) frame, using Tesla (T) as unit. For example

field = geomagnet.field(latitude=45, longitude=3, height=1E+03)

print(f"""\
Field components:
- east:   {field.east} T
- north:  {field.north} T
- upward: {field.upward} T
""")


# =============================================================================
# As an example, let us plot the total intensity of the geomagnetic field over
# the Earth.
#
# First, we define a grid of coordinates, as

latitude = numpy.linspace(-90, 90, 181)
longitude = numpy.linspace(-180, 180, 361)

# Mulder functions can operate over vectorized inputs. However, not directly
# over grids. Thus, we generate flattened coordinates using numpy.meshgrid

x, y = [a.flatten() for a in numpy.meshgrid(longitude, latitude)]

# Then, let us compute the geomagnetic field total intensity at grid points

e, n, u = geomagnet.field(latitude=y, longitude=x)
intensity = numpy.sqrt(e**2 + n**2 + u**2)


# Finally, we plot the result. Note that first, we need to reshape the intensity
# values as a grid.

intensity = intensity.reshape((latitude.size, longitude.size))
intensity *= 1E+04 # G.

plot.style.use("examples/examples.mplstyle")
plot.figure()
plot.pcolormesh(
    longitude,
    latitude,
    intensity,
    vmin=0,
    vmax=0.7,
    cmap="hot"
)
plot.xlabel("longitude (deg)")
plot.ylabel("latitude (deg)")
plot.title("Earth magnetic field (G)")
plot.colorbar()
plot.show()
