#! /usr/bin/env python3
"""This example illustrates the usage of mulder.Reference objects.

Reference objects represent an opensky flux of atmopsheric muons serving as
reference for a mulder.Fluxmeter (see the fluxmeter.py example for more
information on Fluxmeters).
"""

import matplotlib.pyplot as plot
from mulder import Reference
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
# For the present case, let us show how to create a reference flux tabulation
# starting from a simple parameterisation. First, we generate a grid using
# numpy.meshgrid.

cos_theta = numpy.logspace(0, 1, 101)
energy = numpy.logspace(-1, 9, 201)
C, E = [a.flatten() for a in numpy.meshgrid(cos_theta, energy)]

# Then, let us compute the reference flux at grid nodes
