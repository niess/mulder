Mulder
=======
*(MUon fLuxme'DER)*

----

Mulder is a Python package that calculates local variations in the flux of
atmospheric muons caused by geophysical features, as depicted by a Digital
Elevation Model (`DEM`_).

The primary component of Mulder is a fluxmeter, which serves as a muons probe.
The level of detail of fluxmeters is adjustable, ranging from a rapid continuous
approximation, that provides an average flux estimate, to a comprehensive Monte
Carlo, that delivers discrete atmospheric muons.

.. note::

   Mulder only simulates the transport of atmospheric muons, taking into account
   the local features surrounding the fluxmeter. Mulder does not simulate muon
   production by comisc rays. Instead, a reference spectrum of atmospheric muons
   is used as input, providing the opensky flux, i.e. the flux in the absence of
   any ground or other obstacles than the Earth atmosphere.


System of units
---------------

.. note::

   Mulder uses the Metre-Kilogram-Second (MKS) system of units (e.g. kg/m\
   :sup:`3` for a density), except for particles' kinetic energies which are
   expressed in GeV.


Documentation
-------------

.. toctree::
   :maxdepth: 2

   installation
   interface
   references


.. ============================================================================
.. 
.. URL links.
.. 
.. ============================================================================

.. _DEM: https://en.wikipedia.org/wiki/Digital_elevation_model
.. _GeoTIFF: https://fr.wikipedia.org/wiki/GeoTIFF
.. _TOML: https://toml.io/en/
.. _Turtle: https://github.com/niess/turtle
