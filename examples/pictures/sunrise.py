#! /usr/bin/env python3
import matplotlib.pyplot as plot
from matplotlib import animation
import mulder
from mulder.picture import materials, SunLight
import numpy
from pathlib import Path
import pickle


PREFIX = Path(__file__).parent

raw_path = PREFIX / "sunrise.pkl"
if raw_path.exists():
    with raw_path.open("rb") as f:
        raw = pickle.load(f)
else:
    geometry = mulder.EarthGeometry(0.0)
    LOCATION = { "latitude": -45.0, "longitude": 3.0, "altitude": 0.5 }
    camera = mulder.Camera(**LOCATION, azimuth=89.5, elevation=1.0,
                           resolution = (768, 1024), fov = 6)
    raw = camera.shoot(geometry)
    with raw_path.open("wb") as f:
        pickle.dump(raw, f)

rock = materials["Rock"]
rock.metallic = True
rock.roughness = 0.2

sun = SunLight()
times = numpy.linspace(6.00, 6.40, 161)

def develop_picture(frame=0):
    sun.time = times[frame]
    plot.title(f"time = {sun.time}")
    return raw.develop(lights=sun)

fig = plot.figure(figsize=(10.24, 7.68))
mesh = plot.pcolormesh(develop_picture())
mesh.set_animated(True)
plot.axis("off")
anim = animation.FuncAnimation(
    fig,
    lambda frame: mesh.set_array(develop_picture(frame)),
    frames=range(2, len(times)),
    blit=False,
    repeat=False,
)
writer = animation.FFMpegWriter(fps=25, codec="ffv1")  # lossless compression.
anim.save(filename=PREFIX / "sunrise.avi", writer=writer)
