#! /usr/bin/env python3
"""This example illustrates the usage of mulder.Grid objects.

Mulder Grids are regular meshes spanning over a set of basis vectors. However,
by default nodes coordinates are exposed as flat (unstructured) vector arrays.
This is consistent with mulder, since the library functions only accept scalar
or vector data as input. That is, multidimensional arrays are not allowed. For
example, the following would raise an error

>> projection = Projection(x=((1, 2), (3, 4)))

# Thus, grid coordinates need to be flatten (unstructured) first, before being
# passed to a mulder function. However, in some other cases, e.g. plotting, one
# instead needs structured data. These different use cases are conveniently
# managed with a mulder.Grid object.
"""

from mulder import Grid, Projection
import numpy


# =============================================================================
# A mulder Grid is created from a set of base vectors using named arguments. For
# example, the following generates a 2d-grid over (x, y).

grid = Grid(
    x = (1, 2, 3),
    y = (0, 1)
)

# Let us print the grid properties below.

print(f"""\
Grid properties:
- dimension:   {grid.ndim}
- size:        {grid.size}
- shape:       {grid.shape}
- x (flatten): {grid.x}
- x (base):    {grid.base.x}
- y (flatten): {grid.y}
- y (base):    {grid.base.y}
""")

# Note that the grid nodes coordinates can be accessed as attrbutes, but also
# unpacked as argument to a mulder function, as following.

projection = Projection(**grid.nodes)


# =============================================================================
# The Grid.shape property contains the actual grid shape (when viewed as a
# structured grid). Getting back to a multidimensional grid is just a matter of
# reshaping vectorized (flatten) arrays. This can be done with the Grid.reshape
# method, as

x, y = grid.reshape(grid.x, grid.y)

# In the particular case of nodes coordinates, the Grid.structured property
# returns a structured view of the same data. For example, let us print the
# same structured coordinates with some formatting

oneline = lambda s: str(s).replace("\n", ",")

print(f"""\
Structured nodes:
- x: {oneline(grid.structured.x)}
- y: {oneline(grid.structured.y)}
""")

# Let us point out that, in structured view, the order of named arguments
# matters when defining a mulder.Grid object. For example, inverting x and y
# in the previous example would result in a transposed grid, as

transposed = Grid(
    y = grid.base.y,
    x = grid.base.x
)

assert(numpy.array_equal(
    transposed.structured.x,
    grid.structured.x.T
))

# Mulder.Grids are are generated using numpy.meshgrid. The optional *indexing*
# keyword allows to further control the shape of the resulting grid. Note that
# by default, "xy" indexing is used (i.e. x coordinate varies along column
# indices). The other possibility is "ij" indexing (i.e. following indices
# order). Of course, the previous considerations only matter when a specific
# structured view is needed.
