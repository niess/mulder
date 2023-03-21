#! /usr/bin/env python3
"""This example illustrates the usage of mulder.Geometry objects.

Geometry objects are containers for representing a Stratified Earth Geometry
(SEG). They have little capabilities on their own. However, the geometry can be
investigated with a Fluxmeter which is briefly discussed below (see the
fluxmeter.py example for more detailed usage).

Note that this example assumes that you are already familiar with mulder.Layer
objects. Please, check the layer.py example first, otherwise.
"""

import matplotlib.pyplot as plot
import matplotlib.colors as colors
from mulder import Grid, Fluxmeter, Geomagnet, Geometry, Layer
import numpy


# =============================================================================
# A mulder geometry is defined as a vertical stack of Layers composed of, a
# priori, different materials. A layer is limited on top by a topography
# surface, and at bottom by the top of the layer below in the stack. Thus, a
# geometry is created by stacking Layers, as

geometry = Geometry(
    Layer(material="Rock", model="data/GMRT.asc"),
    Layer(material="Water")
)

# Note that top layers have high indices and bottom layers low indices. Thus,
# in the previous example, the water layer is on top of the rock one, but it
# appears after (below) in reading order.

# The geometry object has only a few properties, listed below.

print(f"""\
Geometry properties:
- layers:    {geometry.layers}
- geomagnet: {geometry.geomagnet}
""")

# Note that the stack of *layers* is immutable (it is a tuple). Note also that
# by default the Earth has no magnetic field. The geometry is magnetised as

geometry.geomagnet = Geomagnet()

# See the geomagnet.py example for more information of mulder.Geomagnet(ic)
# fields.


# =============================================================================
# Geometry objects always have a special Atmosphere layer at the very top, that
# was not indicated in the previous list of layers. This layer is composed of
# Air with a variable density.
#
# The atmosphere local properties are obtained with the Geometry.atmosphere
# method. For example, as

density, gradient = geometry.atmosphere(height=1E+04)

print(f"""\
Atmosphere properties (at 10 km):
- density:  {density} kg / m^3
- gradient: {gradient} kg / m^4
""")

# Let draw mulder's atmosphere density profile, for illustration. Note that
# by default the US Standard (USStd) density profile is used, with CORSIKA
# parameterisation.

height = numpy.logspace(2, 5, 301)
atmosphere = geometry.atmosphere(height)

plot.style.use("examples/examples.mplstyle")
plot.figure()
plot.plot(height, atmosphere.density, "k-")
plot.xscale("log")
plot.xlabel("height (m)")
plot.ylabel("atmosphere density (kg / m$^3$)")
plot.show(block=False)


# =============================================================================
# Geometry objects have little capabilities on their own. However, the geometry
# can be investigated with a mulder.Fluxmeter. For example, the
# Fluxmeter.whereami method return the layer index at a given position. Let us
# draw a side view of the geometry using the later method. First, we create a
# Fluxmeter by providing a geometry

fluxmeter = Fluxmeter(geometry)

# Then, let us generate a grid for the side view using a mulder.Grid object (see
# the grids.py example for more information on Grids). We use the rock layer
# metadata in order to get consistent grid coordinates. Thus,

rock = geometry.layers[0]
latitude = 0.5 * (rock.ymin + rock.ymax)

grid = Grid(
    longitude = numpy.linspace(rock.xmin, rock.xmax, 1001),
    height = numpy.linspace(rock.zmin, rock.zmax, 1001)
)

# Then, we obtain the layer indices from the fluxmeter, as

index = fluxmeter.whereami(latitude=latitude, **grid.nodes)

# Finally, let us plot the result using a custom color map. For comparison, we
# also superimpose the corresponding topography, from the rock layer.

cmap = colors.ListedColormap((
    colors.CSS4_COLORS["saddlebrown"],
    colors.CSS4_COLORS["royalblue"],
    colors.CSS4_COLORS["lightblue"]
))

plot.figure()
plot.pcolormesh(
    grid.base.longitude,
    grid.base.height,
    grid.reshape(index),
    cmap = cmap,
    vmin = 0,
    vmax = len(geometry.layers)
)
plot.plot(
    grid.base.longitude,
    rock.height(x=grid.base.longitude, y=latitude),
    "w-"
)
plot.xlabel("longitude (deg)")
plot.ylabel("height (m)")
plot.title("Layer index")
cbar = plot.colorbar()
cbar.set_ticks(range(0, len(geometry.layers) + 1))
plot.show()
