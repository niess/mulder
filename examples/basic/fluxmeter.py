#! /usr/bin/env python3
"""This example illustrates the usage of mulder.Fluxmeter objects.

Fluxmeter objects can be viewed as portable probes providing an estimate of the
local flux of atmospheric muons, deformed by the observer's environment. In
order to compute this estimate, the following ingredients are needed as
input:

- A reference flux of the expected rate of atmospheric muons in the absence of
  obstacle(s) in the observer's environment. This is also known as the opensky
  muon flux.

- A geometry description of matter (obstacles) actually surrounding the
  observer.

Furthermore, it is assumed that no muons are produced in the observer's
environment. Then, computing the local flux is a pure transport problem, between
the reference and the observer (i.e. the fluxmeter location). A mulder.Fluxmeter
object can solve this problem using different methods, balancing computation
speed versus accuracy, as illustrated below.

Note that this example assumes that you are already familiar with Mulder base
concepts, the like mulder.Reference, mulder.Geometry, etc. You might first check
the corresponding examples (`basic/geometry.py`, `basic/reference.py`, etc.),
otherwise.
"""

import matplotlib.pyplot as plot
from mulder import Direction, Fluxmeter, Geometry, Reference, State
import numpy


# =============================================================================
# To start with, let us define a Stratified Earth Geometry (SEG) following the
# geometry.py example. Note that we use a brief notation where Layers are
# implied.

geometry = Geometry(
    Rock  = "data/GMRT.asc",
    Water = 0
)

# Let us point out that that the previous object is a frozen description of the
# matter distribution. Apart from a few parameters, the like layer densities,
# the geometry description cannot be modified.
#
# Then, let us create a fluxmeter for this geometry description. This is done
# simply as

fluxmeter = Fluxmeter(geometry)

# At creation, the fluxmeter object translates the provided geometry description
# to a practical transport problem. That is, to an actual geometry supporting
# ray tracing etc. See e.g. the `basic/geometry.py`, `advanced/grammage.py` and
# `advanced/intersect.py` examples for more details on the geometric
# capabilities of a fluxmeter object.


# =============================================================================
# The Fluxmeter object is created with a default reference flux, given by the
# Fluxmeter.reference attribute. For example

assert(isinstance(
    fluxmeter.reference,
    Reference
))

# For the present case, let us use this default reference model. See the
# `basic/reference.py` example for more details on how to create your own model.
# Changing the reference is done by simply updating the corresponding
# Fluxmeter.reference attribute, e.g. as

fluxmeter.reference = Reference()


# =============================================================================
# A mulder.Fluxmeter object has several modes of operation, determining the
# accuracy of muon transport. Let us print the corresponding Fluxmeter.mode
# attribute.

print(f"""\
Fluxmeter mode: {fluxmeter.mode}
""")

# As can be seen, by default, the fluxmeter is configured in "continuous" mode.
# This is the fastest, but most approximate, transport mode available. Other
# possibilities are "discrete" and "mixed". For the present example, we discuss
# only the continuous mode. See the `advanced/transport.py`, example for more
# details on the different modes of operation. As previously, changing the
# transport mode is done by updating the corresponding Fluxmeter.transport
# attribute, e.g. as

fluxmeter.mode = "continuous"


# =============================================================================
# The Fluxmeter.flux method computes a point estimate of the atmospheric muons
# flux for a given observation state. Let us first define an observation
# position at the center or the rock layer, as

rock = geometry.layers[0]
position = rock.position(
    0.5 * (rock.xmin + rock.xmax),
    0.5 * (rock.ymin + rock.ymax)
)

# Then, we relocate the observation point 10 m below the ground, as

position.height -= 10 # m

# The flux at this location is computed as

flux = fluxmeter.flux(
    position = position,
    azimuth = 90,   # deg
    elevation = 60, # deg
    energy = 10     # GeV
)

# for a observation direction of azimuth = 90 deg (i.e. toward East) and 60 deg
# of elevation above the horizontal, and for a muon kinetic energy of 10 GeV.
# The returned *flux* is a mulder.Flux object, with two attributes as

print(f"""\
Flux estimate:
- value:     {flux.value} 1 / (GeV m^2 s sr)
- asymmetry: {flux.asymmetry}
""")


# =============================================================================
# The Fluxmeter.flux method actually takes a mulder.State object as input
# argument, specifying the observation state. But, as other mulder methods, it
# also supports implicit packing (see e.g. the `basic/arrays.py` example for
# more details). Thus, the following, more lengthy, syntax is equivalent to the
# previous call

state = State(
    position = position,
    direction = Direction(
        azimuth = 90,  # deg
        elevation = 60 # deg
    ),
    energy = 10 # GeV
)

assert(fluxmeter.flux(state) == flux)


# =============================================================================
# As an illustration, let us plot the energy spectrum for the previous
# observation state. First, we define an energy vector, as

energy = numpy.logspace(-1, 3, 401)

# The we compute the flux, as previously, as

flux = fluxmeter.flux(
    position = state.position,
    direction = state.direction,
    energy = energy
)

# Finally, let us plot the result. As a comparison, we also superimpose the
# reference flux spectrum.

reference_flux = fluxmeter.reference.flux(
    elevation = state.elevation,
    energy = energy
)

plot.style.use("examples/examples.mplstyle")
plot.figure()
plot.plot(energy, flux.value, "k-", label="computation")
plot.plot(energy, reference_flux.value, "k--", label="reference")
plot.xscale("log")
plot.yscale("log")
plot.xlabel("kinetic energy (GeV)")
plot.ylabel("flux (GeV$^{-1}$ m$^{2}$ s$^{-1}$ sr$^{-1}$)")
plot.legend()
plot.show()
