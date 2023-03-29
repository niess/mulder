"""Specialised pseudo random generators.
"""

import numpy


class Generator:
    """Generic generator."""

    @property
    def prng(self):
        """Core Pseudo-Random Numbers Generator (PRNG)."""
        return self._prng

    def __init__(self, prng):
        self._prng = prng

    def pdf(self, x):
        """Return the Probability Density Function (PDF) values at x."""
        return 1 / self.invpdf(x)


class Uniform(Generator):
    """Uniform generator over a range (x0, x1)."""

    def __init__(self, prng, x0, x1):
        super().__init__(prng)
        self._dx = x1 - x0
        self._x0 = x0

    def invpdf(self, x):
        """Return the inverse of the Probability Density Function (PDF) values
           at x.
        """
        try:
            size = len(x)
        except TypeError:
            size = None
        return numpy.full(size, abs(self._dx))

    def __call__(self, n=None):
        """Get numbers pseudo-uniformly distributed overs (x0, x1)."""
        return self._x0 + self._dx * self._prng(n)


class LogUniform(Generator):
    """Log-uniform generator over a range (x0, x1)."""

    def __init__(self, prng, x0, x1):
        assert(x0 * x1 > 0)
        super().__init__(prng)
        self._rx = numpy.log(x1 / x0)
        self._x0 = x0

    def invpdf(self, x):
        """Return the inverse of the Probability Density Function (PDF) values
           at x.
        """
        return abs(self._rx * x)

    def __call__(self, n=None):
        """Get numbers log-uniformly distributed overs (x0, x1)."""
        return self._x0 * numpy.exp(self._rx * self._prng(n))


class SinUniform(Generator):
    """Sine-uniform generator over a range (x0, x1), in deg."""

    def __init__(self, prng, x0, x1):
        super().__init__(prng)
        x0 = numpy.sin(numpy.radians(x0))
        x1 = numpy.sin(numpy.radians(x1))
        self._dx = x1 - x0
        self._x0 = x0

    def invpdf(self, x):
        """Return the inverse of the Probability Density Function (PDF) values
           at x.
        """
        try:
            size = len(x)
        except TypeError:
            size = None
        return numpy.full(size, abs(self._dx))

    def __call__(self, n=None):
        """Get numbers sine-uniformly distributed overs (x0, x1), in deg."""
        s = self._x0 + self._dx * self._prng(n)
        return numpy.degrees(numpy.arcsin(s))
