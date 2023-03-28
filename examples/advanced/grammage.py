#! /usr/bin/env python3
"""This example illustrates usage of the Fluxmeter.grammage method.

The Fluxmeter.grammage method is a high level geometry operation. It computes
the column depth along a line of sight, a.k.a. the grammage. The column depth,
X, is relevant for many muons related application since, in the continuous
approximation, there is a monotone mapping between X and the transmitted muon
flux. That is, the larger the grammage, the lower the transmitted flux.
Conversely, muon flux measurements provide an estimate of column depths along
lines of sight, up to composition effects, which is the basis of transmission
muography.

See also the `advanced/intersect.py` example for lower level ray tracing.

Note:
  This example assumes that your are already familiar with mulder geometries,
  i.e. examples `basic/layer.py` and `basic/geometry.py`, as well as with some
  other concepts, the like fluxmeters and grids (see `basic/fluxmeter.py` and
  `basic/grids.py`).

  In addition, this example uses CMasher colormaps, available e.g. from PyPI
  using `pip install cmasher` (see https://cmasher.readthedocs.io/ for more
  details).
"""

import cmasher
import matplotlib.colors as colors
import matplotlib.pyplot as plot
from mulder import Direction, Fluxmeter, Grid, Position
import numpy


# =============================================================================
# To start with, let us define a geometry of interest, and let us instanciate
# the corresponding fluxmeter. Following the `basic/fluxmeter.py` example, this
# is is done in a single step (using brief syntax) as

fluxmeter = Fluxmeter(
    Rock  = "data/GMRT.asc",
    Water = 0
)

# Then, let us set the same observation point than in the `basic/fluxmeter.py`
# example. That is, from a `ship' located North-East of Strombili island.

position = Position(
    latitude = 38.82,
    longitude = 15.24,
    height = 0.5
)

# We define a grid of observation directions using horizontal angular
# coordinates. Note that we introduce a tunable resolution parameter.

resolution = 1
grid = Grid(
    azimuth = -145 + numpy.arange(-45, 45 + 1, resolution),
    elevation = 0.5 + numpy.arange(0, 20 + 1, 0.25 * resolution)
)
direction = Direction(**grid.nodes)


# =============================================================================
# The column depth along the previous lines of sight is is simply obtained as

grammage = fluxmeter.grammage(
    position = position,
    direction = direction
)

# This function returns a `n x m` numpy.ndarray, where `n` is the number of lines
# of sight and `m` the numbers of geometry layers, including the external
# atmosphere layer. Thus

assert(grammage.shape[0] == direction.size)
assert(grammage.shape[1] == len(fluxmeter.geometry.layers) + 1)

# That is, the columns of the grammage array contain the column depths
# accumulated in each layer, ordered by layer index. Note that there is no
# chronological information relative to the actual rays histories.


# =============================================================================
# Let us draw the result for each layer. That for, its is relevant to
# distinguish rays that cross the Rock target from others, since this results in
# very different grammage values, by several orders of magnitude. Thus, let us
# compute the first intersected layer as a proxy for crossing rays

intersection = fluxmeter.intersect(
    position = position,
    direction = direction
)

# Then, we define a common plotting function for both types of rays.

def plot_grammage(layer_index, mask, cmap, vmax):
    """Plot grammage for a given ray type, identified by layer_index and mask.
    """
    plot.pcolormesh(
        grid.base.azimuth,
        grid.base.elevation,
        numpy.ma.masked_array(
            grid.reshape(grammage[:, layer_index]),
            mask = mask
        ),
        cmap = cmap,
        vmin = 0,
        vmax = vmax
    )
    cbar = plot.colorbar(
        location = "top",
        ticks = vmax * numpy.linspace(0, 1, 6),
        aspect = 40,
        anchor = (0, 0)
    )
    cbar.formatter.set_powerlimits((0, 0)) # Force scientific notation

# Finally, let us draw the result.

plot.style.use("examples.mplstyle")
plot.figure()
plot_grammage(
    layer_index = 0, # Rock
    mask = intersection.layer != 0,
    cmap = "cmr.amber_r",
    vmax = 1E+07
)
plot_grammage(
    layer_index = 2, # Air
    mask = intersection.layer == 0,
    cmap = "cmr.freeze_r",
    vmax = 1E+05
)
plot.xlabel("azimuth (deg)")
plot.ylabel("elevation (deg)")
plot.title(
    "column depth (kg m$^{-2}$)",
    y = 1.5
)
plot.show()
