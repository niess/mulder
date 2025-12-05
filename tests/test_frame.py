import mulder
import numpy
from numpy.testing import assert_allclose


def test_frame():
    """Test local frames."""

    frame0 = mulder.LocalFrame()
    assert frame0.latitude == 45
    assert frame0.longitude == 0
    assert frame0.altitude == 0
    assert frame0.inclination == 0
    assert frame0.declination == 0

    frame1 = mulder.LocalFrame(altitude=1, declination=30)
    assert frame1.altitude == 1
    assert frame1.declination == 30

    ex = frame0.transform((1, 0, 0), destination=frame1, mode="vector")
    assert_allclose(ex, [numpy.sqrt(3) / 2, 0.5, 0.0], atol=1E-07)

    ex = frame0.transform((1, 0, 2), destination=frame1, mode="point")
    assert_allclose(ex, [numpy.sqrt(3) / 2, 0.5, 1.0], atol=1E-07)
