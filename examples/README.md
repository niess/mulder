# Mulder examples

The Python examples are structured in two subfolders, [basic][BASIC] and
[advanced][ADVANCED].


## Basic Python examples

The [basic][BASIC] examples are organised by Mulder objects, e.g. arrays,
fluxmeter, geometry, etc. You might jump directly to the
[fluxmeter.py][FLUMETER] example in order to get a practical and complete
example. However, for a more in depth understanding of a Mulder, one would
rather start from the bottom, with [arrays.py][ARRAYS], followed by
[grids.py][GRIDS], [layer.py][LAYER], (optionally [geomagnet.py][GEOMAGNET]),
[geometry.py][GEOMETRY], [reference.py][REFERENCE] and finally
[fluxmeter.py][FLUXMETER].


## Advanced Python examples

The [advanced][ADVANCED] examples are organised by functionalities, e.g. flux
computation, geometry intersection, etc. There is no particular order at this
stage. For a detailed understanding of Mulder algorithms, you could start with
the [transport.py][TRANSPORT] example. The [prng.py][PRNG] example provides
additional information for Monte Carlo applications. The [flux.py][FLUX] example
might be interesting for a better understanding of references in flux
computations. The [grammage.py][GRAMMAGE] and [intersect.py][INTERSECT]
examples, illustrate the ray tracing functionalities of Mulder.


## C example

A basic example of usage is provided as [example.c][C_EXAMPLE]. Please, refer to
Python examples for more detailed information. Let us also recall that the C
library is bare-bones. High level configuration operations, the like preparing
your geometry, a reference flux, material tables etc., are expected to be done
from Python, not directly from C.


[ADVANCED]: advanced
[ARRAYS]: basic/arrays.py
[BASIC]: basic
[C_EXAMPLE]: example.c
[FLUX]: advanced/flux.py
[FLUXMETER]: basic/fluxmeter.py
[GEOMAGNET]: basic/geomagnet.py
[GEOMETRY]: basic/geometry.py
[GRIDS]: basic/grids.py
[GRAMMAGE]: advanced/grammage.py
[INTERSECT]: advanced/intersect.py
[LAYER]: basic/layer.py
[PRNG]: advanced/prng.py
[REFERENCE]: basic/reference.py
[TRANSPORT]: advanced/transport.py
