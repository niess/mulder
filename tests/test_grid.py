import mulder
import numpy
from numpy.testing import assert_allclose
from pathlib import Path


PREFIX = Path(__file__).parent


def test_constructor():
    """Test constructor function."""

    # Test file loader.
    grid = mulder.Grid(PREFIX / "assets/dem.asc")
    assert grid.xlim == (-1.5, 1.5)
    assert grid.ylim == (-1.0, 1.0)
    assert grid.zlim == (0.0, 11.0)
    assert grid.projection == None
