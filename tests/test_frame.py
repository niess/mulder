import mulder
import numpy
from numpy.testing import assert_allclose


def test_frame():
    """Test local frames."""

    frame0 = mulder.LocalFrame()
    assert frame0.latitude == 0
    assert frame0.longitude == 0
    assert frame0.altitude == 0
    assert frame0.azimuth == 0
    assert frame0.elevation == 0

    frame1 = mulder.LocalFrame(altitude=1, azimuth=30)
    assert frame1.altitude == 1
    assert frame1.azimuth == 30

    ex = frame0.transform((1, 0, 0), destination=frame1, mode="vector")
    assert_allclose(ex, [numpy.sqrt(3) / 2, 0.5, 0.0], atol=1E-07)

    ex = frame0.transform((1, 0, 2), destination=frame1, mode="point")
    assert_allclose(ex, [numpy.sqrt(3) / 2, 0.5, 1.0], atol=1E-07)

    frame1 = mulder.LocalFrame(elevation=30)
    assert frame1.elevation == 30

    ex = frame0.transform((0, 1, 0), destination=frame1, mode="vector")
    assert_allclose(ex, [0.0, numpy.sqrt(3) / 2, -0.5], atol=1E-07)
