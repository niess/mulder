#! /usr/bin/env python3
"""This example discusses usage of `Mulder.Prng` objects.

Mulder.Prng objects provide access to the Pseudo-Random Numbers Generator (PRNG)
of a Fluxmeter. These objects have a rather simple interface, presented below.
Prng objects are convenient for performing Monte Carlo integrations over the
observation space. This application is illustrated below with the example of an
idealised vertical detection plane counting muons passing through.

Note:
  This example assumes that your are already familiar with mulder Fluxmeters,
  and flux computations. You might first check other examples otherwise, the
  like `basic/fluxmeter.py`, `advanced/flux.py` or `advanced/transport.py`.
"""

import matplotlib.pyplot as plot
from mulder import Fluxmeter, State
import numpy


# =============================================================================
# For this example, we do not need a sophisticated geometry. Thus, the fluxmeter
# is created with no arguments (i.e. a default pensky geometry), as

fluxmeter = Fluxmeter()

# The corresponding PRNG is available as the Fluxmeter.prng property.

prng = fluxmeter.prng

# This object has a single property that is the seed of the pseudo-random
# stream. Let us print the latter.

print(f"""\
# PRNG properties:
- seed: {prng.seed}
""")

# Note that the seed changes each time that your run this example. By default,
# it is assigned from the OS entropy, i.e. using /dev/urandom. Let us fix the
# seed in the following, in order to get reproducible results.

prng.seed = 1234

# Note also that one can reset the seed to a random value by assigning `None` to
# the PRNG.seed property.


# =============================================================================
# The Mulder.Prng object is callable. This returns the next pseudo-random
# numbers in the stream, as a numpy.ndarray. E.g.

next3 = prng(3)

# Should return the next 3 pseudo-random numbers as

print(f"""\
# PRNG properties:
- seed:       {prng.seed}
- stream[:3]: {next3}
""")

# Note that these numbers are always the same, whenever you run this example
# again. The sequence of values is determined by the seed. However, the
# mulder.Prng stream can only progress forward. That is, calling the Prng again
# results in different numbers (the next ones in the pseudo-random sequence).
# For example

assert(not numpy.array_equal(prng(3), next3))

# Assigning to the seed again resets the stream. Thus

prng.seed = 1234
assert(numpy.array_equal(prng(3), next3))

# In this particular cacse, one could use the Prng.reset method, which is more
# explicit.

prng.reset()
assert(numpy.array_equal(prng(3), next3))


# =============================================================================
# Let us now discuss the actual usage of mulder.Prng objects. So far, in other
# examples, we only considered point estimates. That is, the observation is done
# at a fixed point, or over a grid of points. However, in real life cases, one
# usually has to deal with distributed observations, e.g. an observation
# performed over an energy range, or over angular bins. Then, one could sample
# observations over a thin (sub)mesh, and perform a numeric integration e.g.
# using trapezes or a higher order method. However, this is not always possible,
# nor efficient depending on the use case. An alternative is to perform a Monte
# Carlo integration over the observation space. This is particularly relevant in
# discrete and mixed modes, since flux computations already require a Monte
# Carlo procedure.
#
# As an example, let us first consider observations where the muon kinetic
# energy is distributed. That is, let us assume that we have a toy detector
# selecting muons over a given energy range, as

energy_range = (1E-03, 1E+03) # GeV

# Let us point out that the minimum and maximum values for the kinetic energy
# differ by 6 orders of magnitude. In this case, it is relevant to sample values
# log-uniformly (i.e. with a 1 / x PDF), instead of uniformly. Let us create the
# corresponding pseudo-random generator, as

generator = prng.log(*energy_range)

# The previous generator object is a specialised PRNG built from the
# Uniform(0,1) prng instance. That is, it actually uses values from the
# Fluxmeter.prng stream, to which an inverse CDF transform is applied.
#
# Let us generate some values as an illustration. This is done as with a Prng
# instance, as

values3 = generator(3)

print(f"""\
# Log-Uniform generator:
- prng:       {generator.prng}
- values[:3]: {values3}
""")

# For the following discussion, it is also useful to determine the density
# probabilities corresponding to the generated values. Those can be obtained as

pdf = generator.pdf(values3)

print(f"""\
- pdf[:3]:    {pdf}
""")

# =============================================================================
# Let us now create an observation state for the flux, and let us sample
# `observed' kinetic energy values. First, we create a new vector of States
# initialised with default values as

events = 100000
state = State.new(events)

# where the number or Monte Carlo events has been introduced as a configurable
# parameter. Then, kinetic energy values are generated with the State.generate
# method, as

state.generate("energy", generator)

# Let us print the result.

print(f"""\
# State properties:
- energy: {state.energy}
- weight: {state.weight}
"""
)

# Note that not only the energy values have been modified, but also the weight
# property. The latter contains the generation weight corresponding to the
# generation procedure. This is simply
#
# generation_weight = 1 / generation_pdf(generated_value).                (eq1)
#
# It is important to keep track of this weight in order to properly normalize
# the Monte Carlo integration. Otherwise, results would be arbitrary, i.e.
# dependant on the generation procedure. That is, two different generation
# procedures should lead to the same result, but with potentially different
# Monte Carlo efficiencies (convergence speed).


# =============================================================================
# Let us now perform the Monte Carlo generation over the complete observation
# space. That for, let us assume that muons are counted through an idealised
# vertical detection plane with

height_range = (0, 1) # m
width = 1             # m

# The observation is made for directions spanning a solid angle bin, delimited
# as

azimuth_range = (-0.5, 0.5)   # deg
elevation_range = (9.5, 10.5) # deg

# Accordingly, the observation space is further generated as

state.generate("azimuth",   prng.uniform(*azimuth_range))
state.generate("elevation", prng.sin(*elevation_range))
state.generate("height",    prng.uniform(*height_range))

# Note that the generation procedure actually runs over sin(elevation), not
# directly over the elevation. This is consistent with the definition of muon
# fluxes, which are normalised per solid angle (i.e. using cos(theta) or
# sin(elevation) as integration variable).
#
# Note also that we do not generate positions along the horizontal coordinate.
# It is irrelevant in this simple case since the geometry is invariant by
# rotation around the local vertical. However, we still need to update the
# generation weight accordingly as

state.weight *= width

# That is, we assume that horizontal coordinates are generated uniformly.
#
# In addition, the rate of muons crossing the detection plane is related to the
# flux by a luminance factor corresponding to the cosine of the angle between
# the muon's direction and the normal to the detection plane. In the present
# case, assuming that the detection plane normal is centered in azimuth, this
# factor writes

azimuth_plane = numpy.mean(azimuth_range)
state.weight *= numpy.cos(numpy.radians(state.azimuth - azimuth_plane)) * \
                numpy.cos(numpy.radians(state.elevation))

# Let us finally compute the flux corresponding to the `observed' states. This
# is done as

flux = fluxmeter.flux(state)

# Then, the rate corresponding to each Monte Carlo observation is obtained as

rate = flux.value * state.weight / events

# A Monte Carlo estimate of the total rate of muons crossing the detection plane
# is obtained by summing up the rates of all events. But, let us instead use the
# Flux.reduce method (see e.g. the advanced/transport.py example). Using this
# method provides us with statistics of the Monte Carlo integration as well.
# This is done as

total_rate = flux.reduce(weight=state.weight)

# Note that the weight argument in the previous expression indicates that flux
# values need to be weighted, by the generation weights in this case. Let us
# print the result below.

print(f"""\
# Observation statistics:
- rate:      {total_rate.value} +- {total_rate.value_error} Hz
- asymmetry: {total_rate.asymmetry} +- {total_rate.asymmetry_error}
""")


# =============================================================================
# Let us point out that, from the previous Monte Carlo observations,
# differential distributions can be obtained as well. For example, let us
# compute the differential rate of events as function of the observed kinetic
# energy. That for, let us define energy bins over a logarithmic scale, as

bin_edges = numpy.logspace(
    numpy.log10(min(state.energy)),
    numpy.log10(max(state.energy)),
    41
)

# Then, the PDF of the observed energy is estimated by binning Monte Carlo
# observations with weights given by their rates, as

pdf, _ = numpy.histogram(
    state.energy,
    bin_edges,
    weights = rate,
    density = True
)

# Let us finally draw the result. First, we compute bin centers as

energy = 0.5 * (bin_edges[1:] + bin_edges[:-1])

# Then, the plot.

plot.style.use("examples.mplstyle")
plot.figure()
plot.plot(energy, pdf * total_rate.value, "ko-")
plot.xscale("log")
plot.yscale("log")
plot.xlabel("kinetic energy (GeV)")
plot.ylabel("differential rate (GeV$^{-1}$ s$^{-1}$)")
plot.show()
