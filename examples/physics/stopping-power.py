#! /usr/bin/env python3
import matplotlib.pyplot as plt
import mulder
import mulder.materials as materials
import numpy as np


composite = materials.Composite("HumidRock", composition=("Rock", "Water"))
material = materials.Material(
    "WashedRock",
    composition=(("Rock", 0.5), ("Water", 0.5)),
    density = composite.density
)

energy = np.geomspace(1E-02, 1E+03, 501)

physics = mulder.Physics()
compiled = physics.compile("HumidRock", "WashedRock")

plt.figure()
for i, f in enumerate(np.linspace(0.0, 1.0, 11)):
    composite["Rock"] = 1.0 - f
    composite["Water"] = f
    stopping_power = compiled[0].stopping_power(energy) * 1E+04
    plt.plot(energy, stopping_power, "-", color=f"C{i}", label=f"f = {f:.1f}")
plt.xscale("log")
plt.xlabel("kinetic energy (GeV)")
plt.ylabel("stopping power (MeV cm$^2$ g$^{-1}$)")
plt.legend()
plt.show()
