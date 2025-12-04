import mulder
from numpy.testing import assert_allclose
from pathlib import Path
import pytest


PREFIX = Path(__file__).parent


def test_earth():
    """Test Earth geometry."""

    # Test constructor & attributes.
    geometry = mulder.EarthGeometry(
        mulder.Layer(-1000, material="Rock"),
        mulder.Layer(0.0, material="Water"),
    )

    assert_allclose(geometry.zlim, [-1000, 0])

    assert len(geometry.layers) == 2
    assert geometry.layers[0].material == "Rock"
    assert geometry.layers[0].density == None
    assert geometry.layers[1].material == "Water"
    assert geometry.layers[1].density == None

    # Test the locate method.
    assert geometry.locate(
        position=[0.0, 0.0, -1001.0], frame=mulder.LocalFrame()
    ) == 0
    assert geometry.locate(altitude=-1001) == 0
    assert geometry.locate(
        position=[0.0, 0.0, -1.0], frame=mulder.LocalFrame()
    ) == 1
    assert geometry.locate(altitude=-1) == 1


@pytest.mark.requires_calzone
def test_local():
    """Test local geometry."""

    # Test constructor & attributes.
    geometry = mulder.LocalGeometry(PREFIX / "assets/geometry.toml")

    assert geometry.frame.latitude == 45
    assert geometry.frame.longitude == 0
    assert geometry.frame.altitude == 0
    assert geometry.frame.declination == 0
    assert geometry.frame.inclination == 0

    assert len(geometry.media) == 2
    assert geometry.media[0].material == "G4_AIR"
    assert geometry.media[0].description == "Environment"
    assert geometry.media[0].density == None
    assert geometry.media[1].material == "G4_CALCIUM_CARBONATE"
    assert geometry.media[1].description == "Environment.Ground"
    assert geometry.media[1].density == None

    frame = mulder.LocalFrame(latitude=37, longitude=3)
    geometry = mulder.LocalGeometry(
        PREFIX / "assets/geometry.toml", frame=frame
    )
    assert geometry.frame.latitude == 37
    assert geometry.frame.longitude == 3

    # Test the locate method.
    assert geometry.locate(position=[0.0, 0.0, 1.0]) == 0
    assert geometry.locate(position=[0.0, 0.0, -1.0]) == 1
    assert geometry.locate(latitude=37, longitude=3, altitude=-1.0) == 1
    media = geometry.locate(position=[
        [0.0, 0.0, -1001],
        [0.0, 0.0, -999],
        [0.0, 0.0, 999],
        [0.0, 0.0, 1001],
    ])
    assert_allclose(media, [2, 1, 0, 2])

    # Test the trace method.
    i = geometry.trace(
        position=[0, 0, -5],
        direction=[0, 0, 1],
    )
    assert i["before"] == 1
    assert i["after"] == 0
    assert i["distance"] == 5
    assert_allclose(i["position"], [0, 0, 0])

    i = geometry.trace(
        position=[0, 0, -1005],
        direction=[0, 0, 1],
    )
    assert i["before"] == 2
    assert i["after"] == 1
    assert i["distance"] == 5
    assert_allclose(i["position"], [0, 0, -1000])

    i = geometry.trace(
        position=[0, 0, 1005],
        direction=[0, 0, 1],
    )
    assert i["before"] == 2
    assert i["after"] == 2
    assert i["distance"] == 0
    assert_allclose(i["position"], [0, 0, 1005])

    # Test the scan method.
    d = geometry.scan(
        position=[0, 0, -995],
        direction=[0, 0, 1],
    )
    assert_allclose(d, [1000, 995])

    d = geometry.scan(
        position=[0, 0, -1005],
        direction=[0, 0, 1],
    )
    assert_allclose(d, [1000, 1000])
