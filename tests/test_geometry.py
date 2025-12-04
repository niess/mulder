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

    assert_allclose(geometry.zlim, [-11000, 0])

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
    assert geometry.locate(
        position=[0.0, 0.0, -2.0], frame=mulder.LocalFrame(altitude=1)
    ) == 1
    assert geometry.locate(
        position=[0.0, 0.0, -0.5], frame=mulder.LocalFrame(altitude=1)
    ) == 2

    # Test the trace method.
    i = geometry.trace(altitude=-5, elevation=90)
    assert i["before"] == 1
    assert i["after"] == 2
    assert_allclose(i["distance"], 5)
    assert_allclose(i["altitude"], 0, atol=1E-06)

    i = geometry.trace(position=[0, 0, -5], direction=[0, 0, 1],
                       frame=mulder.LocalFrame())
    assert i["before"] == 1
    assert i["after"] == 2
    assert_allclose(i["distance"], 5)
    assert_allclose(i["position"], [0, 0, 0], atol=1E-06)

    i = geometry.trace(position=[0, 0, -6], direction=[0, 0, 1],
                       frame=mulder.LocalFrame(altitude=1))
    assert i["before"] == 1
    assert i["after"] == 2
    assert_allclose(i["distance"], 5)
    assert_allclose(i["position"], [0, 0, -1], atol=1E-06)

    i = geometry.trace(altitude=-1001, elevation=90)
    assert i["before"] == 0
    assert i["after"] == 1
    assert_allclose(i["distance"], 1, atol=1E-06)
    assert_allclose(i["altitude"], -1000, atol=1E-06)

    # Test the scan method.
    d = geometry.scan(altitude=-1001, elevation=90)
    assert_allclose(d, [1, 1000])

    d = geometry.scan(altitude=-1, elevation=90)
    assert_allclose(d, [0, 1])

    d = geometry.scan(altitude=1, elevation=-90)
    assert_allclose(d, [-geometry.zlim[0] - 1000, 1000])

    d = geometry.scan(position=[0, 0, -1001], direction=[0, 0, 1],
                      frame=mulder.LocalFrame())
    assert_allclose(d, [1, 1000], atol=1E-06)

    d = geometry.scan(position=[0, 0, -1002], direction=[0, 0, 1],
                      frame=mulder.LocalFrame(altitude=1))
    assert_allclose(d, [1, 1000], atol=1E-06)


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
    frame = mulder.LocalFrame(latitude=37, longitude=3, altitude=1)
    assert geometry.locate(position=[0.0, 0.0, -0.5], frame=frame) == 0
    assert geometry.locate(position=[0.0, 0.0, -1.5], frame=frame) == 1

    # Test the trace method.
    i = geometry.trace(
        position=[0, 0, -5],
        direction=[0, 0, 1],
    )
    assert i["before"] == 1
    assert i["after"] == 0
    assert i["distance"] == 5
    assert_allclose(i["position"], [0, 0, 0])

    i = geometry.trace(latitude=37, longitude=3, altitude=-5, elevation=90)
    assert i["before"] == 1
    assert i["after"] == 0
    assert_allclose(i["distance"], 5, atol=1E-07)
    assert_allclose(i["altitude"], 0, atol=1E-07)

    i = geometry.trace(position=[0, 0, -6], direction=[0, 0, 1], frame=frame)
    assert i["before"] == 1
    assert i["after"] == 0
    assert_allclose(i["distance"], 5, atol=1E-07)
    assert_allclose(i["position"], [0, 0, -1], atol=1E-07)

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

    d = geometry.scan(latitude=37, longitude=3, altitude=-995, elevation=90)
    assert_allclose(d, [1000, 995])

    d = geometry.scan(position=[0, 0, -996], direction=[0, 0, 1], frame=frame)
    assert_allclose(d, [1000, 995])
