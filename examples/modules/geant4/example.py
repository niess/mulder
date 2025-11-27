#! /usr/bin/env python3
from ctypes import cast, c_char_p, c_double, CFUNCTYPE, c_int, c_void_p,       \
                   POINTER, Structure
import mulder
import mulder.materials as materials
import numpy
from pathlib import Path


PREFIX = Path(__file__).parent


# Illustrate ctypes interface.
mod = mulder.Module(PREFIX / "module.so")

class CModule(Structure):
    pass

class CElement(Structure):
    pass

ELEMENT_DEL = CFUNCTYPE(None, POINTER(CElement))
ELEMENT_DBL = CFUNCTYPE(c_double, POINTER(CElement))
ELEMENT_INT = CFUNCTYPE(c_int, POINTER(CElement))

CElement._fields_ = [
    ("destroy", ELEMENT_DEL),
    ("A", ELEMENT_DBL),
    ("I", ELEMENT_DBL),
    ("Z", ELEMENT_INT),
]

ELEMENT = CFUNCTYPE(POINTER(CElement), c_char_p)

CModule._fields_ = [
    ("element", ELEMENT),
    ("geometry", c_void_p),
    ("material", c_void_p),
]

cmod = cast(mod.ptr, POINTER(CModule))
el = cmod[0].element(b"G4_O")
print(f"Z = {el[0].Z(el)}")
print(f"A = {el[0].A(el)}")
print(f"I = {el[0].I(el)}")
el[0].destroy(el)

# Check the native interface.
print(mod.element("G4_O"))
print(mod.material("G4_AIR"))
print(materials.Material("G4_AIR"))
print(materials.Element("G4_O"))

geometry = mod.geometry()
print(geometry.media)
print(geometry.locate(position=[0, 0, 1.0]))
print(geometry.trace(position=[0, 0, 1.0], direction=[0, 0, 1.0]))

meter = mulder.Fluxmeter(geometry=geometry)
meter.mode = "continuous"
states = meter.transport(
    position=[0, 0, -15E-02],
    direction=[0, 0, 1]
)
print(states)
