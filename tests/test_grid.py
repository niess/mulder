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
