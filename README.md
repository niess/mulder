# The Mulder library

(_MUon fLux unDER_)


## Description

Mulder is utility library for computing local deformations of the flux of
atmospheric due to geophysical features, e.g. taking into account a local
topography described by a Digital Elevation Model (DEM).

The master component of Mulder is a fluxmeter, which behaves as a portable probe
of the local flux of atmospheric muons. The level of details of fluxmeters is
configurable, from a fast continuous approximation delivering an average flux
estimate, to a detailed Monte Carlo delivering discrete (weighted) atmospheric
muon events (at the observation point).

Note that Mulder only simulates the transport of atmospheric muons, taking into
account the local features surrounding the fluxmeter. That is, Mulder does
not simulate muons production at height. Instead, a (configurable) reference
spectrum of atmospheric muons is used as input, providing the opensky flux, i.e.
the flux in the absence of any ground or other obstacles than the Earth
atmosphere.

Mulder has a high level Python 3 interface, allowing one to configure and run
the core C library (`libmulder`). The C library can also be used directly, for
example as a generator of atmospheric muons for a C/C++ detector simulation.
Note however that the C library is bare-bones. High level customization
operations are intended to be done from Python. The C library only loads tables
at initialisation (e.g. produced from Python), and then it runs (produces muon
events) accordingly.


## Installation

### From PyPI

On Linux, the `mulder` Python package can be installed with `pip` as

```
pip install mulder
```

Note that Python (3.6, or more) is required (thus you might need to use `pip3`
instead of `pip`, depending on your system). The C library is bundled with the
Python package. In order to compile C/C++ projects, the `mulder` executable,
shipped with the Python package might be helpful. For example, the following
returns installation dependant compilation flags

```
mulder config --cflags --libs
```


### From source

Mulder source is available from [GitHub](https://github.com/niess/mulder). Note
that the library depends on other projects (_git submodules_), the like
[Pumas][PUMAS] and [Turtle][TURTLE]. The complete tree, including dependencies,
can be cloned as

```
git clone --recursive https://github.com/niess/mulder.git
```

(_note the `--recursive` in the previous command._)

Then, the Python package can be built with the provided Makefile, as

```
cd mulder
make package
```

This builds the package locally. Thus, you might also add the corresponding path
to your `PYTHONPATH`, e.g. as following in bash

```
export PYTHONPATH=$PWD:$PYTHONPATH
```

The C example(s) can be compiled as

```
make examples
```


## Usage

The Mulder library is currently in alpha stage. As so, there is no dedicated
documentation. However, the Python package has detailed [examples][EXAMPLES]. A
brief example of usage in C is also provided.


## License

The Mulder library is under the GNU LGPLv3 license. See the provided
[LICENSE][LICENSE] and [COPYING.LESSER][COPYING] files. The [examples][EXAMPLES]
however have a separate public domain license allowing them to be copied without
any restriction.


[COPYING]: https://github.com/niess/mulder/blob/master/COPYING.LESSER
[EXAMPLES]: https://github.com/niess/mulder/tree/master/examples
[LICENSE]: https://github.com/niess/mulder/blob/master/LICENSE
[PUMAS]: https://github.com/niess/pumas
[TURTLE]: https://github.com/niess/turtle
