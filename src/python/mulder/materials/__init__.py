def init():
    from .._core import materials
    import sys

    this = sys.modules[__name__]
    setattr(this, "__all__", materials.__all__)
    for name in materials.__all__:
        setattr(this, name, getattr(materials, name))

init()
del init
