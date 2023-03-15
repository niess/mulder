#! /usr/bin/env python3
import matplotlib.pyplot as plot
import numpy

from mulder import Fluxmeter, Geometry


# Define a stratified Earth geometry
geometry = Geometry(
    ("Rock", "data/mns_roche.png"),
    ("Water", "data/mns_eau.png")
)

# Create a fluxmeter
fluxmeter = Fluxmeter(geometry)

# Get layer indices
layer = geometry.layers[0]
x = numpy.linspace(layer.xmin, layer.xmax, layer.nx // 10 + 1)
y = numpy.linspace(layer.ymin, layer.ymax, layer.ny // 10 + 1)
z = 0.5 * (layer.zmin + layer.zmax) - 75

X, Y = [a.flatten() for a in numpy.meshgrid(x, y)]
position = layer.position(X, Y)
position.height = z
i = fluxmeter.whereami(position)
i = i.reshape((y.size, x.size))

plot.figure()
plot.pcolormesh(x, y, i, cmap="gray", vmin=0, vmax=len(geometry.layers))
plot.axis("equal")
plot.axis("off")

x = numpy.linspace(layer.xmin, layer.xmax, layer.nx * 10 + 1)
y = 0.5 * (layer.ymin + layer.ymax) + 500
z0 = geometry.layers[0].height(x, y)
z1 = geometry.layers[1].height(x, y)

plot.figure()
plot.plot(x - x[0], z0, "k-")
plot.fill_between(x - x[0], z0, z1, color="b", alpha=0.5)
plot.xlabel("easting (m)")
plot.ylabel("height (m)")

plot.show()
