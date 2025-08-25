Python interface
================

.. autoclass:: mulder.Atmosphere

   .. method:: __new__(model=None, /)

   .. automethod:: density

----

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

.. autoclass:: mulder.Geometry

   .. method:: __new__(*layers, atmosphere=None, magnet=None)
   .. method:: __getitem__(key, /)

   .. automethod:: locate
   .. automethod:: scan
   .. automethod:: trace

   .. autoattribute:: atmosphere
   .. autoattribute:: layers
   .. autoattribute:: magnet
   .. autoattribute:: z

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

.. autoclass:: mulder.Magnet

   .. method:: __new__(*args, **kwargs)
   .. method:: __call__(*args, **kwargs)

   .. autoattribute:: altitude
   .. autoattribute:: day
   .. autoattribute:: month
   .. autoattribute:: year

----

.. autoclass:: mulder.Materials

   .. method:: __new__(path=None, /)
   .. method:: __getitem__(name, /)

----

.. autoclass:: mulder.Physics

   .. method:: __new__(*args, **kwargs)

   .. automethod:: compile

   .. autoattribute:: bremsstrahlung
   .. autoattribute:: materials
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
