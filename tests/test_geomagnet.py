from datetime import date
import mulder
import numpy
from numpy.testing import assert_allclose
from pathlib import Path


def test_constructor():
    """Test constructor function."""

    # Test default constructor.
    geomagnet = mulder.EarthMagnet()
    assert isinstance(geomagnet.date, date)
    assert str(geomagnet.date) == "2025-06-21"
    assert isinstance(geomagnet.zlim, tuple)
    assert_allclose(geomagnet.zlim, (-1E+03, 6E+05))
    assert geomagnet.model == "IGRF14"

    # Test arguments.
    geomagnet = mulder.EarthMagnet(
        Path(mulder.config.PREFIX) / "data/magnet/IGRF14.COF",
        date = "1978-08-16"
    )
    assert isinstance(geomagnet.date, date)
    assert str(geomagnet.date) == "1978-08-16"
    assert isinstance(geomagnet.zlim, tuple)
    assert_allclose(geomagnet.zlim, (-1E+03, 6E+05))
    assert geomagnet.model == "IGRF14"


def test_field():
    """Test field method."""

    latitude, longitude = 45.8, 3.1
    geomagnet = mulder.EarthMagnet()
    frame = mulder.LocalFrame(latitude=latitude, longitude=longitude)
    field0 = geomagnet.field(frame=frame, position=[0, 0, 0])
    field1 = geomagnet.field(latitude=latitude, longitude=longitude)
    assert_allclose(field0, field1)
