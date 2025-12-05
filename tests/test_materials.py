import mulder.materials as materials
from numpy.testing import assert_allclose
import os
from pathlib import Path
from tempfile import TemporaryDirectory


PREFIX = Path(__file__).parent


def test_composite():
    """Test the composite interface."""

    composite = materials.Composite.define(
        "Composite", composition=("Rock", "Water")
    )
    assert composite.composition == ("Rock", "Water")
    assert_allclose(composite.density, 1473.02, atol=0.01)

    composite = materials.Composite.define("Composite",
        composition=(("Rock", 1.0), ("Water", 0.0))
    )
    assert composite.density == 2.65E+03

    composite["Rock"] = 0.0
    composite["Water"] = 1.0
    assert composite.density == 1.02E+03

    composites = materials.Composite.all()
    assert "Composite" in composites


def test_element():
    """Test the element interface."""

    H = materials.Element("H")
    assert H.Z == 1
    assert H.A == 1.008
    assert H.I == 19.2E-09

    U_238 = materials.Element.define("U-238", Z=92, A=238.0508, I=890E-09)
    assert U_238.Z == 92
    assert U_238.A == 238.0508
    assert_allclose(U_238.I, 890E-09)

    elements = materials.Element.all()
    assert "U-238" in elements


def test_base():
    """Test the base interface."""

    rock = materials.Material("Rock")
    assert rock.composition == (("Rk", 1.0),)
    assert rock.density == 2.65E+03
    assert rock.I == None

    water = materials.Material("Water")
    assert water.composition[0][0] == "H"
    assert water.composition[1][0] == "O"
    assert_allclose(
        [c[1] for c in water.composition],
        [0.111894, 0.888106],
        atol = 1E-04
    )
    assert water.density == 1.02E+03
    assert water.I == None

    air = materials.Material("Air")
    assert [c[0] for c in air.composition] == ["Ar", "C", "N", "O"]
    assert air.density == 1.205
    assert air.I == None

    base = materials.Material.define(
        "TestRock",
        composition = {
            "Rock": 0.95,
            "Ar": 0.05,
        },
        density = 2.0E+03,
        I = 120E-09
    )
    assert base.composition[0][0] == "Ar"
    assert base.composition[1][0] == "Rk"
    assert_allclose(
        [c[1] for c in base.composition],
        [0.05, 0.95]
    )
    assert base.density == 2E+03
    assert base.I == 120E-09

    base2 = materials.Material.define(
        "TestRock2",
        composition = (
            ("Rock", 0.95),
            ("Ar", 0.05),
        ),
        density = 2.0E+03,
        I = 120E-09
    )
    assert base.composition == base2.composition

    H2O = materials.Material.define(
        "PureWater", composition="H2O", density=1E+03
    )
    assert H2O.composition == water.composition

    bases = materials.Material.all()
    assert "PureWater" in bases


def test_dump():
    """Test the dump function."""

    cwd = os.getcwd()
    with TemporaryDirectory() as d:
        os.chdir(d)
        materials.dump("materials.toml")
        materials.load("materials.toml")
        os.chdir(cwd)  # Must be before tmpdir removal, on Windows.


def test_load():
    """Test the load function."""

    materials.load(PREFIX / "assets/materials.toml")

    base = materials.Material("MoistAir")
    assert [c[0] for c in base.composition] == ["Ar", "C", "H", "N", "O"]
    assert base.density == 1.2

    composite = materials.Composite("HumidRock")
    assert composite.composition == ("Rock", "Water")
