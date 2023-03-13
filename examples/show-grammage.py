#! /usr/bin/env python3
from copy import copy
import matplotlib.pyplot as plot
from matplotlib.colors import LogNorm
import numpy

from mulder import Direction, Fluxmeter, Layer, Projection


# Define the geometry
layers = (
    Layer("Rock", "data/mns_roche.png"),
    Layer("Water", "data/mns_eau.png")
)

# Set the observation point
projection = Projection(
    x = 0.5 * (layers[0].xmin + layers[0].xmax) - 150,
    y = 0.5 * (layers[0].ymin + layers[0].ymax) + 650
)
position = layers[0].position(projection)
position.height = layers[1].zmin - 500
direction = Direction(azimuth = 90, elevation = 90)

# Create a fluxmeter
fluxmeter = Fluxmeter(*layers)

# Transform from pixel coordinates to angular ones
nu, nv, f = 201, 201, 1
su, sv = 1, 1

u, v = numpy.linspace(-1, 1, nu), numpy.linspace(-1, 1, nv)
U, V = [a.flatten() for a in numpy.meshgrid(u, v)]
U *= su
V *= sv
theta = numpy.arctan2(numpy.sqrt(U**2 + V**2), f)
phi = numpy.arctan2(V, U)
ct, st = numpy.cos(theta), numpy.sin(theta)
cp, sp = numpy.cos(phi), numpy.sin(phi)
r = numpy.array((cp * st, sp * st, ct))

deg = numpy.pi / 180
theta, phi = (90 - direction.elevation) * deg, (90 - direction.azimuth) * deg
ct, st = numpy.cos(theta), numpy.sin(theta)
cp, sp = numpy.cos(phi), numpy.sin(phi)
R = numpy.array((
    (ct * cp, -sp, st * cp),
    (ct * sp,  cp, st * sp),
    (    -st,   0,      ct)))

rx, ry, rz = numpy.dot(R, r)

deg = 180 / numpy.pi
azimuth = 90 - numpy.arctan2(ry, rx) * deg
elevation = numpy.arctan2(rz, numpy.sqrt(rx**2 + ry**2)) * deg

# Get grammage along line of sights
direction = Direction(azimuth, elevation)
grammages = fluxmeter.grammage(position, direction)
grammages = [a.reshape((nv, nu)) for a in grammages.T]

# Plot the result
for grammage in grammages:
    plot.figure()
    plot.pcolormesh(u, v, grammage, cmap="terrain", norm=LogNorm())
    plot.colorbar()
    plot.axis("off")

plot.show()
