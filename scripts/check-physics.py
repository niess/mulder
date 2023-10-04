#! /usr/bin/env python3
import mulder


physics = mulder.Physics.build("share/mulder/materials")
physics.dump("share/mulder/materials.pumas")
