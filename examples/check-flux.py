#! /usr/bin/env python3
import matplotlib.pyplot as plot
import numpy

from mulder import Fluxmeter, Geometry, Layer, Reference, State


# Define a stratified Earth geometry
layer = Layer("Rock", offset=3E+03)
geometry = Geometry(layer)

# Load the reference flux
reference = Reference(
    "deps/atmospheric-muon-flux/data/simulated/flux-mceq-yfm-gsf-usstd.table")

# Create a fluxmeter and compute the muon spectrum for some observation state
fluxmeter = Fluxmeter(geometry)
fluxmeter.reference = reference

state = State(
    height = layer.offset + 0.5,
    azimuth = 0,
    elevation = 60,
    energy = numpy.logspace(-1, 4, 51)
)

hmax = reference.height_max
reference.height_max = 0 # Disable heights > 0. This is in order to force
                         # the fluxmeter to use reference data at 0 height.
flux = fluxmeter.flux(state)

reference.height_max = hmax # Restore ref. max height.
reference0 = reference.flux(state.elevation, state.energy, height=0)
reference1 = reference.flux(state.elevation, state.energy, height=state.height)

# Get (default) reference flux, for comparison
default = Reference()
default = default.flux(state.elevation, state.energy, height=0)

# Plot normed flux, for comparison with Guan et al. (arxiv.org:1509.06176)
norm = state.energy**2.7 * 1E-04

plot.style.use("examples/paper.mplstyle")

plot.figure()
plot.plot(state.energy, flux.value * norm, "ko", label="CSDA evolution")
plot.plot(state.energy, reference0.value * norm, "k--", label="MCEq (0m)")
plot.plot(state.energy, reference1.value * norm, "k-", label="MCEq (3000m)")
plot.plot(state.energy, default.value * norm, "k:", label="GCCLY (0m)")
plot.xscale("log")
plot.yscale("log")
plot.xlabel("energy, $E$ (GeV)")
plot.ylabel("$E^{2.7} \phi$ (GeV$^{1.7}$ cm$^{-2}$ s$^{-1}$ sr$^{-1}$)")
plot.legend()

# Plot charge asymmetry
plot.figure()
sel = flux.value > 0
plot.plot(state.energy[sel], flux.asymmetry[sel], "ko", label="CSDA evolution")
plot.plot(state.energy, reference0.asymmetry, "k--", label="MCEq (0m)")
plot.plot(state.energy, reference1.asymmetry, "k-", label="MCEq (3000m)")
plot.xscale("log")
plot.xlabel("energy, $E$ (GeV)")
plot.ylabel("charge asymmetry")
plot.legend()

plot.show()
