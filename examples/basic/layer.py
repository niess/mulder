#! /usr/bin/env python3
"""This example illustrates the usage of Layer objects, representing a
Stratified Earth Geometry (SEG).

Please, see the geometry.py example for information on SEGs.
"""

import matplotlib.pyplot as plot

from mulder import Layer


# Create a layer of rocks, with a specific density of 2 g/m^3 (note that Mulder
# uses SI units).
#
# The *model* argument refers to topography data describing the layer's top
# interface. If no model is specified, a flat topography is assumed.
layer = Layer(
    material = "Rock",
    density = 2.0E+03, # kg / m^3
    model = "data/GMRT.asc"
)

# Let us print some metadata related to the topography model.
print(f"""\
Map metadata:
- projection: {layer.projection}
- nx:         {layer.nx}
- ny:         {layer.ny}
- xmin:       {layer.xmin}
- xmax:       {layer.xmax}
- ymin:       {layer.ymin}
- ymax:       {layer.ymax}\
""")

# Mulder uses geographic (GPS-like) coordinates in order to locate some
# position. Let us get the goegraphic coordinates corresponding to the center
# of the map.
#
# Note that the returned *height* coordinates corresponds to the topography
# height at center's position.
x = 0.5 * (layer.xmin + layer.xmax)
y = 0.5 * (layer.ymin + layer.ymax)
center = layer.position(x, y)

print(f"""\
Center coordinates:
- latitude:   {center.latitude} deg
- longitude:  {center.longitude} deg
- height:     {center.height} m\
""")

# The topography data
x, y, z = layer.asarrays()

plot.figure()
plot.pcolormesh(x, y, z, cmap="terrain")
plot.colorbar()
plot.show()
