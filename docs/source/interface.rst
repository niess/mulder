Python interface
================

This section describes the Python user interface of Mulder. The user interface
is organised in five topics, `Geometry <Geometry interface_>`_, `Materials
<Materials interface_>`_, `States <States interface_>`_, `Simulation <Simulation
interface_>`_ and `Pictures <Picture interface_>`_, as described below.


Geometry interface
~~~~~~~~~~~~~~~~~~

.. autoclass:: mulder.EarthGeometry

   .. method:: __new__(*layers, atmosphere=None, magnet=None)
   .. method:: __getitem__(key, /)

   .. automethod:: locate
   .. automethod:: scan
   .. automethod:: trace

   .. autoattribute:: layers
   .. autoattribute:: z

----

.. autoclass:: mulder.ExternalGeometry

   .. method:: __new__(*args, **kwargs)

----

.. autoclass:: mulder.Grid

   .. method:: __new__(*args, **kwargs)
   .. method:: __call__(xy, y=None, /, *, notify=None)

      Computes the altitude value at grid point(s).

   .. method:: __add__(value, /)
   .. method:: __radd__(value, /)
   .. method:: __rsub__(value, /)
   .. method:: __sub__(value, /)

   .. automethod:: gradient

   .. autoattribute:: projection
   .. autoattribute:: x
   .. autoattribute:: y
   .. autoattribute:: z

----

.. autoclass:: mulder.Layer

   .. method:: __new__(*args, **kwargs)
   .. method:: __getitem__(key, /)

   .. automethod:: altitude

   .. autoattribute:: data
   .. autoattribute:: density
   .. autoattribute:: material
   .. autoattribute:: z

----

.. autoclass:: mulder.LocalFrame

   .. method:: __new__(**kwargs)


Materials interface
~~~~~~~~~~~~~~~~~~~

.. autoclass:: mulder.materials.Composite

   .. method:: __new__(name, /, **kwargs)

----

.. autoclass:: mulder.materials.Element

   .. method:: __new__(symbol, /, **kwargs)

----

.. autoclass:: mulder.materials.Mixture

   .. method:: __new__(name, /, **kwargs)


States interface
~~~~~~~~~~~~~~~~

.. autoclass:: mulder.GeographicStates

   .. method:: __new__(states=None, /, **kwargs)

   .. rubric:: Coordinates methods
     :heading-level: 4

   .. automethod:: from_local
   .. automethod:: to_local

   .. rubric:: Array methods
     :heading-level: 4

   .. automethod:: empty
   .. automethod:: full
   .. automethod:: from_array
   .. automethod:: zeros

   .. rubric:: Coordinates attributes
     :heading-level: 4

   .. autoattribute:: altitude
   .. autoattribute:: azimuth
   .. autoattribute:: elevation
   .. autoattribute:: latitude
   .. autoattribute:: longitude

   .. rubric:: State attributes
     :heading-level: 4

   .. autoattribute:: energy
   .. autoattribute:: pid
   .. autoattribute:: weight

   .. rubric:: Array attributes
     :heading-level: 4

   .. autoattribute:: array
   .. autoattribute:: ndim
   .. autoattribute:: shape
   .. autoattribute:: size

----

.. autoclass:: mulder.LocalStates

   .. method:: __new__(states=None, /, *, frame=None, **kwargs)

   .. rubric:: Coordinates methods
     :heading-level: 4

   .. automethod:: from_geographic
   .. automethod:: to_geographic

   .. rubric:: Array methods
     :heading-level: 4

   .. automethod:: empty
   .. automethod:: full
   .. automethod:: from_array
   .. automethod:: zeros

   .. rubric:: Coordinates attributes
     :heading-level: 4

   .. autoattribute:: position
   .. autoattribute:: direction

   .. rubric:: State attributes
     :heading-level: 4

   .. autoattribute:: energy
   .. autoattribute:: pid
   .. autoattribute:: weight

   .. rubric:: Array attributes
     :heading-level: 4

   .. autoattribute:: array
   .. autoattribute:: ndim
   .. autoattribute:: shape
   .. autoattribute:: size


Simulation interface
~~~~~~~~~~~~~~~~~~~~

.. autoclass:: mulder.Atmosphere

   An atmospheric medium.

   This class manages the properties of the atmosphere medium. The atmosphere is
   assumed to be homogeneous in composition, but with a density varying
   vertically.

   .. method:: __new__(model=None, /, *, material=None)

      Create a new atmospheric medium.

      The *model* argument specifies the vertical density profile, provided as
      an :math:`N \times 2` array mapped as :math:`[(z_0, \rho_0), \ldots,
      (z_{N-1}, \rho_{N-1})]` with altitudes (:math:`z`) in meters and densities
      (:math:`\rho`) in :math:`\mathrm{kg} / \mathrm{m}^3`. For instance,

      >>> atmosphere = mulder.Atmosphere((
      ...     (     0, 1.225E+00),
      ...     ( 1_000, 4.135E-01),
      ...     (30_000, 1.841E-02),
      ...     (70_000, 8.283E-05),
      ... ))

      .. note::

         The provided altitude values (:math:`z`) should be strictly increasing
         and the density values (:math:`\rho`) must be strictly positive.

      Alternatively, a predefined model can be specified, e.g. as

      >>> atmosphere = mulder.Atmosphere("midlatitude-summer")

      See the :attr:`models <mulder.Atmosphere.models>` class attribute for a
      list of predefined density models.

      By default, the atmosphere is composed of :python:`"Air"`. This can be
      overriden with the optional *material* argument, for instance as

      >>> atmosphere = mulder.Atmosphere(material="SaturatedAir")

      See the `Materials interface`_ for information on defining custom
      materials.

   .. automethod:: density

      This method is vectorised. It accepts a scalar *altitude* input as well as
      an array of *altitude* values. For instance,

      >>> densities = atmosphere.density(np.linspace(0E+00, 1E+05, 10001))

   .. rubric:: Attributes
     :heading-level: 4

   .. autoattribute:: material

      This is a :underline:`mutable` attribute. For instance, the following
      changes the atmosphere material

      >>> atmosphere.material = "SaturatedAir"

      See the `Materials interface`_ for information on defining custom
      materials.

   .. autoattribute:: model

      This is an :underline:`immutable` attribute containing a copy of the
      density model used when the atmospheric medium was defined.

   .. rubric:: Class attributes
     :heading-level: 4

   .. autoattribute:: models

      Predefined density models according to the MODTRAN 2/3 report [AbAn96]_.

----

.. autoclass:: mulder.EarthMagnet

   .. method:: __new__(*args, **kwargs)
   .. method:: __call__(*args, **kwargs)

   .. autoattribute:: altitude
   .. autoattribute:: day
   .. autoattribute:: month
   .. autoattribute:: year

----

.. autoclass:: mulder.Fluxmeter

   .. method:: __new__(*layers, **kwargs)
   .. method:: __call__(states=None, /, *, events=None, notify=None, **kwargs)

   .. automethod:: grammage
   .. automethod:: transport

   .. autoattribute:: geometry
   .. autoattribute:: mode
   .. autoattribute:: physics
   .. autoattribute:: random
   .. autoattribute:: reference

----

.. autoclass:: mulder.Physics

   .. method:: __new__(*args, **kwargs)

   .. automethod:: compile

   .. autoattribute:: bremsstrahlung
   .. autoattribute:: pair_production
   .. autoattribute:: photonuclear

----

.. autoclass:: mulder.Random

   .. method:: __new__(*args, **kwargs)

   .. automethod:: uniform01

   .. autoattribute:: index
   .. autoattribute:: seed

----

.. autoclass:: mulder.Reference

   .. method:: __new__(model, /, **kwargs)
   .. method:: __call__(states=None, /, **kwargs)

   .. autoattribute:: altitude
   .. autoattribute:: elevation
   .. autoattribute:: energy




Picture interface
~~~~~~~~~~~~~~~~~

.. autoclass:: mulder.Camera

   .. method:: __new__(coordinates=None, /, **kwargs)

   .. automethod:: shoot

   .. autoattribute:: altitude
   .. autoattribute:: azimuth
   .. autoattribute:: elevation
   .. autoattribute:: focal_length
   .. autoattribute:: fov
   .. autoattribute:: latitude
   .. autoattribute:: longitude
   .. autoattribute:: pixels
   .. autoattribute:: ratio
   .. autoattribute:: resolution
