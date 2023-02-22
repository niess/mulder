#! /usr/bin/env python3
import matplotlib.pyplot as plot
import numpy

import mulder


# Define the geometry
layers = (
    mulder.Layer("StandardRock", "data/mns_roche.png"),
    mulder.Layer("Water", "data/mns_eau.png")
)

# Create a fluxmeter
meter = mulder.Fluxmeter(*layers)

# Get layer indices
x = numpy.linspace(layers[0].xmin, layers[0].xmax, layers[0].nx // 10 + 1)
y = numpy.linspace(layers[0].ymin, layers[0].ymax, layers[0].ny // 10 + 1)
z = 0.5 * (layers[0].zmin + layers[0].zmax) - 75

X, Y = [a.flatten() for a in numpy.meshgrid(x, y)]
latitude, longitude = layers[0].geodetic(X, Y)
i = meter.whereami(latitude, longitude, z)
i = i.reshape((y.size, x.size))

plot.figure()
plot.pcolormesh(x, y, i, cmap="gray", vmin=0, vmax=len(layers))
plot.axis("equal")
plot.axis("off")

x = numpy.linspace(layers[0].xmin, layers[0].xmax, layers[0].nx * 10 + 1)
y = 0.5 * (layers[0].ymin + layers[0].ymax) + 500
z0 = layers[0].height(x, y)
z1 = layers[1].height(x, y)

plot.figure()
plot.plot(x - x[0], z0, "k-")
plot.fill_between(x - x[0], z0, z1, color="b", alpha=0.5)
plot.xlabel("easting (m)")
plot.ylabel("height (m)")

plot.show()
