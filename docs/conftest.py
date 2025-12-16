import mulder
import mulder.materials as materials
import numpy
from numpy.testing import assert_allclose
from pathlib import Path
import pytest
import shutil


DOCTEST_INITIALISED = False

PREFIX = Path(__file__).parent.parent

def initialise_doctest():
    """Initialise the doctest environement."""

    shutil.copyfile(
        PREFIX / "tests/assets/dem.asc",
        "./dem.asc"
    )
    shutil.copyfile(
        PREFIX / "tests/assets/geometry.toml",
        "./geometry.toml"
    )
    shutil.copyfile(
        PREFIX / "tests/assets/materials.toml",
        "./materials.toml"
    )
    shutil.copyfile(
        Path(mulder.config.PREFIX) / "data/magnet/IGRF14.COF",
        "./IGRF14.COF"
    )
    DOCTEST_INITIALISED = True


@pytest.fixture(autouse=True)
def _docdir(request, doctest_namespace):

    doctest_plugin = request.config.pluginmanager.getplugin("doctest")
    if isinstance(request.node, doctest_plugin.DoctestItem):
        doctest_namespace["materials"] = materials
        doctest_namespace["mulder"] = mulder
        doctest_namespace["np"] = numpy
        doctest_namespace["assert_allclose"] = assert_allclose
        tmpdir = request.getfixturevalue("tmpdir")
        with tmpdir.as_cwd():
            if not DOCTEST_INITIALISED:
                initialise_doctest()
            yield
    else:
        yield
