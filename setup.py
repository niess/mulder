import os
import re
import subprocess
from setuptools import setup
import subprocess


CLASSIFIERS = """\
Development Status :: 3 - Alpha
Intended Audience :: Science/Research
License :: OSI Approved :: GNU Lesser General Public License v3 (LGPLv3)
Programming Language :: Python :: 3
Topic :: Scientific/Engineering :: Physics
Operating System :: POSIX :: Linux
"""


def main():
    # Get C-library version
    with open("src/mulder.h") as f:
        version = ".".join(
            re.findall("#define MULDER_VERSION_[A-Z ]+([0-9]+)", f.read()))

    # Get git revision hash
    p = subprocess.Popen(
        'git describe --match=NeVeRmAtCh --always --dirty 2> /dev/null || '
            'echo unknown',
        shell=True, stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT)
    stdout, _ = p.communicate()
    try:
        stdout = stdout.decode()
    except AttributeError:
        stdout = str(stdout)
    git_revision = stdout.strip()

    # Dump Python version.py submodule
    with open("mulder/version.py", "w+") as f:
        f.write(f"""\
# This file was generated by setup.py
git_revision = "{git_revision:}"
version = "{version:}"
""")

    try:
        package_data = \
            ["data/" + file_ for file_ in os.listdir("mulder/data")] + \
            ["include/" + file_ for file_ in os.listdir("mulder/include")] + \
            ["lib/" + file_ for file_ in os.listdir("mulder/lib")]
    except FileNotFoundError:
        package_data = []

    with open('README.md') as f:
        long_description = f.read()

    setup(
        name="mulder",
        version=version,
        author="Valentin Niess",
        author_email="valentin.niess@gmail.com",
        description="MUon fLux unDER",
        long_description=long_description,
        long_description_content_type="text/markdown",
        url="https://github.com/niess/mulder",
        download_url = 'https://pypi.python.org/pypi/mulder',
        project_urls = {
            "Bug Tracker" : "https://github.com/niess/mulder/issues",
            "Source Code" : "https://github.com/niess/mulder",
        },
        packages=["mulder"],
        classifiers=[s for s in CLASSIFIERS.split(os.linesep) if s.strip()],
        license='GPLv3',
        platforms=["Linux"],
        python_requires=">=3.6",
        setup_requires=["cffi>=1.0.0", "pcpp>=1.0"],
        cffi_modules=["src/build-wrapper.py:ffi"],
        install_requires=["cffi>=1.0.0", "numpy"],
        include_package_data = True,
        package_data = {"": package_data},
        entry_points = {
            "console_scripts" : (
                "mulder = mulder.__main__:main",)
        }
    )


if __name__ == '__main__':
    main()
