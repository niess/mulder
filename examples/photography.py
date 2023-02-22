#! /usr/bin/env python3
from copy import copy
import matplotlib.pyplot as plot
import numpy

import mulder


# Define the geometry
layers = (
    mulder.Layer("StandardRock", "data/mns_roche.png"),
    mulder.Layer("Water", "data/mns_eau.png")
)

# Set the observation point
x0 = 0.5 * (layers[0].xmin + layers[0].xmax)
y0 = 0.5 * (layers[0].ymin + layers[0].ymax)
latitude, longitude = layers[0].geodetic(x0, y0)
height = layers[0].zmax + 1250
azimuth, elevation = 90, -90

# Create a fluxmeter
meter = mulder.Fluxmeter(*layers)

# Transform from pixel coordinates to angular ones
nu, nv, f = 201, 201, 1
su, sv = -1, 1

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
theta, phi = (90 - elevation) * deg, (90 - azimuth) * deg
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

# Intersect rays with the geometry
i, x, y, z = meter.intersect(latitude, longitude, height, azimuth, elevation)

# Compute target scattering factor
source = (-1, 1, 1)
nx, ny = numpy.zeros(i.size), numpy.zeros(i.size)
sel = i == 0
for j, layer in enumerate(layers):
    sel = (i == j)
    nx[sel], ny[sel] = layer.gradient(x[sel], y[sel])

nz = 1 / numpy.sqrt(1 + nx**2 + ny**2)
nx *= nz
ny *= nz
ux = x - x0
uy = y - y0
uz = z - height
nrm = 1 / numpy.sqrt(ux**2 + uy**2 + uz**2)
ux *= nrm
uy *= nrm
uz *= nrm
nu = ux * nx + uy * ny + uz * nz
rx = ux - 2 * nu * nx
ry = uy - 2 * nu * ny
rz = uz - 2 * nu * nz
c = (rx * source[0] + ry * source[1] + rz * source[2]) / \
    numpy.linalg.norm(source)
c = (1 + c) / 2

# Plot the result
palette = copy(plot.cm.terrain)
palette.set_bad("w", 1)
palette.set_over("w", 1)
palette.set_bad("w", 1)
clr = palette((z - layers[0].zmin) / (layers[0].zmax - layers[0].zmin))
tmp = numpy.outer(c, (1, 1, 1))
clr[:,:3] *= 0.3 + 0.7 * tmp
clr = clr.reshape((v.size, u.size, 4))

plot.figure()
plot.imshow(clr, origin="lower")
plot.axis("off")
plot.show()
