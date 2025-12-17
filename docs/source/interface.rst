Python interface
================

This section describes the Python user interface of Mulder. The user interface
is organised in sub-topics, `Geometry <Geometry interface_>`_, `Materials
<Materials interface_>`_, `Modules <Module interface_>`_, `Physics <Physics
interface_>`_, `States <States interface_>`_, `Simulation <Simulation
interface_>`_ and `Pictures <Picture interface_>`_, as described below.
Moreover, Mulder exhibits some package-level data, as outlined in the
`Configuration <Configuration data_>`_ section.


Geometry interface
~~~~~~~~~~~~~~~~~~

To represent an Earth-like geometry, typically described by a Digital Elevation
Model (`DEM`_), Mulder provides a dedicated :py:class:`~mulder.EarthGeometry`
object. Alternatively, a `Calzone`_ geometry might be imported as a
:py:class:`~mulder.LocalGeometry` object. Other use cases might be implemented
as an external :py:class:`~mulder.Module` (using Mulder's C interface). Mulder
geometry objects rely on a common model, inherited from `Pumas`_ [Nie22]_,
which is discussed in the :doc:`Geometry <geometry>` section.


.. autoclass:: mulder.EarthGeometry

   This class represents a stratified section of the Earth. The strates (or
   :py:class:`Layers <mulder.Layer>`) form distinct propagation media that are
   assumed to be uniform in composition and density. They are delimited
   vertically, typically by a :py:class:`Grid` of elevation values, forming a
   Digital Elevation Model (`DEM`_).

   .. note::

      Mulder uses `Turtle's <Turtle_>`_ algorithm [NBCM20]_ to efficiently
      navigate through :py:class:`EarthGeometry` objects.

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

   .. rubric:: Geometry methods
     :heading-level: 4

   .. note::

      The geometry methods below use the `Coordinates interface <States
      interface_>`_ to specify the coordinates of interest.

   .. automethod:: locate

      The method returns the layer index(es) that correspond to the input
      position(s). For instance,

      >>> layer = geometry.locate(latitude=45, altitude=-5.0)

   .. automethod:: scan

      The method returns an :py:class:`array <numpy.ndarray>` containing the
      thicknesses of the layers along the line(s) of sight specified by the
      input *coordinates*. For instance,

      >>> thickness = geometry.scan(latitude=45, elevation=10)
      >>> thickness[0]  # doctest: +SKIP
      3.0

   .. automethod:: trace

      The method returns a structured :py:class:`array <numpy.ndarray>`
      describing the first intersection(s) along the line(s) of sight specified
      by the input *coordinates*. For instance,

      >>> intersection = geometry.trace(latitude=45, elevation=10)
      >>> intersection["distance"]  # doctest: +SKIP
      3.0

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

.. autoclass:: mulder.Grid

   This class represents a parametric surface, :math:`z = f(x, y)`, described by
   a regularly spaced :py:class:`Grid` of elevation values, :math:`z_{ij} =
   f(x_j, y_i)`, forming a Digital Elevation Model (`DEM`_).

   The elevation values, :math:`z_{ij}`, of a :py:class:`Grid` object may be
   offset by a constant value using the :python:`+` and :python:`-` operators.
   For example, the following will create a new grid offset by :python:`100.0`
   metres w.r.t. the initial one.

   .. doctest::
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

   .. doctest::
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

   .. doctest::
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

.. autoclass:: mulder.LocalGeometry

   This class represents a local geometry on the Earth, w.r.t. a
   :py:class:`~mulder.LocalFrame`. Local geometries can be created by importing
   a `Calzone geometry <Calzone-Geometry_>`_. Alternatively, they might be
   implemented in C within a :py:class:`mulder.Module`.

   .. method:: __new__(data, /, *, frame=None)

      The *data* argument may be a path-string pointing to a
      :py:class:`~mulder.Module` file or to a `Calzone geometry
      <Calzone-Geometry_>`_ file. Alternatively, one might provide a
      :py:class:`calzone.Geometry` object as *data* argument. For instance, the
      following loads a local geometry from a `Calzone geometry
      <Calzone-Geometry_>`_ file.

      >>> geometry = mulder.LocalGeometry("geometry.toml")

      The optional *frame* argument specifies the origin and orientation of the
      local geometry as a :py:class:`~mulder.LocalFrame` object.

   .. rubric:: Geometry methods
     :heading-level: 4

   .. automethod:: locate

      The method returns the layer index(es) that correspond to the input
      position(s). For instance,

      >>> layer = geometry.locate(position=[0, 0, 1])

   .. automethod:: scan

      The method returns an :py:class:`array <numpy.ndarray>` containing the
      thicknesses of the layers along the line(s) of sight specified by the
      input *coordinates*. For instance,

      >>> thickness = geometry.scan(position=[0, 0, 1], direction=[0, 0, -1])
      >>> thickness[0]  # doctest: +SKIP
      1.0

   .. automethod:: trace

      The method returns a structured :py:class:`array <numpy.ndarray>`
      describing the first intersection(s) along the line(s) of sight specified
      by the input *coordinates*. For instance,

      >>> intersection = geometry.trace(position=[0, 0, 1], direction=[0, 0, -1])
      >>> intersection["distance"]  # doctest: +SKIP
      1.0

   .. rubric:: Attributes
     :heading-level: 4

   .. autoattribute:: frame

      The geometry reference frame is mutable. For instance,

      >>> geometry.frame = mulder.LocalFrame(altitude=10.0)

   .. autoattribute:: media

      The geometry media form an **immutable** sequence. A medium is identified
      by its index within this sequence. For instance, the following returns the
      first medium

      >>> first = geometry.media[0]

----

.. autoclass:: mulder.Medium

   This class represents a medium of a :py:class:`~mulder.LocalGeometry`,
   considered to be uniform in composition and density.

   .. autoattribute:: density

      The bulk density is expressed in :math:`\mathrm{kg}/\mathrm{m}^3`. If
      :python:`None`, then the material default density is assumed.

   .. autoattribute:: description

   .. autoattribute:: material

      This attribute is the name of the material. For instance, the following
      changes the medium material to water.

      >>> medium.material = "Water"  # doctest: +SKIP

.. _sec-materials-interface:

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
   slightly differs from that of a base :py:class:`~mulder.materials.Material`
   with the same composition, due to the density effect in ionisation loss.

A material (atomic element) is uniquely identified by its name (atomic symbol),
which maps to a concrete definition, e.g. a
:py:class:`~mulder.materials.Material` (:py:class:`~mulder.materials.Element`).
The mapping may be lazy, i.e. delayed until usage. Once a material (element)
definition has been established, it cannot be modified or removed.

.. note::

   The resolution of unmapped materials (elements) is done in the following
   order.

   1. Firstly, the material name (element symbol) is searched for in
      :py:class:`Modules <mulder.Module>`, in order of loading.

   2. Secondly, Mulder's companion Python package(s) are inspected, e.g.
      `Calzone`_.

   3. Finally, Mulder default definitions are checked.


.. autoclass:: mulder.materials.Composite

   This class represents a macroscopic mixture of :py:class:`Materials
   <Material>`. :py:class:`Composite` objects use the mapping protocol to expose
   their components' mass fractions, which are mutable. For example

   >>> composite["Water"] = 0.1  # doctest: +SKIP

   It is not possible to add or remove a constitutive material once the
   composite has been mapped. However, a specific material may be disabled by
   setting its mass fraction to zero.

   .. method:: __new__(name, /, **kwargs)

      Gets a composite definition.

      Returns the definition of the composite matching *name*. For instance,

      >>> composite = materials.Composite("HumidRock")  # doctest: +SKIP

   .. method:: all()

      Returns all currently mapped composites.

      The composites are returned as a :py:class:`dict` object mapping names to
      definitions.

   .. method:: define(name, /, *, composition)

      Defines a composite material.

      .. note::

         This method explictly maps the material *name* to the provided
         composite definition.

      The composite is defined by specifying its *composition*, for instance as,

      >>> humid_rock = materials.Composite.define(
      ...     "HumidRock",
      ...     composition=("Rock", "Water"),
      ... )

      The mass fractions may also be specified when defining the composite, for
      instance as

      >>> humid_rock = materials.Composite.define(
      ...     "HumidRock",
      ...     composition={"Rock": 0.95, "Water": 0.05},
      ... )

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

   This class represents an atomic element, which may be a specific isotope or a
   mixture of isotopes.

   .. tip::

      Mulder provides default definitions for atomic elements from :python:`H`,
      :python:`D` (:math:`Z=1`) to :python:`Og` (:math:`Z=118`) according to the
      `PDG`_. Furthermore, Mulder provides a fictitious :python:`Rk`
      (:math:`Z=11, A=22`) element to represent `standard rock`_.

   .. method:: __new__(symbol, /, **kwargs)

      Gets an atomic element definition.

      Returns the definition of the atomic element matching *symbol*. For
      instance,

      >>> H = materials.Element("H")

   .. method:: all()

      Returns all currently mapped elements.

      The elements are returned as a :py:class:`dict` object mapping the atomic
      elements symbols to their definitions.

   .. method:: define(symbol, /, *, Z, A, I=None)

      Defines a new atomic element.

      .. note::

         This method explictly maps the element *symbol* to the provided
         element definition.

      If the Mean Excitation Energy (*I*) is omitted, a default value is
      used depending on *Z*. For example,

      >>> U_238 = materials.Element.define("U-238", Z=92, A=238.0508)

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

      Mulder provides default definitions for the :python:`Air`, :python:`Rock`
      and :python:`Water` materials.

   .. method:: __new__(name, /, **kwargs)

      Gets a material definition.

      Returns the definition of the material matching *name*. For instance,

      >>> rock = materials.Material("Rock")

   .. method:: all()

      Returns all currently mapped materials.

      The materials are returned as a :py:class:`dict` object mapping names to
      definitions.

   .. method:: define(name, /, *, composition, density, I=None)

      .. note::

         This method explictly maps the material *name* to the provided
         material definition.

      The *composition* argument may be a :py:class:`str`, specifying the
      material chemical composition, as

      >>> ice = materials.Material.define("Ice", composition="H2O", density=0.92E+03)

      Alternatively, the *composition* argument may be akin to a
      :py:class:`dict` mapping atomic elements or other materials to mass
      fractions. For example, as

      >>> moist_air = materials.Material.define(
      ...     "MoistAir",
      ...     composition={"Air": 0.99, "Water": 0.01},
      ...     density=1.2
      ... )

      See the material *attributes* below for a description of the *density* and
      *I* arguments.

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

   If the *materials* arguments are ommited, then all currently mapped material
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

Module interface
~~~~~~~~~~~~~~~~

.. autoclass:: mulder.Module

   .. method:: __new__(path, /)

      Loads a Module.

      >>> module = mulder.Module("module.so")  # doctest: +SKIP

   .. rubric:: Methods
     :heading-level: 4

   .. automethod:: element

      This method explictly maps the element *symbol* to its module definition.
      For example,

      >>> H = module.element("G4_H")  # doctest: +SKIP

   .. automethod:: geometry

      >>> geometry = module.geometry(frame=frame)  # doctest: +SKIP

   .. automethod:: material

      This method explictly maps the material *name* to its module definition.
      For example,

      >>> air = module.material("G4_AIR")  # doctest: +SKIP

   .. rubric:: Attributes
     :heading-level: 4

   .. autoattribute:: path
   .. autoattribute:: ptr


Physics interface
~~~~~~~~~~~~~~~~~

Mulder builds over the `Pumas`_ transport engine, the physics implementation of
which is described in detail in [Nie22]_. In order to improve performance, some
key physical properties are pre-computed and later interpolated at runtime, e.g.
cross-sections, stopping-powers, etc. This process can be triggered manually
using the :py:meth:`~mulder.Physics.compile` method of a
:py:class:`~mulder.Physics` instance, which translates a set of
:py:class:`~mulder.materials.Material` and
:py:class:`~mulder.materials.Composite` definitions into
:py:class:`CompiledMaterials <mulder.CompiledMaterial>`, providing access to the
tabulated physical properties.

.. note::

   Since their computation can be time consuming, material tables related to a
   specific set of materials and :py:class:`~mulder.Physics` settings are
   cached. See the :py:data:`DEFAULT_CACHE` entry for information on controlling
   the cache location.

.. tip::

   :py:class:`Fluxmeters <mulder.Fluxmeter>` seamlessly manage the generation of
   material tables, negating the need for any explicit compilation.

.. autoclass:: mulder.CompiledMaterial

   This class acts as a proxy for the material tables relating to a specific
   material. The physical properties can be accessed via vectorised class
   methods. For example as,

   .. doctest::
      :hide:

       >>> compiled_material = mulder.Physics().compile("Rock")

   >>> energy = np.geomspace(1E-02, 1E+03, 101)
   >>> stopping_power = compiled_material.stopping_power(energy)

   .. note::

      The :py:class:`CompiledMaterial` class cannot be instantiated directly; it
      must be generated using the :py:meth:`Physics.compile` method instead.

   .. rubric:: Methods
     :heading-level: 4

   .. automethod:: cross_section

      The macroscopic cross-section, expressed in :math:`\mathrm{m}^{-1}`, is
      restricted to hard collisions with a fractionnal energy loss larger than
      the physics :py:attr:`~mulder.Physics.cutoff`. Collisions with a smaller
      energy loss are included in the continuous energy loss given by the
      :py:meth:`stopping_power` method.

      .. note::

         The hard elastic collisions are not included in the macroscopic
         cross-section but in the elastic mean free path given by the
         :py:meth:`elastic_scattering` method.

   .. automethod:: elastic_scattering

      This method returns the mean free path, in metres, restricted to hard
      elastic collisions and the corresponding cutoff angle, in deg. The cutoff
      angle is expressed in the center of mass frame of the collision. It is set
      according to the physics :py:attr:`~Physics.elastic_ratio` following
      Fernandez-Varea et al. [FMBS93]_.

      .. note::

         Soft elastic collisions are taken into account in the multiple
         scattering (see the :py:meth:`transport_path` method).

   .. automethod:: range

      The CSDA range is expressed in metres. See the :py:meth:`stopping_power`
      method for the corresponding continuous energy loss.

      .. note::

         In :python:`mixed` or :python:`discrete` modes, the range does not
         include hard collisions.


   .. automethod:: stopping_power

      The material stopping-power is expressed in
      :math:`\mathrm{GeV}/\mathrm{m}`. See the :py:meth:`range` method for the
      corresponding CSDA range.

      .. note::

         In :python:`mixed` or :python:`discrete` modes, the stopping power does
         not include hard collisions.

   .. automethod:: transport_path

      The transport mean free path, expressed in metres, is restricted to soft
      collisions, including both elastic and inelastic processes.

      .. note::

         The transport m.f.p., :math:`\lambda`, is related to the standard
         deviation of the multiple scattering angle as :math:`\sigma_\theta^2 =
         s / (2 \lambda)`, where :math:`s` is the travelled distance.

   .. rubric:: Attributes
     :heading-level: 4

   .. autoattribute:: definition

      This attribute is an instance of :py:class:`~mulder.materials.Composite`
      or :py:class:`~mulder.materials.Material`, depending on the type of
      material.

   .. autoattribute:: name

----

.. autoclass:: mulder.Physics

   This class provides access to configurable `Pumas`_ settings relevant to muon
   transport physics, as mutable attributes. For further details, please refer
   to [Nie22]_. In addition, the :py:class:`Physics` class provides an interface
   for generating material tables from material definitions, using the
   :py:meth:`compile` method.

   .. method:: __new__(**kwargs)

      Creates a Physics context.

      Configuration settings can be provided as keyword arguments (*kwargs*).
      See the class attributes below for a list of possible parameters. For
      example,

      >>> physics = mulder.Physics(cutoff=5E-02)

   .. rubric:: Methods
     :heading-level: 4

   .. automethod:: compile

      If the *materials* arguments are ommited, then all currently defined
      materials are compiled. The returned :py:class:`CompiledMaterials
      <mulder.CompiledMaterial>` can be extracted to a :py:class:`dict`, for
      instance as

      >>> compiled = { m.name: m for m in physics.compile() } # doctest: +SKIP

      Alternatively, one may explicit the materials to compile, for example as

      >>> ice, rock = physics.compile("Ice", "Rock") # doctest: +SKIP

   .. rubric:: Attributes
     :heading-level: 4

   .. autoattribute:: bremsstrahlung

      The possible values for bremsstralung models are summarised in
      :numref:`tab-bremsstrahlung` below, the default setting is
      :python:`"SSR19"`.

      .. _tab-bremsstrahlung:

      .. list-table:: Available bremsstrahlung models.
         :width: 75%
         :widths: auto
         :header-rows: 1

         * - Model
           - Reference
         * - :python:`"ABB94"`
           - Andreev, Bezrukov and Bugaev, Physics of Atomic Nuclei 57 (1994)
             2066.
         * - :python:`"KKP95"`
           - Kelner, Kokoulin and Petrukhin, Moscow Engineering Physics Inst.,
             Moscow, 1995.
         * - :python:`"SSR19"`
           - `PROPOSAL`_\ 's implementation of [SSR19]_.

   .. autoattribute:: cutoff

      Relative cutoff between soft and hard energy losses. Setting a null or
      negative value results in the default cutoff value to be used i.e. 5%
      which is a good compromise between speed and accuracy for transporting a
      continuous muon spectrumm, see e.g. Sokalski et al. [SBK01]_.

      .. warning::

         Cutoff values lower than 1% are not supported.

   .. autoattribute:: elastic_ratio

      Ratio of the mean free path for hard elastic events to the smallest of the
      transport mean free path or CSDA range. The lower the ratio the more
      detailed the simulation of elastic scattering, see e.g. Fernandez-Varea et
      al. [FMBS93]_. Setting a null or negative value results in the default
      ratio to be used i.e. 5%.

   .. autoattribute:: pair_production

      The possible values for pair-production models are summarised in
      :numref:`tab-pair-production` below, the default setting is
      :python:`"SSR19"`.

      .. _tab-pair-production:

      .. list-table:: Available pair-production models.
         :width: 75%
         :widths: auto
         :header-rows: 1

         * - Model
           - Reference
         * - :python:`"KKP68"`
           - Kelner, Kokoulin and Petrukhin, Soviet Journal of Nuclear Physics 7
             (1968) 237.
         * - :python:`"SSR19"`
           - `PROPOSAL`_\ 's implementation of [SSR19]_.

   .. autoattribute:: photonuclear

      The possible values for photonuclear interaction models are summarised in
      :numref:`tab-photonuclear` below, the default setting is
      :python:`"DRSS01"`.

      .. _tab-photonuclear:

      .. list-table:: Available photonuclear interaction models.
         :width: 75%
         :widths: auto
         :header-rows: 1

         * - Model
           - Reference
         * - :python:`"BBKS03"`
           - Bezrukov, Bugaev, Sov. J. Nucl. Phys. 33 (1981), 635, with improved
             photon-nucleon cross-section according to `Kokoulin`_ and hard
             component from `Bugaev and Shlepin`_.
         * - :python:`"BM02"`
           - Butkevich and Mikheyev, Soviet Journal of Experimental and
             Theoretical Physics 95 (2002) 11.
         * - :python:`"DRSS01"`
           - Dutta, Reno, Sarcevic and Seckel, Phys.Rev. D63 (2001) 094020.

.. _sec-states-interface:

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
   .. automethod:: full(shape=None, /, fill_value=None, **kwargs)
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

   .. method:: __new__(states=None, /, **kwargs)

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
   .. automethod:: full(shape=None, /, fill_value=None, **kwargs)
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

A Mulder simulation is managed using a :py:class:`~mulder.Fluxmeter` object. For
basic use cases, one might simply invoke the :py:meth:`~mulder.Fluxmeter.flux`
methods which returns the muon flux for input `observation states <States
interface_>`_, depending on the :py:class:`~mulder.Fluxmeter` configuration
(:py:attr:`atmosphere <mulder.Fluxmeter.atmosphere>`, :py:attr:`geometry
<mulder.Fluxmeter.geometry>`, :py:attr:`reference <mulder.Fluxmeter.reference>`,
etc.). For more advanced usage, please refer to the :doc:`flux computation
<flux>` section.

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

   This class provides a high-level interface for computing alterations in the
   flux of atmospheric muons, due to :py:attr:`geometrical <geometry>` features,
   w.r.t. to an open-sky :py:attr:`reference` model. For basic use cases one
   might simply use the :py:meth:`flux` method with default settings. For more
   advanced usage, please refer to the :doc:`flux computation <flux>` section.

   .. method:: __new__(*layers, **kwargs)

      Creates a new fluxmeter.

      The *layers* arguments may specify an :py:class:`~mulder.EarthGeometry`.
      For instance, the following creates a meter with a 2-layers geometry.

      >>> meter = mulder.Fluxmeter(
      ...     ("dem.asc", 0.0),
      ...     -100.0
      ... )

      Alternatively, the geometry may be explicitly specified as a named
      argument. For instance, the following creates a meter with a
      :py:class:`~mulder.LocalGeometry` loaded from a file.

      >>> meter = mulder.Fluxmeter(geometry="geometry.toml")

      Other attributes (see below) may be specified as named arguments, as well.
      For instance,

      >>> meter = mulder.Fluxmeter(
      ...     atmosphere = "midlatitude-winter",
      ...     date = "2025-12-25",                # For geomagnetic field.
      ...     mode = "mixed",
      ...     bremsstrahlung = "KKP95",           # For physics model.
      ...     seed = 123456,                      # For random engine.
      ...     reference = "Gaisser90",
      ... )

      Note that specifying a *date* enables the geomagnetic field, which is
      disabled by default. Alternatively, the geomagnetic field might also be
      enabled as,

      >>> meter = mulder.Fluxmeter(geomagnet=True)

   .. automethod:: flux

      This method uses the `States interface`_ for specifying the observation
      states(s) of interest. For instance, the following computes the flux
      at an altitude of 100 m and along an elevation angle of 30 deg.

      >>> flux = meter.flux(altitude=100, elevation=30)

      In mixed or detailed :py:attr:`mode`, the *events* parameter specifies the
      number of reference states that are generated for each observation state
      in order to estimate the flux. In these cases, the method returns the flux
      and error estimates as an :py:class:`~numpy.ndarray`. For instance,

      .. doctest::
         :hide:

         >>> meter.mode = "mixed"
         >>> meter.geomagnet = None

      >>> flux, sigma = meter.flux(altitude=100, elevation=30, events=1000)

   .. automethod:: transport

      This method uses the `States interface`_ for specifying the observation
      states(s) of interest. For instance, the following determines the
      reference state corresponding to an observation altitude of 100 m, along
      an elevation angle of 30 deg.

      >>> state0 = meter.transport(altitude=100, elevation=30)

      In mixed or detailed :py:attr:`mode`, the *events* parameter specifies the
      number of reference states that are generated for each observation state.
      For example, the following returns an :py:class:`~numpy.ndarray`
      containing a thousand reference states.

      >>> states0 = meter.transport(altitude=100, elevation=30, events=1000)

   .. rubric:: Attributes
     :heading-level: 4

   .. autoattribute:: atmosphere

      This attribute is an instance of a :py:class:`mulder.Atmosphere`, which
      controls the atmosphere properties. For convenience, the atmosphere model
      can be provided directly when setting this attribute. For example,

      >>> meter.atmosphere = "us-standard"

   .. autoattribute:: geomagnet

      This attribute is an instance of a :py:class:`mulder.EarthMagnet`, which
      controls the geomagnetic field. For convenience, the geomagnetic model can
      be provided directly when setting this attribute. For example,

      >>> meter.geomagnet = "IGRF14.COF"

      By default, the geomagnetic field is disabled.

   .. autoattribute:: geometry

      This attribute is an :py:class:`~mulder.EarthGeometry` or a
      :py:class:`~mulder.LocalGeometry`. For convenience, local geometry data
      can be provided directly when setting this attribute. For example,

      >>> meter.geometry = "geometry.toml"

   .. autoattribute:: mode

      Possible values are, :python:`"continous"`, :python:`"discrete"` or
      :python:`"mixed"`. By default, the fluxmeter operates in continuous mode.
      For instance, the following switches the fluxmeter to discrete mode.

      >>> meter.mode = "discrete"

   .. autoattribute:: physics

      This attribute is an instance of a :py:class:`mulder.Physics`, which
      controls the physics of the muon transport.

   .. autoattribute:: random

      This attribute is an instance of a :py:class:`mulder.Random`, which
      controls the pseudo-randomness of simulated events.

   .. autoattribute:: reference

      This attribute is an instance of a :py:class:`mulder.Reference`, which
      controls the reference model for flux computations.

----

.. autoclass:: mulder.Random

   This class manages a cyclic sequence of pseudo-random numbers over the
   interval :math:`(0, 1)`. These numbers are exposed as a stream of
   :py:class:`floats <float>`. The sequence is fully determined by the
   :py:attr:`seed` attribute, while the :py:attr:`index` attribute indicates the
   stream state.

   .. note::

      A `Permuted Congruential Generator <WikipediaPCG_>`_ (PCG) is used (namely
      `Mcg128Xsl64`_), which has excellent performances for Monte Carlo
      applications.

   .. method:: __new__(seed=None, *, index=None)

      Creates a new pseudo-random stream.

      If *seed* is :python:`None`, then a random value is picked using the
      system entropy. Otherwise, the specified :py:attr:`seed` value is used.
      For instance,

      >>> prng = mulder.Random(123456789)

   .. automethod:: uniform01

      If *shape* is :python:`None`, then a single number is returned. Otherwise,
      a :external:py:class:`numpy.ndarray` is returned, with the given *shape*.
      For instance, the following returns the next 100 pseudo-random
      numbers from the stream.

      >>> rns = prng.uniform01(100)

   .. rubric:: Attributes
     :heading-level: 4

   .. autoattribute:: index

      This property can be modified, resulting in consuming or rewinding the
      pseudo-random stream. For instance, the following resets the stream.

      >>> prng.index = 0

   .. autoattribute:: seed

      The property fully determines (and identifies) the pseudo-random sequence.
      Note that modifying the seed also resets the stream to index :python:`0`.

----

.. autoclass:: mulder.Reference

   This class represents a reference model of the muon flux. Typically, this is
   the opensky flux, i.e the atmospheric muon flux in the absence of any
   topographic features.

   Mulder expresses the reference flux, :math:`\phi(K, \epsilon, z)`, as
   function of the muon kinetic energy, :math:`K`, and using geographic
   coordinates, where :math:`\epsilon` is the elevation angle of observation and
   :math:`z` the altitude. In addition, the contributions of muons
   (:math:`\phi_-`) and anti-muons (:math:`\phi_+`) are split, as :math:`\phi =
   \phi_+ + \phi_-`.

   Alternatively, the reference flux might be set as flat, typically :math:`\phi
   = 1` over a domain in :math:`(K, \epsilon, z)`. This is especially relevant
   in conjuction with the :py:meth:`Fluxmeter.transport` method, e.g. to
   generate a sample of reference muons.

   .. method:: __new__(model=None, /, **kwargs)

      Creates a reference model.

      The *model* argument might be,

      - an :class:`array <numpy.ndarray>` containing a tabulation of the
        reference flux,

      - a :py:class:`str` indicating a parametric model (see
        :numref:`tab-reference-models`),

      - a :py:class:`~pathlib.Path` to a file containing a tabulated flux model,

      - a :py:class:`float` indicating a flat reference.

      By default, i.e. if *model* is :python:`None`, the parametric model of
      [GCC+15]_ is used. For instance, as

      >>> reference = mulder.Reference()

      .. _tab-reference-models:

      .. list-table:: Parametric reference models.
         :width: 75%
         :widths: auto
         :header-rows: 1

         * - Name
           - Reference
         * - :python:`"GCCLY15"`
           - [GCC+15]_
         * - :python:`"Gaisser90"`
           - [Gai90]_

   .. automethod:: flux

      This method uses the `States interface <States interface_>`_ for
      specifying the observation state of interest. For instance, using
      geographic coordinates,

      >>> flux = reference.flux(elevation=30)

   .. rubric:: Attributes
     :heading-level: 4

   .. note::

      :py:class:`~mulder.Reference` objects are immutable, i.e. the underlying
      model or its support cannot be modified.

   .. autoattribute:: altitude

      Depending on the reference *model*, the altitude might be a
      :py:class:`float` constant or an interval. For instance,

      >>> reference.altitude
      0.0

   .. autoattribute:: elevation
   .. autoattribute:: energy

   .. autoattribute:: model

      Depending on how the reference was created, this attribute may be a
      :py:class:`float`, a :py:class:`~numpy.ndarray` or a :py:class:`str`. For
      instance,

      >>> reference.model
      'GCCLY15'


Picture interface
~~~~~~~~~~~~~~~~~

.. autoclass:: mulder.Camera

   .. method:: __new__(coordinates=None, /, **kwargs)

   .. automethod:: shoot

   .. rubric:: Attributes
     :heading-level: 4

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
'0.3.1'

The available configuration data are listed below.

.. data:: CACHE
   :type: ~pathlib.Path

   The cache location.

   Mulder uses a caching system to reduce the time taken for some
   time-consuming, yet repetitive tasks, such as the compilation of
   :py:class:`~mulder.Physics` tables. This data indicates the location of
   cached files. For instance, the following changes the cache location

   >>> mulder.config.CACHE = "/tmp/mulder"  # doctest: +SKIP

   .. note::

      By default, cache files are stored under :bash:`$HOME/.cache/mulder`. This
      can be overriden by setting the :bash:`MULDER_CACHE` environment variable
      to the desired location.

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
.. _Bugaev and Shlepin: https://doi.org/10.1103/PhysRevD.67.034027
.. _Calzone: https://github.com/niess/calzone
.. _Calzone-Geometry: https://calzone.readthedocs.io/en/latest/geometry.html
.. _Clermont-Ferrand: https://en.wikipedia.org/wiki/Clermont-Ferrand
.. _CRS: https://en.wikipedia.org/wiki/Spatial_reference_system
.. _CSDA: https://en.wikipedia.org/wiki/Continuous_slowing_down_approximation_range
.. _DEM: https://en.wikipedia.org/wiki/Digital_elevation_model
.. _ECEF: https://en.wikipedia.org/wiki/Earth-centered,_Earth-fixed_coordinate_system
.. _EGM96 Grid: https://web.archive.org/web/20130218141358/http://earth-info.nga.mil/GandG/wgs84/gravitymod/egm96/egm96.html
.. _EPSG: https://epsg.io/
.. _GeoTIFF: https://fr.wikipedia.org/wiki/GeoTIFF
.. _HGT: http://fileformats.archiveteam.org/wiki/HGT
.. _IGRF14: https://doi.org/10.1186/s40623-020-01288-x
.. _ISO_8601: https://en.wikipedia.org/wiki/ISO_8601
.. _Kokoulin: https://doi.org/10.1016/S0920-5632(98)00475-7
.. _LTP: https://en.wikipedia.org/wiki/Local_tangent_plane_coordinates
.. _Mcg128Xsl64: https://docs.rs/rand_pcg/latest/rand_pcg/struct.Mcg128Xsl64.html#
.. _PDG: https://pdg.lbl.gov/2025/AtomicNuclearProperties/index.html
.. _Pumas: https://github.com/niess/pumas
.. _PROPOSAL: https://github.com/tudo-astroparticlephysics/PROPOSAL
.. _SRTMGL1.003: https://doi.org/10.5067/MEASURES/SRTM/SRTMGL1.003
.. _Standard Rock: https://pdg.lbl.gov/2025/AtomicNuclearProperties/standardrock.html
.. _Structured arrays: https://numpy.org/doc/stable/user/basics.rec.html
.. _TOML: https://toml.io
.. _Turtle: https://github.com/niess/turtle
.. _WikipediaPCG: https://en.wikipedia.org/wiki/Permuted_congruential_generator
