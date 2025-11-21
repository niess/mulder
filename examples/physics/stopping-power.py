#! /usr/bin/env python3
import matplotlib.pyplot as plt
import mulder
import mulder.materials as materials
import numpy as np


# Define the materials.
composite = materials.Composite("HumidRock", composition=("Rock", "Water"))
for i, f in enumerate((0.25, 0.50, 0.75)):
    composite["Rock"] = 1.0 - f
    composite["Water"] = f
    materials.Material(
        f"HRM_{i}",
        composition=(("Rock", 1.0 - f), ("Water", f)),
        density = composite.density
    )

# Compile the material tables.
physics = mulder.Physics()
compiled = { c.name: c for c in physics.compile() }

# Plot the stopping power for a variety of water fractions.
energy = np.geomspace(1E-02, 1E+03, 501)

plt.figure()
for f in np.linspace(0.0, 1.0, 101):
    composite["Rock"] = 1.0 - f
    composite["Water"] = f
    stopping_power = compiled["HumidRock"].stopping_power(energy)
    color = (1.0 - f, 0.0, f)
    label = "Rock" if f == 0.0 else "Water" if f == 1.0 else None
    plt.plot(energy, stopping_power, "-", color=color, label=label)
plt.xscale("log")
plt.xlabel("kinetic energy (GeV)")
plt.ylabel("stopping-power (GeV / m)")
plt.legend()

# Plot the differences in stopping power between the atomic and macroscopic
# mixtures.
plt.figure()
for i in range(3):
    material = materials.Material(f"HRM_{i}")
    composition = dict(material.composition)
    f = 1.0 - composition["Rk"]
    composite["Rock"] = 1.0 - f
    composite["Water"] = f
    S0 = compiled["HumidRock"].stopping_power(energy)

    S1 = compiled[f"HRM_{i}"].stopping_power(energy)
    delta = S1 / S0 - 1.0
    color = (1.0 - f, 0.0, f)
    plt.plot(energy, 100.0 * delta, "-", color=color, label=f"f = {f:.2f}")
plt.xscale("log")
plt.xlabel("kinetic energy (GeV)")
plt.ylabel("relative difference (%)")
plt.legend()

plt.show()
