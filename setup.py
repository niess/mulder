import os
import re
import subprocess
from setuptools import setup


CLASSIFIERS = """\
Development Status :: 4 - Beta
Intended Audience :: Science/Research
License :: OSI Approved :: GNU Lesser General Public License v3 (LGPLv3)
Programming Language :: Python
Topic :: Scientific/Engineering :: Physics
Operating System :: POSIX :: Linux
"""


def main():
    with open("src/mulder.h") as f:
        version = ".".join(
            re.findall("#define MULDER_VERSION_[A-Z ]+([0-9]+)", f.read()))

    setup(
        name="mulder",
        version=version,
        author="Valentin Niess",
        author_email="valentin.niess@gmail.com",
        description="MUon fLux unDERwater",
        packages=['mulder'],
        classifiers=[s for s in CLASSIFIERS.split(os.linesep) if s.strip()],
        license='GPLv3',
        platforms=['Linux'],
        python_requires='>=3.6',
        setup_requires=['cffi>=1.0.0'],
        cffi_modules=['src/build-core.py:ffi'],
        install_requires=['cffi>=1.0.0', 'numpy'],
    )


if __name__ == '__main__':
    main()
