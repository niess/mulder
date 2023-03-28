#! /usr/bin/env python3
"""This example discusses more in depth the `Fluxmeter.flux` method.

The Fluxmeter.flux method is a high level function for computing muon flux
estimates, given a reference model of atmospheric muons. In this example, the
Fluxmeter.flux method is used in order to infer atmospheric muon fluxes at
different heights. Results are compared to native reference flux values, also
available in this case.

See also the `advanced/transport.py` example for a lower level access to
transport algorithms used in flux computations.

Note:
  This example assumes that your are already familiar with mulder basic
  concepts, the like Fluxmeters, and References (see the corresponding examples
  `basic/fluxmeter.py` and `basic/reference.py`).

  In addition, this example uses a reference model computed with MCEq
  (https://mceq.readthedocs.io) available from

  https://github.com/niess/atmospheric-muon-flux

  under subfolder `data/simulated`.
"""

import matplotlib.pyplot as plot
from mulder import Fluxmeter
import numpy


# =============================================================================
# For this example, we consider a simple opensky geometry with only an
# atmosphere layer. Thus, the fluxmeter is created with no arguments, as

fluxmeter = Fluxmeter()

# Let us recall that, by default, the fluxmeter is configured in continuous
# mode, which we will use for this example. This is usually a good approximation
# for opensky geometries, owing to the low density of the atmosphere.

assert(fluxmeter.mode == "continuous")

# As reference model we consider a tabulation to MCEq results, as below

fluxmeter.reference = "data/flux-mceq-yfm-gsf-usstd.table"

# These results were obtained following Yanez et al., 2019 (arxiv:1909.08365)
# with an US standard atmosphere. The tabulation ranges from 0 to 10,000m high.
# However, we consider only a subset below

height = (0, 5000)

# The reason is that a core assumption of Mulder is that no muons are created
# between the observation point and the reference. This is no more valid when
# reaching towards high heights, and for vertical directions.
#
# Thus let us consider an inclined observation direction, and let us define a
# grid for muon kinetic energy values.

elevation = 30 # deg
energy = numpy.logspace(-2, 2, 401) # Gev


# =============================================================================
# To start with, let us compute the reference flux values for the two extreme
# heights considered  above. This is done in a loop as

flux_ref = []
for h in height:
    flux_ref.append(
        fluxmeter.reference.flux(
            elevation = elevation,
            energy = energy,
            height = h
        )
    )

# Then, let us compute the same flux with mulder, but using the opposite height
# as reference. That for, we need to trick the reference by temporarily
# overriding its height boundaries, in order to enforce muons transport.
#
# Note that in practice one does not need to modify the reference height. Mulder
# automatically takes care of selecting the closest reference point in agreement
# with the geometry. Thus, if we do not override the reference height, in this
# particular case, we would trivially get the same reference flux as above,
# without any transport (since the geometry is empty).
#
# Thus, we start by setting the observation to the minimum height (and the
# reference to the maximum one).

flux = []

fluxmeter.reference.height_min = height[1]
flux.append(
    fluxmeter.flux(
        elevation = elevation,
        energy = energy,
        height = height[0]
    )
)
fluxmeter.reference.height_min = height[0]

# Then, we invert the observation and the reference heights.

fluxmeter.reference.height_max = height[0]
flux.append(
    fluxmeter.flux(
        elevation = elevation,
        energy = energy,
        height = height[1]
    )
)
fluxmeter.reference.height_max = height[1]


# =============================================================================
# Collecting all results, let us draw a comparison figure. In order to avoid
# repetitions, we define a local function, as below.

def plot_property(name):
    """Plot the flux value of its charge asymmetry."""
    plot.fill_between(
        energy,
        numpy.ma.masked_array(
            getattr(flux_ref[0], name),
            mask = flux_ref[0].value <= 0
        ),
        numpy.ma.masked_array(
            getattr(flux_ref[1], name),
            mask = flux_ref[1].value <= 0
        ),
        color = "k",
        alpha = 0.2,
        label = "MCEq"
    )
    for i, hi in enumerate(height):
        plot.plot(
            energy,
            numpy.ma.masked_array(
                getattr(flux_ref[i], name),
                mask = flux_ref[i].value <= 0
            ),
            "k-",
            alpha = 0.4
        )
        s = slice(None, None, 10)
        plot.plot(
            energy[s],
            numpy.ma.masked_array(
                getattr(flux[i], name)[s],
                flux[i].value[s] <= 0
            ),
            "ko",
            markersize = 4,
            label = "Mulder" if i == 0 else None
        )

# Then, let us plot the corresponding figures where the shaded area indicates
# the range of variation of the reference model.

plot.style.use("examples.mplstyle")

plot.figure()
plot_property("value")
plot.legend(loc=3)
plot.xscale("log")
plot.yscale("log")
plot.xlabel("kinetic energy (GeV)")
plot.ylabel("flux (GeV$^{-1}$ m$^{-2}$ s$^{-1}$ sr$^{-1}$)")
plot.show(block=False)

plot.figure()
plot_property("asymmetry")
plot.legend()
plot.xscale("log")
plot.xlabel("kinetic energy (GeV)")
plot.ylabel("charge asymmetry")
plot.show(block=False)

# Let us finally comment the previous results. In this case, a reasonable
# agreement is observed between mulder and MCEq, especially for high energy
# muons. This is expected since MCEq and mulder both rely on the continuous
# approximation, in this case, and the same atmosphere model is used. Thus,
# as long as muons production is negligible, similar results should be obtained.
#
# Interestingly, one also observes that while reference flux values have a fixed
# lower kinetic energy, of 0.1 GeV, mulder results extend below or are limited
# above depending on the use case. Indeed, we are considering down going muons
# with an elevation angle of 30 deg. Thus, when the observer is located below
# the source (i.e. the reference), then muons loose energy in the atmosphere
# before reaching the observer. In the continuous approximation, the
# corresponding energy loss can be obtained simply with the Fluxmeter.transport
# method (see also the `advanced/transport.py` example), as

fluxmeter.reference.height_min = height[1]
state = fluxmeter.transport(
    elevation = elevation,
    energy = energy,
    height = height[0]
)
fluxmeter.reference.height_min = height[0]

# Let us plot the result.

plot.figure()
plot.plot(energy, state.energy, "k-", label="w/ atmosphere")
plot.plot(energy, energy, "k--", label="w/o atmosphere")
plot.xscale("log")
plot.yscale("log")
plot.xlabel("observation energy (GeV)")
plot.ylabel("reference energy (GeV)")
plot.legend(loc=4)
plot.show()

# One sees that at low energies, the energy loss reaches an asymptotic value of
# approximately 2 GeV in this case. Thus, in the converse case, when the
# observer is above the reference, then no muon could be `observed' with a
# kinetic energy below 2 GeV (which is indeed the case, on previous figures 1
# and 2).
#
# Let us point out that the previous reasoning is only valid in the continuous
# approximation. Indeed, inferring the flux above the reference is actually an
# inverse problem, that is non trivial to solve in the discrete case, involving
# a Monte Carlo procedure. However, when considering an opensky geometry, the
# continuous approximation is usually appropriate. Thus, mulder uses it
# transparently in such case. This allows e.g. to use sea level reference models
# while working with a detector located above, in mountains. That is, mulder
# automatically corrects for the reference altitude in such cases. This is done
# as following.
#
# Mulder actually manages two geometries. The user one, and a private opensky
# one. First, muons are backward transported from the observation point to the
# top of the user geometry. This is done using the fluxmeter mode, i.e.
# continuous, discrete or mixed. If the top of the geometry is above the
# reference, then muons are forward transported to the reference, in continuous
# mode, with the private opensky geometry. Transport weights are applied
# accordingly (again, see the `advanced/transport.py` example for more details
# on transport weights).
