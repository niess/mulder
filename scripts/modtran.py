#! /usr/bin/env python3
import matplotlib.pyplot as plot
import numpy
from pathlib import Path
import re

import mulder


PREFIX = Path(__file__).parent

PRINT_DENSITIES = False

plot.style.use(PREFIX / "paper.mplstyle")


def load_constituents():
    """Load tables of atmospheric consituents."""

    molecules, data = [], []
    for index in ("a", "b", "c", "d"):
        path = PREFIX / f"data/table-2{index}.txt"
        with path.open() as f:
            header = f.readline()
        molecules += header.split()[3::2]
        d = numpy.loadtxt(path, comments="#")
        z = d[:,0]
        data.append(d[:,1:])

    # The elements below have been omitted from MODTRAN, since they are not
    # optically relevant. Yet, they contribute to the moale mass (at least for
    # Argon). We add them according to the US Standard atmosphere report (1976).
    others = (
        ("Ar", 9.4E+03),
        ("Ne", 1.818E+01),
        ("He", 5.24E+00),
    )
    for molecule, weight in others:
        molecules.append(molecule)
        data.append(numpy.full((z.size, 1), weight))

    data = numpy.hstack(data)

    constituents = {}
    for i, molecule in enumerate(molecules):
        molecule = molecule      \
            .replace("CL", "Cl") \
            .replace("BR", "Br")
        constituents[molecule] = data[:, i]
    return constituents


def load_profiles():
    """Load tables of atmospheric profiles."""

    profiles = {}
    for index in ("a", "b", "c", "d", "e", "f",):
        path = PREFIX / f"data/table-1{index}.txt"
        with path.open() as f:
            title = f.readline()[12:]
            header = f.readline()
        keys = header.split()[1:]
        data = numpy.loadtxt(path, comments="#")
        profile = {}
        for i, k in enumerate(keys):
            profile[k] = data[:,i]
        profiles[title] = profile
    return profiles


def compute_masses(molecules):
    """Compute the molar masses of chemical consituents."""

    # Atomic masses.
    # Ref: https://pdg.lbl.gov/2025/AtomicNuclearProperties/index.html
    atoms = {
        "H": 1.0087,
        "He": 4.0026022,
        "C": 12.01078,
        "N": 14.0072,
        "O": 15.9993,
        "Ne": 20.17976,
        "F": 18.9984031636,
        "P": 30.9737619985,
        "S": 32.0655,
        "Cl": 35.4532,
        "Ar": 39.9481,
        "Br": 79.9041,
        "I": 126.904473,
    }

    masses = {}
    pattern = re.compile("([A-Z][a-z]?)([0-9]*)")
    for molecule in molecules:
        m = 0.0
        for a, w in pattern.findall(molecule):
            w = int(w) if w else 1
            m += atoms[a] * w
        masses[molecule] = m
    return masses


def compute_mass_densities(profiles, constituents, masses):
    """Compute mass density profiles."""

    NA = 6.022E+23
    for profile in profiles.values():
        mole_density = profile["DENSITY"] / NA
        mole_mass = 0.0
        for ci, wi in constituents.items():
            try:
                wi = profile[ci]
            except KeyError:
                pass
            mole_mass += wi * masses[ci] * 1E-06
        profile["MOLE_MASS"] = mole_mass
        profile["MASS_DENSITY"] = mole_density * mole_mass


if __name__ == "__main__":
    constituents = load_constituents()
    masses = compute_masses(constituents.keys())
    profiles = load_profiles()
    compute_mass_densities(profiles, constituents, masses)

    models = (
        "Tropical", "MidlatitudeSummer", "MidlatitudeWinter", "SubarticSummer",
        "SubarticWinter", "USStandard",
    )
    z = numpy.linspace(0.0, 120E+03, 1201)

    cmap = plot.get_cmap("tab10")
    plot.figure(figsize=(10.24, 7.68))
    for i, (label, profile) in enumerate(profiles.items()):
        if PRINT_DENSITIES:
            densities = ", ".join(
                [f"{d * 1E+03:.5E}" for d in profile["MASS_DENSITY"]]
            )
            print(densities)
        color = cmap(i / 10)
        plot.plot(profile["ALT"], profile["MASS_DENSITY"], "o", color=color,
                  label=label)

        atmosphere = mulder.Atmosphere(models[i])
        rho = atmosphere.density(z) * 1E-03
        plot.plot(z * 1E-03, rho, "-", color=color)


    plot.yscale("log")
    plot.legend(fontsize=10)
    plot.xlabel("altitude (km)")
    plot.ylabel("density (g/cm$^3$)")
    plot.xlim(0.0, 120.0)
    plot.show()
