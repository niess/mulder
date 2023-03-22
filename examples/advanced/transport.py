#! /usr/bin/env python3
"""This example illustrates the usage of the Fluxmeter.transport method.

The Fluxmeter.transport method takes as input a Mulder observation State and
returns its dual state, transported to the reference's height where the
corresponding muon flux is known. The returned State carries a transport weight
resulting from two multiplicative terms:

- A Jacobian factor, expressing the conservation of the total number of muons
  during the transport.

- An attenuation factor representing muon decays. Note that for reverse
  transport this term is actually a regeneration factor, i.e. larger than 1.
"""

import matplotlib.pyplot as plot
from mulder import Fluxmeter, Geometry
import numpy


# =============================================================================
# In continuous mode, muons behave as deterministic point like (charged)
# particles with an averaged energy loss. Thus, in the absence of any
# geomagnetic field, they follow straight line trajectories. This approximation
# tremendously simplifies the transport problem, resulting in rather fast flux
# computations. However, the continuous assumption is valid only for
# intermediary muon kinetic energies, typically 1-100 GeV. Thus, depending on
# your use case, this might be relevant or not.
