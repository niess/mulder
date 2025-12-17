Flux computation
================

Mulder's primary intent is to compute (simulate) alterations (absorption,
scattering) in the flux of atmospheric muons caused by a geometry of interest.
For this purpose, Mulder offers a :py:class:`~mulder.Fluxmeter` object, which
may be regarded as an ideal muon probe. :py:class:`Flumeters <mulder.Fluxmeter>`
may be operated in three modes, `continuous <Continuous mode>`_, `mixed <Mixed
mode>`_ or `discrete <Discrete mode>`_, as described below.

.. note::

   :py:class:`~mulder.Fluxmeter` objects use the Backward Monte Carlo technique
   [NBCL18]_ to compute the altered muon flux.


Continuous mode
~~~~~~~~~~~~~~~

In continuous mode, muons follow a deterministic trajectory with a deterministic
energy-loss obtained within the Continuous Slowing Down Approximation (`CSDA`_).
The continuous approximation is efficient for small to medium-sized geometries
(i.e. less than ~300\ m of rocks [Nie22]_), when scattered muons are negligible
(i.e. typically for buried detectors).

Due to the Earth's magnetic field, the trajectories of muons and anti-muons
differ, as they gyrate in opposite directions. In dense media, the muon CSDA
range is significantly smaller than the magnetic gyro-radius across all
energies. Thus, in many situations, the geomagnetic field can be disregarded.
Then, in the continuous approximation, muons essentially follow straight
lines trajectories.

.. note::

   In the following discussion, for the sake of simplicity, we assume that the
   geomagnetic field is negligible, and therefore treat muons and anti-muons as
   a single entity. If this is not the case, then the following equations should
   be applied separately to both muon charges and the result summed to obtain
   the total flux.

Let us denote :math:`\phi_0` the open-sky reference flux, i.e. the flux of
atmospheric muons in the absence of any geometry. Under the continuous
assumption, the altered muon flux, :math:`\phi_1`, can be related to
:math:`\phi_0` as follows

.. math::
   :label: continuous-flux

   \phi_1(S_1) = \omega_c\left(S_0, S_1\right) \phi_0(S_0),

where :math:`S_1` denotes the observation :ref:`state <sec-states-interface>`,
and where :math:`S_0` is a reference state, connected to :math:`S_1` by
transport equations. The factor :math:`\omega_c(S_0, S_1)` is a transport weight
depending on the matter distribution along the muon trajectory, with and without
the geometry.

The :py:meth:`~mulder.Fluxmeter.transport` method of a
:py:class:`~mulder.Fluxmeter` object returns the reference state, :math:`S_0`,
and the corresponding transport weight, :math:`\omega_c(S_0, S_1)`, given an
observation state, :math:`S_1`. The :py:meth:`~mulder.Fluxmeter.flux` method
directly computes the flux :math:`\phi_1(S_1)` using equation
:eq:`continuous-flux`, in continuous mode. Thus, the two following code snippets
yield the same flux,

.. doctest::
   :hide:

   >>> meter = mulder.Fluxmeter(50.0)
   >>> state1 = mulder.GeographicStates(elevation=50)

>>> state0 = meter.transport(state1)
>>> flux1 = state0.weight * meter.reference.flux(state0)

.. doctest::
   :hide:

   >>> flux1a = flux1

or directly

>>> flux1 = meter.flux(state1)

.. doctest::
   :hide:

   >>> assert_allclose(flux1a, flux1)

Discrete mode
~~~~~~~~~~~~~

The discrete mode is the most realistic but also the most CPU-intensive option.
In discrete mode, the muon trajectory and the energy loss are stochastic.
Therefore, a given observation state, :math:`S_1`, is associated with several
reference states, :math:`S_{0,i}`. In this case, a Monte Carlo estimate of the
altered flux is obtained as

.. math::
   :label: discrete-flux

   \hat{\phi}_{1, N}(S_1) = \frac{1}{N} \sum_{i=1}^N{
       \omega_d\left(S_{0, i}, S_1\right) \phi_0(S_{0, i})} .

In dicrete mode, the :py:meth:`~mulder.Fluxmeter.flux` and
:py:meth:`~mulder.Fluxmeter.transport` methods accept an additional parameter,
*events*, which defines the number :math:`N` of Monte Carlo samples per input
observation state. For instance, the following samples a set of :math:`N`
reference states, from which the flux is estimated using equation
:eq:`discrete-flux`.

.. doctest::
   :hide:

   >>> meter.mode = "discrete"
   >>> meter.random.seed = 123456
   >>> N = 1000

>>> states0 = meter.transport(state1, events=N)
>>> flux1 = sum(states0.weight * meter.reference.flux(states0)) / N

.. doctest::
   :hide:

   >>> assert states0.size == N
   >>> flux1a = flux1

The same flux could have been estimated directly as,

>>> flux1, sigma1 = meter.flux(state1, events=N)

.. doctest::
   :hide:

   >>> assert_allclose(flux1a, flux1, rtol=5E-02)

Note that in the discrete case, the :py:meth:`~mulder.Fluxmeter.flux` method
also returns an error estimate (:python:`sigma1`) on the flux, determined from
the sample variance.


Mixed mode
~~~~~~~~~~

The mixed mode represents a compromise between the continuous and discrete
cases. In mixed mode, the muon trajectory is still deterministic, but stochastic
energy losses may occur. This approximation is efficient for large geometries
(more than 300\ m of rocks) in cases where muon scattering can be disregarded.
In mixed mode, the altered flux is estimated using eq. :eq:`discrete-flux`. The
mixed and discrete modes share the same interface.


.. URL links.
.. _CSDA: https://en.wikipedia.org/wiki/Continuous_slowing_down_approximation_range
