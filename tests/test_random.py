import mulder


def test_Random():
    """Test the Random interface."""

    rng = mulder.Random()
    assert rng.index == 0

    rng = mulder.Random(1)
    assert rng.seed == 1
    assert rng.index == 0
    v = rng.uniform01(10)

    rng = mulder.Random(1, index=9)
    assert rng.seed == 1
    assert rng.index == 9
    assert rng.uniform01() == v[-1]

    rng.seed = 0
    assert (rng.uniform01(10) == v).all()

    rng.index = 5
    assert (rng.uniform01(5) == v[5:]).all()
