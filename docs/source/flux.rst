Flux computation
================

Mulder's primary intent is to compute (simulate) local variations (absorption,
scattering) in the flux of atmospheric muons caused by a local geometry of
interest. For this purpose, Mulder offers a :py:class:`~mulder.Fluxmeter`
object, which may be regarded as an ideal muon probe. :py:class:`Flumeters
<mulder.Fluxmeter>` may be operated in three modes: continuous, mixed or
discrete.

.. note::

   :py:class:`~mulder.Fluxmeter` objects use the Backward Monte Carlo technique
   [NBCL18]_ to compute the local muon flux.

In continuous mode, muons follow a deterministic trajectory (a straight line, in
the absence of any geomagnetic field) with a deterministic energy-loss obtained
within the Continuous Slowing Down Approximation (`CSDA`_). This is an efficient
approximation for small to medium-sized geometries (i.e. less than ~300\ m of
rocks [Nie22]_), when scattered muons are negligible (i.e. typically for buried
detectors). Under the continuous assumption, the local muon flux,
:math:`\phi_1`, can be related to the open-sky reference flux, :math:`\phi_0`,
i.e. the flux in the absence of any local geometry, as follows

.. math::
   :label: continuous-flux

   \phi_1(S_1) = \omega_c\left(S_0, S_1\right) \phi_0(S_0),

where :math:`S_1` denotes the observation :ref:`state <sec-states-interface>`,
and where :math:`S_0` is a reference state, connected to :math:`S_1` by
transport equations. The factor :math:`\omega_c(S_0, S_1)` is a transport weight
depending on the matter distribution, with and without the local geometry.

Conversely, the discrete mode is the most realistic but also the most
CPU-intensive option. In discrete mode, the muon trajectory and the energy
loss are stochastic. Therefore, a given observation state, :math:`S_1`, is
associated with several reference states, :math:`S_{0,i}`. In this case, a Monte
Carlo estimate of the local flux is obtained as

.. math::
   :label: discrete-flux

   \hat{\phi}_{1, N}(S_1) = \frac{1}{N} \sum_{i=1}^N{
       \omega_d\left(S_{0, i}, S_1\right) \phi_0(S_{0, i})} .

The mixed mode represents a compromise between the continuous and discrete
cases. In mixed mode, the muon trajectory is still deterministic, but stochastic
energy losses may occur. This approximation is efficient for large geometries
(more than 300\ m of rocks), but when muon scattering is irelevant. In mixed
mode, the local flux is estimated using eq. :eq:`discrete-flux`.

The :py:meth:`~mulder.Fluxmeter.transport` method of a
:py:class:`~mulder.Fluxmeter` object returns the reference state(s),
:math:`S_{0,i}`, and the corresponding transport weight(s),
:math:`\omega_c(S_{0,i}, S_1)`, given an observation state, :math:`S_1`. The
:py:meth:`~mulder.Fluxmeter.flux` method directly computes the local flux,
:math:`\phi_1(S_1)`, using equation :eq:`continuous-flux` or
:eq:`discrete-flux`, depending on the :py:class:`~mulder.Fluxmeter` mode. For
instance, in continuous mode, the two following code snippets yield the same
flux,

>>> state0 = fluxmeter.transport(state1)
>>> flux1 = state1.weight * fluxmeter.reference.flux(state0)

or directly

>>> flux1 = fluxmeter.flux(state1)

In stochastic modes (mixed or discrete), the *event* parameter defines the number
:math:`N` of Monte Carlo samples per input observation state.


.. URL links.
.. _CSDA: https://en.wikipedia.org/wiki/Continuous_slowing_down_approximation_range
