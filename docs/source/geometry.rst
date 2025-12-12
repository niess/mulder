Geometry definition
===================

Geometry Model
~~~~~~~~~~~~~~

Mulder geometries rely on a common model, inherited from `Pumas`_ [Nie22]_. A
Mulder geometry is defined as a set of propagation media,
:math:`M = \{ m_i \,|\, i \in \left[\!\left[ 0, n - 1 \right]\!\right]\}`,
where the media, :math:`m_i`, differ by their physical properties. The geometry
media delimit volumes in the 3d-space, as described below.

.. note::

   The **spatial structure** of a geometry instance is expected to be
   **immutable**. However, the physical properties of the constitutive media may
   be freely modified.


Space structure
---------------

The space structure of the geometry is defined by a Location function,
:math:`L`, mapping a space *position*, :math:`\vec{r}`, to a medium index,
:math:`i`, as

.. math::

   L:
   \left|
     \begin{array}{rcl}
       \mathbb{R}^3 & \longrightarrow & \left[\!\left[ 0, n \right]\!\right] \\
       \vec{r} & \longmapsto & i \\
     \end{array}
   \right. ,

where the special index :math:`n = \left|M\right|` indicates that
:math:`\vec{r}` lies outside of :math:`M`.

A muon trajectory is modelled as an ordered sequence of (interaction) vertices,
connected by directed line segments. In the present context, it is convenient to
parametrise a directed line segment as,

.. math::

   S(\vec{r}, \vec{u}, s) = \{ \vec{r} + \lambda \vec{u} \,|\, \lambda \in [0, s] \},

where :math:`r` is the segment start, :math:`\vec{u} \in \mathbb{U}_3` a unit
vector of :math:`\mathbb{R}^3`, and :math:`s = |S|` the segment length.
Simulating the geometry traversal of a muon requires solving a succession of ray
tracing problems, i.e. intersecting the media boundaries with directed line
segments. While the location function, :math:`L`, is sufficient for this
purpose, it is usually not efficient. Therefore, the geometry model is
complemented with a tracing function, :math:`T`, mapping directed line segments
to their first (i.e., closest from start) intersected medium boundary, as

.. math::

   T:
   \left|
     \begin{array}{rcl}
       \mathbb{R}^3 \times \mathbb{U}_3 \times \mathbb{R}_+ & \longrightarrow & [0, s] \times \left[\!\left[ 0, n \right]\!\right] \\
       (\vec{r}, \vec{u}, s) & \longmapsto & (\lambda, i) \\
     \end{array}
   \right. ,

where the returned :math:`\lambda` value is the distance to the first boundary,
and :math:`i` the index of the next medium, i.e. on the opposite side of the
boundary w.r.t. the segment start.


Physical properties
-------------------

The physical properties of a medium are defined by,

- the name of a constitutive :ref:`material <sec-materials-interface>`, which
  maps to a :py:class:`~mulder.materials.Material` (or
  :py:class:`~mulder.materials.Composite`) object specifying the actual medium
  atomic composition.

- a bulk :py:attr:`density <mulder.Layer.density>`, that might differ from the
  material one, e.g., to account for a solid porosity or specific gas T,P
  conditions.


.. URL links.
.. _Pumas: https://github.com/niess/pumas
