#! /usr/bin/env python3
"""This example illustrates usage of the Fluxmeter.transport method.

The Fluxmeter.transport method takes as input an observation mulder.State and
returns a conjugated state, transported to the reference's height where the
corresponding muon flux is known. The conjugated State carries a transport
weight resulting from two multiplicative factors:

- A Jacobian factor, expressing the conservation of the total number of muons
  during the transport, independently of kinematic variables used for describing
  the flux.

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
# As a preamble, let us define a geometry and instanciate the corresponding
# fluxmeter. Following the `basic/fluxmeter.py` example, this is done using a
# brief notation where arguments are implicitly packed. Thus

fluxmeter = Fluxmeter(
    Rock  = "data/GMRT.png",
    Water = 0
)

# The observation state is taken at the middle of the map, 10 m below the rock
# layer, as

rock = fluxmeter.geometry.layers[0]
s_obs = State(
    position = rock.middle.position,
    azimuth = 90,   # deg
    elevation = 60, # deg
    energy = 10     # GeV
)
s_obs.height -= 10  # m


# =============================================================================
# Let us first discuss the continuous mode. This is the fastest but most
# approximate algorithm. Conceptually, it is also the most familiar to us.
#
# In continuous mode, muons behave as deterministic point like (charged)
# particles with an averaged energy loss. Thus, in the absence of any
# geomagnetic field, they follow straight line trajectories. This approximation
# tremendously simplifies the transport problem, resulting in rather fast flux
# computations. However, the continuous assumptions are valid only for
# intermediary muon kinetic energies, typically 1-100 GeV. Thus, depending on
# your use case, this might be relevant or not.
#
# In continuous mode, the observation state has a deterministic conjugated state
# within the reference model. This conjugated state is obtained as

s_ref = fluxmeter.transport(s_obs)

# Let us print some properties of this state

print(f"""\
# Conjugated state (at reference):
- height:    {s_ref.height} m
- elevation: {s_ref.elevation} deg
- energy:    {s_ref.energy} GeV
- weight:    {s_ref.weight}
""")

# As expected, the reference state has a null height, corresponding to Mulder's
# default reference model. Let us also point out that the transport weight
# differs from unity (though, only slightly in this case).


# =============================================================================
# The muon flux for the given observation state is simply obtained from the
# conjugated state as
#
# flux(s_obs) = reference_flux(s_ref) * s_ref.weight.                     (eq1)
#
# That is, the observed flux is given by the reference flux for the conjugated
# state times the transport weight. This flux can be obtained directly with the
# State.flux method, as

flux = s_ref.flux(fluxmeter.reference)

print(f"""\
# Observed flux (default reference):
- value:     {flux.value} per GeV m^2 s sr
- asymmetry: {flux.asymmetry}
""")

# Let us point out that the previous result takes into account the transport
# weight carried by the reference state. As a cross-check

tmp = fluxmeter.reference.flux(
    elevation = s_ref.elevation,
    energy = s_ref.energy
)
eq1 = tmp.value * s_ref.weight

assert(flux.value == eq1)

# Note also that the flux value could have been obtained directly with the
# Fluxmeter.flux method, as

assert(flux == fluxmeter.flux(s_obs))

# However, explicitly computing the conjugated state can be handy in some cases.
# In particular, in order to compare results for different reference models.
# Indeed, as can be seen from (eq1), there is usually no need to recompute the
# transport parameters when changing the reference flux (except if the heights
# of the references do not overlap). For example, the observed flux for the PDG
# model would be

reference_pdg = Reference("data/pdg.table")
flux_pdg = s_ref.flux(reference_pdg)

print(f"""\
# Observed flux (PDG reference):
- value:     {flux_pdg.value} per GeV m^2 s sr
- asymmetry: {flux_pdg.asymmetry}
""")


# =============================================================================
# So far, we did not discuss the muon charge asymmetry. Indeed, in the absence
# of geomagnetic field, muons and anti-muons follow identical trajectories.
# Thus, the charge asymmetry is simply obtained from the conjugated state
# without any transport weight, as
#
# A_obs = A_ref(s_ref).                                                   (eq2)
#
# However, when a geomagnetic field is added, muon and anti-muons follow
# different trajectories. Note that this is marginal for many applications.
# Nevertheless, let us add a geomagnetic field, and let us compute the
# corresponding conjugated states for both charges.

fluxmeter.geometry.geomagnet = True

s_obs.pid = 13
s_muon = fluxmeter.transport(s_obs)

s_obs.pid = -13
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

# The resulting asymmetry is given by
#
# A(s_obs) = (A(s_muon) * flux(s_muon) + A(s_anti) * flux(s_anti) /
#            (flux(s_muon) + flux(s_anti)).                               (eq3)
#
# Mulder also supports directly adding two mulder.Flux objects, as

flux = flux_muon + flux_anti

# where the asymmetry is computed according to (eq3). Let us print the result

print(f"""\
# Observed flux (with geomagnetic field):
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
# Let us recall that (anti)muons are actually elementary particles governed by
# quantum physics (Quantum Field Theory, more precisely). Thus, their
# interactions with matter (collisions) are intrinsically non deterministic. The
# continuous picture is only an approximate model, resulting from the
# macroscopic averaging of a large number of atomic collisions. Most of these
# collisions are actually soft, i.e. with a small energy transfer. The
# collective effect of these soft collisions is well rendered by a continuous
# (average) energy loss.
#
# However, as the muon energy increases (typically above a few hundreds of GeV),
# hard (catastrophic) collisions become more and more likely, where the muon
# looses a significant fraction of its kinetic energy. As a result, the energy
# loss of high energy muons strongly fluctuates. This needs to be taken into
# account for precise flux predictions, e.g. with a discrete Monte Carlo
# treatment.
#
# In mixed mode, (anti)muons still follow deterministic straight trajectories,
# in the absence of any geomagnetic field. However, the energy loss of
# catastrophic collisions is simulated in detail (soft collisions are still
# treated continuously). As a result, conjugated states are no more
# deterministic. In practice, the fluxmeter usage is almost identical to the
# continuous case, but applying a Monte Carlo procedure. That is, one needs to
# simulate multiple discrete events for a given observation point. Then, a flux
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
# catastrophic collision occurred. As a result, a higher energy (anti)muon is
# required in order to actually reach the observer.
#
# For the following, let us generate a more significant number of events, and
# let us compute their corresponding fluxes.

s_ref = fluxmeter.transport(
    s_obs,
    events = 10000
)

flux = s_ref.flux(fluxmeter.reference)


# =============================================================================
# The Monte Carlo estimate of the flux at observation point is obtained by
# averaging previous results, as

flux_mean = numpy.mean(flux.value)
flux_std = numpy.std(flux.value) / numpy.sqrt(flux.size - 1)

print(f"""\
# Observed flux (mixed mode):
- value: {flux_mean} +- {flux_std} per GeV m^2 s sr
""")

# In this particular case, the Monte Carlo estimate should be close to the
# previous continuous result. Indeed, as stated previously, fluctuations in the
# energy loss are only significant for high energy muons traversing a
# significant amount of matter (hundreds of meters of rock). As a cross-check,
# you could increase the observation's kinetic energy to 1 TeV (i.e. 1000 GeV)
# and.decrease the height by 1 km, at the top of this example (and run again).
