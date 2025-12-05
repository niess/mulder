import mulder
import os
from pathlib import Path


def test_config():
    """Test config data."""

    # Test the cache.
    HOME = os.getenv("HOME")
    default_cache = Path(HOME) / ".cache/mulder"
    assert isinstance(mulder.config.CACHE, Path)
    assert mulder.config.CACHE == default_cache

    cache = Path("/tmp/mulder")
    mulder.config.CACHE = cache
    assert mulder.config.CACHE == cache

    mulder.config.CACHE = default_cache

    # Test the notify flag.
    assert mulder.config.NOTIFY == True
    mulder.config.NOTIFY = False
    assert mulder.config.NOTIFY == False
    mulder.config.NOTIFY = True

    # Test static data.
    assert isinstance(mulder.config.PREFIX, Path)
    assert isinstance(mulder.config.VERSION, str)
