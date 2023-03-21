#! /usr/bin/env python3
"""This example illustrates the usage of mulder.Reference objects.

Reference objects represent an opensky flux of atmopsheric muons serving as
reference for a mulder.Fluxmeter (see the fluxmeter.py example for more
information on Fluxmeters).
"""

import matplotlib.pyplot as plot
from mulder import Flux, FluxGrid, Reference
import numpy


# =============================================================================
# By default, mulder uses a parametric model as reference muon flux, according
# to Guan et al., 2015 (arxiv:1509.06176). This default model is obtained as

reference = Reference()

# Let us print the corresponding properties.

print(f"""\
Reference properties:
- energy min: {reference.energy_min}
- energy max: {reference.energy_max}
- height min: {reference.height_min}
- height max: {reference.height_max}
""")

# Note that the previous flux model is a parametrisation of measurements at sea
# level. Thus, the validity range for height is set to 0 m. The previous
# properties can be modified, though it is usually not necessary to do so, as
# discussed hereafter.


# =============================================================================
# Specific flux values are obtained with the Reference.flux method, e.g. as

flux = reference.flux(
    elevation = 90, # Observation elevation angle, in deg
    energy = 1      # Observed kinetic energy, in GeV
)

# Note that, in the previous example, a height value could have been specified
# as well, as thid (named) argument. The result is a mulder.Flux object, which
# has two fields, as follow

print(f"""\
Flux properties:
- value:     {flux.value} 1 / (GeV m^2 s sr)
- asymmetry: {flux.asymmetry}
""")

# The *value* property indicates the total flux, summing up muons and antimuons,
# while the second property corresponds to the charge *asymmetry*.


# =============================================================================
# Mulder does not ship with an extensive library of reference muon fluxes.
# However, it supports tabulations using its own *.table binary format. Examples
# of opensky tabulated fluxes, computed with MCEq, are available from GitHub, as
#
# https://github.com/niess/atmospheric-muon-flux
#
# For the present case, let us show how to create a reference flux table
# starting from a simple parameterisation. This is done with a FluxGrid object,
# a specialised mulder.Grid for flux tables. Let us first generate grid nodes as

grid = FluxGrid(
    energy = numpy.logspace(-1, 9, 201), # GeV
    cos_theta = numpy.linspace(0, 1, 101)
)

# Let us print a recap of the grid properties.

print(f"""\
Flux grid:
- shape:      {grid.shape}
- energy:     {len(grid.base.energy)} values
- cos(theta): {len(grid.base.cos_theta)} values
- height:     {numpy.size(grid.base.height)} value
""")

# Note that since no height was specified, mulder assumes a value of 0 for the
# latter.
#
# Then, let us compute the reference flux at grid nodes. That for, we use a
# simple model from the Particle Data Group (PDG 2022, ch. 30) (a.k.a. Gaisser
# model). Thus

def pdg_flux(cos_theta, energy, height=None):
    """PDG muon flux model, in 1 / (GeV m^2 sr s).

    Note: a constant charge asymmetry is assumed.
    """

    x = cos_theta * energy
    return Flux(
        value = 1.4E+03 * energy**-2.7 * \
                (1 / (1 + x / 115) + 0.054 / (1 + x / 850)),
        asymmetry = 0.1215
    )

grid.flux[:] = pdg_flux(**grid.nodes)

# Then, the corresponding table file can be generated as

grid.create_table("data/pdg.table")


# =============================================================================
# The previous command should have generated a *pdg.table* file under the data
# folder. A mulder.Reference object can created from such a file as

reference = Reference("data/pdg.table")

# As a cross-check, let us plot the resulting values, and compare to the initial
# model. First, let us define an observation condition.

energy = numpy.logspace(-1, 9, 1001) # GeV
elevation = 30 # deg
cos_theta = numpy.cos(numpy.radians(90 - elevation))

# Then, we compute the corresponding flux values

flux_ref = reference.flux(elevation, energy).value
flux_pdg = pdg_flux(cos_theta, energy).value

# Finally, we plot the results.

plot.style.use("examples/examples.mplstyle")
plot.figure()
plot.plot(energy, flux_pdg, "k-", label="PDG model")
plot.plot(energy[::20], flux_ref[::20], "ko", label="from table")
plot.xscale("log")
plot.yscale("log")
plot.xlabel("kinetic energy (GeV)")
plot.ylabel("flux (GeV$^{-1}$ m$^{-2}$ s$^{-1}$ sr$^{-1}$)")
plot.legend()
plot.show()
