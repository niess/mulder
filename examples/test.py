#! /usr/bin/env python3
import matplotlib.pyplot as plot
import numpy

from mulder import Fluxmeter, Geomagnet, Layer, State


# Define the geometry
layers = (
    Layer("Rock", "data/mns_roche.png"),
    Layer("Water", "data/mns_eau.png")
)

# Get the geomagnetic field (optional)
magnet = Geomagnet()

# Get projected (map) coordinates at center of top layer
x0 = 0.5 * (layers[0].xmin + layers[0].xmax)
y0 = 0.5 * (layers[0].ymin + layers[0].ymax)

# Get the corresponding geographic position (and offset height below ground)
position = layers[0].position(x0, y0)
position.height -= 30

# Create a fluxmeter and compute the differential muon flux for some
# observation state
fluxmeter = Fluxmeter(*layers)
fluxmeter.geomagnet = magnet

state = State(
    position = position,
    azimuth = 0,
    elevation = 25,
    energy = numpy.logspace(0, 4, 401)
)

flux = fluxmeter.flux(state)

# Compute the reference flux for similar observation conditions (for comparison)
reference = fluxmeter.reference.flux(
    state.elevation,
    state.energy
)

# Plot normed flux, for comparison with Guan et al. (arxiv.org:1509.06176)
norm = state.energy**2.7 * 1E-04

plot.style.use("examples/paper.mplstyle")
plot.figure()
plot.plot(state.energy, flux.value * norm, "k-", label="computation")
plot.plot(state.energy, reference.value * norm, "k--", label="reference")
plot.xscale("log")
plot.yscale("log")
plot.xlabel("energy, $E$ (GeV)")
plot.ylabel("$E^{2.7} \phi$ (GeV$^{1.7}$ cm$^{-2}$ s$^{-1}$ sr$^{-1}$)")
plot.legend()
plot.show()
