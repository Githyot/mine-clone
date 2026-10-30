#!/usr/bin/env python3
"""Pictures of a double chest's halves, made from the single chest's 64x64 pictures.

    python3 tools/gen_double_chest.py

Each picture of the single chest is stretched to twice its width without stretching its
pixels: the outer frame and the latch keep their size and plain boards fill the middle. The
double picture is then cut in two: chest_front_left/right, chest_side_left/right (the back),
and the lid, cut west/east (chest_top_w/e) or, turned a quarter, north/south (chest_top_n/s).
Run it again after the chest's own pictures change.
"""

import os

import numpy as np
from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BLOCK = os.path.join(ROOT, "packs", "vanilla", "textures", "block")


def load(name):
    return np.asarray(Image.open(os.path.join(BLOCK, name + ".png")).convert("RGBA"))


def save(img, name):
    Image.fromarray(img, "RGBA").save(os.path.join(BLOCK, name + ".png"), optimize=True)


def widen(img, fill, keep=None):
    """`img` (N wide) made 2N wide: its left and right quarters at the ends, boards taken from
    columns `fill` (start, end) repeated in between, and columns `keep` put back in the middle."""
    n = img.shape[1]
    q = n // 4
    out = np.zeros((img.shape[0], 2 * n, 4), dtype=img.dtype)
    out[:, :q] = img[:, :q]
    out[:, 2 * n - q:] = img[:, n - q:]
    a, b = fill
    for x in range(q, 2 * n - q):
        out[:, x] = img[:, a + (x - q) % (b - a)]
    if keep:
        k0, k1 = keep
        w = k1 - k0
        out[:, n - w // 2:n - w // 2 + w] = img[:, k0:k1]
    return out


def main():
    n = load("chest_front").shape[1]
    q = n // 4
    # The latch sits in the middle 3/8..5/8 of the front.
    latch = (n * 3 // 8, n * 5 // 8)
    front = widen(load("chest_front"), (q // 2, latch[0]), keep=latch)
    side = widen(load("chest_side"), (q, n - q))
    top = widen(load("chest_top"), (q, n - q))
    save(front[:, :n].copy(), "chest_front_left")
    save(front[:, n:].copy(), "chest_front_right")
    save(side[:, :n].copy(), "chest_side_left")
    save(side[:, n:].copy(), "chest_side_right")
    save(top[:, :n].copy(), "chest_top_w")
    save(top[:, n:].copy(), "chest_top_e")
    # Along z the lid runs down the picture: north at the top.
    tall = np.rot90(top, -1)
    save(tall[:n].copy(), "chest_top_n")
    save(tall[n:].copy(), "chest_top_s")
    print("wrote 8 double chest pictures")


if __name__ == "__main__":
    main()
