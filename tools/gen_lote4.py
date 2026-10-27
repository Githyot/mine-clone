#!/usr/bin/env python3
"""Lote 4: original 16x16 designs for the blocks of the new biome variants (pale garden,
sunflower plains, warm oceans).

Drawn in code like tools/gen_lote1.py (no art copied from Minecraft or any pack). The pale oak
and pale moss are this repo's own dark oak and moss designs re-coloured to the pale garden's
greys; the sunflower, sea grass and corals are drawn here.

Usage:
    python3 tools/gen_lote4.py [--force]
    python3 tools/tex64/build.py --only <names printed at the end>
"""

import math
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gen_items import outline, put  # noqa: E402
from gen_lote1 import PACK, load, lum, ramp_at, recolor, to_png  # noqa: E402
from gen_textures import N, canvas, fbm, pal, rng_for  # noqa: E402

PALE_BARK = pal("#3e3b38", "#4f4b47", "#605b56", "#736d67", "#867f78", "#99928a")
PALE_WOOD = pal("#8f857c", "#a2988e", "#b4aaa0", "#c5bcb2", "#d5cdc4", "#e2dbd3")
PALE_RINGS = pal("#4f4b47", "#8f857c", "#a89e94", "#bfb5ab", "#d3cbc2", "#e2dbd3")
PALE_LEAF = pal("#3e4838", "#525e4a", "#67735d", "#7d8970", "#939e85", "#a9b29a")
PALE_MOSS = pal("#5e645a", "#71786b", "#858b7d", "#989e90", "#abb0a3", "#bcc1b5")
STEM = pal("#2c4a1a", "#3a5e22", "#4a742c", "#5c8a36")
PETAL = pal("#b07812", "#cc9418", "#e2ae24", "#f0c63c", "#f8dc66")
SEED = pal("#2a1a0c", "#3c2612", "#52361a", "#684624")
SEAGRASS = pal("#1e4a1c", "#2a6024", "#367a2c", "#449236", "#56a842")
CORALS = {
    "tube": pal("#1a2c8a", "#2440a8", "#3256c4", "#4a6ed8", "#6a8ae6"),
    "brain": pal("#8a2a5c", "#a83a72", "#c44e8a", "#d86ca2", "#e68cba"),
    "bubble": pal("#5a1a7a", "#742494", "#8e32ae", "#a64cc4", "#bc6cd6"),
    "fire": pal("#7a1414", "#981c1c", "#b42626", "#cc3a30", "#de5a44"),
    "horn": pal("#8a7212", "#a88a18", "#c4a424", "#d8bc3a", "#e6d05c"),
}


def masked(img):
    return img[..., 3] > 0


def pale_oak_log():
    img = load("block", "dark_oak_log")
    return recolor(img, masked(img), PALE_BARK)


def pale_oak_log_top():
    img = load("block", "dark_oak_log_top")
    return recolor(img, masked(img), PALE_RINGS)


def pale_oak_planks():
    img = load("block", "dark_oak_planks")
    return recolor(img, masked(img), PALE_WOOD)


def pale_oak_leaves():
    img = load("block", "dark_oak_leaves")
    return recolor(img, masked(img), PALE_LEAF)


def pale_oak_sapling():
    img = load("block", "dark_oak_sapling")
    m = masked(img)
    rgb = img[..., :3]
    green = m & (rgb[..., 1] > rgb[..., 0] + 4)
    recolor(img, green, PALE_LEAF)
    if (m & ~green).any():
        recolor(img, m & ~green, PALE_BARK)
    return img


def pale_moss_block():
    img = load("block", "moss_block")
    return recolor(img, masked(img), PALE_MOSS)


def pale_hanging_moss():
    """Strands of grey moss hanging from the top edge, ragged at the ends."""
    img = canvas()
    rng = rng_for("pale_hanging_moss")
    for x in range(1, 15):
        if rng.random() < 0.25:
            continue
        length = int(6 + rng.random() * 9)
        sway = rng.random() * 6.28
        for y in range(length):
            xx = int(round(x + 0.6 * math.sin(sway + y * 0.5)))
            if 0 <= xx < N:
                t = 1.0 - y / max(length, 1) * 0.7 + rng.random() * 0.15
                put(img, xx, y, ramp_at(PALE_MOSS, min(t, 0.999)))
        if length > 8 and rng.random() < 0.6:
            put(img, x, length, PALE_MOSS[1])
    return img


def sunflower():
    """A tall stem with two leaves and a big face of petals round a brown centre."""
    img = canvas()
    for y in range(7, 16):
        put(img, 7, y, STEM[2])
        put(img, 8, y, STEM[1])
    for (y, side) in ((11, -1), (13, 1)):
        for i in range(1, 5):
            x = (7 if side < 0 else 8) + side * i
            yy = y - (i // 2)
            put(img, x, yy, STEM[3] if i < 3 else STEM[2])
            put(img, x, yy + 1, STEM[1])
    cx, cy = 7.5, 4.5
    for y in range(0, 10):
        for x in range(2, 14):
            d = math.hypot(x - cx, y - cy)
            if d < 2.3:
                t = 0.2 + 0.6 * ((x * 3 + y * 5) % 4) / 4
                put(img, x, y, ramp_at(SEED, t))
            elif d < 4.6:
                a = math.atan2(y - cy, x - cx)
                ridge = 0.5 + 0.5 * math.cos(a * 9)
                t = 0.25 + 0.6 * ridge * (1.0 - (d - 2.3) / 2.6) + 0.15
                put(img, x, y, ramp_at(PETAL, min(max(t, 0.0), 0.999)))
    return outline(img)


def seagrass():
    """Long thin blades swaying from the bottom."""
    img = canvas()
    rng = rng_for("seagrass")
    for k, x0 in enumerate((2, 5, 7, 10, 13)):
        h = 9 + int(rng.random() * 7)
        ph = rng.random() * 6.28
        for i in range(h):
            y = 15 - i
            x = int(round(x0 + 1.2 * math.sin(ph + i * 0.35) * i / h))
            t = 0.25 + 0.7 * i / h
            put(img, x % N, y, ramp_at(SEAGRASS, min(t, 0.999)))
    return img


def coral(kind):
    """A branching coral fan, its own shape per kind."""
    p = CORALS[kind]
    img = canvas()
    rng = rng_for(kind + "_coral")
    seed = {"tube": 3, "brain": 4, "bubble": 5, "fire": 6, "horn": 7}[kind]

    def branch(x, y, a, length, depth):
        for i in range(length):
            x += math.cos(a)
            y -= math.sin(a)
            xi, yi = int(round(x)), int(round(y))
            if 0 <= xi < N and 0 <= yi < N:
                put(img, xi, yi, ramp_at(p, min(0.3 + 0.12 * depth + 0.05 * i, 0.999)))
                if kind in ("tube", "bubble") and i == length - 1:
                    put(img, xi, max(yi - 1, 0), p[-1])
        if depth < 3:
            spread = 0.5 + 0.1 * seed
            for s in (-1, 1):
                if rng.random() < 0.85:
                    branch(x, y, a + s * spread * (0.6 + rng.random() * 0.5), max(2, length - 1 - int(rng.random() * 2)), depth + 1)

    branch(7.5, 15.5, math.pi / 2, 5, 0)
    if kind == "brain":
        # The brain coral is a rounded lump, ridged.
        img = canvas()
        for y in range(6, 16):
            for x in range(2, 14):
                d = math.hypot((x - 7.5) / 6.0, (y - 15.0) / 9.0)
                if d < 1.0:
                    ridge = 0.5 + 0.5 * math.sin(x * 1.3 + math.sin(y * 0.9) * 2.0)
                    put(img, x, y, ramp_at(p, min(0.2 + 0.7 * ridge * (1 - d * 0.5), 0.999)))
    return outline(img, k=0.3)


def coral_block(kind):
    """A block of living coral: its colour in soft noise, dotted with dark pores."""
    p = CORALS[kind]
    img = canvas()
    rng = rng_for(kind + "_coral_block")
    f = fbm(rng)
    f = (f - f.min()) / (f.max() - f.min() + 1e-9)
    img[..., :3] = ramp_at(p, 0.25 + 0.6 * f)
    img[..., 3] = 255
    for _ in range(14):
        x, y = int(rng.random() * N), int(rng.random() * N)
        put(img, x, y, p[0])
        put(img, (x + 1) % N, y, p[1])
    return img


BLOCKS = {
    "pale_oak_log": pale_oak_log, "pale_oak_log_top": pale_oak_log_top, "pale_oak_planks": pale_oak_planks,
    "pale_oak_leaves": pale_oak_leaves, "pale_oak_sapling": pale_oak_sapling, "pale_moss_block": pale_moss_block,
    "pale_hanging_moss": pale_hanging_moss, "sunflower": sunflower, "seagrass": seagrass,
}
BLOCKS.update({f"{k}_coral": (lambda k=k: coral(k)) for k in CORALS})
BLOCKS.update({f"{k}_coral_block": (lambda k=k: coral_block(k)) for k in CORALS})


def main():
    force = "--force" in sys.argv
    clash = [n for n in BLOCKS if os.path.exists(os.path.join(PACK, "block", n + ".png"))]
    if clash and not force:
        sys.exit("refusing to overwrite existing textures (use --force to re-run lote 4): " + ", ".join(clash))
    for name, fn in BLOCKS.items():
        img = fn()
        assert img.shape == (N, N, 4), name
        to_png(img, os.path.join(PACK, "block", name + ".png"))
    print(f"wrote {len(BLOCKS)} 16x16 designs; now run:")
    print("python3 tools/tex64/build.py --only " + ",".join(sorted(BLOCKS)))


if __name__ == "__main__":
    main()
