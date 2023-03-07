#! /usr/bin/env python3
import matplotlib.pyplot as plot
import numpy

import mulder


# Define the geometry
layers = (
    mulder.Layer("Rock", "data/mns_roche.png"),
    mulder.Layer("Water", "data/mns_eau.png")
)

# Get coordinates at center of top map
x = 0.5 * (layers[0].xmin + layers[0].xmax)
y = 0.5 * (layers[0].ymin + layers[0].ymax)

# Get corresponding topography height (and offset it below ground)
z = layers[0].height(x, y)
latitude, longitude = layers[0].geodetic(x, y)
z -= 30.

# Create a fluxmeter and compute the differential muon flux along some
# direction of observation
meter = mulder.Fluxmeter(*layers)

azimuth, elevation = 0, 25
energy = numpy.logspace(0, 4, 401)
flux = meter.flux(latitude, longitude, z, azimuth, elevation, energy)
reference = meter.reference.flux(elevation, energy)

# Plot normed flux, for comparison with Guan et al. (arxiv.org:1509.06176)
norm = energy**2.7 * 1E-04

plot.style.use("examples/paper.mplstyle")
plot.figure()
plot.plot(energy, flux.value * norm, "k-")
plot.plot(energy, reference * norm, "k--")
plot.xscale("log")
plot.yscale("log")
plot.xlabel("energy, $E$ (GeV)")
plot.ylabel("$E^{2.7} \phi$ (GeV$^{1.7}$ cm$^{-2}$ s$^{-1}$ sr$^{-1}$)")
plot.show()
