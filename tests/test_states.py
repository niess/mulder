import mulder
import numpy
from numpy.testing import assert_allclose
import pytest


STATES = (
    mulder.GeographicStates,
    mulder.LocalStates,
)


def test_constructors():
    """Test constructor functions."""

    for States in STATES:
        for meth in ("empty", "full", "zeros"):
            shape = []
            a = getattr(States, meth)(shape)
            assert a.shape == tuple(shape)
            assert a.size == 1
            assert a.ndim == 0
            assert a.pid == None

            shape = 3
            if meth == "full":
                fill_value = { "energy": 1.0, "pid": 13 }
                a = getattr(States, "full")(shape, **fill_value)
            else:
                a = getattr(States, meth)(shape)
            assert a.shape == (shape,)
            assert a.size == 3
            assert a.ndim == 1

            if meth == "full":
                for k, v in fill_value.items():
                    assert numpy.all(a[k] == v)

                b = getattr(States, "from_array")(a.array)
                assert numpy.all(a == b)
                a["energy"] = 0
                assert numpy.all(a != b)

                b = getattr(States, "from_array")(a.array, copy=False)
                assert numpy.all(a == b)
                a["energy"] = 0
                assert numpy.all(a == b)

            elif meth == "zeros":
                for ai in a:
                    assert isinstance(ai, States)
                    for aij in ai:
                        assert aij == 0

            shape = (1, 2, 3)
            a = getattr(States, meth)(shape)
            assert a.shape == shape
            assert a.size == 6
            assert a.ndim == 3

        energy = (1, 2, 3)
        a = States(energy=energy)
        assert a.size == 3
        assert a.shape == (3,)
        assert a.ndim == 1
        assert numpy.all(a["energy"] == energy)

        a = States.zeros()
        assert a.dtype() == a.array.dtype

        a = States.zeros(tagged=True)
        assert numpy.all(a["pid"] == 0)
        assert a.dtype(tagged=True) == a.array.dtype


def test_attributes():
    """Test attributes interface."""

    for States in STATES:
        if States == mulder.GeographicStates:
            attr = {
                "energy": (1, 2, 3),
                "latitude": (45, 48, 51),
                "longitude": (3, 5, 7),
                "altitude": (4, 5, 6),
                "azimuth": (-2, -3, -4),
                "elevation": (8, 9, 10),
                "weight": (3.6, 7.2, 5.9),
            }
        else:
            attr = {
                "energy": (1, 2, 3),
                "position": ((1, 2, 3), (4, 5, 6), (7, 8, 9)),
                "direction": ((-1, -2, -3), (-4, -5, -6), (-7, -8, -9)),
                "weight": (3.6, 7.2, 5.9),
            }

        states = States(**attr)
        for k, v in attr.items():
            assert numpy.all(states[k] == v)
            assert numpy.all(getattr(states,k) == v)
            states[k] = 0
            assert numpy.all(getattr(states,k) == 0)
            setattr(states, k, 1)
            assert numpy.all(states[k] == 1)

        with pytest.raises(ValueError):
            assert states["pid"] == None
        assert states.pid == None

        pid = (13, -13, 13)
        states = States(pid=pid)
        assert numpy.all(states["pid"] == pid)
        assert numpy.all(states.pid == pid)
        states["pid"] = 0
        assert numpy.all(states.pid == 0)
        states.pid = 1
        assert numpy.all(states["pid"] == 1)

    frame = mulder.LocalFrame(latitude=-33, longitude=65)
    states = mulder.LocalStates(frame=frame, **attr)
    assert states.frame == frame
    with pytest.raises(ValueError):
        states["frame"]


def test_conversions():
    """Test conversions between states."""

    def normed(v):
        return numpy.array(v) / numpy.linalg.norm(v)

    a = mulder.LocalStates(energy=(1, 2, 3), direction=normed((1, 0, 1)))
    b = a.to_geographic()
    assert_allclose(b.latitude, 45.0)
    assert_allclose(b.longitude, 0.0)
    assert_allclose(b.altitude, 0.0, atol=1E-07)
    assert_allclose(b.azimuth, 90.0)
    assert_allclose(b.elevation, 45.0)
    b = b.to_local(a.frame)
    assert_allclose(a.direction, b.direction, atol=1E-07)

    a = mulder.LocalStates(direction=normed((0, 1, 1)))
    b = a.to_geographic()
    assert_allclose(b.azimuth, 0.0, atol=1E-07)
    assert_allclose(b.elevation, 45.0)
    b = b.to_local(a.frame)
    assert_allclose(a.direction, b.direction, atol=1E-07)

    a = mulder.LocalStates(direction=normed((1, 1, 0)))
    b = a.to_geographic()
    assert_allclose(b.azimuth, 45.0)
    assert_allclose(b.elevation, 0.0, atol=1E-07)
    b = b.to_local(a.frame)
    assert_allclose(a.direction, b.direction, atol=1E-07)

    frame = mulder.LocalFrame(
        latitude=-33,
        longitude=68,
        altitude=102,
        declination=28,
        inclination=-15,
    )
    a = mulder.LocalStates(
        frame=frame,
        position=(1, 2, 3),
        direction=normed((3, 2, 1))
    )
    b = a.to_geographic().to_local(a.frame)
    assert_allclose(a.position, b.position, atol=1E-07)
    assert_allclose(a.direction, b.direction, atol=1E-07)

    a = mulder.LocalStates(position=(0, 0, 100))
    b = mulder.GeographicStates.from_local(a)
    assert_allclose(a.position[2], b.altitude)

    frame1 = mulder.LocalFrame()
    a0 = mulder.LocalStates(
        frame=frame,
        position=(1, 2, 3),
        direction=normed((3, 2, 1))
    )
    a1 = a0.transform(destination=frame1)
    b1 = a0.to_geographic().to_local(frame1)
    assert_allclose(a1.position, b1.position)
    assert_allclose(a1.direction, b1.direction)
