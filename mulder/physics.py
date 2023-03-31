"""Physics related utilities.
"""

import os
from pathlib import Path

from .ffi import lib, tostr


def generate_physics(path, destination=None):
    """Generate physics tables for Pumas."""

    pathdir = str(Path(path).parent)
    if destination is None:
        destination = pathdir

    if not os.path.exists(destination):
        os.makedirs(destination)

    dump = str(Path(destination) / Path(path).with_suffix(".pumas").name)

    rc = lib.mulder_generate_physics(
        tostr(path),
        tostr(destination),
        tostr(dump)
    )
    if rc != lib.MULDER_SUCCESS:
        raise LibraryError()

    if pathdir != destination:
        shutil.copy(path, destination)
