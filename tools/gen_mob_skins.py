#!/usr/bin/env python3
"""Procedurally generate original 64x128 mob skins.

The box models (sizes, origins and uv offsets) are read from
packs/vanilla/mobs.ron and checked against the part roles listed below.  Every
face of every box is painted pixel by pixel from hand-picked short palettes,
value noise and a few hand-placed features (eyes, snouts, ribs ...).  Nothing
external is read except mobs.ron itself.

Box unwrap (same as the comment at the top of mobs.ron), for a box w x h x d
whose skin region starts at (u, v):
    top    (u+d,     v)   w x d      bottom (u+d+w,   v)   w x d
    right  (u,       v+d) d x h      front  (u+d,     v+d) w x h
    left   (u+d+w,   v+d) d x h      back   (u+2d+w,  v+d) w x h

Orientation assumed for each face (local box coords x in [0,w], y in [0,h],
z in [0,d]; row 0 of a side face is the top of the box):
    front (+Z): column i -> x = i          back (-Z): column i -> x = w-1-i
    right (-X): column i -> z = i          left (+X): column i -> z = d-1-i
    top:  column i -> x = i, row j -> z = j  (last row touches the front)
    bottom: column i -> x = i, row j -> z = d-1-j (first row touches the front)
This wraps the four side faces into one continuous strip, so patterns that are
computed from 3D noise line up across box edges.

Usage:
    python3 tools/gen_mob_skins.py

The bottom half of the skin (same uv, v + 64) holds the `layers` overlays
listed in mobs.ron: wool, fur, hair, rags...  The renderer turns every opaque
pixel of a layer into a small block standing `inflate` pixels off the body, so
these are painted as sparse masks with holes.  Transparent pixels are (0,0,0,0).

Output: packs/vanilla/textures/entity/<mob>.png (64x128 RGBA, unused pixels
fully transparent) and preview.png (every skin at 4x on a checkerboard).
Deterministic: all randomness is hash-based with fixed seeds.
"""

import os
import re
import zlib
from types import SimpleNamespace

from PIL import Image, ImageDraw

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
MOBS_RON = os.path.join(ROOT, "packs", "vanilla", "mobs.ron")
OUT_DIR = os.path.join(ROOT, "packs", "vanilla", "textures", "entity")
SKIN = 64
SKIN_H = 128
LAYER_V = 64
BASE_SEED = 0x0B5C_A1E5

# Role of each part, in the order the parts appear in mobs.ron.  The geometry
# itself comes from mobs.ron; HARDCODED below must match it exactly (checked).
ROLES = {
    "pig": ["body", "head", "snout", "leg", "leg", "leg", "leg"],
    "cow": ["body", "head", "horn", "horn", "leg", "leg", "leg", "leg"],
    "sheep": ["body", "head", "leg", "leg", "leg", "leg"],
    "chicken": ["body", "head", "beak", "wattle", "wing", "wing", "leg", "leg"],
    "zombie": ["head", "body", "arm", "arm", "leg", "leg"],
    "skeleton": ["head", "body", "arm", "arm", "leg", "leg"],
    "stalker": ["head", "body", "leg", "leg", "leg", "leg"],
    "spider": ["head", "neck", "abdomen", "leg", "leg", "leg", "leg",
               "leg", "leg", "leg", "leg"],
    "player": ["head", "body", "arm", "arm", "leg", "leg"],
    "deer": ["body", "neck", "head", "ear", "ear", "antler", "antler", "tine", "tine",
             "tail", "leg", "leg", "leg", "leg"],
    "fox": ["body", "head", "snout", "ear", "ear", "tail", "leg", "leg", "leg", "leg"],
    "rabbit": ["body", "head", "ear", "ear", "tail", "foreleg", "foreleg", "hindleg", "hindleg"],
    "bear": ["body", "head", "snout", "ear", "ear", "hump", "leg", "leg", "leg", "leg"],
    "duck": ["body", "head", "bill", "tail", "wing", "wing", "leg", "leg"],
    "forest_guardian": ["head", "body", "arm", "arm", "leg", "leg", "branch", "branch"],
    "npc_guide": ["head", "body", "arm", "arm", "leg", "leg"],
    "npc_merchant": ["head", "body", "arm", "arm", "leg", "leg"],
    "npc_nurse": ["head", "body", "arm", "arm", "leg", "leg"],
    "npc_wizard": ["head", "body", "arm", "arm", "leg", "leg"],
    "npc_smith": ["head", "body", "arm", "arm", "leg", "leg"],
    "npc_guard": ["head", "body", "arm", "arm", "leg", "leg"],
}

# (size, origin, uv) per part, mirroring mobs.ron.
HARDCODED = {
    "pig": [((10, 8, 16), (-5, 6, -8), (0, 0)), ((8, 8, 8), (-4, 8, 7), (0, 24)),
            ((4, 3, 1), (-2, 9, 15), (32, 24)), ((4, 6, 4), (-5, 0, -7), (0, 40)),
            ((4, 6, 4), (1, 0, -7), (0, 40)), ((4, 6, 4), (-5, 0, 3), (0, 40)),
            ((4, 6, 4), (1, 0, 3), (0, 40))],
    "cow": [((12, 10, 18), (-6, 10, -9), (0, 0)), ((8, 8, 6), (-4, 14, 8), (0, 28)),
            ((1, 3, 1), (-5, 20, 11), (28, 28)), ((1, 3, 1), (4, 20, 11), (28, 28)),
            ((4, 10, 4), (-6, 0, -8), (0, 42)), ((4, 10, 4), (2, 0, -8), (0, 42)),
            ((4, 10, 4), (-6, 0, 4), (0, 42)), ((4, 10, 4), (2, 0, 4), (0, 42))],
    "sheep": [((10, 10, 16), (-5, 9, -8), (0, 0)), ((6, 6, 8), (-3, 14, 6), (0, 26)),
              ((4, 10, 4), (-5, 0, -7), (0, 40)), ((4, 10, 4), (1, 0, -7), (0, 40)),
              ((4, 10, 4), (-5, 0, 3), (0, 40)), ((4, 10, 4), (1, 0, 3), (0, 40))],
    "chicken": [((6, 6, 8), (-3, 4, -4), (0, 0)), ((4, 6, 3), (-2, 8, 2), (0, 14)),
                ((4, 2, 2), (-2, 10, 5), (14, 14)), ((2, 2, 1), (-1, 8, 5), (26, 14)),
                ((1, 4, 6), (-4, 6, -3), (0, 23)), ((1, 4, 6), (3, 6, -3), (0, 23)),
                ((2, 4, 2), (-2, 0, -1), (28, 0)), ((2, 4, 2), (0, 0, -1), (28, 0))],
    "zombie": [((8, 8, 8), (-4, 24, -4), (0, 0)), ((8, 12, 4), (-4, 12, -2), (16, 16)),
               ((4, 12, 4), (-8, 12, -2), (40, 16)), ((4, 12, 4), (4, 12, -2), (40, 16)),
               ((4, 12, 4), (-4, 0, -2), (0, 16)), ((4, 12, 4), (0, 0, -2), (0, 16))],
    "skeleton": [((8, 8, 8), (-4, 24, -4), (0, 0)), ((8, 12, 4), (-4, 12, -2), (16, 16)),
                 ((2, 12, 2), (-6, 12, -1), (40, 16)), ((2, 12, 2), (4, 12, -1), (40, 16)),
                 ((2, 12, 2), (-3, 0, -1), (0, 16)), ((2, 12, 2), (1, 0, -1), (0, 16))],
    "stalker": [((8, 8, 8), (-4, 18, -4), (0, 0)), ((8, 12, 4), (-4, 6, -2), (16, 16)),
                ((4, 6, 4), (-4, 0, -6), (0, 16)), ((4, 6, 4), (0, 0, -6), (0, 16)),
                ((4, 6, 4), (-4, 0, 2), (0, 16)), ((4, 6, 4), (0, 0, 2), (0, 16))],
    "spider": [((8, 8, 8), (-4, 4, 3), (32, 4)), ((6, 6, 6), (-3, 5, -3), (0, 0)),
               ((10, 8, 12), (-5, 4, -15), (0, 20))]
              + [((16, 2, 2), (-19, 7, z), (0, 40)) for z in (2, 0, -2, -4)]
              + [((16, 2, 2), (3, 7, z), (0, 40)) for z in (2, 0, -2, -4)],
    # right/left arm and leg have their own uv so they can differ (bracer, patch)
    "player": [((8, 8, 8), (-4, 24, -4), (0, 0)), ((8, 12, 4), (-4, 12, -2), (16, 16)),
               ((4, 12, 4), (-8, 12, -2), (40, 16)), ((4, 12, 4), (4, 12, -2), (32, 48)),
               ((4, 12, 4), (-4, 0, -2), (0, 16)), ((4, 12, 4), (0, 0, -2), (16, 48))],
    "deer": [((8, 8, 14), (-4, 12, -7), (0, 0)), ((4, 8, 4), (-2, 17, 4), (44, 0)),
             ((6, 5, 7), (-3, 23, 4), (0, 22)), ((3, 2, 1), (-6, 26, 5), (26, 22)),
             ((3, 2, 1), (3, 26, 5), (26, 22)), ((1, 6, 1), (-3, 28, 5), (34, 22)),
             ((1, 6, 1), (2, 28, 5), (34, 22)), ((3, 1, 1), (-6, 31, 5), (38, 22)),
             ((3, 1, 1), (3, 31, 5), (38, 22)), ((2, 4, 1), (-1, 15, -8), (46, 22)),
             ((2, 12, 2), (-4, 0, -6), (0, 34)), ((2, 12, 2), (2, 0, -6), (0, 34)),
             ((2, 12, 2), (-4, 0, 4), (0, 34)), ((2, 12, 2), (2, 0, 4), (0, 34))],
    "fox": [((6, 6, 10), (-3, 5, -5), (0, 0)), ((6, 5, 5), (-3, 7, 4), (32, 0)),
            ((2, 2, 3), (-1, 7, 9), (54, 0)), ((2, 2, 1), (-3, 12, 6), (54, 5)),
            ((2, 2, 1), (1, 12, 6), (54, 5)), ((4, 4, 9), (-2, 4, -13), (0, 16)),
            ((2, 5, 2), (-3, 0, -4), (26, 16)), ((2, 5, 2), (1, 0, -4), (26, 16)),
            ((2, 5, 2), (-3, 0, 2), (26, 16)), ((2, 5, 2), (1, 0, 2), (26, 16))],
    "rabbit": [((4, 4, 6), (-2, 2, -3), (0, 0)), ((4, 4, 4), (-2, 5, 2), (20, 0)),
               ((1, 4, 1), (-2, 9, 3), (36, 0)), ((1, 4, 1), (1, 9, 3), (36, 0)),
               ((2, 2, 1), (-1, 3, -4), (40, 0)), ((1, 2, 1), (-2, 0, 1), (46, 0)),
               ((1, 2, 1), (1, 0, 1), (46, 0)), ((2, 2, 4), (-2, 0, -3), (0, 10)),
               ((2, 2, 4), (0, 0, -3), (0, 10))],
    "bear": [((12, 11, 20), (-6, 10, -10), (0, 0)), ((8, 8, 7), (-4, 14, 9), (0, 31)),
             ((4, 3, 3), (-2, 14, 16), (30, 31)), ((2, 2, 1), (-4, 22, 12), (44, 31)),
             ((2, 2, 1), (2, 22, 12), (44, 31)), ((10, 3, 7), (-5, 21, 2), (16, 46)),
             ((4, 10, 4), (-6, 0, -9), (0, 46)), ((4, 10, 4), (2, 0, -9), (0, 46)),
             ((4, 10, 4), (-6, 0, 5), (0, 46)), ((4, 10, 4), (2, 0, 5), (0, 46))],
    "duck": [((6, 5, 9), (-3, 3, -5), (0, 0)), ((4, 5, 4), (-2, 7, 2), (30, 0)),
             ((4, 1, 3), (-2, 8, 6), (46, 0)), ((4, 2, 2), (-2, 7, -7), (46, 4)),
             ((1, 4, 7), (-4, 3, -4), (0, 14)), ((1, 4, 7), (3, 3, -4), (0, 14)),
             ((1, 3, 1), (-2, 0, 0), (16, 14)), ((1, 3, 1), (1, 0, 0), (16, 14))],
    "forest_guardian": [((10, 10, 8), (-5, 26, -4), (0, 0)), ((12, 14, 6), (-6, 12, -3), (0, 18)),
                        ((4, 16, 4), (-10, 10, -2), (36, 0)), ((4, 16, 4), (6, 10, -2), (36, 0)),
                        ((5, 12, 4), (-6, 0, -2), (36, 20)), ((5, 12, 4), (1, 0, -2), (36, 20)),
                        ((2, 6, 2), (-6, 34, -1), (54, 0)), ((2, 6, 2), (4, 34, -1), (54, 0))],
}
# Town NPCs share the player's model.
for _n in ("npc_guide", "npc_merchant", "npc_nurse", "npc_wizard", "npc_smith", "npc_guard"):
    HARDCODED[_n] = HARDCODED["player"]


# `layers` per mob as (part index, inflate), mirroring mobs.ron (checked).
HARDCODED_LAYERS = {
    "pig": [(0, 0.5), (1, 0.5)],
    "cow": [(0, 0.6), (1, 0.5)],
    "sheep": [(0, 1.5), (1, 1.0), (2, 0.6), (3, 0.6), (4, 0.6), (5, 0.6)],
    "chicken": [(0, 0.5), (4, 0.4), (5, 0.4)],
    "zombie": [(0, 0.6), (1, 0.5), (2, 0.4), (3, 0.4), (4, 0.4), (5, 0.4)],
    "skeleton": [(1, 0.5)],
    "stalker": [(0, 0.8), (1, 0.8)],
    "spider": [(1, 0.6), (2, 0.7)],
    "player": [(0, 0.5), (1, 0.3), (2, 0.3), (3, 0.3), (4, 0.3), (5, 0.3)],
    "deer": [(0, 0.4), (1, 0.4)],
    "fox": [(0, 0.4), (5, 0.6)],
    "rabbit": [(0, 0.3), (4, 0.4)],
    "bear": [(0, 0.7), (1, 0.5), (5, 0.7), (6, 0.4)],
    "duck": [(0, 0.3), (4, 0.3), (5, 0.3)],
    "forest_guardian": [(0, 0.8), (1, 0.6), (2, 0.5), (3, 0.5)],
}
for _n in ("npc_guide", "npc_merchant", "npc_nurse", "npc_wizard", "npc_smith", "npc_guard"):
    HARDCODED_LAYERS[_n] = HARDCODED_LAYERS["player"]


# --------------------------------------------------------------------------
# mobs.ron parsing
# --------------------------------------------------------------------------

PART_RE = re.compile(
    r"\(size:\s*\((\d+),\s*(\d+),\s*(\d+)\),\s*origin:\s*\((-?\d+),\s*(-?\d+),\s*(-?\d+)\),"
    r"\s*uv:\s*\((\d+),\s*(\d+)\)")
NAME_RE = re.compile(r'name:\s*"([a-z_]+)"')
LAYERS_RE = re.compile(r"layers:\s*\[(.*?)\]", re.S)
LAYER_RE = re.compile(r"\(part:\s*(\d+),\s*inflate:\s*([0-9.]+)\)")


def parse_mobs(path):
    text = open(path, encoding="utf-8").read()
    names = list(NAME_RE.finditer(text))
    mobs, layers = {}, {}
    for k, m in enumerate(names):
        end = names[k + 1].start() if k + 1 < len(names) else len(text)
        parts = []
        for p in PART_RE.finditer(text, m.end(), end):
            g = [int(v) for v in p.groups()]
            parts.append((tuple(g[0:3]), tuple(g[3:6]), tuple(g[6:8])))
        mobs[m.group(1)] = parts
        lm = LAYERS_RE.search(text, m.end(), end)
        layers[m.group(1)] = [(int(a), float(b)) for a, b in LAYER_RE.findall(lm.group(1))] if lm else []
    return mobs, layers


# --------------------------------------------------------------------------
# Box unwrap
# --------------------------------------------------------------------------


def face_rects(size, uv):
    w, h, d = size
    u, v = uv
    return {
        "top": (u + d, v, w, d),
        "bottom": (u + d + w, v, w, d),
        "right": (u, v + d, d, h),
        "front": (u + d, v + d, w, h),
        "left": (u + d + w, v + d, d, h),
        "back": (u + 2 * d + w, v + d, w, h),
    }


def local_pos(face, i, j, size):
    """Centre of face pixel (i, j) in local box coordinates."""
    w, h, d = size
    if face == "front":
        return (i + .5, h - j - .5, d)
    if face == "back":
        return (w - i - .5, h - j - .5, 0)
    if face == "right":
        return (0, h - j - .5, i + .5)
    if face == "left":
        return (w, h - j - .5, d - i - .5)
    if face == "top":
        return (i + .5, h, j + .5)
    return (i + .5, 0, d - j - .5)  # bottom


# --------------------------------------------------------------------------
# Deterministic noise / colour helpers
# --------------------------------------------------------------------------


def hash01(seed, *vals):
    h = seed & 0xFFFFFFFF
    for v in vals:
        h ^= (int(v) * 0x27D4EB2D) & 0xFFFFFFFF
        h = (h * 0x165667B1 + 0x9E3779B9) & 0xFFFFFFFF
        h ^= h >> 15
        h = (h * 0x85EBCA6B) & 0xFFFFFFFF
        h ^= h >> 13
        h = (h * 0xC2B2AE35) & 0xFFFFFFFF
        h ^= h >> 16
    return h / 4294967296.0


def _smooth(t):
    return t * t * (3 - 2 * t)


def vnoise(seed, x, y, z):
    import math
    x0, y0, z0 = math.floor(x), math.floor(y), math.floor(z)
    tx, ty, tz = _smooth(x - x0), _smooth(y - y0), _smooth(z - z0)

    def c(a, b, e):
        return hash01(seed, x0 + a, y0 + b, z0 + e)

    def lerp(a, b, t):
        return a + (b - a) * t
    return lerp(
        lerp(lerp(c(0, 0, 0), c(1, 0, 0), tx), lerp(c(0, 1, 0), c(1, 1, 0), tx), ty),
        lerp(lerp(c(0, 0, 1), c(1, 0, 1), tx), lerp(c(0, 1, 1), c(1, 1, 1), tx), ty),
        tz)


def fbm(seed, x, y, z, scale, octaves=3):
    total, amp, norm, f = 0.0, 1.0, 0.0, 1.0 / scale
    for o in range(octaves):
        total += amp * vnoise(seed + o * 101, x * f, y * f, z * f)
        norm += amp
        amp *= 0.5
        f *= 2.0
    return total / norm


def hexrgb(hx):
    hx = hx.lstrip("#")
    return tuple(int(hx[i:i + 2], 16) for i in (0, 2, 4))


def pal(*hexes):
    """Palette ordered dark -> light."""
    return [hexrgb(hx) for hx in hexes]


def pick(palette, t):
    k = int(t * len(palette))
    return palette[max(0, min(len(palette) - 1, k))]


def mul(rgb, k):
    return tuple(max(0, min(255, int(round(c * k)))) for c in rgb)


def mix(a, b, t):
    return tuple(int(round(a[i] + (b[i] - a[i]) * t)) for i in range(3))


SIDES = ("front", "back", "left", "right")


# --------------------------------------------------------------------------
# Painters.  Each receives a context `c` describing one face pixel:
#   c.role, c.face, c.i, c.j (pixel in face), c.fw, c.fh (face size),
#   c.x, c.y, c.z (local position), c.w, c.h, c.d (box size),
#   c.gx, c.gy, c.gz (model-space position), c.px, c.py (skin pixel),
#   c.jit (per-pixel random 0..1), c.seed
# and returns an (r, g, b) colour.
# --------------------------------------------------------------------------

# ---- pig -----------------------------------------------------------------
PIG = pal("#c8737c", "#dc8c92", "#e9a2a4", "#f3b6b4", "#f9c8c3")
PIG_SNOUT = pal("#b85e6c", "#c8707b", "#d6828a", "#e19399")
PIG_HOOF = pal("#4a2e2e", "#5e3b39")


def paint_pig(c):
    n = fbm(c.seed, c.gx, c.gy, c.gz, 5.0)
    t = 0.18 + 0.62 * n + 0.28 * (c.jit - 0.5)
    if c.role == "body":
        if c.face == "bottom":
            t -= 0.2
        if c.face == "top":
            t += 0.1
        col = pick(PIG, t)
        if c.face == "back":  # a little curly tail
            tail = {(4, 1), (5, 1), (5, 2), (4, 3), (5, 3)}
            if (c.i, c.j) in tail:
                return PIG_SNOUT[0]
            if (c.i, c.j) in {(4, 2)}:
                return PIG[1]
        return col
    if c.role == "head":
        col = pick(PIG, t)
        if c.face == "front":
            eyes = {(1, 3): (250, 246, 240), (2, 3): (24, 18, 20),
                    (5, 3): (24, 18, 20), (6, 3): (250, 246, 240)}
            if (c.i, c.j) in eyes:
                return eyes[(c.i, c.j)]
            if c.j == 2 and c.i in (1, 2, 5, 6):  # soft brow shadow
                return mul(col, 0.9)
            if c.j == 0 and c.i in (0, 1, 6, 7):  # ear flaps
                return PIG[1] if c.jit < 0.6 else PIG[0]
            if c.j == 7 and 2 <= c.i <= 5:  # mouth line under the snout
                return PIG[0]
        if c.face in ("left", "right") and c.j <= 1 and 5 <= c.i <= 7:
            return PIG[1]  # ears seen from the side
        return col
    if c.role == "snout":
        col = pick(PIG_SNOUT, 0.25 + 0.6 * c.jit)
        if c.face == "front":
            if (c.i, c.j) in ((1, 1), (2, 1)):
                return (92, 40, 52)
            if c.j == 0:
                return PIG_SNOUT[3]
        return col
    # legs
    if c.face == "bottom" or (c.face in SIDES and c.j >= c.fh - 1):
        return pick(PIG_HOOF, c.jit)
    if c.face in SIDES and c.j == c.fh - 2:
        return pick(PIG, 0.1 + 0.2 * c.jit)
    return pick(PIG, t)


# ---- cow -----------------------------------------------------------------
COW_BROWN = pal("#3a2416", "#4a2f1d", "#5a3a25", "#6a462e")
COW_WHITE = pal("#cfc9bd", "#ddd8cd", "#e9e5dc", "#f4f1ea")
COW_MUZZLE = pal("#b98f7f", "#cba292", "#d9b3a3")
COW_HOOF = pal("#1e1712", "#2c221a")
COW_HORN = pal("#a99f86", "#cfc6aa", "#e6dec5")


def cow_coat(c, seed_off=0):
    n = fbm(c.seed + seed_off, c.gx * 1.0, c.gy * 1.3, c.gz * 0.9, 7.0, 3)
    t = 0.2 + 0.6 * fbm(c.seed + 77, c.gx, c.gy, c.gz, 3.0, 2) + 0.3 * (c.jit - 0.5)
    edge = abs(n - 0.5) < 0.025
    if n > 0.5:
        return pick(COW_WHITE, t + (-0.25 if edge else 0))
    return pick(COW_BROWN, t + (0.2 if edge else 0))


def paint_cow(c):
    if c.role == "body":
        col = cow_coat(c)
        if c.face == "bottom":
            col = mul(col, 0.88)
            # udder near the rear
            if 4 <= c.i <= 7 and 12 <= c.j <= 15:
                return pick(COW_MUZZLE, 0.3 + 0.6 * c.jit)
        return col
    if c.role == "head":
        if c.face == "front":
            if 5 <= c.j <= 7 and 1 <= c.i <= 6:  # muzzle
                if c.j == 6 and c.i in (2, 5):
                    return (64, 40, 36)
                if c.j == 7 and c.i in (1, 6):
                    return COW_MUZZLE[0]
                return pick(COW_MUZZLE, 0.35 + 0.55 * c.jit)
            if c.j == 3 and c.i in (1, 6):
                return (14, 10, 10)
            if c.j == 3 and c.i in (2, 5):
                return (236, 232, 226)
            if c.j == 2 and c.i in (1, 2, 5, 6):
                return COW_BROWN[0]
            if 3 <= c.i <= 4 and c.j <= 4:  # white blaze
                return pick(COW_WHITE, 0.4 + 0.5 * c.jit)
            return pick(COW_BROWN, 0.3 + 0.6 * c.jit)
        if c.face in ("left", "right") and c.j == 2 and c.i in (1, 2):
            return COW_BROWN[0]  # ear flap shading
        if c.face == "top":
            return pick(COW_BROWN, 0.3 + 0.6 * c.jit)
        return cow_coat(c)
    if c.role == "horn":
        if c.face == "top" or (c.face in SIDES and c.j == 0):
            return COW_HORN[2]
        if c.face in SIDES and c.j == 2 or c.face == "bottom":
            return COW_HORN[0]
        return COW_HORN[1]
    # legs: local coordinates so all four legs look the same
    lc = SimpleNamespace(**vars(c))
    lc.gx, lc.gy, lc.gz = c.x + 20, c.y, c.z + 20
    if c.face == "bottom" or (c.face in SIDES and c.j >= c.fh - 2):
        return pick(COW_HOOF, c.jit)
    if c.face in SIDES and c.j == c.fh - 3:
        return pick(COW_WHITE, 0.1 + 0.3 * c.jit)
    if c.face in SIDES and c.j >= 5:
        return pick(COW_WHITE, 0.2 + 0.7 * c.jit)
    return cow_coat(lc, 5)


# ---- sheep ---------------------------------------------------------------
WOOL = pal("#b9b1a3", "#cbc4b7", "#d9d3c8", "#e6e1d8", "#f1eee7")
SHEEP_FACE = pal("#6f6152", "#85766a", "#9a8b7c", "#ad9f8f")
SHEEP_HOOF = pal("#2e2622", "#3d332d")


def wool(c, scale=2.2):
    n = fbm(c.seed, c.gx, c.gy, c.gz, scale, 2)
    curl = hash01(c.seed + 9, c.px // 2, c.py // 2)
    t = 0.15 + 0.65 * n + 0.25 * (c.jit - 0.5) + 0.12 * (curl - 0.5)
    return pick(WOOL, t)


def paint_sheep(c):
    if c.role == "body":
        col = wool(c)
        if c.face == "bottom":
            col = mul(col, 0.9)
        return col
    if c.role == "head":
        t = 0.3 + 0.6 * c.jit
        if c.face == "front":
            if c.j == 0 or (c.j == 1 and c.i in (0, 5)):
                return wool(c)
            if c.j == 2 and c.i in (1, 4):
                return (16, 12, 10)
            if c.j == 2 and c.i in (0, 5):
                return (226, 220, 206)
            if c.j == 4 and c.i in (2, 3):
                return SHEEP_FACE[0]
            if c.j == 5 and 2 <= c.i <= 3:
                return SHEEP_FACE[1]
            return pick(SHEEP_FACE, t)
        if c.face in ("left", "right"):
            # wool cap on the back half / top rows; skin near the face
            near_face = c.z > c.d - 3
            if c.j <= 1 or not near_face:
                if c.j >= 4 and near_face:
                    return pick(SHEEP_FACE, t)
                return wool(c)
            return pick(SHEEP_FACE, t)
        if c.face == "bottom":
            return pick(SHEEP_FACE, t - 0.2) if c.z > c.d - 3 else wool(c)
        return wool(c)
    # legs: woolly top half, skin below, dark hooves
    if c.face == "top":
        return wool(c)
    if c.face == "bottom" or c.j >= c.fh - 1:
        return pick(SHEEP_HOOF, c.jit)
    if c.j < 5 or (c.j == 5 and c.jit < 0.5):
        return wool(c)
    return pick(SHEEP_FACE, 0.2 + 0.6 * c.jit)


# ---- chicken ---------------------------------------------------------------
FEATHER = pal("#cfcfc6", "#e0e0d8", "#ecece6", "#f8f8f4")
WING = pal("#9e9e96", "#b4b4ac", "#c8c8c0", "#dadad2")
BEAK = pal("#c98a1c", "#e2aa2a", "#f2c845")
WATTLE = pal("#8e1a1e", "#b52428", "#d8383a")
CHICK_LEG = pal("#b8701c", "#d68e26", "#eaaa38")


def paint_chicken(c):
    t = 0.25 + 0.6 * c.jit
    if c.role == "body":
        col = pick(FEATHER, t)
        if c.face == "back" and c.j <= 1 and 2 <= c.i <= 3:
            return FEATHER[0]  # tail tuft shadow
        if c.face == "bottom":
            col = mul(col, 0.92)
        return col
    if c.role == "head":
        col = pick(FEATHER, t)
        if c.face == "front" and c.j == 1 and c.i in (0, 3):
            return (14, 12, 12)
        if c.face in ("left", "right") and c.j == 1 and c.i == c.fw - 1:
            return (14, 12, 12)
        if c.face == "top" and c.j == c.fh - 1 and c.i in (1, 2):
            return WATTLE[2]  # little red comb
        if c.face == "top" and c.i in (1, 2):
            return WATTLE[1]
        return col
    if c.role == "beak":
        if c.face == "top":
            return BEAK[2]
        if c.face == "bottom" or (c.face in SIDES and c.j == 1):
            return BEAK[0] if c.face != "front" else BEAK[1]
        return BEAK[2] if c.jit > 0.3 else BEAK[1]
    if c.role == "wattle":
        if c.face == "bottom" or c.j == 1:
            return WATTLE[0] if c.jit < 0.5 else WATTLE[1]
        return WATTLE[2] if c.jit > 0.4 else WATTLE[1]
    if c.role == "wing":
        if c.face in ("left", "right"):
            # feather rows; darker tips at the bottom and the back end
            if c.j == c.fh - 1 or (c.j >= 2 and c.i <= 1):
                return pick(WING, 0.1 + 0.3 * c.jit)
            if c.j == 1 and c.i % 2 == 0:
                return WING[1]
            return pick(WING, 0.4 + 0.6 * c.jit)
        return pick(WING, 0.3 + 0.5 * c.jit)
    # legs
    if c.face == "bottom" or (c.face in SIDES and c.j == c.fh - 1):
        return CHICK_LEG[0]
    return pick(CHICK_LEG, 0.35 + 0.6 * c.jit)


# ---- zombie ----------------------------------------------------------------
ZSKIN = pal("#3e5640", "#4c6a4c", "#5a7a57", "#6b8c66", "#7d9c74")
ZSHIRT = pal("#173f44", "#1f5257", "#286468", "#33787a")
ZPANTS = pal("#1c1b3a", "#25244a", "#2f2e5a", "#3a3a6c")
ZSHOE = pal("#1d1714", "#2a211b")
ZROPE = pal("#6e5532", "#8c6d40")
STITCH = (28, 22, 24)
SCAR = (134, 120, 112)


def zskin(c):
    n = fbm(c.seed + 3, c.gx, c.gy, c.gz, 3.0, 2)
    return pick(ZSKIN, 0.15 + 0.6 * n + 0.3 * (c.jit - 0.5))


def zshirt(c):
    n = fbm(c.seed + 5, c.gx, c.gy, c.gz, 3.0, 2)
    return pick(ZSHIRT, 0.15 + 0.6 * n + 0.3 * (c.jit - 0.5))


def paint_zombie(c):
    if c.role == "head":
        col = zskin(c)
        if c.face == "front":
            # hollow eyes: black socket, dim green glint, darker rim
            sockets = {(1, 3), (2, 3), (1, 4), (2, 4), (5, 3), (6, 3), (5, 4), (6, 4)}
            if (c.i, c.j) in sockets:
                return (40, 255 // 4, 44) if (c.i, c.j) in ((2, 4), (5, 4)) else (12, 14, 12)
            if c.j == 2 and c.i in (1, 2, 5, 6):
                return ZSKIN[0]
            if c.j == 5 and c.i in (3, 4):
                return ZSKIN[0]  # nose holes
            if c.j == 6 and 2 <= c.i <= 5:  # crooked mouth
                return (30, 24, 24) if c.i != 4 else (190, 178, 150)
            if c.j == 7 and c.i == 5:
                return (30, 24, 24)
            # stitched scar from forehead down the left cheek
            scar = {(5, 0), (6, 1), (7, 2)}
            if (c.i, c.j) in scar:
                return SCAR
            if (c.i, c.j) in {(6, 0), (7, 1)}:
                return STITCH
            return col
        if c.face == "top":
            # thin, patchy dark hair
            if fbm(c.seed + 11, c.gx, c.gy, c.gz, 2.0, 2) < 0.45:
                return (34, 32, 28) if c.jit < 0.6 else (48, 44, 38)
            return col
        if c.face in ("back", "left", "right") and c.j <= 1:
            if fbm(c.seed + 11, c.gx, c.gy, c.gz, 2.0, 2) < 0.5:
                return (34, 32, 28)
        if c.face == "left" and c.i == 4 and 3 <= c.j <= 4:
            return ZSKIN[0]  # ear
        if c.face == "right" and c.i == 3 and 3 <= c.j <= 4:
            return ZSKIN[0]
        return col
    if c.role == "body":
        if c.face in ("top", "bottom"):
            return zshirt(c) if c.face == "top" else ZPANTS[1]
        # rope belt on the last row
        if c.j == c.fh - 1:
            return pick(ZROPE, c.jit)
        # ragged hem: skin peeks through above the belt
        hem = 8 + int(hash01(c.seed + 21, int(c.gx * 2), int(c.gz * 2)) * 3)
        if c.j >= hem:
            return zskin(c)
        col = zshirt(c)
        if c.face == "front":
            # torn hole on the right chest with stitched edge
            if (c.i, c.j) in {(5, 3), (6, 3), (5, 4), (6, 5)}:
                return zskin(c)
            if (c.i, c.j) in {(4, 3), (4, 4), (5, 5)}:
                return ZSHIRT[0]
            # ochre patch, sewn on with dark stitches
            if 1 <= c.i <= 2 and 5 <= c.j <= 6:
                return (156, 118, 46) if c.jit > 0.3 else (134, 98, 38)
            if (c.i, c.j) in {(0, 5), (3, 6), (1, 7), (2, 4)}:
                return STITCH
            # collar
            if c.j == 0 and 2 <= c.i <= 5:
                return zskin(c) if 3 <= c.i <= 4 else ZSHIRT[0]
        if c.face == "back" and (c.i, c.j) in {(2, 2), (3, 3), (4, 2), (3, 1), (2, 4)}:
            return zskin(c)  # gash in the back
        return col
    if c.role == "arm":
        if c.face == "top":
            return zshirt(c)
        if c.face == "bottom":
            return zskin(c)
        # short sleeve with a ragged edge, then green arm; darker hands
        sleeve = 3 + (1 if hash01(c.seed + 31, c.i, zlib.crc32(c.face.encode())) > 0.5 else 0)
        if c.j < sleeve:
            if c.j == sleeve - 1 and c.jit < 0.3:
                return ZSHIRT[0]
            return zshirt(c)
        col = zskin(c)
        if c.j >= c.fh - 3:
            col = mul(col, 0.88)
        if c.face == "front" and c.j == 6 and c.i in (1, 2):
            return STITCH if c.i == 1 else SCAR
        return col
    # legs
    if c.face == "bottom" or (c.face in SIDES and c.j >= c.fh - 2):
        return pick(ZSHOE, c.jit)
    if c.face == "top":
        return ZPANTS[1]
    n = fbm(c.seed + 7, c.x + 30, c.y, c.z, 3.0, 2)
    col = pick(ZPANTS, 0.15 + 0.6 * n + 0.3 * (c.jit - 0.5))
    if c.face == "front" and 5 <= c.j <= 6 and 1 <= c.i <= 2:
        return (64, 58, 52)  # knee patch
    if c.face == "front" and c.j == 7 and c.i == 3:
        return STITCH
    if c.face in ("left", "right") and c.j == c.fh - 3:
        return ZPANTS[0]  # frayed cuff
    return col


# ---- skeleton --------------------------------------------------------------
BONE = pal("#8a877d", "#a8a59a", "#c2bfb3", "#d8d5c9", "#e8e6dc")
VOID = (26, 24, 24)
VOID2 = (44, 41, 40)


def paint_skeleton(c):
    t = 0.3 + 0.6 * c.jit
    if c.role == "head":
        col = pick(BONE, t)
        if c.face == "bottom":
            return pick(BONE, t - 0.35)
        if c.face == "front":
            sockets = {(1, 3), (2, 3), (1, 4), (2, 4), (5, 3), (6, 3), (5, 4), (6, 4)}
            if (c.i, c.j) in sockets:
                return VOID
            if c.j == 2 and c.i in (1, 2, 5, 6):
                return BONE[1]
            if (c.i, c.j) in ((3, 5), (4, 5)):
                return VOID2  # nose cavity
            if c.j == 6:
                if 1 <= c.i <= 6:
                    return VOID if c.i % 2 == 0 else BONE[4]
                return BONE[1]
            if c.j == 7:
                return BONE[2] if 1 <= c.i <= 6 else BONE[1]
            if c.j == 5 and c.i in (0, 7):
                return BONE[1]  # cheekbone shadow
        if c.face in ("left", "right") and c.j >= 5:
            return pick(BONE, t - 0.2)
        if c.face == "top" and (c.i, c.j) in {(2, 2), (3, 3), (3, 4), (4, 5)}:
            return BONE[1]  # hairline crack
        return col
    if c.role == "body":
        if c.face in ("top", "bottom"):
            return VOID2 if not (3 <= c.i <= 4) else pick(BONE, t)
        if c.face in ("front", "back"):
            if 3 <= c.i <= 4 and c.j <= 8:  # spine
                return BONE[3] if (c.j % 2 == 0) == (c.i == 3) else BONE[2]
            if c.j in (1, 3, 5, 7) and 1 <= c.i <= 6:  # ribs
                return pick(BONE, t)
            if c.j in (2, 4, 6) and c.i in (0, 7):  # rib ends curving down
                return BONE[1]
            if c.j >= 9:  # pelvis
                if c.j == 9 and 1 <= c.i <= 6:
                    return pick(BONE, t)
                if c.j == 10 and c.i in (1, 2, 5, 6):
                    return pick(BONE, t - 0.2)
                if c.j == 11 and c.i in (2, 5):
                    return BONE[1]
            return VOID
        # sides: rib ends
        if c.j in (1, 3, 5, 7) or (c.j == 9):
            return pick(BONE, t - 0.25)
        return VOID
    # arms and legs: bone shaft with a knobby joint in the middle
    if c.face in ("top", "bottom"):
        return BONE[2] if c.face == "top" else BONE[1]
    shade = 0.0
    if c.face in ("back", "right"):
        shade = -0.2
    if c.i == c.fw - 1:
        shade -= 0.12
    if c.j in (5, 6):
        return pick(BONE, 0.75 + shade) if c.j == 5 else pick(BONE, 0.2 + shade)
    if c.j == c.fh - 1:
        return BONE[1] if c.role == "leg" else BONE[2]
    return pick(BONE, t + shade)


# ---- stalker ---------------------------------------------------------------
MOSS = pal("#24381c", "#2f4a24", "#3d5e2c", "#4d7234", "#5f863f", "#749a4e")
LICHEN = pal("#8d9c6a", "#a7b37e", "#bfc596")
ROOT_C = pal("#2c2a1a", "#3b3822", "#4a462b")
FACE_DARK = (14, 18, 12)
FACE_MID = (30, 40, 24)


def moss(c):
    n = fbm(c.seed, c.gx, c.gy, c.gz, 3.5, 3)
    t = 0.05 + 0.85 * n + 0.32 * (c.jit - 0.5)
    lich = fbm(c.seed + 50, c.gx, c.gy, c.gz, 1.6, 2)
    if lich > 0.68:
        return pick(LICHEN, (lich - 0.68) * 3.2 + 0.2 * (c.jit - 0.5))
    return pick(MOSS, t)


def paint_stalker(c):
    if c.role == "head":
        col = moss(c)
        if c.face == "front":
            # sad face on a paler lichen mask: brows that droop outwards, two
            # small round eyes with a wet glint, a thin vertical slit mouth
            # and one tear.  (Deliberately not a blocky 2x2-eyes face.)
            mask = 1 <= c.i <= 6 and 1 <= c.j <= 7 and (c.i, c.j) not in (
                (1, 1), (6, 1), (1, 7), (6, 7))
            if (c.i, c.j) in {(1, 2), (2, 1), (5, 1), (6, 2)}:
                return FACE_MID  # brows, inner ends raised
            if (c.i, c.j) in {(2, 4), (5, 4)}:
                return FACE_DARK  # small round eyes
            if (c.i, c.j) in {(2, 3), (5, 3)}:
                return MOSS[3]  # lid shadow
            if (c.i, c.j) == (2, 5):
                return (150, 196, 178)  # a single tear
            if c.i == 3 and 5 <= c.j <= 7:
                return FACE_DARK  # vertical slit mouth
            if c.i == 4 and 5 <= c.j <= 7:
                return FACE_MID if c.j != 5 else MOSS[1]
            if mask:
                return pick(MOSS, 0.62 + 0.4 * c.jit) if c.jit < 0.9 else LICHEN[0]
            return col
        if c.face == "top":
            # leafy crown: four leaves radiating from a small bud
            leaf = {(3, 3): 2, (4, 4): 2, (3, 4): 3, (4, 3): 3,
                    (2, 2): 5, (1, 1): 4, (5, 2): 5, (6, 1): 4,
                    (2, 5): 5, (1, 6): 4, (5, 5): 5, (6, 6): 4}
            if (c.i, c.j) in leaf:
                k = leaf[(c.i, c.j)]
                if k == 3:
                    return (150, 64, 72)  # bud
                if k == 2:
                    return (110, 40, 52)
                return MOSS[k]
        return col
    if c.role == "body":
        col = moss(c)
        if c.face in SIDES:
            # darker vertical vine ribs
            k = c.i if c.face in ("front", "back") else c.i * 2
            if k % 3 == 1 and hash01(c.seed + 60, c.px, c.py // 3) < 0.6:
                return mul(col, 0.78)
            if c.face == "front" and (c.i, c.j) in {(3, 5), (4, 6), (3, 7)}:
                return (60, 30, 36)  # faint reddish vein (it's going to pop)
        return col
    # legs: moss above, rooty bark below
    if c.face == "bottom":
        return pick(ROOT_C, c.jit)
    if c.face in SIDES and c.j >= c.fh - 2:
        return pick(ROOT_C, 0.2 + 0.7 * c.jit)
    if c.face in SIDES and c.j == c.fh - 3 and c.jit < 0.5:
        return ROOT_C[2]
    lc = SimpleNamespace(**vars(c))
    lc.gx, lc.gy, lc.gz = c.x + 40, c.y, c.z + 40
    return moss(lc)


# ---- spider ----------------------------------------------------------------
SPIDER = pal("#120e0c", "#1c1612", "#261e18", "#30261e", "#3c3026")
HAIR = (78, 64, 50)
RED_EYE = (210, 28, 30)
RED_EYE_HI = (255, 112, 96)
RED_EYE_DIM = (140, 20, 24)


def spider_fur(c, bias=0.0):
    n = fbm(c.seed, c.gx, c.gy, c.gz, 2.5, 2)
    h = hash01(c.seed + 99, c.px, c.py)
    if h > 0.9:
        return HAIR
    if h > 0.82:
        return SPIDER[4]
    return pick(SPIDER, 0.1 + 0.55 * n + 0.3 * (c.jit - 0.5) + bias)


def paint_spider(c):
    if c.role == "head":
        col = spider_fur(c)
        if c.face == "front":
            # eight eyes: two big ones, a tiny centre pair, two pairs at the sides
            if (c.i, c.j) in {(2, 3), (5, 3)}:
                return RED_EYE_HI
            if (c.i, c.j) in {(2, 4), (5, 4)}:
                return RED_EYE
            if (c.i, c.j) in {(3, 1), (4, 1)}:
                return RED_EYE if c.i == 3 else RED_EYE_DIM
            if (c.i, c.j) in {(0, 2), (7, 2), (0, 4), (7, 4)}:
                return RED_EYE_DIM
            # fangs
            if (c.i, c.j) in {(2, 6), (5, 6)}:
                return (150, 132, 104)
            if (c.i, c.j) in {(2, 7), (5, 7)}:
                return (96, 82, 64)
            if c.j >= 6:
                return SPIDER[0]
        return col
    if c.role == "neck":
        return spider_fur(c, -0.2)
    if c.role == "abdomen":
        col = spider_fur(c, 0.05)
        if c.face == "top":
            # faint rusty chevrons down the back
            k = c.j % 4
            off = abs(c.i * 2 + 1 - c.w)  # 1,3,5,... from the centre line
            if off // 2 == k and c.j < 11:
                return (96, 44, 30) if c.jit > 0.3 else (74, 34, 24)
        return col
    # legs: 16 long; lighter knobby joints at 1/3 and 2/3, darker claw tips
    if c.face in ("left", "right"):
        return SPIDER[0]
    along = c.x  # 0..16 local along the leg
    if 5.0 < along < 6.0 or 10.0 < along < 11.0:
        return (88, 72, 56) if c.jit > 0.35 else (70, 56, 44)
    if along < 1.5 or along > 14.5:
        return SPIDER[0]
    return spider_fur(c, 0.05)


# ---- player ----------------------------------------------------------------
# An original adventurer: warm skin, dark auburn hair, forest-green hooded
# tunic with a leather belt and satchel strap, rust trousers, dark boots and a
# leather bracer on the left wrist.
PSKIN = pal("#a8694a", "#b97a58", "#c78a65", "#d29a74", "#dca984")
PHAIR = pal("#3a170d", "#4f2114", "#652c1a", "#7a3720", "#8e4428")
PTUNIC = pal("#1c3a20", "#26492a", "#305a33", "#3b6b3c", "#467b45")
PLEATHER = pal("#3f2514", "#52311b", "#653e23", "#784c2c")
PTROUSER = pal("#7a3c20", "#8a4626", "#99512d", "#a75d35")
PBOOT = pal("#1e1510", "#291d16", "#35261c", "#42301f")
PBRASS = (184, 146, 70)
PEYE_WHITE = (238, 234, 226)
PIRIS = (52, 86, 60)
PBROW = (58, 26, 15)
PLIP = (150, 84, 66)


def p_noise(c, pal_, salt, base=0.15, span=0.6, jit=0.3, scale=3.0):
    n = fbm(c.seed + salt, c.gx, c.gy, c.gz, scale, 2)
    return pick(pal_, base + span * n + jit * (c.jit - 0.5))


def pskin(c):
    return p_noise(c, PSKIN, 3, 0.3, 0.4, 0.2)


def phair(c):
    # streaky: stretched along y so locks read as strands
    n = fbm(c.seed + 9, c.gx * 1.8, c.gy * 0.6, c.gz * 1.8, 2.0, 2)
    return pick(PHAIR, 0.1 + 0.7 * n + 0.35 * (c.jit - 0.5))


def ptunic(c):
    return p_noise(c, PTUNIC, 5, 0.2, 0.55, 0.3)


def ptrouser(c):
    return p_noise(c, PTROUSER, 7, 0.15, 0.6, 0.3)


def pleather(c):
    return p_noise(c, PLEATHER, 13, 0.2, 0.5, 0.4, 2.0)


def p_left(c):
    """True for the part on the +X side (the character's left)."""
    return c.gx - c.x >= 0


def p_strap_x(y):
    """Satchel strap column for body row y (0 = shoulder): right shoulder -> left hip."""
    return 1 + int(round(y * 5 / 7))


def paint_player(c):
    if c.role == "head":
        if c.face == "bottom":
            return mul(pskin(c), 0.85)
        if c.face == "top":
            return phair(c)
        if c.face == "front":
            i, j = c.i, c.j
            # tousled fringe swept to the character's left
            if j == 0 or (j == 1 and (i <= 4 or i == 7)) or (j == 2 and i in (0, 1, 7)):
                return phair(c)
            if j == 3 and i in (0, 7):
                return PHAIR[0]  # sideburns
            if j == 3 and i in (1, 2, 5, 6):
                return PBROW
            if j == 4:
                if i in (1, 6):
                    return PEYE_WHITE
                if i in (2, 5):
                    return PIRIS
            if j == 5 and i in (3, 4):
                return mul(pskin(c), 0.86 if i == 3 else 0.93)  # nose shadow
            if j == 5 and i in (1, 6):
                return mix(pskin(c), (200, 100, 86), 0.25)  # cheeks
            if j == 6 and i in (3, 4):
                return PLIP
            if j == 6 and i in (2, 5):
                return mul(pskin(c), 0.92)
            return pskin(c)
        # sides and back: hair, ragged lower edge; longer at the back
        edge = (6 if c.face == "back" else 3) + int(
            hash01(c.seed + 17, int(c.gx * 2), int(c.gz * 2)) * 2)
        if c.face in ("left", "right") and c.z > 6 and c.j >= 2:
            edge = 2 if c.j < 4 else 0  # temple/sideburn region near the face
            if c.z > 7 and c.j == 3:
                return PHAIR[0]
        if c.j < edge:
            return phair(c)
        if c.face in ("left", "right") and 3.5 < c.z < 5 and c.j in (4, 5):
            return mul(pskin(c), 0.84)  # ear
        return pskin(c)
    if c.role == "body":
        if c.face == "top":
            if 2 < c.x < 6 and 1 < c.z < 3:
                return pskin(c)  # neck
            return ptunic(c)
        if c.face == "bottom":
            return ptrouser(c)
        j = c.j
        xi = int(c.x)
        if j in (8, 9):
            if c.face == "front" and j == 8 and xi in (3, 4):
                return PBRASS if xi == 3 else mul(PBRASS, 0.78)
            return pleather(c) if j == 8 else mul(pleather(c), 0.8)
        if c.face in ("front", "back") and j < 8 and xi == p_strap_x(j):
            return PLEATHER[2] if c.jit > 0.4 else PLEATHER[1]
        if c.face == "front" and j < 8 and xi == p_strap_x(j) - 1 and c.jit < 0.25:
            return PLEATHER[0]  # strap shadow
        if c.face == "front" and j <= 2:
            # V-neck with lacing
            if j == 0 and xi in (3, 4):
                return pskin(c)
            if j == 1 and xi in (3, 4):
                return mul(pskin(c), 0.9) if xi == 3 else (210, 196, 160)
            if j == 2 and xi == 3:
                return (210, 196, 160)
        if c.face == "left" and 7 <= j <= 11 and j != 8 and j != 9:
            return pleather(c)  # satchel hanging at the left hip
        col = ptunic(c)
        if j == c.fh - 1:
            col = mul(col, 0.8)  # hem
        return col
    if c.role == "arm":
        left = p_left(c)
        if c.face == "top":
            return ptunic(c)
        if c.face == "bottom":
            return pskin(c)
        j = c.j
        if left and 7 <= j <= 9:
            if j == 8 and c.i in (1, 2) and c.face in SIDES:
                return PBRASS if c.i == 1 else (54, 36, 22)  # buckle / lace
            return pleather(c)
        if not left and j == 8:
            return mix(ptunic(c), (196, 186, 150), 0.25)  # rolled cuff
        if j < (7 if left else 8):
            col = ptunic(c)
            if j == 0:
                col = mul(col, 1.08)
            return col
        col = pskin(c)
        if j == c.fh - 1:
            col = mul(col, 0.9)
        if c.face == "front" and j == c.fh - 1 and c.i in (1, 3):
            col = mul(col, 0.88)  # knuckles
        return col
    # legs
    left = p_left(c)
    if c.face == "bottom":
        return PBOOT[0]
    if c.face == "top":
        return ptrouser(c)
    j = c.j
    if j >= 8:
        if j == 8:
            return PBOOT[3] if c.jit > 0.3 else PBOOT[2]  # folded boot top
        if j == c.fh - 1:
            return PBOOT[0]  # sole
        if c.face == "front" and j == 9 and c.i in (1, 2):
            return (120, 96, 66)  # laces
        return pick(PBOOT, 0.2 + 0.5 * c.jit)
    col = ptrouser(c)
    if c.face == "front" and j in (4, 5):
        col = mul(col, 0.9)  # knee crease
    if left and c.face == "front" and 3 <= j <= 4 and 1 <= c.i <= 2:
        return (118, 92, 58) if c.jit > 0.3 else (104, 80, 50)  # knee patch
    if left and c.face == "front" and (c.i, j) in {(0, 3), (3, 4)}:
        return (40, 24, 18)  # patch stitches
    if c.face in ("left", "right") and j in (0, 1, 2) and c.i == (1 if c.face == "left" else 2):
        return mul(col, 0.85)  # seam
    return col


# ---- deer ------------------------------------------------------------------
DEER = pal("#5e3e22", "#6e4a2a", "#7f5733", "#8f643b", "#9e7244", "#ad8150")
DEER_PALE = pal("#b8a482", "#ccb994", "#ddcca8", "#ece0c2")
DEER_DARK = pal("#2a1c14", "#3a2a1e")
ANTLER = pal("#7a6a50", "#9a8a6a", "#b8aa88", "#d6caa6")
NOSE = (20, 16, 16)
EYE_HI = (200, 190, 170)


def deer_coat(c, bias=0.0):
    n = fbm(c.seed, c.gx, c.gy * 1.4, c.gz, 4.0)
    return pick(DEER, 0.15 + 0.6 * n + 0.3 * (c.jit - 0.5) + bias)


def paint_deer(c):
    if c.role == "body":
        if c.face == "bottom":
            return pick(DEER_PALE, 0.2 + 0.6 * c.jit)
        if c.face == "top":
            # darker stripe down the spine
            return deer_coat(c, -0.25 if 3 <= c.i <= 4 else 0.0)
        if c.face in ("left", "right") and c.j >= c.fh - 2:
            return pick(DEER_PALE, 0.1 + 0.5 * c.jit) if c.j == c.fh - 1 else deer_coat(c, 0.25)
        if c.face == "back":
            # pale rump patch around the tail
            if 1 <= c.i <= 6 and 1 <= c.j <= 5 and not (c.i in (1, 6) and c.j in (1, 5)):
                return pick(DEER_PALE, 0.4 + 0.6 * c.jit)
        if c.face == "front" and c.j >= 4:
            return pick(DEER_PALE, 0.1 + 0.5 * c.jit)  # chest
        return deer_coat(c)
    if c.role == "neck":
        if c.face == "front" or (c.face in ("left", "right") and c.z > c.d - 1):
            return pick(DEER_PALE, 0.1 + 0.6 * c.jit)  # throat
        return deer_coat(c, 0.05)
    if c.role == "head":
        if c.face == "front":
            # muzzle end: black nose, pale lips and chin
            if c.j in (1, 2) and 2 <= c.i <= 3:
                return NOSE
            if c.j == 1 and c.i in (1, 4):
                return DEER_DARK[1]
            if c.j >= 3:
                return pick(DEER_PALE, 0.4 + 0.5 * c.jit) if c.j == 4 else pick(DEER_PALE, 0.1 + 0.4 * c.jit)
            return deer_coat(c, 0.1)
        if c.face in ("left", "right"):
            front = c.z > c.d - 3  # the narrow muzzle
            if c.j == 1 and 3.0 < c.z < 4.0:
                return (14, 10, 8)  # eye
            if c.j == 1 and 2.0 < c.z < 3.0:
                return DEER_DARK[1]  # eye line
            if c.j == 0 and 3.0 < c.z < 4.0:
                return EYE_HI if c.jit > 0.6 else deer_coat(c, 0.2)
            if c.j >= 3 and front:
                return pick(DEER_PALE, 0.2 + 0.5 * c.jit)
            if c.j >= 4:
                return pick(DEER_PALE, 0.1 + 0.4 * c.jit)
            return deer_coat(c, 0.1 if front else 0.0)
        if c.face == "bottom":
            return pick(DEER_PALE, 0.2 + 0.5 * c.jit)
        if c.face == "top" and c.z > c.d - 3:
            return deer_coat(c, -0.15)
        return deer_coat(c)
    if c.role == "ear":
        if c.face == "front":
            return pick(DEER_PALE, 0.0 + 0.3 * c.jit)  # inner ear
        return deer_coat(c, -0.1)
    if c.role in ("antler", "tine"):
        if c.face == "top" or (c.role == "antler" and c.j == 0):
            return ANTLER[3]
        if c.role == "antler" and c.j >= c.fh - 1:
            return ANTLER[0]
        return pick(ANTLER, 0.3 + 0.6 * c.jit)
    if c.role == "tail":
        if c.face in ("back", "bottom") or (c.face in SIDES and c.j >= 2):
            return pick(DEER_PALE, 0.5 + 0.5 * c.jit)  # white flag underneath
        return deer_coat(c, -0.1)
    # legs: same look on all four, pale inside, dark hooves
    lc = SimpleNamespace(**vars(c))
    lc.gx, lc.gy, lc.gz = c.x + 30, c.y, c.z + 30
    if c.face == "bottom" or (c.face in SIDES and c.j >= c.fh - 1):
        return pick(DEER_DARK, c.jit)
    if c.face in SIDES and c.j == c.fh - 2:
        return pick(DEER_PALE, 0.0 + 0.3 * c.jit)
    if c.face in SIDES and c.j >= 6:
        return deer_coat(lc, 0.2)
    return deer_coat(lc)


# ---- fox -------------------------------------------------------------------
FOX = pal("#963e16", "#ac4a1a", "#c0561e", "#d06424", "#e0742e", "#ea8638")
FOX_WHITE = pal("#cfc6b6", "#e0d8ca", "#ece6dc", "#f6f2ea")
FOX_BLACK = pal("#1e1612", "#2a1e18", "#38281e")
FOX_EYE = (196, 132, 30)


def fox_coat(c, bias=0.0):
    n = fbm(c.seed, c.gx, c.gy, c.gz, 3.5)
    return pick(FOX, 0.2 + 0.6 * n + 0.3 * (c.jit - 0.5) + bias)


def paint_fox(c):
    if c.role == "body":
        if c.face == "bottom":
            return pick(FOX_WHITE, 0.2 + 0.6 * c.jit)
        if c.face == "front":
            return pick(FOX_WHITE, 0.3 + 0.6 * c.jit) if c.j >= 2 else fox_coat(c)
        if c.face in ("left", "right") and c.j == c.fh - 1 and c.z > 5:
            return pick(FOX_WHITE, 0.1 + 0.4 * c.jit)
        if c.face == "top":
            return fox_coat(c, -0.15 if 2 <= c.i <= 3 else 0.0)
        return fox_coat(c)
    if c.role == "head":
        if c.face == "front":
            if c.j == 1 and c.i in (1, 4):
                return (16, 12, 10)  # pupils
            if c.j == 1 and c.i in (0, 5):
                return FOX_EYE
            if c.j == 0 and c.i in (1, 4):
                return FOX[0]  # brow
            if c.j >= 3 and (c.i <= 1 or c.i >= 4):
                return pick(FOX_WHITE, 0.3 + 0.6 * c.jit)  # cheeks
            if c.j >= 3:
                return pick(FOX_WHITE, 0.1 + 0.4 * c.jit)
            return fox_coat(c, 0.1)
        if c.face in ("left", "right"):
            if c.j >= 3 and c.z > 1:
                return pick(FOX_WHITE, 0.2 + 0.6 * c.jit)
            return fox_coat(c)
        if c.face == "bottom":
            return pick(FOX_WHITE, 0.2 + 0.5 * c.jit)
        return fox_coat(c)
    if c.role == "snout":
        if c.face == "front":
            if c.j == 0:
                return FOX_BLACK[0]  # nose
            return pick(FOX_WHITE, 0.3 + 0.5 * c.jit)
        if c.face == "top":
            return FOX_BLACK[1] if c.j == c.fh - 1 else fox_coat(c, 0.15)
        if c.face == "bottom" or (c.face in SIDES and c.j == 1):
            return pick(FOX_WHITE, 0.2 + 0.6 * c.jit)
        return fox_coat(c, 0.15)
    if c.role == "ear":
        if c.j == 0 or c.face == "top":
            return FOX_BLACK[1]  # black tips
        if c.face == "front":
            return FOX_BLACK[2] if c.jit < 0.5 else FOX_WHITE[0]
        return fox_coat(c, -0.1)
    if c.role == "tail":
        if c.z < 2.5:
            return pick(FOX_WHITE, 0.4 + 0.6 * c.jit)  # white tip
        if c.z < 3.5 and c.jit > 0.5:
            return pick(FOX_WHITE, 0.2)
        if c.face == "bottom":
            return fox_coat(c, -0.15)
        return fox_coat(c)
    # legs: black stockings
    if c.face == "top":
        return fox_coat(c)
    if c.face == "bottom" or c.j >= 2:
        return pick(FOX_BLACK, 0.2 + 0.7 * c.jit)
    return fox_coat(c, -0.1)


# ---- rabbit ----------------------------------------------------------------
RABBIT = pal("#5a4836", "#6a5640", "#7c6650", "#8c765e", "#9c866c", "#ab967c")
RABBIT_PALE = pal("#c4b8a4", "#d6ccba", "#e6ded0", "#f2ece2")
RABBIT_PINK = pal("#b07a74", "#c88e86", "#dca49a")


def rabbit_coat(c, bias=0.0):
    n = fbm(c.seed, c.gx * 1.5, c.gy * 1.5, c.gz * 1.5, 3.0)
    return pick(RABBIT, 0.2 + 0.6 * n + 0.3 * (c.jit - 0.5) + bias)


def paint_rabbit(c):
    if c.role == "body":
        if c.face == "bottom":
            return pick(RABBIT_PALE, 0.2 + 0.6 * c.jit)
        if c.face == "front" and c.j >= 1:
            return pick(RABBIT_PALE, 0.1 + 0.5 * c.jit)
        if c.face in ("left", "right") and c.j == c.fh - 1:
            return rabbit_coat(c, 0.3)
        return rabbit_coat(c)
    if c.role == "head":
        if c.face == "front":
            if c.j == 2 and c.i in (1, 2):
                return RABBIT_PINK[1]  # nose
            if c.j == 3 and c.i in (1, 2):
                return RABBIT_PALE[2]
            if c.j == 3:
                return RABBIT_PALE[1]
            return rabbit_coat(c, 0.15)
        if c.face in ("left", "right"):
            if c.j == 1 and 2.0 < c.z < 3.0:
                return (12, 8, 8)  # eye
            if c.j == 0 and 2.0 < c.z < 3.0:
                return (180, 170, 160)
            if c.j == 3:
                return pick(RABBIT_PALE, 0.1 + 0.4 * c.jit)
            return rabbit_coat(c, 0.05)
        if c.face == "bottom":
            return pick(RABBIT_PALE, 0.3 + 0.5 * c.jit)
        return rabbit_coat(c)
    if c.role == "ear":
        if c.face == "front" and c.j >= 1:
            return RABBIT_PINK[1] if c.j < 3 else RABBIT_PINK[0]  # inside of the ear
        if c.j == 0 or c.face == "top":
            return RABBIT[1]
        return rabbit_coat(c, -0.05)
    if c.role == "tail":
        return pick(RABBIT_PALE, 0.5 + 0.5 * c.jit)
    if c.role == "foreleg":
        if c.face == "bottom" or c.j == c.fh - 1:
            return pick(RABBIT_PALE, 0.2 + 0.5 * c.jit)
        return rabbit_coat(c, 0.1)
    # hind legs: big pale feet
    if c.face in ("bottom", "front") or (c.face in SIDES and c.j == c.fh - 1):
        return pick(RABBIT_PALE, 0.1 + 0.6 * c.jit)
    return rabbit_coat(c)


# ---- bear ------------------------------------------------------------------
BEAR = pal("#2e1c12", "#3a2416", "#4a2e1c", "#5a3824", "#6a442c", "#7a5034")
BEAR_MUZZLE = pal("#6e5238", "#8a6a48", "#a07e58", "#b48e66")
BEAR_CLAW = (206, 196, 172)


def bear_coat(c, bias=0.0):
    n = fbm(c.seed, c.gx, c.gy, c.gz, 5.0)
    streak = hash01(c.seed + 3, c.px, c.py // 2)
    return pick(BEAR, 0.2 + 0.55 * n + 0.25 * (c.jit - 0.5) + 0.15 * (streak - 0.5) + bias)


def paint_bear(c):
    if c.role in ("body", "hump"):
        if c.face == "bottom":
            return bear_coat(c, -0.25)
        if c.face == "top":
            return bear_coat(c, 0.1)
        return bear_coat(c)
    if c.role == "head":
        if c.face == "front":
            if c.j == 2 and c.i in (1, 6):
                return (10, 8, 8)  # small eyes
            if c.j == 1 and c.i in (1, 6):
                return BEAR[0]
            if c.j >= 4 and 1 <= c.i <= 6:
                return pick(BEAR_MUZZLE, 0.1 + 0.5 * c.jit)  # around the snout
            return bear_coat(c, 0.1)
        if c.face == "bottom":
            return bear_coat(c, -0.2)
        return bear_coat(c, 0.05)
    if c.role == "snout":
        if c.face == "front":
            if c.j == 0 and 1 <= c.i <= 2:
                return (16, 12, 12)  # nose
            if c.j == 0:
                return (40, 30, 26)
            if c.j == 2 and 1 <= c.i <= 2:
                return BEAR_MUZZLE[0]
            return pick(BEAR_MUZZLE, 0.3 + 0.6 * c.jit)
        if c.face == "top" and c.j == c.fh - 1 and 1 <= c.i <= 2:
            return (24, 18, 16)
        return pick(BEAR_MUZZLE, 0.2 + 0.6 * c.jit)
    if c.role == "ear":
        if c.face == "front":
            return BEAR_MUZZLE[0]
        return bear_coat(c, -0.1)
    # legs: same on all four, darker paws with claws in front
    lc = SimpleNamespace(**vars(c))
    lc.gx, lc.gy, lc.gz = c.x + 40, c.y, c.z + 40
    if c.face == "front" and c.j == c.fh - 1:
        return BEAR_CLAW if c.i % 2 == 0 else BEAR[0]
    if c.face == "bottom" or (c.face in SIDES and c.j >= c.fh - 2):
        return pick(BEAR, 0.0 + 0.25 * c.jit)
    return bear_coat(lc, -0.05)


# ---- duck ------------------------------------------------------------------
DUCK_GRAY = pal("#7a7872", "#8e8c86", "#a2a09a", "#b6b4ae", "#c8c6c0")
DUCK_BACK = pal("#4a4034", "#5a4e40", "#6a5c4c")
DUCK_CHEST = pal("#5a301e", "#6e3c26", "#80482e", "#905636")
DUCK_GREEN = pal("#163a22", "#1e4a2c", "#24603a", "#2e7a48", "#3a9058")
DUCK_BILL = pal("#b08a18", "#c8a020", "#e0b830", "#f0d050")
DUCK_FOOT = pal("#b86418", "#d07a20", "#e8902a")
SPECULUM = pal("#22388a", "#2c4aa8", "#3a62c8")
WHITE = pal("#d8d6d0", "#eceae4", "#f8f6f2")


def paint_duck(c):
    t = 0.2 + 0.6 * c.jit
    if c.role == "body":
        if c.face == "front":
            return pick(DUCK_CHEST, t)  # chestnut breast
        if c.face == "top":
            return pick(DUCK_BACK, t) if c.z < c.d - 2 else pick(DUCK_CHEST, t)
        if c.face == "bottom":
            return pick(DUCK_GRAY, 0.5 + 0.4 * c.jit)
        if c.face in ("left", "right"):
            if c.z > c.d - 2.5 and c.j <= 3:
                return pick(DUCK_CHEST, t)
            if c.j == 0:
                return pick(DUCK_BACK, t)
            # fine wavy barring on the flanks
            return pick(DUCK_GRAY, t + (0.15 if (c.j + int(c.z)) % 3 == 0 else 0.0))
        if c.face == "back":
            return pick(DUCK_BACK, t) if c.j <= 1 else pick(WHITE, t)
        return pick(DUCK_GRAY, t)
    if c.role == "head":
        # glossy green head, white collar on the bottom row
        if c.j == c.fh - 1 and c.face in SIDES:
            return pick(WHITE, t) if c.face != "front" else pick(DUCK_CHEST, t)
        if c.face == "bottom":
            return pick(DUCK_CHEST, t)
        if c.face in ("left", "right") and c.j == 1 and 2.0 < c.z < 3.0:
            return (10, 10, 8)  # eye
        sheen = 0.25 if (c.face == "top" or c.j == 0) else 0.0
        return pick(DUCK_GREEN, 0.15 + 0.6 * c.jit + sheen)
    if c.role == "bill":
        if c.face == "front" and c.i in (1, 2):
            return DUCK_BILL[0]
        if c.face == "top" and c.j == c.fh - 1 and c.i in (1, 2):
            return (40, 34, 20)  # nail at the tip
        if c.face == "bottom":
            return DUCK_BILL[1]
        return pick(DUCK_BILL, 0.4 + 0.6 * c.jit)
    if c.role == "tail":
        if c.face == "top" or (c.face == "back" and c.j == 0):
            return (24, 22, 20)  # black curl
        return pick(WHITE, t)
    if c.role == "wing":
        if c.face in ("left", "right"):
            # brown-gray with the blue patch near the rear, edged in white
            rear = c.z < 3.0
            if rear and c.j == 1:
                return WHITE[2]
            if rear and c.j == 2:
                return pick(SPECULUM, 0.3 + 0.7 * c.jit)
            if rear and c.j == 3:
                return WHITE[1]
            return pick(DUCK_BACK, t + 0.2) if c.j == 0 else pick(DUCK_GRAY, t - 0.2)
        return pick(DUCK_BACK, t)
    # legs
    if c.face == "bottom" or c.j == c.fh - 1:
        return DUCK_FOOT[0]
    return pick(DUCK_FOOT, 0.3 + 0.6 * c.jit)


# ---- forest guardian (boss) ------------------------------------------------
GBARK = pal("#2a1c10", "#3a2816", "#4c341c", "#5e4224", "#70522e")
GLEAF = pal("#1c3a12", "#28501a", "#356a22", "#44822c", "#5a9a3a")
GLOW = (120, 255, 140)
GLOW_DIM = (40, 160, 70)


def gbark(c, bias=0.0):
    n = fbm(c.seed + 7, c.gx, c.gy * 0.5, c.gz, 2.6, 3)
    # vertical grooves in the bark
    groove = (int(c.gx + c.gz * 0.7) % 3 == 0) and c.face in SIDES
    t = 0.2 + 0.65 * n + 0.25 * (c.jit - 0.5) + bias - (0.25 if groove else 0.0)
    return pick(GBARK, t)


def paint_forest_guardian(c):
    if c.role == "head":
        if c.face == "front":
            # two glowing eyes deep under a heavy brow, a hollow knot for a mouth
            if (c.i, c.j) in {(2, 4), (3, 4), (6, 4), (7, 4)}:
                return GLOW if c.i in (3, 6) else GLOW_DIM
            if (c.i, c.j) in {(2, 5), (3, 5), (6, 5), (7, 5)}:
                return GLOW_DIM
            if c.j == 3 and 1 <= c.i <= 8:
                return GBARK[0]  # brow shadow
            if 3 <= c.i <= 6 and 7 <= c.j <= 8:
                return (14, 10, 6) if 4 <= c.i <= 5 else GBARK[0]
        return gbark(c, 0.05)
    if c.role == "body":
        if c.face == "front" and 4 <= c.i <= 7 and 4 <= c.j <= 7:
            # a green heart-light showing through a crack in the chest
            core = (c.i in (5, 6) and c.j in (5, 6))
            return GLOW if core else GLOW_DIM if c.jit > 0.4 else GBARK[0]
        return gbark(c)
    if c.role == "arm":
        if c.j >= c.fh - 3 and c.face in SIDES:
            return pick(GBARK, 0.1 + 0.3 * c.jit)  # gnarled fingers
        return gbark(c, 0.05)
    if c.role == "leg":
        if c.face == "bottom" or (c.face in SIDES and c.j >= c.fh - 2):
            return pick(ROOT_C, c.jit)
        return gbark(c, -0.05)
    # branches on the head: bark with a leaf at the tip
    if c.face == "top" or (c.face in SIDES and c.j == 0):
        return pick(GLEAF, 0.4 + 0.5 * c.jit)
    return gbark(c, 0.1)


def overlay_forest_guardian(c):
    if c.face == "bottom":
        return None
    if c.role == "head":
        if c.face == "front":
            return pick(GLEAF, 0.3 + 0.6 * c.jit) if c.j == 0 and c.jit > 0.3 else None
        if c.face == "top" or c.j <= 2 + int(c.jit * 3):
            return pick(GLEAF, 0.2 + 0.7 * c.jit) if c.jit > 0.15 else None  # leafy crown
        return None
    if c.role == "body":
        if c.face == "top" or (c.j <= 2 and c.jit > 0.3):
            return pick(MOSS, 0.2 + 0.7 * c.jit)  # moss on the shoulders
        if clustered(c, 71, 2.0, 0.62) and c.jit > 0.35:
            return pick(MOSS, 0.3 + 0.6 * c.jit)
        return None
    if c.role == "arm":
        if c.face == "top" or (c.j <= 3 and c.jit > 0.35):
            return pick(MOSS, 0.2 + 0.7 * c.jit)
        return None
    return None


# ---- town NPCs: the player's painter in each one's own colours --------------
TOWNSFOLK = {
    "npc_guide": dict(PHAIR=pal("#2a1a0e", "#3a2412", "#4c3018", "#5e3c1e", "#6e4824"),
                      PTUNIC=pal("#1c2e4a", "#243a5c", "#2e486e", "#385680", "#426490"),
                      PTROUSER=pal("#6a5a3a", "#7a6844", "#8a764e", "#988458")),
    "npc_merchant": dict(PHAIR=pal("#5a5a5a", "#6e6e6e", "#828282", "#969696", "#aaaaaa"),
                         PTUNIC=pal("#4a2c10", "#5c3816", "#6e441c", "#805022", "#925c28"),
                         PTROUSER=pal("#2a3a20", "#344628", "#3e5230", "#485e38"),
                         PSKIN=pal("#c08a66", "#cc9874", "#d6a582", "#e0b290", "#e8bf9e")),
    "npc_nurse": dict(PHAIR=pal("#8a3a1a", "#a04622", "#b6542a", "#c86234", "#d6703e"),
                      PTUNIC=pal("#c8c8c8", "#d6d6d6", "#e2e2e2", "#ececec", "#f6f6f6"),
                      PTROUSER=pal("#b8b8c0", "#c6c6ce", "#d2d2da", "#dedee6"),
                      PLEATHER=pal("#8a1a1a", "#a02222", "#b42a2a", "#c83434"),
                      PSKIN=pal("#d0a080", "#daac8c", "#e4b898", "#ecc4a4", "#f2d0b2")),
    "npc_wizard": dict(PHAIR=pal("#b8b8b8", "#c8c8c8", "#d8d8d8", "#e6e6e6", "#f4f4f4"),
                       PTUNIC=pal("#2a1448", "#341a5a", "#40206c", "#4c287e", "#5a3090"),
                       PTROUSER=pal("#2a1448", "#341a5a", "#40206c", "#4c287e"),
                       PLEATHER=pal("#8a6a10", "#a07c16", "#b88e1c", "#cca024"),
                       PIRIS=(70, 60, 140)),
    "npc_smith": dict(PHAIR=pal("#141010", "#1e1814", "#282018", "#32281e", "#3c3024"),
                      PTUNIC=pal("#2a2a2c", "#343436", "#3e3e40", "#48484a", "#525254"),
                      PTROUSER=pal("#3a2a1a", "#463220", "#523a26", "#5e442c"),
                      PSKIN=pal("#7a4a30", "#8a563a", "#9a6244", "#a86e4e", "#b47a58")),
    "npc_guard": dict(PHAIR=pal("#2a1a0e", "#3a2412", "#4c3018", "#5e3c1e", "#6e4824"),
                      PTUNIC=pal("#5a5e66", "#6c7078", "#80848c", "#9498a0", "#a8acb4"),
                      PTROUSER=pal("#1e2a48", "#263456", "#2e3e64", "#384872"),
                      PLEATHER=pal("#5a3a1a", "#6c4620", "#7e5226", "#905e2c")),
}


def townsfolk(name, base, colours=None):
    colours = colours or TOWNSFOLK[name]

    def paint(c):
        g = globals()
        saved = {k: g[k] for k in colours}
        g.update(colours)
        try:
            return base(c)
        finally:
            g.update(saved)
    return paint


# Recoloured copies of another mob's model and painter: name -> (base, palette overrides).
RECOLORED = {
    "goblin_trader": ("player", dict(
        PSKIN=pal("#4e6e2a", "#5c7e32", "#6a8e3a", "#789e44", "#86ac4e"),
        PHAIR=pal("#1a1410", "#241c16", "#2e241c", "#382c22", "#423428"),
        PTUNIC=pal("#5a2a14", "#6c341a", "#7e3e20", "#904826", "#a2522c"),
        PTROUSER=pal("#2e2a22", "#38342a", "#423e32", "#4c483a"),
        PLEATHER=pal("#8a6a10", "#a07c16", "#b88e1c", "#cca024"),
        PIRIS=(200, 150, 30))),
    "mutant_zombie": ("zombie", dict(
        ZSKIN=pal("#2e4430", "#3a5438", "#466444", "#52724e", "#5e7e58"),
        ZSHIRT=pal("#2a1a14", "#36221a", "#422a20", "#4e3226"),
        ZPANTS=pal("#141420", "#1c1c2c", "#242438", "#2c2c44"))),
    "polar_bear": ("bear", dict(
        BEAR=pal("#b8b4a8", "#c8c4b8", "#d6d2c8", "#e2dfd6", "#ecead2", "#f6f4ec"),
        BEAR_MUZZLE=pal("#a8a498", "#bab6aa", "#ccc8bc", "#dcd8cc"))),
    "penguin": ("duck", dict(
        DUCK_GRAY=pal("#16181e", "#1e2028", "#262a32", "#2e323c", "#363a46"),
        DUCK_BACK=pal("#101216", "#16181e", "#1e2026"),
        DUCK_CHEST=pal("#d8d6d0", "#e4e2dc", "#eeece6", "#f8f6f2"),
        DUCK_GREEN=pal("#0e1014", "#14161c", "#1a1d24", "#20242c", "#262a34"),
        DUCK_BILL=pal("#b85a14", "#d06c1a", "#e07e22", "#f0902c"),
        SPECULUM=pal("#16181e", "#1e2028", "#262a32"))),
    "boar": ("pig", dict(
        PIG=pal("#3a2a1e", "#4a3626", "#5a422e", "#6a4e36", "#7a5a40"),
        PIG_SNOUT=pal("#6a4a3a", "#7a5848", "#8a6656", "#9a7464"),
        PIG_HOOF=pal("#1a1210", "#2a1e18"))),
    "squirrel": ("fox", dict(
        FOX=pal("#6a3a1e", "#7c4624", "#8e522a", "#a05e32", "#b06a3a", "#be7844"),
        FOX_WHITE=pal("#bcae98", "#ccc0aa", "#dcd2c0", "#e8e0d2"),
        FOX_BLACK=pal("#2a1a12", "#36241a", "#422e20"))),
}
for _n, (_b, _) in RECOLORED.items():
    ROLES[_n] = ROLES[_b]
    HARDCODED[_n] = HARDCODED[_b]

for _n, (_b, _) in RECOLORED.items():
    HARDCODED_LAYERS[_n] = HARDCODED_LAYERS[_b]


PAINTERS = {
    "pig": paint_pig, "cow": paint_cow, "sheep": paint_sheep,
    "chicken": paint_chicken, "zombie": paint_zombie,
    "skeleton": paint_skeleton, "stalker": paint_stalker, "spider": paint_spider,
    "player": paint_player,
    "deer": paint_deer, "fox": paint_fox, "rabbit": paint_rabbit, "bear": paint_bear,
    "duck": paint_duck, "forest_guardian": paint_forest_guardian,
}
for _n in TOWNSFOLK:
    PAINTERS[_n] = townsfolk(_n, paint_player)
for _n, (_b, _c) in RECOLORED.items():
    PAINTERS[_n] = townsfolk(_n, PAINTERS[_b], _c)


# --------------------------------------------------------------------------
# Layer overlays.  Same context as the painters; return an (r, g, b) for an
# opaque relief pixel or None to leave the pixel empty.
# --------------------------------------------------------------------------


def clustered(c, salt, scale, threshold):
    """True where low-frequency noise says a tuft/patch lives here."""
    return fbm(c.seed + salt, c.gx, c.gy, c.gz, scale, 2) > threshold


def overlay_sheep(c):
    if c.role == "head":
        # wool cap: top face, upper rows of the sides, nothing on the face
        if c.face == "front" and c.j > 0:
            return None
        if c.face == "bottom":
            return None
        if c.face in ("left", "right", "back") and c.j > 2:
            return None
    elif c.role == "leg":
        if c.face == "bottom" or (c.face in SIDES and c.j > 3):
            return None
    elif c.face == "bottom":
        return None
    if c.jit < 0.25:
        return None  # ~25% holes so it reads as fluff
    tip = hash01(c.seed + 13, c.px, c.py)
    if tip > 0.88:
        return (248, 246, 240)
    return wool(c, 1.8)


def overlay_cow(c):
    if c.role == "head":
        # ear nubs at the top back corners of the sides, a few hair tufts on top
        if c.face in ("left", "right") and c.j <= 1 and c.z < 2:
            return pick(COW_BROWN, 0.5 + 0.4 * c.jit)
        if c.face == "top" and c.z < 2 and c.jit > 0.55:
            return pick(COW_BROWN, 0.6 + 0.4 * c.jit)
        if c.face == "top" and 2 <= c.i <= 5 and c.j >= 4 and c.jit > 0.5:
            return pick(COW_WHITE, 0.4 + 0.4 * c.jit)  # forelock
        return None
    # body: shaggy ruff around the neck, sparse fur patches on the back
    if c.face == "bottom":
        return None
    if c.face == "front" and c.j <= 4 and c.jit > 0.35:
        return cow_coat(c)
    if c.face == "top" and c.j >= c.fh - 3 and c.jit > 0.45:
        return cow_coat(c)
    if clustered(c, 71, 4.0, 0.62) and c.jit > 0.4:
        return cow_coat(c)
    return None


def overlay_pig(c):
    if c.role == "head":
        # floppy ear flaps hanging from the top back corners of the head sides
        if c.face in ("left", "right") and c.j <= 2 and c.z < 3 and not (c.j == 2 and c.z < 1):
            return pick(PIG, 0.25 + 0.3 * c.jit)
        if c.face == "top" and c.z < 2 and c.x not in (3, 4):
            return pick(PIG, 0.3 + 0.3 * c.jit)
        return None
    # body: sparse dried-mud spots, mostly on the lower sides and belly
    if c.face == "top" and c.jit < 0.9:
        return None
    low = c.face == "bottom" or (c.face in SIDES and c.j >= c.fh // 2)
    if clustered(c, 37, 3.0, 0.66) and c.jit > (0.3 if low else 0.7):
        return (104, 76, 50) if c.jit > 0.6 else (124, 92, 60)
    return None


def overlay_chicken(c):
    if c.role == "wing":
        if c.face in ("left", "right") and (c.j == c.fh - 1 or c.i <= 1) and c.jit > 0.3:
            return pick(WING, 0.2 + 0.5 * c.jit)
        return None
    # body: ruffled feathers on the sides and back, a tail tuft
    if c.face in ("bottom", "top", "front"):
        return None
    if c.face == "back" and c.j <= 2 and c.jit > 0.25:
        return pick(FEATHER, 0.3 + 0.6 * c.jit)
    if c.j >= 2 and (c.i + c.j) % 2 == 0 and c.jit > 0.35:
        return pick(FEATHER, 0.2 + 0.6 * c.jit)
    return None


def overlay_zombie(c):
    if c.role == "head":
        if c.face == "top" and c.jit > 0.45:
            return (34, 32, 28) if c.jit < 0.8 else (52, 48, 40)
        if c.face in ("back", "left", "right") and c.j == 0 and c.jit > 0.5:
            return (34, 32, 28)
        return None
    if c.role == "body":
        if c.face in ("top", "bottom"):
            return None
        rag = hash01(c.seed + 41, c.px // 2, c.py)
        if c.j == 0 and c.jit > 0.4:
            return zshirt(c)  # torn shoulder seams
        if c.j >= 7 and rag > 0.45 and c.jit > 0.25:
            return zshirt(c) if c.j < 11 or c.jit > 0.5 else ZSHIRT[0]  # jacket flaps
        if c.face in ("front", "back") and clustered(c, 43, 2.5, 0.6) and c.jit > 0.6:
            return zshirt(c)
        return None
    if c.role == "arm":
        if c.face in ("top", "bottom"):
            return None
        if c.j <= 3 and c.jit > 0.55:
            return zshirt(c)  # ragged sleeve edge
        if c.j >= 9 and hash01(c.seed + 45, c.px, 0) > 0.5 and c.jit > 0.3:
            return ZSHIRT[1] if c.jit > 0.6 else ZSHIRT[0]  # hanging rags
        return None
    # legs
    if c.face in ("top", "bottom"):
        return None
    if c.j >= 8 and hash01(c.seed + 47, c.px, 1) > 0.45 and c.jit > 0.3:
        return pick(ZPANTS, 0.3 + 0.5 * c.jit)
    return None


def overlay_skeleton(c):
    if c.face in ("top", "bottom"):
        return None
    cloth = pal("#4a4238", "#5c5346", "#6e6454")
    # a few tattered vertical strips hanging from the shoulders
    col = c.px
    strip = hash01(c.seed + 51, col, 0)
    if strip < 0.3:
        return None
    length = 4 + int(hash01(c.seed + 53, col, 1) * 8)
    if c.j < length and c.jit > 0.3:
        return pick(cloth, 0.3 + 0.6 * c.jit)
    return None


def overlay_stalker(c):
    if c.role == "head" and c.face == "front" and 1 <= c.i <= 6 and 1 <= c.j <= 7:
        return None  # keep the face clear
    if c.face == "bottom":
        return None
    n = fbm(c.seed + 61, c.gx, c.gy, c.gz, 2.5, 2)
    if n + 0.35 * (c.jit - 0.5) < 0.52:
        return None
    leaf = hash01(c.seed + 63, c.px, c.py)
    if leaf > 0.86:
        return pick(LICHEN, 0.3 + 0.6 * c.jit)
    if leaf > 0.7:
        return MOSS[5] if c.jit > 0.5 else MOSS[4]
    return pick(MOSS, 0.3 + 0.6 * c.jit)


def overlay_spider(c):
    if c.face == "bottom":
        return None
    h = hash01(c.seed + 81, c.px, c.py)
    if h > 0.9:
        return HAIR
    if c.role == "abdomen" and c.face in SIDES:
        # short vertical hairs: 1px wide, 2px long, scattered
        k = hash01(c.seed + 83, c.px, c.py // 2)
        if k > 0.86:
            return SPIDER[4] if c.py % 2 else HAIR
    return None


def overlay_player(c):
    j = c.j
    if c.role == "head":
        if c.face == "bottom":
            return None
        if c.face == "front":
            # a few locks falling over the forehead, never over the eyes
            if j == 0 and c.jit > 0.2:
                return phair(c)
            if j == 1 and c.i in (1, 2, 3) and c.jit > 0.35:
                return phair(c)
            if j == 2 and c.i == 2 and c.jit > 0.3:
                return PHAIR[1]
            return None
        if c.face == "top":
            if clustered(c, 91, 2.5, 0.42) and c.jit > 0.3:
                return phair(c)  # tousled tufts
            if c.z < 1.5 and c.jit > 0.35:
                return ptunic(c)  # hood rim lying behind the crown
            return None
        # hood pushed back: folds at the back of the head and the rear of the sides
        if c.face == "back" and j >= 4 and c.jit > 0.18:
            return ptunic(c) if j > 4 else mul(ptunic(c), 0.82)
        if c.face in ("left", "right") and c.z < 2.5 and j >= 3 and c.jit > 0.25:
            return mul(ptunic(c), 0.9 if c.z < 1.5 else 0.8)
        if c.face == "back" and j < 4 and hash01(c.seed + 93, c.px, 0) > 0.4 and c.jit > 0.3:
            return phair(c)  # locks over the hood
        if c.face in ("left", "right") and j <= 2 and c.jit > 0.45:
            return phair(c)
        return None
    if c.role == "body":
        if c.face == "bottom":
            return None
        if c.face == "top":
            # hood bunched around the back of the neck
            if c.z < 1.5 and c.jit > 0.2:
                return ptunic(c)
            return None
        xi = int(c.x)
        if j == 8:
            if c.face == "front" and xi in (3, 4):
                return PBRASS if xi == 3 else mul(PBRASS, 0.78)
            return pleather(c)
        if c.face in ("front", "back") and j < 8 and xi == p_strap_x(j):
            return PLEATHER[2] if c.jit > 0.4 else PLEATHER[1]
        if c.face == "back" and j <= 1 and c.jit > 0.25:
            return mul(ptunic(c), 0.85)  # hood base
        if c.face == "front" and j == 0 and xi not in (3, 4) and c.jit > 0.3:
            return mul(ptunic(c), 0.9)  # collar
        if c.face == "left" and 9 <= j <= 11 and c.jit > 0.15:
            return pleather(c) if j > 9 else PLEATHER[3]  # satchel flap
        return None
    if c.role == "arm":
        if c.face in ("top", "bottom"):
            return None
        if p_left(c):
            if j in (7, 9) and c.jit > 0.15:
                return pleather(c)  # bracer edges
            if j == 8 and c.i == 1 and c.face == "front":
                return PBRASS
            return None
        if j == 8 and c.jit > 0.2:
            return mix(ptunic(c), (196, 186, 150), 0.2)  # rolled cuff
        return None
    # legs
    if c.face in ("top", "bottom"):
        return None
    if j == 7 and c.jit > 0.3:
        return mul(ptrouser(c), 0.85)  # trouser cuff over the boot
    if j == 8 and c.jit > 0.15:
        return PBOOT[3] if c.jit > 0.5 else PBOOT[2]  # boot top fold
    return None


def overlay_deer(c):
    if c.face == "bottom":
        return None
    if c.role == "neck":
        # a shaggy ruff under the throat
        if c.face == "front" and c.j >= 3 and c.jit > 0.3:
            return pick(DEER_PALE, 0.3 + 0.5 * c.jit)
        if c.face in ("left", "right") and c.j >= 4 and c.z > 2 and c.jit > 0.5:
            return deer_coat(c, 0.1)
        return None
    # body: sparse fur along the spine and over the shoulders
    if c.face == "top" and 2 <= c.i <= 5 and c.jit > 0.45:
        return deer_coat(c, -0.2)
    if c.face in ("left", "right") and c.j <= 1 and c.jit > 0.7:
        return deer_coat(c, -0.1)
    if c.face == "front" and c.j >= 3 and c.jit > 0.4:
        return pick(DEER_PALE, 0.4 + 0.5 * c.jit)
    return None


def overlay_fox(c):
    if c.role == "tail":
        if c.jit < 0.3:
            return None
        if c.z < 2.5:
            return pick(FOX_WHITE, 0.5 + 0.5 * c.jit)
        return fox_coat(c, 0.1)
    if c.face == "bottom":
        return None
    # white bib on the chest, a few tufts on the back
    if c.face == "front" and c.j >= 1 and c.jit > 0.25:
        return pick(FOX_WHITE, 0.4 + 0.6 * c.jit)
    if c.face == "top" and clustered(c, 101, 2.5, 0.6) and c.jit > 0.4:
        return fox_coat(c, 0.15)
    return None


def overlay_rabbit(c):
    if c.role == "tail":
        if c.jit < 0.15:
            return None
        return pick(RABBIT_PALE, 0.6 + 0.4 * c.jit)
    if c.face == "bottom":
        return None
    if clustered(c, 111, 2.0, 0.55) and c.jit > 0.45:
        return rabbit_coat(c, 0.1)
    return None


def overlay_bear(c):
    if c.face == "bottom":
        return None
    if c.role == "head":
        # fur on the crown and cheeks, the face stays clear
        if c.face == "front":
            if c.i in (0, 7) and c.j >= 2 and c.jit > 0.3:
                return bear_coat(c, -0.05)
            return None
        if c.face == "top" and c.jit > 0.35:
            return bear_coat(c, 0.05)
        if c.face in ("left", "right") and c.jit > 0.5:
            return bear_coat(c)
        return None
    if c.role == "leg":
        if c.face == "top" or c.j > 4:
            return None
        return bear_coat(c, -0.05) if c.jit > 0.35 else None
    # body and hump: thick shaggy coat
    if c.jit < 0.28:
        return None
    if c.role == "body" and c.face == "front" and 2 <= c.i <= 9 and c.j <= 7:
        return None  # where the head sits
    tip = hash01(c.seed + 121, c.px, c.py)
    return bear_coat(c, 0.15 if tip > 0.85 else -0.05)


def overlay_duck(c):
    if c.role == "wing":
        if c.face in ("left", "right") and (c.j == c.fh - 1 or c.z < 1.5) and c.jit > 0.3:
            return pick(DUCK_BACK, 0.3 + 0.6 * c.jit)  # primary feather tips
        return None
    if c.face in ("bottom", "top", "front"):
        return None
    if c.face == "back" and c.j >= 2 and c.jit > 0.4:
        return pick(WHITE, 0.3 + 0.6 * c.jit)
    if c.face in ("left", "right") and c.j >= 2 and (c.i + c.j) % 2 == 0 and c.jit > 0.4:
        return pick(DUCK_GRAY, 0.3 + 0.6 * c.jit)
    return None


OVERLAYS = {
    "pig": overlay_pig, "cow": overlay_cow, "sheep": overlay_sheep,
    "chicken": overlay_chicken, "zombie": overlay_zombie,
    "skeleton": overlay_skeleton, "stalker": overlay_stalker, "spider": overlay_spider,
    "player": overlay_player,
    "deer": overlay_deer, "fox": overlay_fox, "rabbit": overlay_rabbit, "bear": overlay_bear,
    "duck": overlay_duck, "forest_guardian": overlay_forest_guardian,
}
for _n in TOWNSFOLK:
    OVERLAYS[_n] = townsfolk(_n, overlay_player)
for _n, (_b, _c) in RECOLORED.items():
    OVERLAYS[_n] = townsfolk(_n, OVERLAYS[_b], _c)


# --------------------------------------------------------------------------
# Driver
# --------------------------------------------------------------------------


def face_pixels(name, role, size, origin, uv, seed, v_off):
    """Yield (sx, sy, context) for every pixel of every face of a box."""
    w, h, d = size
    for face, (x0, y0, fw, fh) in face_rects(size, uv).items():
        for j in range(fh):
            for i in range(fw):
                sx, sy = x0 + i, y0 + j + v_off
                lx, ly, lz = local_pos(face, i, j, size)
                c = SimpleNamespace(
                    role=role, face=face, i=i, j=j, fw=fw, fh=fh,
                    x=lx, y=ly, z=lz, w=w, h=h, d=d,
                    gx=origin[0] + lx, gy=origin[1] + ly, gz=origin[2] + lz,
                    px=sx, py=sy, jit=hash01(seed + 1, sx, sy), seed=seed)
                yield sx, sy, c


def render_skin(name, parts, layers):
    img = Image.new("RGBA", (SKIN, SKIN_H), (0, 0, 0, 0))
    px = img.load()
    owner = {}
    seed = BASE_SEED ^ zlib.crc32(name.encode())
    painter = PAINTERS[name]
    done_uv = set()
    for role, (size, origin, uv) in zip(ROLES[name], parts):
        if uv in done_uv:
            continue  # shares pixels with an earlier part
        done_uv.add(uv)
        for sx, sy, c in face_pixels(name, role, size, origin, uv, seed, 0):
            assert 0 <= sx < SKIN and 0 <= sy < LAYER_V, (name, role, c.face)
            prev = owner.get((sx, sy))
            assert prev is None, f"{name}: {role}.{c.face} overlaps {prev} at {(sx, sy)}"
            owner[(sx, sy)] = f"{role}.{c.face}"
            rgb = painter(c)
            px[sx, sy] = (*rgb[:3], 255)
    overlay = OVERLAYS[name]
    done_uv = set()
    for idx, _inflate in layers:
        size, origin, uv = parts[idx]
        if uv in done_uv:
            continue
        done_uv.add(uv)
        role = ROLES[name][idx]
        for sx, sy, c in face_pixels(name, role, size, origin, uv, seed + 500, LAYER_V):
            assert LAYER_V <= sy < SKIN_H, (name, role, c.face)
            prev = owner.get((sx, sy))
            assert prev is None, f"{name}: layer {role}.{c.face} overlaps {prev} at {(sx, sy)}"
            owner[(sx, sy)] = f"layer {role}.{c.face}"
            rgb = overlay(c)
            if rgb is not None:
                px[sx, sy] = (*rgb[:3], 255)
    return img


def checker(size, cell=8):
    bg = Image.new("RGBA", size, (60, 60, 66, 255))
    d = ImageDraw.Draw(bg)
    for y in range(0, size[1], cell):
        for x in range(0, size[0], cell):
            if (x // cell + y // cell) % 2:
                d.rectangle([x, y, x + cell - 1, y + cell - 1], fill=(78, 78, 86, 255))
    return bg


def main():
    parsed, parsed_layers = parse_mobs(MOBS_RON)
    # Models painted by their own scripts (tools/gen_minecart.py).
    for other in ("minecart",):
        parsed.pop(other, None)
        parsed_layers.pop(other, None)
    assert set(parsed) == set(ROLES), f"mob list differs: {sorted(parsed)}"
    for name, parts in parsed.items():
        assert parts == HARDCODED[name], f"{name}: parts in mobs.ron changed"
        assert len(parts) == len(ROLES[name]), name
        assert parsed_layers[name] == HARDCODED_LAYERS[name], f"{name}: layers in mobs.ron changed"
        for idx, _ in parsed_layers[name]:
            assert 0 <= idx < len(parts), (name, idx)
    os.makedirs(OUT_DIR, exist_ok=True)
    order = ["pig", "cow", "sheep", "chicken", "zombie", "skeleton", "stalker", "spider",
             "player", "deer", "fox", "rabbit", "bear", "duck", "forest_guardian",
             "npc_guide", "npc_merchant", "npc_nurse", "npc_wizard", "npc_smith",
             "goblin_trader", "mutant_zombie", "npc_guard", "polar_bear", "penguin", "boar", "squirrel"]
    scale, gap = 4, 8
    cols = 5
    rows = (len(order) + cols - 1) // cols
    sheet_w = cols * SKIN * scale + (cols + 1) * gap
    sheet_h = rows * SKIN_H * scale + (rows + 1) * gap
    sheet = Image.new("RGBA", (sheet_w, sheet_h), (24, 24, 28, 255))
    for k, name in enumerate(order):
        img = render_skin(name, parsed[name], parsed_layers[name])
        path = os.path.join(OUT_DIR, f"{name}.png")
        img.save(path)
        print("wrote", os.path.relpath(path, ROOT))
        big = img.resize((SKIN * scale, SKIN_H * scale), Image.NEAREST)
        tile = checker(big.size)
        tile.alpha_composite(big)
        x = gap + (k % cols) * (SKIN * scale + gap)
        y = gap + (k // cols) * (SKIN_H * scale + gap)
        sheet.paste(tile, (x, y))
    path = os.path.join(OUT_DIR, "preview.png")
    sheet.save(path)
    print("wrote", os.path.relpath(path, ROOT))


if __name__ == "__main__":
    main()
