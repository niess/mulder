#! /usr/bin/env python3
import matplotlib.pyplot as plot
import numpy

import mulder


# Define the geometry
layers = (
    mulder.Layer("Rock", "data/mns_roche.png"),
    mulder.Layer("Water", "data/mns_eau.png")
)

# Get the geomagnetic field (optional)
magnet = mulder.Geomagnet()

# Get map coordinates at center of top layer
x = 0.5 * (layers[0].xmin + layers[0].xmax)
y = 0.5 * (layers[0].ymin + layers[0].ymax)

# Get the corresponding geographic coordinates (and offset height below ground)
coordinates = layers[0].coordinates(x, y)
coordinates.height -= 30

# Create a fluxmeter and compute the differential muon flux along some
# direction of observation
meter = mulder.Fluxmeter(*layers)
meter.geomagnet = magnet

azimuth, elevation = 0, 25
energy = numpy.logspace(0, 4, 401)
flux = meter.flux(
    coordinates.latitude, coordinates.longitude, coordinates.height,
    azimuth, elevation, energy)
reference = meter.reference.flux(elevation, energy)

# Plot normed flux, for comparison with Guan et al. (arxiv.org:1509.06176)
norm = energy**2.7 * 1E-04

plot.style.use("examples/paper.mplstyle")
plot.figure()
plot.plot(energy, flux.value * norm, "k-")
plot.plot(energy, reference.value * norm, "k--")
plot.xscale("log")
plot.yscale("log")
plot.xlabel("energy, $E$ (GeV)")
plot.ylabel("$E^{2.7} \phi$ (GeV$^{1.7}$ cm$^{-2}$ s$^{-1}$ sr$^{-1}$)")
plot.show()
