# The Mulder package <img src="https://github.com/niess/mulder/blob/master/docs/source/_static/images/logo.svg" height="30px"> [![][RTD_BADGE]][RTD]

(_MUon fLuxme'DER_)


## Description

Mulder is a Python package that calculates local variations in the flux of
atmospheric muons caused by geophysical features, as depicted by a Digital
Elevation Model ([DEM][DEM]).

The primary component of Mulder is a fluxmeter, which serves as a muons probe.
The level of detail of fluxmeters is adjustable, ranging from a rapid continuous
approximation, that provides an average flux estimate, to a comprehensive Monte
Carlo, that delivers discrete atmospheric muons.

Please refer to the online [documentation][RTD] for further information.


## Installation

Binary distributions of Mulder are available from [PyPI][PyPI], for Linux, OSX
and Windows as

```
pip3 install mulder
```

Alternatively, in order to build Mulder from the source, the [Rust
toolchain][RUST_TOOLCHAIN] is required. Please refer to the corresponding
[documentation][RTD_INSTALLATION] entry for further instructions.


## License

The Mulder package is under the GNU LGPLv3 license. See the provided
[LICENSE][LICENSE] and [COPYING.LESSER][COPYING] files.


[COPYING]: https://github.com/niess/mulder/blob/master/COPYING.LESSER
[DEM]: https://en.wikipedia.org/wiki/Digital_elevation_model
[EXAMPLES]: https://github.com/niess/mulder/tree/master/examples
[LICENSE]: https://github.com/niess/mulder/blob/master/LICENSE
[PUMAS]: https://github.com/niess/pumas
[PyPI]: https://pypi.org/project/mulder/
[RTD]: https://mulder.readthedocs.io/en/latest/?badge=latest
[RTD_BADGE]: https://readthedocs.org/projects/mulder/badge/?version=latest
[RTD_INSTALLATION]: https://mulder.readthedocs.io/en/latest/installation.html
[RUST_TOOLCHAIN]: https://www.rust-lang.org/tools/install
[TURTLE]: https://github.com/niess/turtle
