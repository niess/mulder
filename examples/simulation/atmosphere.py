#! /usr/bin/env python3
import matplotlib.pyplot as plt
import mulder
import numpy as np


KM = 1E-03 # metres to kilometres

# International Standard Atmosphere (ISA)
# Ref: https://en.wikipedia.org/wiki/International_Standard_Atmosphere
ISA = (
    (0.0, 1.225),
    (11_019, 0.3639),
    (20_063, 0.0880),
    (32_162, 0.0132),
    (47_350, 0.0014),
    (51_412, 0.0009),
    (71_802, 0.0001),
    # (86_000, 0.0)  # We remove this point since Mulder does not allow for
                     # null densities.
)

z = np.linspace(0.0, 80.0 / KM, 8001)
for i, model in enumerate(mulder.Atmosphere.models):
    atmosphere = mulder.Atmosphere(model)
    plt.plot(z * KM, atmosphere.density(z), color=f"C{i}", label=model)

atmosphere = mulder.Atmosphere(ISA)
plt.plot(z * KM, atmosphere.density(z), color=f"C{i + 1}", label="ISA table")

plt.yscale("log")
plt.xlabel("z (km)")
plt.ylabel("density (kg/m$^3$)")
plt.legend()
plt.show()
