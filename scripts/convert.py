#! /usr/bin/env python3
import argparse
from pathlib import Path
from typing import Optional

from geotiff import GeoTiff
import mulder
import numpy


def convert(path: Path, offset: Optional[float]=None):
    """Convert GeoTIFF data to Turtle PNG"""

    assert(path.suffix == ".tif") # Input file must be in GeoTIFF format
    g = GeoTiff(path)

    known_crs = {
        2154: "Lambert 93"
    }
    try:
        projection = known_crs[g.crs_code]
    except KeyError:
        raise ValueError(f"unknown CRS ({g.crs_code})")

    data = numpy.array(g.read())
    if offset is not None:
        data = data + offset

    ny, nx = g.tif_shape
    (xmin, ymin), (xmax, ymax) = g.tif_bBox

    path = path.with_suffix(".png")
    mulder.create_map(str(path), projection, (xmin, xmax), (ymin, ymax),
                      data)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(prog="convert",
        description="Convert GeoTIFF data to Turtle PNG")
    parser.add_argument("path", help="path to the initial GeoTIFF file")
    parser.add_argument("--offset", help="any altitude offset", type=float)

    args = parser.parse_args()
    convert(Path(args.path), args.offset)
