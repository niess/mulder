#! /usr/bin/env python3
"""This example illustrates usage of the Fluxmeter.intersect method.

The Fluxmeter.intersect method computes the first intersection of the geometry
with ray segments. Thus, it provides access to mulder's ray tracing algorithms.
In this example, we show how this method can be combined with layer data in
order to realize a photographic like picture of the geometry.

Note:
  This example assumes that your are already familiar with mulder geometries,
  i.e. examples `basic/layer.py` and `basic/geometry.py`, as well as with some
  other concepts, the like fluxmeters and grids (see `basic/fluxmeter.py` and
  `basic/grids.py`).
"""

import matplotlib.colors as colors
import matplotlib.pyplot as plot
from mulder import Fluxmeter, PixelGrid, Position
from mulder.matplotlib import LightSource, TerrainColormap, set_cursor_data
import numpy


# =============================================================================
# To start with, let us set the scene by defining a geometry and its
# corresponding fluxmeter. Following the `basic/fluxmeter.py` example, this is
# is done as

fluxmeter = Fluxmeter(
    Rock  = "data/GMRT.asc",
    Water = 0
)

# Then, let us define a PixelGrid as observation plane. Note that we introduce
# a configurable resolution parameter, that might be increased in order to
# get higher quality picture (at the expanse of longer computing time).

resolution = 50
grid = PixelGrid(
    u = numpy.linspace(-2, 2, 4 * resolution + 1),
    v = numpy.linspace(-0.5, 1, int(1.5 * resolution) + 1),
    focus = 3
)

# Let us position the camera at the same observation point than in the
# `basic/fluxmeter.py` example. That is, a `boat' located North-East of
# Strombili island.

position = Position(
    latitude = 38.82,  # deg
    longitude = 15.24, # deg
    height = 0.5       # m
)


# =============================================================================
# The first intersections of photographic projection lines with the geometry are
# obtained as

intersection = fluxmeter.intersect(
    position = position,
    direction = grid.direction(
        azimuth = -145,
        elevation = 0
    )
)

# The returned object is a mulder.Intersection. It has two properties, the index
# of the intersected layer and the position of the intersection point. Let us
# print those

print(f"""\
# Intersection data:
- layer index: {intersection.layer}
- position:    {intersection.position}
""")

# Note that a negative layer index indicates that no intersection was found up
# to the geometry boundary.


# =============================================================================
# Let us draw a picture of the intersected layers. That for, we define a custom
# colormap with 4 entries corresponding to, None (-1), Rock (0), Water (1) and
# Air (2).

cmap = colors.ListedColormap((
    colors.CSS4_COLORS["white"],
    colors.CSS4_COLORS["saddlebrown"],
    colors.CSS4_COLORS["royalblue"],
    colors.CSS4_COLORS["lightblue"]
))

# Below, we plot the result taking care of setting the color range accordingly.

plot.style.use("examples.mplstyle")
plot.figure()
plot.pcolormesh(
    grid.base.u,
    grid.base.v,
    grid.reshape(intersection.layer),
    cmap = cmap,
    vmin = -1,
    vmax = len(fluxmeter.geometry.layers)
)
cbar = plot.colorbar()
cbar.set_ticks(range(-1, len(fluxmeter.geometry.layers) + 1))
cbar.ax.set_yticklabels((
    "None",
    "Rock",
    "Water",
    "Air"
))
plot.xlabel("pixel $u$")
plot.ylabel("pixel $v$")
plot.title("Intersected layer index")
plot.show(block=False)

# On the resulting figure, one should clearly see the shape of mount Stromboli.
# One might also notice that no intersection with air is found. The reason is
# that the observation point is already located inside the atmopshere layer.
# But, the Fluxmeter.intersect method only returns the first intersection with
# the next layer, which thus cannot be Air.


# =============================================================================
# Let us now compute the distance from the camera (i.e. the observation point)
# to the intersection points. That for, we use a local projection assuming a
# spherical Earth, as an approximation. The projection parameters are given as

rE = 6400E+03 # m
rx = rE * numpy.cos(numpy.radians(position.latitude)) * numpy.radians(1)
ry = rE * numpy.radians(1)

# where the numpy.radians factor arises from the fact that latitude and
# longitude are expressed in degrees. Thus, the viewpoint vector, oriented from
# the intersection to the observer, has the following coordinates

x = rx * (position.longitude - intersection.position.longitude)
y = ry * (position.latitude - intersection.position.latitude)
z = position.height - intersection.position.height

viewpoint = numpy.vstack((x, y, z)).T

# From which we compute the distance to the intersection as

distance = numpy.linalg.norm(viewpoint, axis=1)

# Let us plot the result using a logarithmic scale.

plot.figure()
plot.pcolormesh(
    grid.base.u,
    grid.base.v,
    numpy.ma.masked_array(
        grid.reshape(distance),
        mask = intersection.layer == -1
    ),
    cmap = "hot_r",
    norm = colors.LogNorm()
)
plot.colorbar()
plot.xlabel("pixel $u$")
plot.ylabel("pixel $v$")
plot.title("distance to intersection (m)")
plot.show(block=False)


# =============================================================================
# In order to get a realistic picture of the scene, we need to render specular
# effects. That is scattering on the topography with favoured directions
# corresponding to reflection angles. Thus, let us compute the normal to the
# topography surface at intersection points. This can be done by looping over
# layers, as

gx, gy  = [grid.zeros() for _ in range(2)]
for layer_index in numpy.unique(intersection.layer):
    if layer_index == -1: # no intersection.
        continue
    else:
        layer = fluxmeter.geometry.layers[layer_index]
        sel = intersection.layer == layer_index
        projection = layer.project(intersection.position[sel])
        gx[sel], gy[sel] = layer.gradient(projection)

# The previous loop computes the local gradient, w.r.t. map coordinates, that is
# latitude and longitude. The corresponding normal direction, in the local
# tangent frame writes

normal = numpy.vstack((
    gx / rx,
    gy / ry,
    grid.ones()
)).T

# As an illustration, let us plot the vertical component of the normal
# unit-vector.

nz = normal[:, 2] / numpy.linalg.norm(normal, axis=1)

plot.figure()
plot.pcolormesh(
    grid.base.u,
    grid.base.v,
    numpy.ma.masked_array(
        grid.reshape(nz),
        mask = intersection.layer == -1
    ),
    cmap = "gray",
    vmin = 0,
    vmax = 1
)
plot.colorbar()
plot.xlabel("pixel $u$")
plot.ylabel("pixel $v$")
plot.title("normal, $n_z$, at intersection")
plot.show(block=False)

# It can be seen from the latter figure that the gradient information is
# perceptually relevant. That is, it is interpreted as relief features.

# =============================================================================
# To conclude this example, let us combine previous results in order to build a
# photographic like picture of the scene. That for, we define a LightSource
# object, in order to color height data with gradient information, using a
# specular model of light propagation (see also the `basic/layer.py` example).
# Thus

light = LightSource(
    intensity = 0.5,      # Intensity of ambiant light.
    direction = (0, 0, 1) # Light source direction, for specular effects.
)

# The base information is topography height values at intersection points. It is
# represented using a TerrainColormap, with sea level at zero. In order to
# clearly separate the water level from land, we slightly offset the height of
# the former (which is exactly zero in this case).

data = intersection.height.copy()
sel = intersection.layer == 1
data[sel] = -0.01

# Then, we colorize data according to the light model.

pixel_colors = light.colorize(
    data,
    normal,
    viewpoint = viewpoint,
    cmap = TerrainColormap(),
    vmin = -1000
)
pixel_colors = pixel_colors.reshape((*grid.shape, 4))

# Sky regions are overwritten using a uniform lightblue color.

sel = grid.reshape(intersection.layer) == -1
pixel_colors[sel, :] = colors.to_rgba("lightblue")

# Lt us finally draw the resulting picture.

plot.figure(figsize=(12, 5))
image = plot.imshow(
    pixel_colors,
    origin = "lower",
    extent = [
        min(grid.base.u),
        max(grid.base.u),
        min(grid.base.v),
        max(grid.base.v)
    ]
)
set_cursor_data(image, grid.reshape(data))
plot.axis("off")
plot.show()
