#! /usr/bin/env python3
"""This example illustrates the usage of mulder.Array objects.

Mulder Arrays provide a bridge between numpy structured arrays and C arrays of
structures, accessed through cffi. Thus, Arrays are the base input and output
objects of the mulder Python package.
"""

from mulder import arrayclass, Flatgrid, Layer, Projection


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
# Mulder functions only accept scalar or vector data. That is, multidimensional
# arrays are not allowed. For example, the following raises an error

try:
    projection = Projection(x=((1, 2), (3, 4)))
except ValueError:
    pass

# Thus, grided data need to be flatened first, before being passed to a mulder
# function. This can be done conveniently with a mulder.Flatgrid object. For
# example, the following generates a 2d-grid over (x, y)

x = (1, 2, 3)
y = (0, 1)
grid = Flatgrid(x=x, y=y)

print(f"""\
Flat grid:
- shape: {grid.shape}
- size: {grid.size}
- x: {grid.x}
- y: {grid.y}
""")

# Note that only named arguments are allowed when creating a Flatgrid object.
# Note also that the grid can be directly unpacked as argument to a mulder
# function, as

projection = Projection(**grid)

# The Flatgrid.shape property contains the actual grid shape (before
# flattening). Getting back to the multidimensional grid is just a matter of
# reshaping vectorized (flattened) arrays. This can be done as

x, y = grid.unflatten(grid.x, grid.y)

# Let us print the result with some formatting

oneline = lambda s: str(s).replace("\n", ",")

print(f"""\
Two-dimensional grid:
- x: {oneline(x)}
- y: {oneline(y)}
""")


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
numpy dtype: {Point.numpy_dtype}
numpy data:  {p.numpy_array.data}
data size:   {p.numpy_array.nbytes}
data stride: {p.numpy_stride}
""")

# The memory managed by numpy could be forwarded to a C library, using cffi, as

print(f"""\
cffi pointer: {q.cffi_pointer}
""")

# Note that in the previous example we did not use our Point object. It would
# throw an exception, since `struct point` has not been declared to cffi.
