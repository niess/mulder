import datetime
import mulder
import numpy
from numpy.testing import assert_allclose
from pathlib import Path
import pytest


PREFIX = Path(__file__).parent


def test_constructor():
    """Test the constructor."""

    meter = mulder.Fluxmeter()
    assert isinstance(meter.geometry, mulder.EarthGeometry)
    assert len(meter.geometry.layers) == 0
    assert meter.atmosphere.material == "Air"

    meter = mulder.Fluxmeter(1.0)
    assert isinstance(meter.geometry, mulder.EarthGeometry)
    assert len(meter.geometry.layers) == 1
    assert meter.geometry.layers[0].material == "Rock"
    assert meter.geometry.layers[0].data == (1.0,)
    assert meter.atmosphere.material == "Air"

    meter = mulder.Fluxmeter(geometry=PREFIX / "assets/geometry.toml")
    assert isinstance(meter.geometry, mulder.LocalGeometry)

    meter = mulder.Fluxmeter(
        atmosphere = "midlatitude-winter",
        date = "2025-12-25",
        mode = "mixed",
        bremsstrahlung = "KKP95",
        seed = 123456,
        reference = "Gaisser90",
    )

    assert meter.atmosphere.model == "midlatitude-winter"
    assert meter.geomagnet.date == datetime.date(2025, 12, 25)
    assert meter.mode == "mixed"
    assert meter.physics.bremsstrahlung == "KKP95"
    assert meter.random.seed == 123456
    assert meter.reference.model == "Gaisser90"

    meter = mulder.Fluxmeter(reference = 1)
    assert meter.reference.model == 1

    with pytest.raises(TypeError) as e:
        meter = mulder.Fluxmeter(
            geomagnet=mulder.EarthMagnet(),
            date="2025-12-25"
        )
    meter = mulder.Fluxmeter(geomagnet=mulder.EarthMagnet(date="2025-12-25"))
    assert meter.geomagnet.date == datetime.date(2025, 12, 25)

    with pytest.raises(TypeError) as e:
        meter = mulder.Fluxmeter(
            physics=mulder.Physics(),
            bremsstrahlung="KKP95"
        )
    meter = mulder.Fluxmeter(physics=mulder.Physics(bremsstrahlung="KKP95"))
    assert meter.physics.bremsstrahlung == "KKP95"

    with pytest.raises(TypeError) as e:
        meter = mulder.Fluxmeter(
            random=mulder.Random(),
            seed=123456
        )
    meter = mulder.Fluxmeter(random=mulder.Random(seed=123456))
    assert meter.random.seed == 123456


def test_attributes():
    """Test the attributes."""

    meter = mulder.Fluxmeter()

    meter.atmosphere = "midlatitude-winter"
    assert meter.atmosphere.model == "midlatitude-winter"

    meter.atmosphere = mulder.Atmosphere("midlatitude-winter")
    assert meter.atmosphere.model == "midlatitude-winter"


def test_continuous():
    """Test continuous mode."""

    meter = mulder.Fluxmeter(0.0)
    assert meter.mode == "continuous"

    s1 = dict(altitude=-30.0, elevation=70.0)
    s0 = meter.transport(**s1)
    assert_allclose(s0.altitude, meter.reference.altitude)

    f1a = meter.reference.flux(s0) * s0.weight
    f1b = meter.flux(**s1)
    assert_allclose(f1a, f1b)

    meter = mulder.Fluxmeter(0.0, geomagnet=True)

    s1a = dict(altitude=-30.0, elevation=70.0, pid=13)
    s0a = meter.transport(**s1a)
    s1b = dict(altitude=-30.0, elevation=70.0, pid=-13)
    s0b = meter.transport(**s1b)
    f1a = meter.reference.flux(s0a) * s0a.weight + \
          meter.reference.flux(s0b) * s0b.weight
    f1b = meter.flux(**s1)
    assert_allclose(f1a, f1b)


def test_stochastic():
    """Test stochastic modes."""

    for mode in ("discrete", "mixed"):
        meter = mulder.Fluxmeter(0.0, mode=mode, seed=123456)
        assert meter.mode == mode

        s1 = dict(altitude=-30.0, elevation=70.0)
        N = 1000
        s0 = meter.transport(**s1, events=N)
        assert_allclose(s0.altitude, meter.reference.altitude)

        f1a = sum(meter.reference.flux(s0) * s0.weight) / N
        f1b, s1b = meter.flux(**s1, events=N)
        assert_allclose(f1a, f1b, rtol=5E-02)
