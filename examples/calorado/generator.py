#! /usr/bin/env python3
from mulder import Direction, Fluxmeter, Grid, PixelGrid
import numpy
import uproot 

# Create the mulder fluxmeter object using the DEMs as arguments
fluxmeter = Fluxmeter(
    Rock  = "data/mns_roche.png",
    Water = "data/mns_eau.png"
)

# Specify the calculation mode
fluxmeter.mode = "continuous"

def generate_events(latitude, longitude, depth, radius, n):
    """Generate events for given coordinates."""

    # Get map coordinates of platform and corresponding water level.
    layer = fluxmeter.geometry.layers[1] # Water layer
    platform = layer.project(latitude, longitude) # Coordinates of the platform (To Lambert)
    water_level = layer.height(platform) # Altitude of the platform

    # Generate random point coordinates on the source sphere
    # theta is polar angle counted from vertical axis (z)
    # phi is orientation angle
    costheta = fluxmeter.prng(n) 
    phi = fluxmeter.prng(n) * 2 * numpy.pi
    sintheta = numpy.sqrt(1 - costheta**2)
    normal_x = sintheta * numpy.cos(phi)
    normal_y = sintheta * numpy.sin(phi)
    normal_z = costheta
    x = radius * normal_x
    y = radius * normal_y 
    z = radius * normal_z 

    # Express the point coordinates in the reference frame of mulder
    position = layer.position(platform.x + x, platform.y + y)
    position.height = water_level - depth + z
    weight = 2 * numpy.pi * radius**2 # 1 / PDF

    # Generate direction.
    costheta = fluxmeter.prng(n)
    phi = fluxmeter.prng(n) * 2 * numpy.pi
    sintheta = numpy.sqrt(1 - costheta**2)
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

    # Print events. First, let us select ingoing events (i.e. with a positive
    # rate).
    mm = 1E+03
    cm = 1E+02
    MeV = 1E+03
    good = numpy.where(rate >= 0)
    xW = x[good]*mm
    yW = y[good]*mm
    zW = z[good]*mm
    dxW = -direction_x[good]
    dyW = -direction_y[good]
    dzW = -direction_z[good]
    eW = energy[good]*MeV
    pid = len(eW) * [13]
    weight = numpy.ones(xW.shape)

    # Then, data are converted to a numpy structured array, for serialisation
    # with uproot. Note that GATE expects float32 data, not float64.
    dtype = [
        ("PDGCode", "i4"),
        ("X", "f4"),
        ("Y", "f4"),
        ("Z", "f4"),
        ("dX", "f4"),
        ("dY", "f4"),
        ("dZ", "f4"),
        ("Ekine", "f4"),
        ("Weight", "f4"),
    ]

    array = numpy.array(
        list(zip(pid, xW, yW, zW, dxW, dyW, dzW, eW, weight)),
        dtype = dtype
    )

    # Finally, we Write data to a ROOT file
    with uproot.recreate("events.root") as file:
        file["PhaseSpace"] = array
        file["PhaseSpace"].show()


if __name__ == "__main__":
    latitude, longitude = (45.49577410498454, 2.88797550129628)

    generate_events(
        latitude = latitude,
        longitude = longitude,
        depth = 10,
        radius = 0.1,
        n = 100
    )
