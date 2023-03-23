#! /usr/bin/env python3
"""This example illustrates the usage of mulder.Array objects.

Mulder Arrays provide a bridge between numpy structured arrays and C arrays of
structures, accessed through cffi. Thus, Arrays are the base input and output
objects of the mulder Python package.
"""

from mulder import Layer, Projection
from mulder.arrays import Algebraic, arrayclass

import numpy


# =============================================================================
# Let us illustrate some base proparties of mulder.arrays with the Projection
# object. This object stores map (projected) coordinates (see e.g. the layer.py
# example).
#
# A mulder.Array object is instanciated following the NamedTuple syntax. For
# example

p = Projection(1, 2)
q = Projection(x = 1, y = 2)
assert(p == q)

# Omitted fields are initialised to zero. For example

p = Projection(0, 1)
q = Projection(y = 1)
assert(p == q)

# Field values can be retrived as attributes.

x = p.x
assert(x == 0)

# Fields can be unpacked as well, e.g. as

x, y = p
assert(x == p.x)
assert(y == p.y)


# =============================================================================
# Many mulder function take an Array object as input and output another Array
# object. Alternatively, packed and unpacked forms can be used as well, with
# automatic type conversion. For example, considering a flat Layer object,
# instanciated as

layer = Layer()

# the following syntaxes are equivalent

gradient = layer.gradient(p)
gradient = layer.gradient(x, y)
gx, gy = layer.gradient(x=x, y=y)

assert(gx == gradient.x)
assert(gy == gradient.y)


# =============================================================================
# Mulder Arrays can store scalar or vector data. For example, a vector of
# projections can be instanciated as

v = Projection(x=(1, 1, 1), y=(1, 2, 3))
assert(v.y[1] == 2)

# The vector length is given by the size property (or equivalently using the len
# function). Note that scalar Arrays return a None size value.

assert(p.size is None)
assert(v.size == 3)
assert(v.size == len(v))

# When a parameter is constant over the vector, then a scalar value can be given
# as input. For example

w = Projection(1, (1, 2, 3))
assert(v == w)

# Empty or zeroed arrays are constructed as
e = Projection.empty(3)
z = Projection.zeros(size=3)

assert(e.size == 3)
assert(z.size == 3)


# =============================================================================
# Accessing specific elements of a mulder Array is done with the usual slice
# syntax. The returned object is also a mulder.Array. Note that it might be a
# reference to the initial array or a copy (following numpy's mechanic). For
# example

w = v[1:]

assert(isinstance(w, type(v)))
assert(len(w) == 2)
assert(w.y[0] == v.y[1])

# In the previous case w holds a reference to v since no memory copy is
# required. Thus modifying w actually modifies v as well

w.x[0] = 0
assert(v.x[1] == 0)

# But, the following (non contiguous) slice generates a copy, in order to
# satisfy to numpy memory model.

w = v[v.y % 2 == 1]
w.x[0] = 1

assert(w.size == 2)
assert(v.x[1] == 0)

# Explicit copies are obtained with the Array.copy method, e.g. as

w = v.copy()
assert(w == v)

w.x[0] = 1
assert(v.x[1] == 0)


# =============================================================================
# Mulder Arrays representing Cartesian coordinates also support algebraic
# operations. This is the case for example for the Projection type. Checking
# for algebraic support can be done as

assert(isinstance(v, Algebraic))

# Then, for example, the distance between two points could be computed as

a = Projection(2, 1)
b = Projection(1, 0)
c = a - b
distance = (c.x**2 + c.y**2)**0.5

# Or directly as

assert(distance == (a - b).norm())

# Algebraic objects can also be simply viewed as unstructured numpy arrays. This
# can be convenient e.g. in order to apply numpy functions, as

data = c.unstructured()
print(f"""\
# Data properties (unstructured view):
- shape: {data.shape}
- size:  {data.size}
""")

distance = numpy.linalg.norm(data)

# Note that by default a reference to the initial data is returned. Thus

assert(c.x == 1)
data *= 2
assert(c.x == 2)

# does modify the original Array. The converse Algebraic.from_unstructured
# method allows one to create an Algebraic Array instance from a numpy.ndarray,
# as

d = Projection.from_unstructured(data, copy=True)
assert(d == c)

# Note that in this case we explictly requested a copy (by default copy is
# False, as previously). Thus *d* refers to new data.

d.x = 1
assert(c.x == 2)


# =============================================================================
# Finally, let us briefly discuss the arrayclass decorator, as well as some
# technicalities of mulder Arrays. Note that the following is not required for
# basic usage of mulder.
#
# Let us consider a C library which defines the following struct object
#
# struct point {
#   double x;
#   double y;
# };
#
# The corresponding mulder.Array object would be defined as

@arrayclass
class Point:
    # C array of point structures.
    ctype = "struct point *"

    # Python attributes with associated numpy dtype.
    properties = (
            ("x", "f8", "The x coordinate"),
            ("y", "f8", "The y coordinate"),
    )

# This object manages an internal numpy structured array that can be used as
# an array of `struct point` by the C library, using cffi. In order to
# illustrate this point, let us create a Point object and show some of its
# internal properties.

p = Point(x=1, y=(1, 2))

print(f"""\
# Point properties:
- numpy dtype: {Point.numpy_dtype}
- numpy data:  {p.numpy_array.data}
- data size:   {p.numpy_array.nbytes}
- data stride: {p.numpy_stride}
""")

# The memory managed by numpy could be forwarded to a C library, using cffi, as

print(f"""\
- cffi pointer: {q.cffi_pointer}
""")

# Note that in the previous example we did not use our Point object. It would
# throw an exception, since `struct point` has not been declared to cffi.
