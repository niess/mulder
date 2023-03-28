#! /usr/bin/env python3
"""This example discusses usage of `Mulder.Prng` objects.

Mulder.Prng objects provide access to the Pseudo-Random Numbers Generator (PRNG)
of a Fluxmeter. These objects have a rather simple interface, presented below.
Actually, the main difficulty, according to us, is to properly use *any* PRNG
with a mulder Monte Carlo. This is also discussed below.

Note:
  This example assumes that your are already familiar with mulder Fluxmeters,
  and flux computations. You might first check other examples otherwise, the
  like `basic/fluxmeter.py`, `advanced/flux.py` or `advanced/transport.py`.
"""

import matplotlib.pyplot as plot
from mulder import Fluxmeter
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
# Let us now discuss the actual usage of Prngs. So far, in other examples, we
# mostly discussed point estimates. That is, the observation is done at a fixed
# point, or over a grid of points. However, in real life cases, one usually has
# to deal with distributed observations, e.g. an observation performed over an
# energy range, or over angular bins. Then, one could sample observations over a
# thin (sub)mesh, and perform a numeric integration e.g. using trapezes or a
# higher order method. However, this is not always possible, nor efficient
# depending on the use case. For example, except in continuous mode, flux
# estimates anyway require a Monte Carlo averaging. Then, one would as well rely
# on a complete Monte Carlo integration, including the observation space.
#
# As an example, let us consider a distributed observation over energy and
# direction. That is, let us assume that we have a detector selecting muons over
# a given energy range and angular bin, as

energy_min, energy_max = 1E-03, 1E+03
elevation_min, elevation_max = 9.5, 10.5
azimuth_min, azimuth_max = -0.5, 0.5


