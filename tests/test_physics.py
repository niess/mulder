import mulder


def test_physics():
    """Test the physics interface."""

    physics = mulder.Physics()
    rock = physics.compile("HumidRock")
    assert isinstance(rock.definition, mulder.materials.Composite)
