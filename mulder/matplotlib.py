"""Matplotlib related utilities
"""
import matplotlib.colors as colors
import matplotlib.pyplot as plot
import numpy


class TerrainColormap(colors.LinearSegmentedColormap):
    """Terrain colormap for representing joint topography and bathymetry data.
    """

    def __init__(cls, name=None, N=None):
        """
        Return a modified terrain colormap that has land and ocean clearly
        delineated and of the same length.

        The code below was adapated from Matplotlib's TwoSlopeNorm example.
        """

        if name is None: name = "mulder.terrain"
        if N is None: N = 256

        # Build color table
        cmap = plot.cm.terrain
        half = N // 2
        colors = numpy.empty((N, 4))
        colors[:half,:] = cmap(numpy.linspace(0.00, 0.2, half))
        colors[half:,:] = cmap(numpy.linspace(0.25, 1.0, half))
        r, g, b, a = colors.T

        # Reformat as a color dictionary
        x = numpy.linspace(0, 1, N)
        cdict = {
            "red":   numpy.column_stack((x, r, r)),
            "green": numpy.column_stack((x, g, g)),
            "blue":  numpy.column_stack((x, b, b)),
            "alpha": numpy.column_stack((x, a, a))
        }

        super().__init__(name, cdict, N)


class TerrainNorm(colors.TwoSlopeNorm):
    """Two slope norm consistent with modified terrain colormap."""

    def __init__(self, vmin=None, vmax=None, sealevel=None):
        if sealevel is None: sealevel = 0
        super().__init__(vcenter=sealevel, vmin=vmin, vmax=vmax)


class LightSource:
    """Light source for colorizing images with specular effects.
    """

    @property
    def intensity(self):
        """Intensity of ambiant lighting (in [0, 1])"""
        return self._intensity

    @intensity.setter
    def intensity(self, v):
        self._intensity = numpy.clip(v, 0, 1)

    @property
    def direction(self):
        """Direction of specular lighting"""
        return self._direction

    @direction.setter
    def direction(self, v):
        self._direction[:] = v

    def __init__(self, intensity=None, direction=None):
        self._intensity = 0
        self._direction = numpy.zeros(3)

        if intensity is None: intensity = 0.5
        if direction is None: direction = (-1, -1, -1)

        self.intensity = intensity
        self.direction = direction

    def colorize(self, data, normal, viewpoint=None, cmap=None, norm=None,
                 vmin=None, vmax=None):
        """Colorize data using a combination of ambiant and specular lights.
        """

        assert(isinstance(data, numpy.ndarray))
        assert(isinstance(normal, numpy.ndarray))

        if cmap is None:
            cmap = TerrainColormap()
        elif isinstance(cmap, str):
            cmap = plot.get_cmap(cmap)

        if norm is None:
            if isinstance(cmap, TerrainColormap):
                norm = TerrainNorm(vmin=vmin, vmax=vmax)
            else:
                norm = colors.Normalize(vmin=vmin, vmax=vmax)

        if viewpoint is None:
            viewpoint = numpy.array((-1, -1, 1))
        else:
            viewpoint = numpy.asarray(viewpoint)

        # Compute cosine of specular reflection direction with normal
        ux, uy, uz = self._direction / numpy.linalg.norm(self._direction)
        nx, ny, nz = normal.T / numpy.linalg.norm(normal, axis=1)
        vx, vy, vz = viewpoint.T / numpy.linalg.norm(viewpoint,
                                                     axis=viewpoint.ndim - 1)
        nu = ux * nx + uy * ny + uz * nz
        rx = ux - 2 * nu * nx
        ry = uy - 2 * nu * ny
        rz = uz - 2 * nu * nz
        c = rx * vx + ry * vy + rz * vz

        # Scattered intensity model
        r = numpy.clip(0.5 * (1 + c), 0, 1)

        # Combine specular and ambiant light
        clrs = cmap(norm(data.flatten()))
        tmp = numpy.outer(r, (1, 1, 1))
        clrs[:,:3] *= self._intensity + (1 - self._intensity) * tmp

        return clrs
