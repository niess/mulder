#! /usr/bin/env python3
"""This example illustrates usage of the Fluxmeter.transport method.

The Fluxmeter.transport method takes as input an observation mulder.State and
returns a conjugated state, transported to the reference's height where the
corresponding muon flux is known. The returned conjugated State carries a
transport weight resulting from two multiplicative factors:

- A Jacobian factor, expressing the conservation of the total number of muons
  during the transport, in the absence of sources and sinks.

- An attenuation factor representing muon decays. Note that for reverse
  transport this term is actually a regeneration factor, i.e. it is larger than
  one.

Depending on the Fluxmeter.mode flag, different transport algorithms are used.
Possible values for the mode flag are "continuous", "discrete" or "mixed". The
corresponding algorithms are discussed below, together with examples of
application.

Note:
  This example uses data produced by the `basic/layer.py` and
  `basic/reference.py` examples. Please, run these examples first.
"""

import matplotlib.pyplot as plot
from mulder import Fluxmeter, Reference, State
import numpy


# =============================================================================
# As a preamble, let us define a geometry and let us instanciate the
# corresponding fluxmeter. Following the `basic/fluxmeter.py` example, this is
# is done in a single step (using brief syntax) as

fluxmeter = Fluxmeter(
    Rock  = "data/GMRT.png",
    Water = 0
)

# The observation state is taken at the middle of the map, 10 m below the rock
# layer, as

rock = fluxmeter.geometry.layers[0]
s_obs = State(
    position = rock.middle.position,
    azimuth = 90,   # deg (i.e. looking towards East)
    elevation = 60, # deg (i.e. 30 deg below Zenith)
    energy = 10     # GeV
)
s_obs.height -= 10  # m


# =============================================================================
# Let us first discuss the continuous mode. This is the fastest but most
# approximate algorithm. Conceptually, it is also the most familiar.
#
# In continuous mode, muons behave as deterministic point like (charged)
# particles with an average(d) energy loss. Thus, in the absence of any
# geomagnetic field, they follow straight line trajectories. This approximation
# tremendously simplifies the transport problem, resulting in rather fast flux
# computations. However, the continuous assumption is valid only for
# intermediary muon kinetic energies, typically 1-100 GeV. Thus, depending on
# your use case, this might be relevant or not.
#
# In continuous mode, the observation state has a deterministic conjugated state
# within the reference model. This conjugated state is obtained as

s_ref = fluxmeter.transport(s_obs)

# Let us print some of its properties

print(f"""\
# Conjugated state (at reference):
- height:    {s_ref.height} m
- elevation: {s_ref.elevation} deg
- energy:    {s_ref.energy} GeV
- weight:    {s_ref.weight}
""")

# As expected, the conjugated state has a null height, corresponding to Mulder's
# default reference model. Let us also point out that the transport weight
# differs from unity (though, only slightly in this case).


# =============================================================================
# The muon flux for the given observation state is obtained from the conjugated
# one as
#
# flux(s_obs) = reference_flux(s_ref) * s_ref.weight.                     (eq1)
#
# That is, the `observed' flux is given by the reference flux for the conjugated
# state times the transport weight. This flux is obtained with the State.flux
# method, as

flux = s_ref.flux(fluxmeter.reference)

print(f"""\
# Observation flux (default reference):
- value:     {flux.value} per GeV m^2 s sr
- asymmetry: {flux.asymmetry}
""")

# Let us point out that the previous result takes into account the transport
# weight carried by the reference state (c.f. (eq1)). As a cross-check

flux_ref = fluxmeter.reference.flux(
    elevation = s_ref.elevation,
    energy = s_ref.energy
)
eq1 = flux_ref.value * s_ref.weight

assert(flux.value == eq1)

# Let us also point out that the flux value could have been obtained directly
# from the Fluxmeter.flux method, as

assert(flux == fluxmeter.flux(s_obs))

# However, explicitly computing the conjugated state can be handy in some cases.
# In particular, this is usefull when comparing results for different reference
# models. Indeed, as can be seen from (eq1), there is usually no need to
# recompute the conjugated state when changing the reference flux (except if the
# heights of the references do not overlap). For example, the observation flux
# for the PDG model would be

reference_pdg = Reference("data/pdg.table")
flux_pdg = s_ref.flux(reference_pdg)

print(f"""\
# Observation flux (PDG reference):
- value:     {flux_pdg.value} per GeV m^2 s sr
- asymmetry: {flux_pdg.asymmetry}
""")


# =============================================================================
# So far, we did not discuss the muon charge asymmetry. Indeed, in the absence
# of geomagnetic field, muons and anti-muons follow identical trajectories.
# Thus, the charge asymmetry is simply forwarded from the conjugated state to
# the observation one without any transport weight, as
#
# A_obs = A_ref(s_ref).                                                   (eq2)
#
# However, when a geomagnetic field is added, muon and anti-muons follow
# different trajectories. Note that this is marginal for many applications.
# Nevertheless, let us add a geomagnetic field, and let us compute the
# corresponding conjugated states for both charges.

fluxmeter.geometry.geomagnet = True

s_obs.pid = 13  # This is a muon, using PDG numebering scheme
s_muon = fluxmeter.transport(s_obs)

s_obs.pid = -13 # This is an antimuon, using PDG numbering scheme
s_anti = fluxmeter.transport(s_obs)

print(f"""\
# Comparison of muon and anti-muon states:
- elevation: ({s_muon.elevation}, {s_anti.elevation}) deg
- energy:    ({s_muon.energy}, {s_anti.energy}) GeV
- weight:    ({s_muon.weight}, {s_anti.weight})
""")

# As can be seen, results are very similar for both states. The most significant
# difference is on the angular elevation. The corresponding fluxes at
# observation point are

flux_muon = s_muon.flux(fluxmeter.reference)
flux_anti = s_anti.flux(fluxmeter.reference)

print(f"""\
- flux:      ({flux_muon.value}, {flux_anti.value}) per GeV m^2 s sr
- asymmetry: ({flux_muon.asymmetry}, {flux_anti.asymmetry})
""")

# The resulting charge asymmetry is given by
#
# A(s_obs) = (A(s_muon) * flux(s_muon) + A(s_anti) * flux(s_anti) /
#            (flux(s_muon) + flux(s_anti)).                               (eq3)
#
# Mulder supports directly adding two Flux objects, as

flux = flux_muon + flux_anti

# where the asymmetry is computed according to (eq3). Let us print the result

print(f"""\
# Observation flux (with geomagnetic field):
- value:     {flux.value} per GeV m^2 s sr
- asymmetry: {flux.asymmetry}
""")

# Note that, as previously, we could have obtained this result directly with the
# Fluxmeter.flux method, as

s_obs.pid = 0
assert(flux == fluxmeter.flux(s_obs))

# Let us also point out that the purpose of the previous example was only to
# illustrate the capabilities of the Fluxmeter.transport method. In order to
# perform a consistent flux estimate, the reference flux model should actually
# not be considered at sea level in such a case. Or, it should take geomagnetic
# effects into account, otherwise. Thus, for the remaining discussion, let us
# deactivate the geomagnetic field.

fluxmeter.geometry.geomagnet = None


# =============================================================================
# Let us now discuss the two other transport modes, i.e. "discrete" and "mixed".
# Let us recall that muons are actually elementary particles governed by quantum
# physics (actually, Quantum Field Theory (QFT), to the best of our current
# knowledge). Thus, their interactions with matter (collisions) are
# intrinsically non deterministic. The continuous picture is only an approximate
# model, resulting from the macroscopic averaging of a large number of atomic
# collisions. Most of these collisions are soft, i.e. occuring with a small
# energy transfer. The collective effect of these soft collisions is well
# rendered by a continuous (average) energy loss.
#
# However, as the muon energy increases (typically above a few hundreds of GeV),
# hard (catastrophic) collisions become more and more likely, where the muon
# looses a significant fraction of its initial kinetic energy. As a result, the
# energy loss of high energy muons strongly fluctuates. This needs to be taken
# into account for precise flux predictions, e.g. with a discrete Monte Carlo
# treatment.
#
# In mixed mode, muons still follow deterministic trajectories, straight paths
# in the absence of geomagnetic field. However, the energy loss of catastrophic
# collisions is simulated in detail (soft collisions are still treated
# continuously). As a result, a given observation state now has multiple
# conjugated states. In practice, the fluxmeter usage is almost identical to the
# continuous case, but applying a Monte Carlo procedure. That is, one needs to
# simulate multiple discrete events for a given observation. Then, a flux
# estimate is obtained by averaging the individual result.
#
# As an example, let us switch to mixed mode, and let us generate a bunch of
# conjugated states for the observation.

fluxmeter.mode = "mixed"

s_ref = fluxmeter.transport(
    s_obs,
    events = 5
)

print(f"""\
# Conjugated states (mixed case):
- elevation: {s_ref.elevation} deg
- energy:    {s_ref.energy} GeV
- weight:    {s_ref.weight}
""")

# Note that you get different results each time that you run this example. Thus,
# you should see that sometimes the energy is higher. This indicates that a
# catastrophic collision occurred. As a result, a higher energy muon is required
# in order to actually reach the observer.
#
# For the following, let us now generate a more significant number of events,
# and let us compute their corresponding fluxes.

s_ref = fluxmeter.transport(
    s_obs,
    events = 10000
)

flux = s_ref.flux(fluxmeter.reference)


# =============================================================================
# The Monte Carlo estimate of the flux at observation point is obtained by
# averaging previous results, as

flux_mean = numpy.mean(flux.value)

# As an estimate of the uncertainty on the flux, we consider the standard
# deviation of the same series of value, as

flux_std = numpy.std(flux.value) / numpy.sqrt(flux.size - 1)

# Let us print the result

print(f"""\
# Observation flux (mixed mode):
- value:     {flux_mean} +- {flux_std} per GeV m^2 s sr
""")

# In this particular case, the Monte Carlo estimate should be close to the
# continuous result. Indeed, as stated previously, fluctuations in the energy
# loss are only significant for high energy muons traversing a significant
# amount of matter (hundreds of meters of rock). As a cross-check, you could
# increase the observation's kinetic energy to 1 TeV (i.e. 1000 GeV)
# and.decrease the height by 1 km, at the top of this example (and run again).
#
# The charge assymetry is obtained by generalising previous (eq3). That for, let
# us associate observation probabilities to Monte Carlo events, as

p = flux.value / numpy.sum(flux.value)

# Then, the charge asymmetry at observation point writes as the expectation of
# Monte Carlo asymmetries, as

asymmetry_mean = numpy.sum(p * flux.asymmetry)

# As previously, let us consider the variance of the same series as an
# uncertainty estimate for the mean. Thus

asymmetry_std = (
    max(numpy.sum(p * flux.asymmetry**2) - asymmetry_mean**2, 0) /
    (flux.size - 1)
)

# Let us print the result

print(f"""\
- asymmetry: {asymmetry_mean} +- {asymmetry_std}
""")

# Note that in this particular case the reference's charge asymmetry is
# constant. Thus, we should recover the corresponding value, as well as a null
# uncertainty estimate, up to numerical errors.


# =============================================================================
# Mulder provides a utility class for reducing Monte Carlo flux data, that is
# to a mulder.ReducedFlux. The syntax is as follow

flux = flux.reduce()

# The new ReducedFlux object inherits from a scalar mulder.flux with extra
# statistical properties. Let us print those

print(f"""\
# Flux statistics (mixed mode):
- value:     {flux.value} +- {flux.value_error} per GeV m^2 s sr
- asymmetry: {flux.asymmetry} +- {flux.asymmetry_error}

- events:    {flux.events}
- value_min: {flux.value_min}
- value_max: {flux.value_max}
- zeros:     {flux.zeros}
""")

# Note that the flux estimate might slightly differ from previous direct
# computation due to limited floating point accuracy, when normalising.

precision = 1E-09
assert(abs(flux.value - flux_mean) < precision)
assert(abs(flux.asymmetry - asymmetry_mean) < precision)


# =============================================================================
# Let us finally discuss the specificities of the discrete transport mode.
#
# Let us emphasize that the collective effect of soft collisions not only
# results in muon energy loss, but also in multiple scattering away from its
# initial trajectory. The smaller the energy of the muon, the larger the
# multiple scattering angle. As a result, low energy muons (typically below 1
# GeV in dense media) do not follow straight line trajectories. In order to
# properly render the scattering of these low energy muons, a more complete
# Monte Carlo procedure is required.

# In discrete mode, muons are transported according to a detailed Monte Carlo
# algorithm. That is, collisions are still classified as soft or hard, where
# soft collisions are treated collectively. But, collective soft collisions are
# no more deterministic. The resulting energy loss and scattering angle are
# distributed. Hard collisions are simulated individually, as in mixed mode, but
# taking into account muon's deflection.
#
# Let us switch to discrete mode, and let us illustrate the previous points by
# transporting a bunch of events.

fluxmeter.mode = "discrete"

s_ref = fluxmeter.transport(
    s_obs,
    events = 5
)

print(f"""\
# Conjugated states (discrete case):
- elevation: {s_ref.elevation} deg
- energy:    {s_ref.energy} GeV
- weight:    {s_ref.weight}
""")

# As can be seen, in discrete mode conjugated states all differ, both in energy
# and in elevation angle.


# =============================================================================
# Obviously, the discrete transport mode is the most accurate one, but also the
# most demanding one, cpu-wise. Depending on your use case, the increased
# accuracy might be relevant or not.
#
# Let us further investigate this latter point. That for, lest first generate a
# more significant set of events.

s_ref = fluxmeter.transport(
    s_obs,
    events = 10000
)

# Let us print the corresponding flux statistics.

flux = s_ref.flux(fluxmeter.reference).reduce()

print(f"""\
# Flux statistics (discrete mode):
- value:     {flux.value} +- {flux.value_error} per GeV m^2 s sr
- asymmetry: {flux.asymmetry} +- {flux.asymmetry_error}

- events:    {flux.events}
- value_min: {flux.value_min}
- value_max: {flux.value_max}
- zeros:     {flux.zeros}
""")

# As can be seen, in this case no significant difference is observed on the flux
# estimate, w.r.t. the continuous mode. Going one step further, it is
# interesting to have a look at the angular distribution of events, at
# reference. For the elevation angle, this can be obtained as

flux = s_ref.flux(fluxmeter.reference)
pdf, bin_edges = numpy.histogram(
    s_ref.elevation,
    bins = 20,
    weights = flux.value,
    density = True
)
elevation = 0.5 * (bin_edges[1:] + bin_edges[:-1])

# Let us plot the result

plot.style.use("examples.mplstyle")
plot.figure()
plot.plot(elevation, pdf, "k.-")
plot.xlabel("elevation angle (deg)")
plot.ylabel("pdf (deg$^{-1}$)")
plot.show()

# As can be seen, in this case the distribution is peaked at a single value
# consistent with the straight line expectation, i.e. close to the observation
# angle. The angular distribution has a spread of ~1 deg. However, this does not
# significantly impact the observation flux. That is, larger and smaller
# elevation values, w.r.t. the straight line trajectory, compensate linearly
# (for small enough spread). Thus, this is clearly a case where the continuous
# mode is appropriate.
#
# As a comparison, you could lower the kinetic energy to 0.1 GeV and offset the
# height by only 10 cm below the ground, at the begining of this example (and
# run again). Then, you should observe a much larger angular spread, resulting
# in a small but statistically significant bias on the flux estimate in
# continuous or mixed modes, of approximately 2-3%.


# =============================================================================
# As a final word, let us point out that actually, depending on the geometry of
# your problem, muons scattering can significantly modify the observed flux (up
# to orders of magnitude in extreme cases). This is the case when the straight
# line trajectory traverses a significantly larger amount of matter than some
# alternative -more complex but less attenuated- trajectories. For example, this
# can be problematic for surface detectors where the observation receives
# contributions from low energy muons bouncing on the target, instead of going
# through (see e.g. Niess 2022, arXiv:2206.01457, Figure 11).
