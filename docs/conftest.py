import mulder
import numpy
import pytest


DOCTEST_INITIALISED = False

def initialise_doctest():
    """Initialise the doctest environement."""

    DOCTEST_INITIALISED = True


@pytest.fixture(autouse=True)
def _docdir(request, doctest_namespace):

    doctest_plugin = request.config.pluginmanager.getplugin("doctest")
    if isinstance(request.node, doctest_plugin.DoctestItem):
        doctest_namespace["mulder"] = mulder
        doctest_namespace["np"] = numpy
        tmpdir = request.getfixturevalue("tmpdir")
        with tmpdir.as_cwd():
            if not DOCTEST_INITIALISED:
                initialise_doctest()
            yield
    else:
        yield
