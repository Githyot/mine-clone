#!/usr/bin/env python3
"""Lote 9: original 16x16 designs for the rest of the pale garden (pale oak door, trapdoor and
boat, the eyeblossom, the creaking heart) and the resin family.

Drawn in code like tools/gen_lote4.py (no art copied from Minecraft or any pack). The pale oak
pieces are this repo's own dark oak designs re-coloured to the pale greys; resin is our bricks
and honeycomb re-coloured to a warm orange; the flowers, the heart and the clump are drawn here.

Usage:
    python3 tools/gen_lote9.py [--force]
    python3 tools/tex64/build.py --only <names printed at the end>
"""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gen_items import outline, put  # noqa: E402
from gen_lote1 import PACK, load, ramp_at, recolor, to_png  # noqa: E402
from gen_lote4 import PALE_BARK, PALE_LEAF, PALE_RINGS, PALE_WOOD, masked  # noqa: E402
from gen_textures import N, canvas, fbm, pal, rng_for  # noqa: E402

RESIN = pal("#6a2a08", "#8c3c0c", "#b05212", "#cc6a1c", "#e2862c", "#f2a648")
RESIN_MORTAR = pal("#3a2c22", "#4e3c2e", "#62503e")
EYE = pal("#7a2e04", "#b04a08", "#e07010", "#f89a28", "#ffc85a")
STEM = pal("#2e3a28", "#3e4c36", "#516046", "#657458")
CLOSED = pal("#4a4a50", "#5e5e66", "#74747c", "#8a8a92", "#a0a0a8")


def pale_oak_door_lower():
    img = load("block", "dark_oak_door_lower")
    return recolor(img, masked(img), PALE_WOOD)


def pale_oak_door_upper():
    img = load("block", "dark_oak_door_upper")
    return recolor(img, masked(img), PALE_WOOD)


def pale_oak_trapdoor():
    img = load("block", "dark_oak_trapdoor")
    return recolor(img, masked(img), PALE_WOOD)


def pale_oak_door_item():
    img = load("item", "dark_oak_door")
    return recolor(img, masked(img), PALE_WOOD)


def pale_oak_boat_item():
    img = load("item", "dark_oak_boat")
    return recolor(img, masked(img), PALE_WOOD)


def eyeblossom(open_):
    """A grey-green stem with two leaves; shut, a pale grey bud; open, petals round a glowing
    orange eye."""
    img = canvas()
    for y in range(8, 16):
        put(img, 7, y, STEM[2])
        put(img, 8, y, STEM[1])
    for (y, side) in ((12, -1), (14, 1)):
        for i in range(1, 4):
            x = (7 if side < 0 else 8) + side * i
            put(img, x, y - i // 2, STEM[3] if i < 2 else STEM[2])
    cx, cy = 7.5, 5.0
    if open_:
        for y in range(0, 11):
            for x in range(2, 14):
                d = math.hypot(x - cx, (y - cy) * 1.1)
                a = math.atan2(y - cy, x - cx)
                if d < 1.6:
                    put(img, x, y, ramp_at(EYE, 0.95 - d * 0.2))
                elif d < 2.6:
                    put(img, x, y, ramp_at(EYE, 0.45))
                elif d < 4.6 + 0.6 * math.cos(a * 5):
                    t = 0.35 + 0.5 * (1.0 - (d - 2.6) / 2.6)
                    put(img, x, y, ramp_at(CLOSED, min(max(t, 0.0), 0.999)))
    else:
        for y in range(1, 9):
            for x in range(4, 12):
                d = math.hypot((x - cx) / 2.4, (y - cy) / 3.6)
                if d < 1.0:
                    ridge = 0.5 + 0.5 * math.cos((x - cx) * 1.9)
                    put(img, x, y, ramp_at(CLOSED, min(0.25 + 0.6 * ridge * (1.0 - d * 0.6), 0.999)))
        put(img, 7, 8, STEM[3])
        put(img, 8, 8, STEM[3])
    return outline(img, k=0.3)


def creaking_heart():
    """A pale oak log split down the middle, an orange eye glowing in the crack."""
    img = load("block", "dark_oak_log")
    recolor(img, masked(img), PALE_BARK)
    for y in range(N):
        w = 2.2 * math.sin(math.pi * (y + 0.5) / N) + 0.4
        for x in range(N):
            d = abs(x - 7.5)
            if d < w:
                t = 1.0 - d / (w + 1e-6)
                k = 1.0 - abs(y - 7.5) / 8.0
                put(img, x, y, ramp_at(EYE, min(0.15 + 0.8 * t * k, 0.999)))
            elif d < w + 1.0:
                put(img, x, y, PALE_BARK[0])
    return img


def creaking_heart_top():
    img = load("block", "dark_oak_log_top")
    recolor(img, masked(img), PALE_RINGS)
    for y in range(6, 10):
        for x in range(6, 10):
            d = math.hypot(x - 7.5, y - 7.5)
            put(img, x, y, ramp_at(EYE, min(0.9 - d * 0.25, 0.999)))
    return img


def resin_block():
    img = load("block", "honeycomb_block")
    return recolor(img, masked(img), RESIN)


def resin_bricks():
    """Our bricks in resin orange, with dark mortar."""
    img = load("block", "bricks")
    m = masked(img)
    L = img[..., :3] @ [0.3, 0.59, 0.11]
    mortar = m & (L > L[m].mean() + 18)
    recolor(img, m & ~mortar, RESIN)
    if mortar.any():
        recolor(img, mortar, RESIN_MORTAR)
    return img


def chiseled_resin_bricks():
    """A resin tile with a carved eye in a ring."""
    img = canvas()
    rng = rng_for("chiseled_resin_bricks")
    f = fbm(rng)
    f = (f - f.min()) / (f.max() - f.min() + 1e-9)
    img[..., :3] = ramp_at(RESIN, 0.35 + 0.35 * f)
    img[..., 3] = 255
    for i in range(N):
        put(img, i, 0, RESIN[5])
        put(img, 0, i, RESIN[4])
        put(img, i, 15, RESIN[0])
        put(img, 15, i, RESIN[1])
    for y in range(2, 14):
        for x in range(2, 14):
            d = math.hypot(x - 7.5, y - 7.5)
            if 4.4 < d < 5.6:
                put(img, x, y, RESIN[0])
            elif 3.6 < d <= 4.4:
                put(img, x, y, RESIN[5])
            elif d < 2.0:
                put(img, x, y, RESIN[1] if d > 1.0 else RESIN[0])
    return img


def resin_clump():
    """A sticky orange lump with a bright drip."""
    img = canvas()
    for y in range(3, 14):
        for x in range(2, 14):
            d = math.hypot((x - 7.5) / 5.0, (y - 8.5) / 4.6) + 0.12 * math.sin(x * 1.7 + y)
            if d < 1.0:
                t = 0.25 + 0.6 * (1.0 - d) + 0.2 * (y < 8) * (x < 8)
                put(img, x, y, ramp_at(RESIN, min(t, 0.999)))
    for y in range(13, 16):
        put(img, 9, y, RESIN[3 - (y - 13)])
    put(img, 5, 6, RESIN[5])
    put(img, 6, 5, RESIN[5])
    return outline(img)


def resin_brick_item():
    img = load("item", "brick")
    return recolor(img, masked(img), RESIN)


BLOCKS = {
    "pale_oak_door_lower": pale_oak_door_lower, "pale_oak_door_upper": pale_oak_door_upper,
    "pale_oak_trapdoor": pale_oak_trapdoor, "open_eyeblossom": lambda: eyeblossom(True),
    "closed_eyeblossom": lambda: eyeblossom(False), "creaking_heart": creaking_heart,
    "creaking_heart_top": creaking_heart_top, "resin_block": resin_block, "resin_bricks": resin_bricks,
    "chiseled_resin_bricks": chiseled_resin_bricks,
}
ITEMS = {
    "pale_oak_door": pale_oak_door_item, "pale_oak_boat": pale_oak_boat_item,
    "open_eyeblossom": lambda: eyeblossom(True), "closed_eyeblossom": lambda: eyeblossom(False),
    "resin_clump": resin_clump, "resin_brick": resin_brick_item,
}


def main():
    force = "--force" in sys.argv
    clash = [f"block/{n}" for n in BLOCKS if os.path.exists(os.path.join(PACK, "block", n + ".png"))]
    clash += [f"item/{n}" for n in ITEMS if os.path.exists(os.path.join(PACK, "item", n + ".png"))]
    if clash and not force:
        sys.exit("refusing to overwrite existing textures (use --force to re-run lote 9): " + ", ".join(clash))
    for kind, table in (("block", BLOCKS), ("item", ITEMS)):
        for name, fn in table.items():
            img = fn()
            assert img.shape == (N, N, 4), name
            to_png(img, os.path.join(PACK, kind, name + ".png"))
    print(f"wrote {len(BLOCKS)} block and {len(ITEMS)} item designs; now run:")
    print("python3 tools/tex64/build.py --only " + ",".join(sorted(set(BLOCKS) | set(ITEMS))))


if __name__ == "__main__":
    main()
