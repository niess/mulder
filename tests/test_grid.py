import mulder
import numpy
from numpy.testing import assert_allclose
from pathlib import Path


PREFIX = Path(__file__).parent


def test_constructor():
    """Test constructor function."""

    x = numpy.linspace(-1.5, 1.5, 4)
    y = numpy.linspace(-1.0, 1.0, 3)
    z = numpy.arange(12).reshape(3, 4)
    xlim = (x[0], x[-1])
    ylim = (y[0], y[-1])

    PARAMETERS = (
        (PREFIX / "assets/dem.asc", {}),
        (PREFIX / "assets/dem.grd", {}),
        (z, {"xlim": xlim, "ylim": ylim}),
    )

    CRSS = \
        list(range(27571, 27575)) + \
        [2154] + \
        [4326] + \
        list(range(32601, 32660)) + \
        list(range(32701, 32760))

    # Test loaders.
    for data, kwargs in PARAMETERS:
        grid = mulder.Grid(data, **kwargs)
        assert grid.xlim == (-1.5, 1.5)
        assert grid.ylim == (-1.0, 1.0)
        assert grid.zlim == (0.0, 11.0)
        assert grid.crs == 4326
        assert_allclose(grid.z(x, y), z, atol=1E-04)

        for crs in CRSS:
            grid = mulder.Grid(data, crs=crs, **kwargs)
            assert grid.crs == crs


def test_methods():
    """Test grid methods."""

    grid = mulder.Grid(PREFIX / "assets/dem.asc")
    assert grid.z(0.0, 0.0) == 5.5
    assert grid.z((0.0, 0.0)) == 5.5

    x = numpy.linspace(-1.5, 1.5, 4)
    y = numpy.linspace(-1.0, 1.0, 3)
    X, Y = numpy.meshgrid(x, y)
    xy = list(zip(X.flatten(), Y.flatten()))
    z0 = grid.z(xy)
    z1 = grid.z(x, y).flatten()
    assert_allclose(z0, z1)
    g0 = grid.gradient(xy)
    g1 = grid.gradient(x, y)
    assert g1.shape == (3, 4, 2)
    assert_allclose(g0, g1.reshape(g0.shape))

    x = numpy.linspace(-1.5, 1.5, 21)
    y = numpy.linspace(-1.0, 1.0, 21)
    xy = list(zip(x, y))
    z = grid.z(xy)
    assert z.shape == (21,)
    for i, (x, y) in enumerate(xy):
        zi = grid.z(x, y)
        assert zi == grid.z((x, y))
        assert zi == z[i]
        assert_allclose(grid.gradient(x, y), grid.gradient((x, y)))

    dz = grid.gradient(0.0, 0.0)
    assert_allclose(dz, (1.0, 4.0), atol=1E-03)
