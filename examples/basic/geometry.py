#! /usr/bin/env python3
"""This example illustrates the usage of mulder.Geometry objects.

Geometry objects are containers for representing a Stratified Earth Geometry
(SEG). They have little to no capabilities on their own. They can be
investigated with a Fluxmeter which is briefly discussed below (see the
fluxmeter.py example for more detailed usage).

Note that this example assumes that you are already familiar with mulder.Layer
objects. Please, check the layer.py example first, otherwise.
"""

import matplotlib.pyplot as plot
from mulder import Fluxmeter, Geomagnet, Geometry, Layer


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

# Note that the stack of *layers* is immutable (it is a tuple).


# =============================================================================
# Geometry objects always have a special Atmosphere layer at the very top, that
# was not indicated in the previous list of layers. This layer is composed of
# Air with a variable density.
