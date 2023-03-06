#! /usr/bin/env python3
import matplotlib.pyplot as plot
import numpy

import mulder


# Define the geometry
layer = mulder.Layer("Rock", offset=3E+03)

# Load the reference flux
reference = mulder.Reference(
    "deps/atmospheric-muon-flux/data/simulated/flux-mceq-yfm-gsf-usstd.table")

# Create a fluxmeter and compute the muon spectrum along some direction of
# observation
meter = mulder.Fluxmeter(layer)
meter.reference = reference

latitude, longitude, height = 45, 3, layer.offset + 0.5
azimuth, elevation = 0, 60
energy = numpy.logspace(-1, 4, 51)

hmax = reference.height_max
reference.height_max = 0 # Disable heights > 0. This is in order to force
                         # the fluxmeter to use reference data at 0 height.
flux = meter.flux(latitude, longitude, height, azimuth, elevation, energy)

reference.height_max = hmax # Restore ref. max height.


# Plot normed flux, for comparison with Guan et al. (arxiv.org:1509.06176)
norm = energy**2.7 * 1E-04

plot.style.use("examples/paper.mplstyle")
plot.figure()
plot.plot(energy, flux * norm, "ko", label="CSDA evolution")
plot.plot(energy, reference.flux(elevation, energy, height=0) * norm, "k--",
    label="MCEq (0m)")
plot.plot(energy, reference.flux(elevation, energy, height=height) * norm, "k-",
    label="MCEq (3000m)")
plot.xscale("log")
plot.yscale("log")
plot.xlabel("energy, $E$ (GeV)")
plot.ylabel("$E^{2.7} \phi$ (GeV$^{1.7}$ cm$^{-2}$ s$^{-1}$ sr$^{-1}$)")
plot.legend()
plot.show()
