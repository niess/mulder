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

Simulating the geometry traversal of a muon is a ray-tracing problem. While the
location function, :math:`L`, is sufficient for this purpose, it is not
efficient. Therefore, the geometry model is complemented with a Tracing
function, :math:`T`. Let us parametrise a ray as

.. math::

   R(\vec{r}, \vec{u}) = \{ \vec{r} + \lambda \vec{u} \,|\, \lambda \in \mathbb{R}_+ \} ,

where :math:`r` is the ray origin and :math:`\vec{u} \in \mathbb{U}_3` a unit
vector of :math:`\mathbb{R}^3`. Then, the Tracing function is defined as

.. math::

   T:
   \left|
     \begin{array}{rcl}
       \mathbb{R}^3 \times \mathbb{U}_3 & \longrightarrow & \mathbb{R}_+ \times \left[\!\left[ 0, n \right]\!\right] \\
       (\vec{r}, \vec{u}) & \longmapsto & (\lambda^*, j) \\
     \end{array}
   \right. ,

where the returned :math:`\lambda^*` value is the distance to the first (closest
from :math:`\vec{r}`) intersected medium boundary, and :math:`j` the index of
the next medium, i.e. on the opposite side of the boundary w.r.t. the ray
origin, :math:`\vec{r}`.


Physical properties
-------------------

The physical properties of a medium are defined by,

- the name of a constitutive :ref:`material <sec-materials-interface>`, which
  maps to a :py:class:`~mulder.materials.Material` or
  :py:class:`~mulder.materials.Composite` object, specifying the medium atomic
  composition.

- a bulk :py:attr:`density <mulder.Layer.density>`, that might differ from the
  material one, e.g., to account for a solid porosity or specific gas T,P
  conditions.


.. URL links.
.. _Pumas: https://github.com/niess/pumas
