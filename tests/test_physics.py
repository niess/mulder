import mulder


def test_physics():
    """Test the physics interface."""

    physics = mulder.Physics(cutoff=1E-02)
    assert physics.bremsstrahlung == "SSR19"
    assert physics.pair_production == "SSR19"
    assert physics.photonuclear == "DRSS01"
    assert physics.cutoff == 1E-02
    assert physics.elastic_ratio == 5E-02

    physics.cutoff = 5E-2
    assert physics.cutoff == 5E-02

    rock = physics.compile("HumidRock")
    assert isinstance(rock.definition, mulder.materials.Composite)

    s = rock.stopping_power(1.0)
    assert s.size == 1
    assert s.shape == ()

    s = rock.stopping_power((1.0, 2.0))
    assert s.size == 2
    assert s.shape == (2,)
