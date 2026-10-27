#!/usr/bin/env python3
"""Lote 7: original 16x16 designs for the command block.

Drawn in code like tools/gen_lote1.py (no art copied from Minecraft or any pack): a riveted
bronze casing round a dark glass screen. The sides show a typed prompt ("> _" in green), the
top and bottom a plain grille.

Usage:
    python3 tools/gen_lote7.py [--force]
    python3 tools/tex64/build.py --only command_block_side,command_block_top
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gen_items import put  # noqa: E402
from gen_lote1 import PACK, ramp_at, to_png  # noqa: E402
from gen_textures import N, canvas, fbm, pal, rng_for  # noqa: E402

CASING = pal("#4a2c16", "#64401f", "#7e552a", "#996c38", "#b38549", "#c99c5e")
SCREEN = pal("#0c1012", "#121a1c", "#182326", "#1f2d30")
GLOW = pal("#2e7a3a", "#3f9a4a", "#58b85e", "#7ad27a")
RIVET = pal("#3a2410", "#d8b47a")


def casing(name):
    img = canvas()
    f = fbm(rng_for(name))
    for y in range(N):
        for x in range(N):
            edge = min(x, y, N - 1 - x, N - 1 - y)
            t = 0.35 + 0.35 * f[y, x]
            if edge == 0:
                t -= 0.25
            elif edge == 1:
                t += 0.2
            put(img, x, y, ramp_at(CASING, min(max(t, 0.0), 0.999)))
    for (x, y) in ((1, 1), (14, 1), (1, 14), (14, 14)):
        put(img, x, y, RIVET[1])
        put(img, x + (1 if x < 8 else -1), y + (1 if y < 8 else -1), RIVET[0])
    return img


def screen(img, x0, y0, x1, y1, rng):
    for y in range(y0, y1):
        for x in range(x0, x1):
            t = 0.2 + 0.5 * rng.random() * 0.4 + (0.3 if y == y0 else 0.0)
            put(img, x, y, ramp_at(SCREEN, min(t, 0.999)))
    for x in range(x0 - 1, x1 + 1):
        put(img, x, y0 - 1, CASING[0])
        put(img, x, y1, CASING[4])
    for y in range(y0 - 1, y1 + 1):
        put(img, x0 - 1, y, CASING[0])
        put(img, x1, y, CASING[4])


def command_block_side():
    img = casing("command_block_side")
    screen(img, 3, 4, 13, 12, rng_for("command_block_screen"))
    # ">" prompt and a cursor.
    for (x, y) in ((5, 6), (6, 7), (7, 8), (6, 9), (5, 10)):
        put(img, x, y - 1, GLOW[2])
    for x in range(9, 12):
        put(img, x, 9, GLOW[1] if x != 10 else GLOW[2])
    return img


def command_block_top():
    img = casing("command_block_top")
    screen(img, 3, 3, 13, 13, rng_for("command_block_grille"))
    for y in range(4, 12, 2):
        for x in range(4, 12):
            put(img, x, y, CASING[2] if (x + y) % 4 else CASING[3])
    return img


BLOCKS = {"command_block_side": command_block_side, "command_block_top": command_block_top}


def main():
    force = "--force" in sys.argv
    clash = [n for n in BLOCKS if os.path.exists(os.path.join(PACK, "block", n + ".png"))]
    if clash and not force:
        sys.exit("refusing to overwrite existing textures (use --force to re-run lote 7): " + ", ".join(clash))
    for name, fn in BLOCKS.items():
        img = fn()
        assert img.shape == (N, N, 4), name
        to_png(img, os.path.join(PACK, "block", name + ".png"))
    print(f"wrote {len(BLOCKS)} 16x16 designs; now run:")
    print("python3 tools/tex64/build.py --only " + ",".join(sorted(BLOCKS)))


if __name__ == "__main__":
    main()
