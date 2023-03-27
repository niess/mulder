#! /usr/bin/env python3
"""This example illustrates usage of the Fluxmeter.intersect method.

The Fluxmeter.intersect method takes as input ...

Note:
  This example uses data produced by the `basic/layer.py` and
  `basic/reference.py` examples. Please, run these examples first.
"""

import matplotlib.pyplot as plot
from mulder import Fluxmeter, PixelGrid, Position
import numpy


# =============================================================================
# To start with, let us define a geometry and let us instanciate the
# corresponding fluxmeter. Following the `basic/fluxmeter.py` example, this is
# is done in a single step (using brief syntax) as

fluxmeter = Fluxmeter(
    Rock  = "data/GMRT.asc",
    Water = 0
)

resolution = 50
grid = PixelGrid(
    u = numpy.linspace(-2, 2, 4 * resolution + 1),
    v = numpy.linspace(0, 1, resolution + 1),
    focus = 3
)

position = Position(
    latitude = 38.82,
    longitude = 15.24
)

grammage = fluxmeter.grammage(
    position = position,
    direction = grid.direction(
        azimuth = -145,
        elevation = 0.5
    )
)
grammage = grammage.reshape((*grid.shape, 3))

plot.figure()
plot.pcolormesh(
    grid.base.u,
    grid.base.v,
    grammage[:,:,0]
)
plot.colorbar()
plot.axis("equal")
plot.show()
