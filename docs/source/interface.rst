Python interface
================

This section describes the Python user interface of Mulder. The user interface
is organised in five topics, `Geometry <Geometry interface_>`_, `Materials
<Materials interface_>`_, `States <States interface_>`_, `Simulation <Simulation
interface_>`_ and `Pictures <Picture interface_>`_, as described below.


Geometry interface
~~~~~~~~~~~~~~~~~~

.. autoclass:: mulder.EarthGeometry

   This class represents a stratified section of the Earth. The strates (or
   :py:class:`~mulder.Layer`\ s) form distinct propagation media that are
   assumed to be uniform in composition and density. They are delimited by
   parametric surfaces, :math:`z = f(x, y)`, typically described by a
   :py:class:`Grid` of elevation values, :math:`z_{ij} = f(x_j, y_i)`, forming a
   Digital Elevation Model (`DEM`_).

   .. note::

      :py:class:`~mulder.EarthGeometry` objects are immutable, i.e. their
      structure cannot be modified. However, the
      :py:attr:`~mulder.Layer.density` and :py:attr:`~mulder.Layer.material` of
      :py:attr:`~mulder.EarthGeometry.layers` is mutable.

   .. method:: __new__(*layers)

      Creates a new Earth geometry.

      The *layers* are provided in index order, i.e. the first layer has index
      :python:`0` and is thus the bottom strate. Each individual layer argument
      may be either an explicit :py:class:`~mulder.Layer` object, or data-like
      objects coercing to the latter. For instance, the following syntaxes lead
      to the same geometry.

      >>> geometry = mulder.EarthGeometry(
      ...     mulder.Layer("dem.tif", 0.0),
      ...     mulder.Layer(-100.0)
      ... )

      >>> geometry = mulder.EarthGeometry(
      ...     ("dem.tif", 0.0),
      ...     -100.0
      ... )

   .. rubric:: Methods
     :heading-level: 4

   .. automethod:: locate

      This method uses the `Position interface <States interface_>`_ to specify
      the coordinates of the point(s) to locate. It returns the corresponding
      layer indices. For instance,

      >>> layer = geometry.locate(latitude=45, altitude=-5.0)

   .. automethod:: scan
   .. automethod:: trace

   .. rubric:: Attributes
     :heading-level: 4

   .. autoattribute:: layers

      The first layer (of index :python:`0`) is the bottom strate, while the
      last layer is the top-most strate. The latter can be accessed as

      >>> top = geometry.layers[-1]

   .. autoattribute:: zlim

----

.. autoclass:: mulder.ExternalGeometry

   .. method:: __new__(*args, **kwargs)

----

.. autoclass:: mulder.Grid

   .. method:: __new__(data, /, *, x=None, y=None, projection=None)
   .. method:: __call__(xy, y=None, /, *, notify=None)

      Computes the altitude value at grid point(s).

   .. method:: __add__(value, /)
   .. method:: __radd__(value, /)
   .. method:: __rsub__(value, /)
   .. method:: __sub__(value, /)

   .. automethod:: gradient

   .. autoattribute:: projection
   .. autoattribute:: xlim
   .. autoattribute:: ylim
   .. autoattribute:: zlim

----

.. autoclass:: mulder.Layer

   .. method:: __new__(*data, density=None, material=None)
   .. method:: __getitem__(key, /)

   .. automethod:: altitude

   .. autoattribute:: data
   .. autoattribute:: density
   .. autoattribute:: material
   .. autoattribute:: zlim

----

.. autoclass:: mulder.LocalFrame

   An Earth-local reference-frame.

   This class specifies a Local-Tangent-Plane (`LTP`_) reference frame on the
   Earth. Optionnaly, the frame can be inclined (w.r.t. the vertical) or
   declined (w.r.t. the geographic north).

   .. method:: __new__(position=None, /, *, declination=None, inclination=None, **kwargs)

      Creates a new Earth-local reference-frame.

      The *position* argument specifies the frame origin, using the `Position
      interface <States interface_>`_. For example, the following defines a
      local frame close to `Clermont-Ferrand`_, France.

      >>> frame = mulder.LocalFrame(latitude=45.8, longitude=3.1)

   .. automethod:: transform

      The quantity, *q*, is transformed from the *self* :py:class:`LocalFrame`
      to the *destination* one. The *mode* parameter specifies the nature of *q*
      (i.e., :python:`"point"` or :python:`"vector"`). For example, the
      following computes the coordinates, in :python:`frame1`, of the
      :math:`\vec{e}_x` basis vector of :python:`frame0`.

      .. doctest::
         :hide:

         >>> frame0 = frame
         >>> frame1 = mulder.LocalFrame()

      >>> ex = frame0.transform((1, 0, 0), destination=frame1, mode="vector")

   .. rubric:: Attributes
     :heading-level: 4

   .. note:: :py:class:`LocalFrame` instances are :underline:`immutable`.

   .. autoattribute:: altitude
   .. autoattribute:: declination
   .. autoattribute:: latitude
   .. autoattribute:: longitude
   .. autoattribute:: inclination


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

A Mulder state is a set of variables used to characterise a flux of muons.
Typically, a state specifies a view point (position and direction of
observation) together with the kinetic energy of the observed muons.

.. tip::

   State variables may be vectorised, e.g. to represent a collection of muons,
   or a spectrum. Mulder follows a straightforward broadcasting rule that
   requires variables to be either scalar or to share the same size, regardless
   of their array shape.

Mulder considers two different representations of a set of states,
:py:class:`~mulder.GeographicStates` and :py:class:`~mulder.LocalStates`,
differing by their coordinate system. :py:class:`~mulder.GeographicStates`
represent the position and direction of observation using geographic-like
variables (e.g. latitude, longitude), while :py:class:`~mulder.LocalStates` use
Cartesian coordinates w.r.t. a :py:class:`~mulder.LocalFrame`. The
correspondence between the two representations is outlined in
:numref:`tab-state-representations` below. Conversion methods (e.g.
:py:meth:`~mulder.LocalStates.to_geographic`,
:py:meth:`~mulder.GeographicStates.to_local`) can be used to transform between
the two representations.

.. _tab-state-representations:

.. list-table:: State variables.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Geographic representation
     - Local representation
   * - :py:attr:`~mulder.GeographicStates.latitude`, :py:attr:`~mulder.GeographicStates.longitude`, :py:attr:`~mulder.GeographicStates.altitude`
     - :py:attr:`~mulder.LocalStates.position`
   * - :py:attr:`~mulder.GeographicStates.azimuth`, :py:attr:`~mulder.GeographicStates.elevation`
     - :py:attr:`~mulder.LocalStates.direction`
   * - :py:attr:`~mulder.GeographicStates.energy`, :py:attr:`~mulder.GeographicStates.pid`, :py:attr:`~mulder.GeographicStates.weight`
     - :py:attr:`~mulder.LocalStates.energy`, :py:attr:`~mulder.LocalStates.pid`, :py:attr:`~mulder.LocalStates.weight`

.. note::

   The direction :underline:`of observation` variable(s) specifies the opposite
   of the muon propagation direction, in both the Geographic and Local
   representations.

.. note::

   The pid variable is optional. It categorises a state as a muon
   (:python:`pid = 13`) or as an anti-muon (:python:`pid = -13`). If ommitted,
   each state is regarded as a superposition of muons and anti-muons.

.. tip::

   State variables are stored internally as NumPy `structured arrays <Structured
   arrays_>`_ accessible via the :py:attr:`~mulder.GeographicStates.array`
   attribute. For the sake of convenience, shape related attributes
   (:py:attr:`~mulder.GeographicStates.ndim`,
   :py:attr:`~mulder.GeographicStates.shape`,
   :py:attr:`~mulder.GeographicStates.size`) are also forwarded.

States objects are used as input to Mulder functions, for instance as follows

.. doctest::
   :hide:

   >>> def some_state_function(states=None, /, *, frame=None, **kwargs):
   ...     pass

>>> states = mulder.GeographicStates(
...     latitude = 45.0,
...     energy = np.geomspace(1E-02, 1E+04, 61)
... )
>>> result = some_state_function(states)

Alternatively, state variables can be provided directly as named arguments. For
instance, the following syntax produces the same *result* as the previous
example.

>>> result = some_state_function(
...     latitude = 45.0,
...     energy = np.geomspace(1E-02, 1E+04, 61)
... )

Some Mulder functions use only a subset of state variables, thus defining
sub-interfaces. Functions that require only position (position and direction)
variables are said to follow the :underline:`Position interface`
(:underline:`Coordinates interface`). These functions will also accept states
objects as positional arguments, but only position (position and direction)
variables as named arguments.


.. autoclass:: mulder.GeographicStates

   .. method:: __new__(states=None, /, **kwargs)

      Creates state(s) using geographic coordinates.

      This class method uses the `States interface`_. For instance,

      >>> states = mulder.GeographicStates(
      ...     latitude = 45,
      ...     energy = np.geomspace(1E-02, 1E+04, 61)
      ... )

   .. rubric:: Coordinates methods
     :heading-level: 4

   .. automethod:: from_local(states, /)
   .. automethod:: to_local

   .. rubric:: Array methods
     :heading-level: 4

   .. note::

      Depending on the *tagged* argument, the array methods described below
      return tagged muon or anti-muon states, or untagged ones (i.e. a
      superposition of muons and anti-muons).

   .. automethod:: dtype(*, tagged=False)
   .. automethod:: empty(shape=None, /, *, tagged=False)
   .. automethod:: full
   .. automethod:: from_array(array, /, *, copy=True)

         The input NumPy *array* must be of :py:class:`GeographicStates.dtype
         <mulder.GeographicStates.dtype>`. If *copy* is :python:`False`, the
         returned :py:class:`~mulder.GeographicStates` object refers to the
         input *array*.

   .. automethod:: zeros(shape=None, /, *, tagged=False)

   .. rubric:: Coordinates attributes
     :heading-level: 4

   .. note::

      The direction :underline:`of observation` is the opposite of the muon
      propagation direction.

   .. note::

      The :py:attr:`~mulder.GeographicStates.azimuth` and
      :py:class:`~mulder.GeographicStates.elevation` angles refer to
      :py:class:`~mulder.LocalFrame`\ s, the origins of which are defined by the
      :py:attr:`~mulder.GeographicStates.latitude`,
      :py:attr:`~mulder.GeographicStates.longitude` and
      :py:attr:`~mulder.GeographicStates.altitude` attributes.

   .. autoattribute:: altitude
   .. autoattribute:: azimuth
   .. autoattribute:: elevation
   .. autoattribute:: latitude
   .. autoattribute:: longitude

   .. rubric:: Common state attributes
     :heading-level: 4

   .. autoattribute:: energy
   .. autoattribute:: pid

      For untagged states this attribute is immutably :python:`None`.

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

      Creates state(s) using local coordinates.

      This class method uses the `States interface`_. For instance,

      >>> states = mulder.LocalStates(
      ...     direction = (0, 0, 1),
      ...     energy = np.geomspace(1E-02, 1E+04, 61)
      ... )

   .. rubric:: Coordinates methods
     :heading-level: 4

   .. automethod:: from_geographic
   .. automethod:: to_geographic
   .. automethod:: transform

   .. rubric:: Array methods
     :heading-level: 4

   .. note::

      Depending on the *tagged* argument, the array methods described below
      return tagged muon or anti-muon states, or untagged ones (i.e. a
      superposition of muons and anti-muons).

   .. automethod:: dtype(*, tagged=False)
   .. automethod:: empty(shape=None, /, *, tagged=False)
   .. automethod:: full
   .. automethod:: from_array(array, /, *, copy=True, frame=None)

      The input NumPy *array* must be of :py:class:`LocalStates.dtype
      <mulder.LocalStates.dtype>`. If *copy* is :python:`False`, the returned
      :py:class:`~mulder.LocalStates` object refers to the input *array*.

   .. automethod:: zeros(shape=None, /, *, tagged=False)

   .. rubric:: Coordinates attributes
     :heading-level: 4
   .. autoattribute:: frame
   .. autoattribute:: position
   .. autoattribute:: direction

   .. note::

      The direction :underline:`of observation` is the opposite of the muon
      propagation direction.

   .. rubric:: Common state attributes
     :heading-level: 4

   .. autoattribute:: energy
   .. autoattribute:: pid

      For untagged states this attribute is immutably :python:`None`.

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
   assumed to be homogeneous in composition, but with a density that varies
   vertically.

   .. method:: __new__(model=None, /, *, material=None)

      Creates a new atmospheric medium.

      The *model* argument specifies the vertical density profile, which is
      provided as an :math:`N \times 2` array mapped as :math:`[(z_0, \rho_0),
      \ldots, (z_{N-1}, \rho_{N-1})]` with altitudes (:math:`z`) in meters and
      densities (:math:`\rho`) in :math:`\mathrm{kg} / \mathrm{m}^3`. For
      instance,

      >>> atmosphere = mulder.Atmosphere((
      ...     (     0, 1.225E+00),
      ...     ( 1_000, 4.135E-01),
      ...     (30_000, 1.841E-02),
      ...     (70_000, 8.283E-05),
      ... ))

      .. note::

         The provided altitude values (:math:`z`) should be strictly increasing,
         and the density values (:math:`\rho`) must be strictly positive.

      Alternatively, a predefined model can be specified, e.g. as

      >>> atmosphere = mulder.Atmosphere("midlatitude-summer")

      See the :attr:`models <mulder.Atmosphere.models>` class attribute for a
      list of predefined density models.

      By default, the atmosphere is composed of :python:`"Air"`. This can be
      overridden using the optional *material* argument, for examples as follows

      >>> atmosphere = mulder.Atmosphere(material="SaturatedAir")

      See the `Materials interface`_ for information on defining custom
      materials.

   .. automethod:: density

      This method is vectorised. It can accomodate a scalar *altitude* input or
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

   A snapshot of the geomagnetic field.

   This class provides an interface to a geomagnetic model, parametrised by
   spherical harmonics. The default model used by Mulder is `IGRF14`_.

   .. method:: __new__(model=None, /,  *, date=None)

      Creates a new snapshot of the geomagnetic field.

      If provided, the *model* argument should point to a :bash:`*.COF` file
      containing the geomagnetic model coefficients.

      The optional *date* argument allows the user to specify the date of the
      snapshot, as a :py:class:`datetime.date` object, or as an `ISO 8601
      <ISO_8601_>`_-formatted string. For instance,

      >>> from datetime import date
      >>> magnet = mulder.EarthMagnet(date=date.today())

      or

      >>> magnet = mulder.EarthMagnet(date="1978-08-16")

   .. automethod:: field

      This method uses the `Position interface <States interface_>`_ for
      specifying the position(s) of interest. For instance, using geographic
      coordinates

      >>> field = magnet.field(latitude=45, longitude=3)

      The returned field is expressed in Tesla (T) units, with the coordinates
      frame depending on the input position. For geographic positions, `ENU
      <LTP_>`_ coordinates are returned. For local positions, the field is
      returned in the local frame of the input positions.

   .. rubric:: Attributes
     :heading-level: 4

   .. note:: :py:class:`EarthMagnet` instances are :underline:`immutable`.

   .. autoattribute:: date
   .. autoattribute:: model
   .. autoattribute:: zlim

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


.. URL links.
.. _Clermont-Ferrand: https://en.wikipedia.org/wiki/Clermont-Ferrand
.. _DEM: https://en.wikipedia.org/wiki/Digital_elevation_model
.. _IGRF14: https://doi.org/10.1186/s40623-020-01288-x
.. _LTP: https://en.wikipedia.org/wiki/Local_tangent_plane_coordinates
.. _ISO_8601: https://en.wikipedia.org/wiki/ISO_8601
.. _Structured arrays: https://numpy.org/doc/stable/user/basics.rec.html
