> [!CAUTION]  
> Mulder is currently being ported to Rust. See the [cffi
> branch](https://github.com/niess/mulder/tree/cffi) for previous version 0.1.0.

# The Mulder library

(_MUon fLuxme'DER_)


## Description

Mulder is utility library for computing local changes in the flux of atmospheric
muons due to geophysical features, e.g. due to a topography described by a
Digital Elevation Model (DEM).

The master component of Mulder is a fluxmeter, which behaves as a portable probe
of the flux of atmospheric muons. The level of details of fluxmeters is
configurable, from a fast continuous approximation delivering an average flux
estimate, to a detailed Monte Carlo delivering discrete (weighted) atmospheric
muon events (at the observation point).

Note that Mulder only simulates the transport of atmospheric muons, taking into
account the local features surrounding the fluxmeter. That is, Mulder does
not simulate muons production at height. Instead, a (configurable) reference
spectrum of atmospheric muons is used as input, providing the opensky flux, i.e.
the flux in the absence of any ground or other obstacles than the Earth
atmosphere.


## Installation

### From PyPI

On Linux, the `mulder` Python package can be installed with `pip` as

```
pip install mulder
```

Note that Python (3.8, or more) is required (thus you might need to use `pip3`
instead of `pip`, depending on your system).


### From source

Mulder source is available from [GitHub](https://github.com/niess/mulder). Note
that the library depends on other projects (_git submodules_), the like
[Pumas][PUMAS] and [Turtle][TURTLE]. The complete tree, including dependencies,
can be cloned as

```
git clone --recursive https://github.com/niess/mulder.git
```

(_note the `--recursive` in the previous command._)

Then, building the Python package requires the [Rust
toolchain](https://www.rust-lang.org/tools/install).


## Usage

The Mulder library is still in alpha stage. As so, there is no dedicated
documentation.


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
