#! /usr/bin/env python3
import matplotlib.pyplot as plot
import numpy

from mulder import Fluxmeter, Geometry, State


# Define a stratified Earth geometry
geometry = Geometry(
    ("Rock", "data/mns_roche.png"),
    ("Water", "data/mns_eau.png"),
    geomagnet = True
)

# Get projected (map) coordinates at center of top layer
layer = geometry.layers[0]
x0 = 0.5 * (layer.xmin + layer.xmax)
y0 = 0.5 * (layer.ymin + layer.ymax)

# Get the corresponding geographic position (and offset height below ground)
position = layer.position(x0, y0)
position.height -= 30

# Create a fluxmeter and compute the differential muon flux for some
# observation state
fluxmeter = Fluxmeter(geometry)

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

plot.style.use("examples.mplstyle")
plot.figure()
plot.plot(state.energy, flux.value * norm, "k-", label="computation")
plot.plot(state.energy, reference.value * norm, "k--", label="reference")
plot.xscale("log")
plot.yscale("log")
plot.xlabel("energy, $E$ (GeV)")
plot.ylabel("$E^{2.7} \phi$ (GeV$^{1.7}$ cm$^{-2}$ s$^{-1}$ sr$^{-1}$)")
plot.legend()
plot.show()
