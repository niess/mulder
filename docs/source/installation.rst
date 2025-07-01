Installing Mulder
=================

.. topic:: Python version

   Mulder requires Python **3.8** or higher.


From PyPI
---------

Binary distributions of Mulder are available from `PyPI`_, for Linux, OSX and
Windows as

.. code:: bash

   pip3 install mulder


From source
-----------

Mulder source is available from `GitHub`_, e.g. as

.. code:: bash

   git clone --recursive https://github.com/niess/mulder

To build Mulder from source, you will require the `Rust toolchain`_ with a
**C99** compliant compiler. Mulder is built as a Rust shared library, using
`PyO3`_. For example, on Linux, the following commands builds the Mulder package
in-source (under `src/python/mulder
<https://github.com/niess/mulder/tree/master/src/python/mulder>`_).

.. code:: bash

   # Build the Mulder binary.
   cargo build --release

   # Link the resulting binary.
   ln -rs target/release/libmulder.so src/python/mulder/mulder.so

.. caution::

   The GNU toolchain must be used on Windows, as MSVC does not support C99. For
   example, set `CARGO_BUILD_TARGET=x86_64-pc-windows-gnu` for an `x86_64`
   machine.


.. ============================================================================
.. 
.. URL links.
.. 
.. ============================================================================

.. _GeoTIFF: https://en.wikipedia.org/wiki/GeoTIFF
.. _GitHub: https://github.com/niess/mulder/
.. _PyO3: https://pyo3.rs/
.. _PyPI: https://pypi.org/project/mulder/
.. _Rust toolchain: https://www.rust-lang.org/tools/install
