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

    v = (1E+04, 2E+04, 3E+04)
    frame0 = mulder.LocalFrame(latitude=30, longitude=15)
    frame1 = mulder.LocalFrame(position=v, frame=frame0)
    frame2 = frame0.translated(v)
    assert_allclose(frame1.latitude, frame2.latitude)
    assert_allclose(frame1.longitude, frame2.longitude)
    assert_allclose(frame1.altitude, frame2.altitude)
    assert_allclose(frame1.azimuth, frame2.azimuth)
    assert_allclose(frame1.elevation, frame2.elevation)

    frame1 = frame0.looking_at(position=(1, 0, 0))
    assert_allclose(frame1.azimuth, 90.0, atol=1E-07)
    assert_allclose(frame1.elevation, 0.0, atol=1E-07)
