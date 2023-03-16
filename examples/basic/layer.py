#! /usr/bin/env python3
"""This example illustrates the usage of Layer objects.

Layers are the building bricks of a Stratified Earth Geometry (SEG). See the
geometry.py example for information on SEGs.

This example uses Global Multi-Resolution Topography (GMRT) data obtained from
https://www.gmrt.org/. For example, using the GridServer Web Service, the
following url

http://www.gmrt.org/services/GridServer?north=38.85&west=15.15&east=15.30&south=38.75&format=esriascii

should download a map of the Stromboli island, in ESRI ASCII format, that is
stored under data/GMRT.asc.
"""

import matplotlib.pyplot as plot
from mulder import Layer, create_map
from mulder.matplotlib import LightSource
import numpy


# The example below should create a layer made of *Rock*, with a bulk *density*
# of 2 g/cm^3. Note that Mulder actually uses SI units, thus kg/m^3. Note also
# that specifying a bulk density is optional. If no value is provided, then the
# material intrinsic density is assumed.
#
# The *model* argument refers to topography data describing the layer's top
# interface. If no *model* is specified, then a flat topography is assumed.
# Optionaly, an *offset* could be added as well (which we do not here).

layer = Layer(
    material = "Rock",
    density = 2.0E+03, # kg / m^3
    model = "data/GMRT.asc",
)

# Let us print some metadata related to the topography model, for illustrative
# purpose.

print(f"""\
Map metadata:
- model:      {layer.model}
- projection: {layer.projection}
- nx:         {layer.nx}
- ny:         {layer.ny}
- xmin:       {layer.xmin}
- xmax:       {layer.xmax}
- ymin:       {layer.ymin}
- ymax:       {layer.ymax}
""")

# Mulder uses geographic (GPS-like) coordinates in order to locate a position.
# Let us get the geographic coordinates at the map center.

x = 0.5 * (layer.xmin + layer.xmax)
y = 0.5 * (layer.ymin + layer.ymax)
latitude, longitude, height = layer.position(x, y)

# The returned *height* coordinates corresponds to the topography height at the
# given center position. Let us print the result below.

print(f"""\
Center coordinates:
- latitude:   {latitude} deg
- longitude:  {longitude} deg
- height:     {height} m
""")

# Conversely, map (projected) coordinates are obtained as

projection = layer.project(latitude, longitude, height)
assert(abs(x - projection.x) < 1E-07)
assert(abs(y - projection.y) < 1E-07)

# Note that for this example the projection is trivial, since the map uses
# geographic (longitude, latitude) coordinates.

# The topography data can be retrieved as numpy arrays using the asarrays
# method.

x, y, z = layer.asarrays()

# Note that the returned arrays are a copy of the layer internal data. That is,
# modifying *z* does not alter the instanciated layer object. Yet, a new
# topography file could be created from the (potentially modified) arrays, as

create_map("data/GMRT.png", layer.projection, x, y, z)

# which could then be loaded back as another Layer object. Note that Mulder uses
# its own .png format in order to store the new map.

# In the following, let us illustrate some additional properties of Layers by
# drawing the topography content. First, let us interpolate data over a thiner
# grid, defined as

scaling = 10 # Upsampling factor.
x = numpy.linspace(layer.xmin, layer.xmax, scaling * (layer.nx - 1) + 1)
y = numpy.linspace(layer.ymin, layer.ymax, scaling * (layer.ny - 1) + 1)

# Then, we flatten grid data using numpy's meshgrid

X, Y = [a.flatten() for a in numpy.meshgrid(x, y)]

# The height method returns interpolated height values of the topography, as

z = layer.height(X, Y)

# In order to add specular effects to the drawing, we also need to compute the
# outgoing normal to the topography surface. The later is obtained from the
# gradient, as

gx, gy = layer.gradient(X, Y)
normal = numpy.vstack((gx, gy, numpy.ones(z.size))).T

# Following, we associate a set of colors to topography data using a LightSource
# model. By default, colors are taken from a custom mulder.TerrainColormap,
# with sea-level assumed to be at a height of zero.

light = LightSource(
    intensity = 0.7,          # Intensity of ambiant light.
    direction = (-1, -1, -1)  # Light direction, for specular effects.
)

colors = light.colorize(
    z,
    normal,
    viewpoint = (-1, -1, 1), # Optional viewpoint direction.
    cmap = None              # Optional as well, see comment above.
)

# The result needs to be recast in grid shape, for the plotting.

colors = colors.reshape((y.size, x.size, 4))

# Finally, we plot the resulting picture.

plot.style.use("examples/examples.mplstyle")
plot.figure()
plot.imshow(
    colors,
    origin="lower",
    extent=[layer.xmin, layer.xmax, layer.ymin, layer.ymax]
)
plot.xlabel("longitude (deg)")
plot.ylabel("latitude (deg)")
plot.show()
