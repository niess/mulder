def init():
    from . import _core
    import sys

    for submodule in ("materials", "picture"):
        _core.__all__.remove(submodule)

    this = sys.modules[__name__]
    setattr(this, "__all__", _core.__all__)
    for name in _core.__all__:
        setattr(this, name, getattr(_core, name))

init()
del init
