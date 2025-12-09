import mulder
import numpy
from numpy.testing import assert_allclose
import pytest


DEFAULT_RATIO = 1.2766

TABLE = numpy.zeros((2, 2, 2))
TABLE[0,0,:] = (0.5, 1.0)
TABLE[0,1,:] = (1.0, 2.0)
TABLE[1,0,:] = (2.0, 4.0)
TABLE[1,1,:] = (4.0, 8.0)


def test_parametric():
    """Test parametric models."""

    models = ("GCCLY15", "Gaisser90")
    for model in models:
        reference = mulder.Reference(model)
        assert_allclose(reference.energy, (1E-03, 1E+12))
        assert_allclose(reference.altitude, 0)
        assert_allclose(reference.elevation, (0, 90))

        f0 = reference.flux()
        f1 = reference.flux(pid=13)
        f2 = reference.flux(pid=-13)
        assert_allclose(f0, f1 + f2)
        assert_allclose(f2 / f1, DEFAULT_RATIO)

        with pytest.raises(TypeError) as e:
            reference = mulder.Reference(model, altitude=5)
        assert 'altitude' in str(e.value)


def test_flat():
    """Test flat model."""

    reference = mulder.Reference(2)
    assert_allclose(reference.energy, (1E-03, 1E+12))
    assert_allclose(reference.altitude, (-11E+03, 120E+03))
    assert_allclose(reference.elevation, (-90, 90))

    f0 = reference.flux()
    f1 = reference.flux(pid=13)
    f2 = reference.flux(pid=-13)
    assert_allclose(f0, 2.0)
    assert_allclose(f0, f1 + f2)
    assert_allclose(f2 / f1, DEFAULT_RATIO)


def test_tabulated():
    """Test tabulated model."""

    ALTITUDE = (-1, 1)
    TEST_CASES = (
        ((2, 2), None, 1.0, DEFAULT_RATIO),
        ((2, 2, 2), None, 2.0, 1.0),
        ((2, 2, 2), ALTITUDE, 1.0, DEFAULT_RATIO),
        ((2, 2, 2, 2), ALTITUDE, 2.0, 1.0),
    )

    for (shape, altitude, expected, ratio) in TEST_CASES:
        reference = mulder.Reference(
            numpy.ones(shape),
            energy=(0.1, 10),
            altitude=altitude,
        )
        f0 = reference.flux()
        f1 = reference.flux(pid=13)
        f2 = reference.flux(pid=-13)
        assert_allclose(f0, expected, atol=1E-07)
        assert_allclose(f0, f1 + f2, atol=1E-07)
        assert_allclose(f2 / f1, ratio, atol=1E-04)

    energy = numpy.geomspace(1E-03, 1E+12, 151)
    cos_theta = numpy.linspace(0, 1.0, 51)
    X, Y = numpy.meshgrid(energy, cos_theta)
    reference0 = mulder.Reference("GCCLY15")
    elevation = 90.0 - numpy.degrees(numpy.arccos(Y[:]))
    flux = reference0.flux(energy=X[:], elevation=elevation)
    table = numpy.empty((51, 151, 2))
    table[:,:,0] = flux / (1 + DEFAULT_RATIO)
    table[:,:,1] = flux * DEFAULT_RATIO / (1 + DEFAULT_RATIO)

    reference1 = mulder.Reference(
        table,
        energy=(energy[0], energy[-1]),
        cos_theta=(cos_theta[0], cos_theta[-1]),
    )

    assert_allclose(reference1.energy, (energy[0], energy[-1]))
    assert_allclose(reference1.elevation, (0.0, 90.0))
    assert reference1.altitude == 0.0

    TEST_VALUES = [
        (1.5E+00, 47),
        (3.0E+02, 15),
        (5.0E+06, 87),
        (2.0E+09, 55),
    ]

    for (energy, elevation) in TEST_VALUES:
        kwargs = dict(energy=energy, elevation=elevation)
        f0 = reference0.flux(**kwargs)
        f1 = reference1.flux(**kwargs)
        f2 = reference1.flux(**kwargs, pid=13)
        f3 = reference1.flux(**kwargs, pid=-13)
        assert_allclose(f0, f1, rtol=1E-02)
        assert_allclose(f1, f2 + f3)
        assert_allclose(f3 / f2, DEFAULT_RATIO, atol=1E-04)
