#! /usr/bin/env python3
import matplotlib.pyplot as plot
from mulder import Direction, Fluxmeter, Grid, PixelGrid
import numpy

# import uproot 


fluxmeter = Fluxmeter(
    Rock  = "data/mns_roche.png",
    Water = "data/mns_eau.png"
)

fluxmeter.mode = "continuous"


def generate_events(latitude, longitude, depth, radius, n):
    """Generate events for given coordinates."""

    # Get map coordinates of platform and corresponding water level.
    layer = fluxmeter.geometry.layers[1]
    platform = layer.project(latitude, longitude) # To Lambert
    water_level = layer.height(platform)

    costheta = fluxmeter.prng(n)
    phi = fluxmeter.prng(n) * 2 * numpy.pi
    sintheta = numpy.sqrt(1 - costheta**2)
    normal_x = sintheta * numpy.cos(phi)
    normal_y = sintheta * numpy.sin(phi)
    normal_z = costheta
    x = radius * normal_x
    y = radius * normal_y 
    z = radius * normal_z 

    position = layer.position(platform.x + x, platform.y + y)
    position.height = water_level - depth + z
    weight = 2 * numpy.pi * radius**2 # 1 / PDF

    # Generate direction.
    costheta = fluxmeter.prng(n)
    phi = fluxmeter.prng(n) * 2 * numpy.pi
    direction_x = sintheta * numpy.cos(phi)
    direction_y = sintheta * numpy.sin(phi)
    direction_z = costheta
    direction = Direction(
        azimuth = 90 - numpy.degrees(phi),
        elevation = numpy.degrees(numpy.arcsin(costheta))
    )
    weight *= 2 * numpy.pi # 1 / PDF

    # Generate energy.
    energy_min = 1E-02 # GeV
    energy_max = 1E+03 # GeV
    rE = numpy.log(energy_max / energy_min)
    energy = energy_min * numpy.exp(rE * fluxmeter.prng(n))
    weight *= rE * energy # 1 / PDF

    # Compute flux (and charge asymmetry)
    flux = fluxmeter.flux(
        position = position,
        direction = direction,
        energy = energy
    )

    # Compute rate, taking into account crossing factor.
    rate = flux.value * weight * (normal_x * direction_x +
        normal_y * direction_y + normal_z * direction_z) # Hz
    rate /= n

    # Print events.
    mm = 1E+03
    MeV = 1E-03
    for i, ri in enumerate(rate):
        if ri < 0: continue
        print(
            f"mu- "
            f"{x[i] * mm:8.0f} "
            f"{y[i] * mm:8.0f} "
            f"{z[i] * mm:8.0f} "
            f"{direction_x[i]:12.5E} "
            f"{direction_y[i]:12.5E} "
            f"{direction_z[i]:12.5E} "
            f"{energy[i] * MeV:12.5E} "
            f"{ri:12.5E} "
        )


if __name__ == "__main__":
    latitude, longitude = (45.49577410498454, 2.88797550129628)

    generate_events(
        latitude = latitude,
        longitude = longitude,
        depth = 10,
        radius = 0.2,
        n = 100
    )
