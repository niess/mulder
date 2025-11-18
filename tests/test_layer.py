import mulder
import numpy
from numpy.testing import assert_allclose
from pathlib import Path


PREFIX = Path(__file__).parent


def test_constructor():
    """Test constructor function."""

    layer = mulder.Layer(PREFIX / "assets/dem.asc")
    assert len(layer.data) == 1
    assert isinstance(layer.data[0], mulder.Grid)
    assert layer.density == None
    assert layer.description == None
    assert layer.material == "Rock"

    layer = mulder.Layer(1.0, description="toto")
    assert len(layer.data) == 1
    assert layer.data[0] == 1.0
    assert layer.description == "toto"

    layer = mulder.Layer(PREFIX / "assets/dem.asc", 1.0, material="Water",
                         density=1.0)
    assert len(layer.data) == 2
    assert isinstance(layer.data[0], mulder.Grid)
    assert layer.data[1] == 1.0
    assert layer.material == "Water"
    assert layer.density == 1.0


def test_methods():
    """Test grid methods."""

    layer = mulder.Layer(PREFIX / "assets/dem.asc")
    assert layer.altitude(0.0, 0.0) == 5.5
    assert layer.altitude((0.0, 0.0)) == 5.5
    assert_allclose(layer.altitude(1.0, 0.0), 9.5, atol=1E-03)
    assert_allclose(layer.altitude(0.0, 1.0), 6.5, atol=1E-03)
    assert numpy.isnan(layer.altitude((2.0, 0.0)))
    assert (layer.normal((2.0, 0.0)) == 0.0).all()

    layer = mulder.Layer(PREFIX / "assets/dem.asc", 0.0)
    assert layer.altitude(0.0, 0.0) == 5.5
    assert layer.altitude((2.0, 0.0)) == 0.0
    assert (layer.normal((2.0, 0.0)) != 0.0).any()

    layer = mulder.Layer(1.0)
    assert layer.altitude(0.0, 0.0) == 1.0
    assert_allclose(layer.normal((0.0, 0.0)), (1.0, 0.0, 0.0), atol=1E-06)
