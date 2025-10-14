import mulder
import numpy
from numpy.testing import assert_allclose


MODELS = (
    "midlatitude-summer",
    "midlatitude-winter",
    "subartic-summer",
    "subartic-winter",
    "tropical",
    "us-standard"
)

TABLE = numpy.array((
    (      0, 1E+00),
    ( 10_000, 1E-01),
    (100_000, 1E-02),
))

def test_constructor():
    """Test constructor function."""

    # Test default constructor.
    atmosphere = mulder.Atmosphere()
    assert atmosphere.material == "Air"
    assert atmosphere.model == "us-standard"

    # Test predefined models.
    for model in MODELS:
        atmosphere = mulder.Atmosphere(model)
        assert atmosphere.material == "Air"
        assert atmosphere.model == model

    # Test table case.
    atmosphere = mulder.Atmosphere(TABLE)
    assert atmosphere.material == "Air"
    assert_allclose(atmosphere.model, TABLE)
    assert atmosphere.model.flags.writeable == False

    # Test material case.
    atmosphere = mulder.Atmosphere(MODELS[0], material="N2")
    assert atmosphere.material == "N2"
    assert atmosphere.model == MODELS[0]

    atmosphere = mulder.Atmosphere(TABLE, material="N2")
    assert atmosphere.material == "N2"
    assert_allclose(atmosphere.model, TABLE)
    assert atmosphere.model.flags.writeable == False


def test_density():
    """Test density method."""

    atmosphere = mulder.Atmosphere(TABLE)
    assert_allclose(atmosphere.density(TABLE[:,0]), TABLE[:,1])
    for z, rho in TABLE:
        assert atmosphere.density(z) == rho

    for i in range(len(TABLE) - 1):
        z = 0.5 * (TABLE[i + 1, 0] + TABLE[i, 0])
        assert atmosphere.density(z) < TABLE[i, 1]
        assert atmosphere.density(z) > TABLE[i + 1, 1]
