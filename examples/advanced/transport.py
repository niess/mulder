#! /usr/bin/env python3
"""This example illustrates usage of the Fluxmeter.transport method.

The Fluxmeter.transport method takes as input a Mulder observation State and
returns a conjugated state, transported to the reference's height where the
corresponding muon flux is known. The returned State carries a transport weight
resulting from two multiplicative factors:

- A Jacobian factor, expressing the conservation of the total number of muons
  during the transport.

- An attenuation factor representing muon decays. Note that for reverse
  transport this term is actually a regeneration factor, i.e. larger than 1.

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
# brief notation where arguments packing is implicit. Thus

fluxmeter = Fluxmeter(
    Rock  = "data/GMRT.png",
    Water = 0
)

# The observation state is taken at the middle of the map, 10 m below the rock
# layer, as

rock = fluxmeter.geometry.layers[0]
s_obs = State(
    position = rock.position(
        0.5 * (rock.xmin + rock.xmax),
        0.5 * (rock.ymin + rock.ymax)
    ),
    azimuth = 90,   # deg
    elevation = 60, # deg
    energy = 10     # GeV
)
s_obs.height -= 10  # m


# =============================================================================
# Let us first discuss the continuous mode. This is the fastest but most
# approximate algorithm.
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
- height    = {s_ref.height} m
- elevation = {s_ref.elevation} deg
- energy    = {s_ref.energy} GeV
- weight    = {s_ref.weight}
""")

# As expected, the reference state has a null height, corresponding to Mulder's
# default reference model. Let us also point out that the transport weight
# differs from 1 (though, only slightly in this case).


# =============================================================================
# The muon flux for the given observation state is simply obtained from the
# conjugated state as
#
# phi_obs = phi_ref(s_ref) * s_ref.weight . (eq1)
#
# That is, the observed flux is given by the flux for the conjugated state, i.e.
# transported to the reference, times the transport weight. This flux can be
# obtained directly with the State.flux method, as

flux = s_ref.flux(fluxmeter.reference)

print(f"""\
# Observed flux (default reference):
- value     = {flux.value}
- asymmetry = {flux.asymmetry}
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
- value     = {flux_pdg.value}
- asymmetry = {flux_pdg.asymmetry}
""")
