Python interface
================

This section describes the Python user interface of Mulder. The user interface
is organised in five topics, `Geometry <Geometry interface_>`_, `Materials
<Materials interface_>`_, `States <States interface_>`_, `Simulation <Simulation
interface_>`_ and `Pictures <Picture interface_>`_, as described below.
Moreover, Mulder exhibits some package-level data, as outlined in the
`Configuration <Configuration data_>`_ section.


Geometry interface
~~~~~~~~~~~~~~~~~~

.. autoclass:: mulder.EarthGeometry

   This class represents a stratified section of the Earth. The strates (or
   :py:class:`~mulder.Layer`\ s) form distinct propagation media that are
   assumed to be uniform in composition and density. They are delimited
   vertically, typically by a :py:class:`Grid` of elevation values, forming a
   Digital Elevation Model (`DEM`_).

   .. method:: __new__(*layers)

      Creates a new Earth geometry.

      The *layers* are provided in index order, i.e. the first layer has index
      :python:`0` and is thus the bottom strate. Each individual layer argument
      may be either an explicit :py:class:`~mulder.Layer` object, or data-like
      objects coercing to the latter. For instance, the following two syntaxes
      lead to the same geometry.

      >>> geometry = mulder.EarthGeometry(
      ...     mulder.Layer("dem.asc", 0.0),
      ...     mulder.Layer(-100.0)
      ... )

      >>> geometry = mulder.EarthGeometry(
      ...     ("dem.asc", 0.0),
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

   .. note::

      :py:class:`~mulder.EarthGeometry` objects are immutable, i.e. their
      structure cannot be modified. However, the
      :py:attr:`~mulder.Layer.density` and :py:attr:`~mulder.Layer.material` of
      :py:attr:`~mulder.EarthGeometry.layers` is mutable.

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

   This class represents a parametric surface, :math:`z = f(x, y)`, described by
   a regularly spaced :py:class:`Grid` of elevation values, :math:`z_{ij} =
   f(x_j, y_i)`, forming a Digital Elevation Model (`DEM`_).

   The elevation values, :math:`z_{ij}`, of a :py:class:`Grid` object may be
   offset by a constant value using the :python:`+` and :python:`-` operators.
   For example, the following will create a new grid offset by :python:`100.0`
   metres w.r.t. the initial one.

   .. doctest:
      :hide:

      >>> initial_grid = mulder.Grid("dem.asc")

   >>> new_grid = initial_grid + 100.0

   .. tip::

      The offsetting of a grid creates a reference to the data of the initial
      grid, i.e. data is not duplicated.

   .. method:: __new__(data, /, *, xlim=None, ylim=None, crs=None)

      Creates a new grid.

      The *data* argument may refer to:

      * A file containing a `DEM`_ (see :numref:`tab-dem-formats` for supported
        file formats).
      * A folder containing the tiles of a Global Digital Elevation Model
        (GDEM), such as `SRTMGL1.003`_.
      * A 2D array containing the :math:`z_{ij}` values in row-major order.

      In the latter case, the *xlim* and *ylim* arguments must specify the DEM
      limits along the :math:`x` and :math:`y`-axes.

      Depending on the *data* argument, a Coordinate Reference System (`CRS`_)
      may be specified (by providing its `EPSG`_ code). See :numref:`tab-crs`
      for a list of supported *crs* values. By default, the WGS84 / GPS system
      is assumed.

      For instance, the following loads elevation data stored in `ASCII Grid`_
      format using UTM 31N coordinates, i.e. EPSG:32631.

      >>> grid = mulder.Grid("dem.asc", crs=32631)

      .. _tab-dem-formats:

      .. list-table:: Supported file formats.
         :width: 75%
         :widths: auto
         :header-rows: 1

         * - Description
           - Extension
         * - `ASCII Grid`_
           - :bash:`.asc`
         * - `GeoTIFF`_
           - :bash:`.tif`
         * - `EGM96 Grid`_
           - :bash:`.grd`
         * - `HGT`_
           - :bash:`.hgt`

      .. _tab-crs:

      .. list-table:: Supported Coordinate Reference Systems.
         :width: 75%
         :widths: auto
         :header-rows: 1

         * - Description
           - EPSG code(s)
         * - NTF / Lambert I-IV
           - 27571-27574
         * - RGF93 / Lambert 93
           - 2154
         * - WGS84 / GPS
           - 4326
         * - WGS84 / UTM 1-60 N
           - 32601-32660
         * - WGS84 / UTM 1-60 S
           - 32701-32760


   .. rubric:: Methods
     :heading-level: 4

   .. automethod:: gradient

      This method returns the gradient w.r.t. the :math:`x` and :math:`y`
      coordinates. The interface is the same as the :py:meth:`Grid.z` method.
      Please refer to the latter for a description of the arguments.

   .. automethod:: z

      This method is vectorised. It accepts either a sequence of :math:`(x_k,
      y_k)` values as the first argument, or two sequences of :math:`x_j` and
      :math:`y_i` values as the first and second arguments. In the latter case,
      the method returns the :math:`z_{ij}` values corresponding to the outer
      product :math:`(x_j, y_i)`. For instance, the following returns a 2D array
      of elevation values, *z*, with shape :python:`(41, 21)`.

      >>> x, y = np.linspace(-1, 1, 21), np.linspace(-2, 2, 41)
      >>> z = grid.z(x, y)

   .. doctest:
      :hide:

      >>> assert z.shape == (41, 21)

   .. rubric:: Attributes
     :heading-level: 4

   .. note:: :py:class:`Grid` instances are :underline:`immutable`.

   .. autoattribute:: crs

      The grid Coordinate Reference System (`CRS`_) is encoded according to the
      `EPSG`_ standard. For example,

      >>> grid.crs
      32631

   .. autoattribute:: xlim
   .. autoattribute:: ylim
   .. autoattribute:: zlim

----

.. autoclass:: mulder.Layer

   This class represents a layer (or strate) of an
   :py:class:`~mulder.EarthGeometry`, considered to be uniform in composition
   and density. A layer is delimited by a top surface, typically described by
   one or more :py:class:`Grids <mulder.Grid>` of elevation values. The bottom
   of a layer is determined by the top of its underlying layer within the
   :py:class:`~mulder.EarthGeometry`.

   .. method:: __new__(*data, density=None, description=None, material=None)

      Creates a new layer.

      The *data* argument determines the top of the layer. It must be akin to a
      :py:class:`~mulder.Grid` object. Alternatively, a :py:class:`float` value
      can be provided to specify a flat topography. Multiple *data* can be
      provided to specify successive fallback models. For example, the following
      creates a new layer whose top surface is defined by two data sets, as

      >>> layer = mulder.Layer("dem.asc", 0.0)

      The corresponding top surface matches the Digital Elevation Model (`DEM`_)
      from the file :bash:`dem.asc` within its domain of definition, but falls
      back to a constant elevation value of :python:`0` outside this domain.

      See the layer attributes below for the meaning of the optional
      :py:attr:`~mulder.Layer.density`, :py:attr:`~mulder.Layer.description` and
      :py:attr:`~mulder.Layer.material` arguments.


   .. rubric:: Methods
     :heading-level: 4

   .. automethod:: altitude

      This method is vectorised. It accepts either a sequence of :math:`(\phi_k,
      \lambda_k)` values as the first argument, where :math:`\phi` denotes the
      latitude and :math:`\lambda` the longitude, or two sequences of
      :math:`\phi_i` and :math:`\lambda_j` values as the first and second
      arguments. In the latter case, the method returns the :math:`z_{ij}`
      values corresponding to the outer product :math:`(\lambda_j, \phi_i)`. For
      instance, the following returns a 2D array of altitude values with shape
      :python:`(181, 361)`.

      >>> lat, lon = np.linspace(-90, 90, 181), np.linspace(-180, 180, 361)
      >>> altitudes = layer.altitude(lat, lon)

   .. doctest:
      :hide:

      >>> assert altitudes.shape == (181, 361)

   .. automethod:: normal

      This method returns the normal to the top surface at the latitude
      (:math:`\phi`) and longitude (:math:`\lambda`) coordinates. The interface
      is the same as the :py:meth:`Layer.altitude` method. Please refer to the
      latter for a description of the arguments.

      The optional *frame* argument specifies the coordinates system (as a
      :py:class:`~mulder.LocalFrame`) in which the normal should be expressed.
      If this argument is omitted, geocentric (`ECEF`_) coordinates will be used
      instead.

   .. rubric:: Attributes
     :heading-level: 4

   .. autoattribute:: data

      .. note:: This attribute is immutable.

   .. autoattribute:: density

      The bulk density is expressed in :math:`\mathrm{kg}/\mathrm{m}^3`. If
      :python:`None`, then the material default density is assumed.

   .. autoattribute:: description

   .. autoattribute:: material

      This attribute is the name of the material. For instance, the following
      changes the layer material to water.

      >>> layer.material = "Water"

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


----

.. autoclass:: mulder.Medium

   .. autoattribute:: density
   .. autoattribute:: description
   .. autoattribute:: material


Materials interface
~~~~~~~~~~~~~~~~~~~

Mulder makes a distinction between base :py:class:`Materials
<mulder.materials.Material>` and :py:class:`~mulder.materials.Composite`
materials. A base :py:class:`~mulder.materials.Material` is a microscopic
mixture of atomic :py:class:`Elements <mulder.materials.Element>`. A
:py:class:`~mulder.materials.Composite` material, by contrast, is a macroscopic
mixture of base :py:class:`Materials <mulder.materials.Material>`, typically a
rock composed of various minerals.

.. note::

   The stopping-power of a :py:class:`~mulder.materials.Composite` material
   differs from that of a base :py:class:`~mulder.materials.Material` with the
   same composition, due to the density effect in ionisation loss.

.. important::

   Materials (atomic elements) are defined at the global scope. They are
   uniquely identified by their name (atomic symbol). It is not possible to
   modify or remove a material (atomic element) within a given Python instance.

.. autoclass:: mulder.materials.CompiledMaterial

----

.. autoclass:: mulder.materials.Composite

   This class represents a macroscopic mixture of :py:class:`Materials
   <Material>`. :py:class:`Composite` objects use the mapping protocol to expose
   their components' mass fractions, which are mutable. For example

   >>> composite["Water"] = 0.1  # doctest: +SKIP

   It is not possible to add or remove a constitutive material once the
   composite has been defined. However, a specific material may be disabled by
   setting its mass fraction to zero.

   .. method:: __new__(name, /, **kwargs)

      Gets or defines a composite material.

      Without *kwargs*, this constructor simply returns the definition of the
      composite matching *name*. If the composite does not exists, it can be
      defined by specifying its composition as *kwargs*. For instance,

      >>> humid_rock = materials.Composite(
      ...     "HumidRock",
      ...     composition=("Rock", "Water"),
      ... )

      The mass fractions may be specified when defining the composite, for
      instance as

      >>> humid_rock = materials.Composite(
      ...     "HumidRock",
      ...     composition={"Rock": 0.95, "Water": 0.05},
      ... )

   .. method:: all()

      Returns all currently defined composites.

      The composites are returned as a :py:class:`dict` object mapping names to
      definitions.

   .. rubric:: Attributes
     :heading-level: 4

   .. note:: :py:class:`Composite` instances are :underline:`immutable` appart
      from their components' mass fractions.

   .. autoattribute:: composition

      The composite content is returned as a :py:class:`tuple`. For example

      >>> humid_rock.composition
      ('Rock', 'Water')

      The corresponding mass fractions may be accessed using the mapping
      protocol, as

      >>> humid_rock["Water"]
      0.05

   .. autoattribute:: density

      .. note::

         The composite density depends on the mass fractions of its constitutive
         materials.

----

.. autoclass:: mulder.materials.Element

   This class serves as a proxy for the definition of an atomic element, which
   may represent a specific isotope or a mixture of isotopes.

   .. tip::

      Mulder predefines atomic elements from :python:`H`, :python:`D`
      (:math:`Z=1`) to :python:`Og` (:math:`Z=118`) according to the `PDG`_.
      Furthermore, Mulder defines a fictitious :python:`Rk` (:math:`Z=11, A=22`)
      element to represent `standard rock`_.

   .. method:: __new__(symbol, /, **kwargs)

      Gets or defines an atomic element.

      Without *kwargs*, this constructor simply returns the definition of the
      atomic element matching *symbol*. For instance,

      >>> H = materials.Element("H")

      If the element does not exists, it can be defined by specifying its
      properties as *kwargs*. For instance,

      >>> U_238 = materials.Element("U-238", Z=92, A=238.0508, I=890E-09)

   .. method:: all()

      Returns all currently defined elements.

      The elements are returned as a :py:class:`dict` object mapping the atomic
      elements symbols to their definitions.

   .. rubric:: Attributes
     :heading-level: 4

   .. note:: :py:class:`Element` instances are :underline:`immutable`.

   .. autoattribute:: A
   .. autoattribute:: I
   .. autoattribute:: Z


----

.. autoclass:: mulder.materials.Material

   This class represents a homogeneous material, at the microscopic scale. A
   :py:class:`Material` object may be composed of a single atomic
   :py:class:`Element` (e.g., C), a molecule (e.g., H2O) or be a mixture (e.g.,
   air). In addition to the atomic :py:attr:`composition`, the material
   structure is essentially summarised by its :py:attr:`density` and its mean
   excitation energy (:py:attr:`I`).

   .. tip::

      Mulder predefines the :python:`Air`, :python:`Rock` and :python:`Water`
      materials.

   .. method:: __new__(name, /, **kwargs)

      Gets or defines a material.

      Without *kwargs*, this constructor simply returns the definition of the
      material matching *name*. For instance,

      >>> rock = materials.Material("Rock")

      If the material does not exists, it can be defined by specifying its
      properties as *kwargs*. For instance,

      >>> ice = materials.Material("Ice", composition="H2O", density=0.92E+03)

      The *composition* argument may be a :py:class:`str`, specifying the
      material chemical composition, or akin to a :py:class:`dict` mapping
      atomic elements or other materials to mass fractions. For example, as

      >>> moist_air = materials.Material(
      ...     "MoistAir",
      ...     composition={"Air": 0.99, "Water": 0.01},
      ...     density=1.2
      ... )

   .. method:: all()

      Returns all currently defined materials.

      The materials are returned as a :py:class:`dict` object mapping names to
      definitions.

   .. rubric:: Attributes
     :heading-level: 4

   .. note:: :py:class:`Material` instances are :underline:`immutable`.

   .. autoattribute:: composition

      The atomic mass composition is returned as a :py:class:`tuple`. For
      example

      >>> moist_air.composition
      (('Ar', 0.0126987...), ..., ('O', 0.2383443...))

   .. autoattribute:: density

   .. autoattribute:: I

      If :python:`None` then the mean excitation energy is computed from the
      material's atomic content assuming Bragg additivity [BrKl05]_.

----

.. autofunction:: mulder.materials.dump

   If the *materials* arguments are ommited, then the current material
   definitions are dumped to a `TOML`_ file, for instance as

   >>> materials.dump("materials.toml")

   Alternatively, one may explicit the material definitions to dump, for example
   as

   >>> materials.dump("materials.toml", "Ice", "MoistAir", "HumidRock")

----

.. autofunction:: mulder.materials.load

   The definition file must be in `TOML`_ format. For example, the following
   :bash:`materials.toml` file defines two :py:class:`Materials <Material>`
   (:python:`Ice` and :python:`MoistAir`) and one :py:class:`Composite`
   (:python:`HumidRock`).

   .. literalinclude:: include/materials.toml
      :language: toml

   The corresponding material definitions are loaded as

   >>> materials.load("materials.toml")


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


Configuration data
~~~~~~~~~~~~~~~~~~

Configuration data can be accessed via the :python:`mulder.config` singleton
class. For instance, as

>>> mulder.config.VERSION
'0.2.2'

The available configuration data are listed below. Apart from :py:data:`NOTIFY`,
these data are immutable.

.. data:: DEFAULT_CACHE
   :type: ~pathlib.Path

   Default cache path.

   Mulder uses a caching system to reduce the time taken for some
   time-consuming, yet repetitive tasks, such as the compilation of
   :py:class:`~mulder.Physics` tables. This data indicates the default location
   of cached files.

   .. note::

      To change the location of cached files set the :bash:`MULDER_CACHE`
      environment variable to the desired value.

.. data:: NOTIFY
   :type: bool

   Default status for notifications.

   For operations that are potentially time-consuming, Mulder reports its
   progress to the terminal using a progress bar. By setting this flag to
   :python:`False`, such reports will be disabled, unless individual function
   calls explicitly set their *notify* argument to :python:`True`.

.. data:: PREFIX
   :type: ~pathlib.Path

   The package installation prefix.

.. data:: VERSION
   :type: str

   The package version.


.. URL links.
.. _ASCII Grid: https://en.wikipedia.org/wiki/Esri_grid
.. _Clermont-Ferrand: https://en.wikipedia.org/wiki/Clermont-Ferrand
.. _CRS: https://en.wikipedia.org/wiki/Spatial_reference_system
.. _DEM: https://en.wikipedia.org/wiki/Digital_elevation_model
.. _ECEF: https://en.wikipedia.org/wiki/Earth-centered,_Earth-fixed_coordinate_system
.. _EGM96 Grid: https://web.archive.org/web/20130218141358/http://earth-info.nga.mil/GandG/wgs84/gravitymod/egm96/egm96.html
.. _EPSG: https://epsg.io/
.. _GeoTIFF: https://fr.wikipedia.org/wiki/GeoTIFF
.. _HGT: http://fileformats.archiveteam.org/wiki/HGT
.. _IGRF14: https://doi.org/10.1186/s40623-020-01288-x
.. _LTP: https://en.wikipedia.org/wiki/Local_tangent_plane_coordinates
.. _ISO_8601: https://en.wikipedia.org/wiki/ISO_8601
.. _PDG: https://pdg.lbl.gov/2025/AtomicNuclearProperties/index.html
.. _SRTMGL1.003: https://doi.org/10.5067/MEASURES/SRTM/SRTMGL1.003
.. _Standard Rock: https://pdg.lbl.gov/2025/AtomicNuclearProperties/standardrock.html
.. _Structured arrays: https://numpy.org/doc/stable/user/basics.rec.html
.. _TOML: https://toml.io
