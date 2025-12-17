External modules
================

External software may be interfaced with Mulder as modules, to extend Mulder
functionalities with new materials and geometries. Mulder modules implement a
common C-interface, detailed `below <C interface_>`_. This C interface serves as
a bridge between the external software and Mulder's :ref:`Python interface
<sec-module-interface>`.

Mulder modules are distributed as shared libraries, using a specific entry
point, :clang:`mulder_initialise()`. Thus, a mulder module may in fact be
embedded with a Python `extension module`_.

.. note::

   Mulder comes with a Geant4 interface, which also serves as an example of a
   module implementation. For further details, please refer to the
   `modules/geant4 <Geant4 example>`_ example.


C interface
~~~~~~~~~~~

Mulder C interface is defined in file :bash:`mulder.h`, which is bundled with
the Mulder package. For instance, in bash

.. code-block:: bash

   MULDER_H="$(python3 -m mulder --prefix)/interface/mulder.h"

For convenience, the content of the :bash:`mulder.h` file is mirrored below.
In addition, let us point out that,

- a Mulder module must implement the entry point function,
  :clang:`mulder_initialise()`, which returns a :clang:`struct mulder_module`.
  However, the module might ommit to implement any of the :clang:`element()`,
  :clang:`geometry()` or :clang:`material()` functions (which corresponding
  field must then be set to :clang:`NULL`).

- Implementing a geometry (:clang:`struct mulder_geometry`) requires
  implementing both the locator (:clang:`struct mulder_locator`) and tracer
  (:clang:`struct mulder_tracer`) functionalities. See the :ref:`Geometry model
  <sec-geometry-model>` section for additional information.

.. literalinclude:: include/mulder.h
   :language: C
   :name: mulder-h


.. URL links.
.. _Extension module: https://docs.python.org/3/extending/extending.html
.. _Geant4 example: https://github.com/niess/mulder/tree/master/examples/modules/geant4
