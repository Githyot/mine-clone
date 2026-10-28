#!/usr/bin/env python3
"""Original 16x16 designs for the barrel (storage that opens like the chest).

Drawn from scratch in code (no art copied from Minecraft or any pack), in the palettes of
tools/gen_lote1.py.

Usage:
    python3 tools/gen_storage.py [--force]
    python3 tools/tex64/build.py --only barrel_side,barrel_top,barrel_bottom
"""

import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gen_lote1 import DARK_IRON, IRON, PACK, WOOD, to_png  # noqa: E402
from gen_textures import N, from_rgb, normalize, rng_for, shade, value_noise  # noqa: E402


def staves(name, vertical=True):
    """Boards with long grain and a dark seam between them."""
    rng = rng_for(name)
    grain = 0.7 * value_noise(rng, 1, 8) + 0.3 * value_noise(rng, 2, 4)
    if not vertical:
        grain = grain.T
    img = from_rgb(shade(0.25 + 0.55 * normalize(grain), WOOD[1:5], 0.45))
    rgb = img[..., :3]
    for c in (0, 4, 8, 12):
        if vertical:
            rgb[:, c] = WOOD[0]
            rgb[:, c + 1] = np.minimum(rgb[:, c + 1] * 1.08, 255)
        else:
            rgb[c, :] = WOOD[0]
            rgb[c + 1, :] = np.minimum(rgb[c + 1, :] * 1.08, 255)
    return img


def barrel_side():
    img = staves("barrel_side")
    rgb = img[..., :3]
    # Two iron hoops, lit along their top edge, with a rivet on each stave.
    for y in (2, 12):
        rgb[y, :] = IRON[3]
        rgb[y + 1, :] = IRON[1]
        rgb[y + 2, :] = DARK_IRON[2]
        for x in (2, 6, 10, 14):
            rgb[y + 1, x] = IRON[5]
    return img


def barrel_top():
    img = staves("barrel_top", vertical=False)
    rgb = img[..., :3]
    # The lid: a dark rim, a cross batten and an iron pull ring.
    rgb[0, :] = rgb[15, :] = WOOD[0]
    rgb[:, 0] = rgb[:, 15] = WOOD[0]
    rgb[1, 1:15] = WOOD[4]
    rgb[1:15, 1] = WOOD[3]
    rgb[1:15, 7:9] = WOOD[2]
    rgb[1:15, 7] = WOOD[3]
    rgb[1:15, 9] = WOOD[0]
    for (y, x) in [(6, 7), (6, 8), (7, 6), (8, 6), (7, 9), (8, 9), (9, 7), (9, 8)]:
        rgb[y, x] = IRON[4] if y < 8 else DARK_IRON[3]
    return img


def barrel_bottom():
    img = staves("barrel_bottom", vertical=False)
    rgb = img[..., :3]
    rgb[0, :] = rgb[15, :] = WOOD[0]
    rgb[:, 0] = rgb[:, 15] = WOOD[0]
    rgb[1:15, 1:15] = np.clip(rgb[1:15, 1:15] * 0.85, 0, 255)
    return img


BLOCKS = {"barrel_side": barrel_side, "barrel_top": barrel_top, "barrel_bottom": barrel_bottom}


def main():
    force = "--force" in sys.argv
    for name, fn in BLOCKS.items():
        path = os.path.join(PACK, "block", name + ".png")
        if os.path.exists(path) and not force:
            sys.exit(f"refusing to overwrite {path} (use --force)")
        img = fn()
        assert img.shape == (N, N, 4), name
        to_png(img, path)
    print("python3 tools/tex64/build.py --only " + ",".join(BLOCKS))


if __name__ == "__main__":
    main()
