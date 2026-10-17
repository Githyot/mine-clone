"""Helpers shared by the mod art modules (see tools/gen_mod_textures.py). They wrap the
helpers of gen_textures.py (blocks) and gen_items.py (items) so the mod art keeps the base
game's classic look."""

import os
import sys

import numpy as np  # noqa: F401

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import gen_textures as T  # noqa: E402
import gen_items as I  # noqa: E402

N = 16
pal, put, line, canvas, from_rgb, shade = T.pal, T.put, T.line, T.canvas, T.from_rgb, T.shade
rng_for, value_noise, fbm, normalize, hexrgb = T.rng_for, T.value_noise, T.fbm, T.normalize, T.hexrgb
from_map, outline, pmap = I.from_map, I.outline, I.pmap

BLOCKS = {}
ITEMS = {}


def block(name):
    def deco(fn):
        BLOCKS[name] = fn
        return fn
    return deco


def item(name):
    def deco(fn):
        ITEMS[name] = fn
        return fn
    return deco


