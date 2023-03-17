"""Matplotlib related utilities
"""
from types import MethodType

import matplotlib.colors as colors
import matplotlib.pyplot as plot
import matplotlib.transforms as transforms
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


def set_cursor_data(image, data):
    """This function overrides cursor data.

    This is achieved by monkey patching the AxesImage.get_cursor_data. Note that
    data must have the same width and length than the image array.
    """

    arr = image.get_array()
    assert(data.shape[0] == arr.shape[0])
    assert(data.shape[1] == arr.shape[1])

    def get_cursor_data(self, event):
        """Patched method, modified from matplotlib source code."""

        xmin, xmax, ymin, ymax = self.get_extent()
        if self.origin == 'upper':
            ymin, ymax = ymax, ymin
        arr = self.get_array()
        data_extent = transforms.Bbox([[xmin, ymin], [xmax, ymax]])
        array_extent = transforms.Bbox([[0, 0], [arr.shape[1], arr.shape[0]]])
        trans = self.get_transform().inverted()
        trans += transforms.BboxTransform(
            boxin = data_extent,
            boxout = array_extent
        )
        point = trans.transform([event.x, event.y])
        if any(numpy.isnan(point)):
            return None
        j, i = point.astype(int)
        # Clip the coordinates at array bounds
        if not (0 <= i < arr.shape[0]) or not (0 <= j < arr.shape[1]):
            return None
        else:
            return data[i, j]

    # Monkey patch the AxisImage.get_cursor_data
    image.get_cursor_data = MethodType(get_cursor_data, image)
