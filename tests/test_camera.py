import mulder
import numpy
from numpy.testing import assert_allclose
from pathlib import Path


PREFIX = Path(__file__).parent


def test_camera():
    """Test camera object."""

    frame = mulder.LocalFrame(azimuth=45, elevation=30)
    camera = frame.camera()
    assert camera.frame == frame
    assert camera.fov == 60
    assert camera.ratio == 4 / 3
    assert camera.resolution == (90, 120)
    assert_allclose(camera.focal, 1.732051, atol=1E-05)

    camera = frame.camera(focal=1)
    assert_allclose(camera.focal, 1)
    assert_allclose(camera.fov, 90.0, atol=1E-05)

    camera = frame.camera((18, 32))
    assert camera.ratio == 16 / 9
    assert camera.resolution == (18, 32)
    assert_allclose(camera.focal, 1.732051, atol=1E-05)


def test_projection():
    """Test projection object."""

    geometry = mulder.EarthGeometry(0)
    frame = mulder.LocalFrame(altitude=1)
    camera = frame.camera((9, 13))
    projection = camera.project(geometry)
    assert projection.frame == frame
    assert projection.medium.shape == (9, 13)
    expected = numpy.full((9, 13), 0, dtype="i4")
    expected[4:,:] = 1
    assert_allclose(projection.medium, expected)
    expected = numpy.full((9, 13), 0, dtype="i4")
    expected[4:,:] = 120E+03
    assert_allclose(projection.altitude, expected, atol=1E-04)

    geometry = mulder.LocalGeometry(PREFIX / "assets/geometry.toml")
    frame = mulder.LocalFrame(altitude=1)
    camera = frame.camera((9, 13))
    projection = camera.project(geometry)
    assert projection.frame == frame
    assert projection.medium.shape == (9, 13)
    expected = numpy.full((9, 13), 1, dtype="i4")
    expected[4:,:] = 0
    assert_allclose(projection.medium, expected)


def test_pixels():
    """Test pixels object."""

    frame = mulder.LocalFrame(azimuth=45, elevation=30)
    camera = frame.camera((9, 13))
    pixels = camera.pixels
    assert pixels.u.shape == (camera.resolution[1],)
    assert pixels.v.shape == (camera.resolution[0],)
    assert pixels.u[6] == 0.5
    assert pixels.v[4] == 0.5
    assert pixels.azimuth.shape == camera.resolution
    assert_allclose(pixels.azimuth[4, 6], frame.azimuth, atol=1E-07)
    assert pixels.elevation.shape == camera.resolution
    assert_allclose(pixels.elevation[4, 6], frame.elevation, atol=1E-07)
