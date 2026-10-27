"""YUNG's Cave Biomes art: blue ice and an icicle. Our own 16x16 pixels."""

import numpy as np

from .lib import BLOCKS, N, T, canvas, hexrgb, pal, put, rng_for


def blue_ice():
    rng = rng_for("blue_ice")
    p = pal("#2a5cc8", "#3a6cd6", "#4a7ee0", "#6292ea", "#86b0f4")
    img = canvas()
    for y in range(N):
        for x in range(N):
            v = 0.5 + 0.35 * np.sin((x + 2 * y) / 3.0) + 0.25 * rng.random()
            img[y, x, :3] = p[int(np.clip(v * 3.2, 0, 4))]
            img[y, x, 3] = 255
    # pale fracture lines
    for (x0, y0, ln) in ((1, 13, 7), (7, 10, 6), (10, 5, 5)):
        for i in range(ln):
            put(img, x0 + i, y0 - i, p[4])
    return img


def icicle():
    img = canvas()
    edge, mid, hi = hexrgb("#6e94d0"), hexrgb("#a6c6f2"), hexrgb("#e8f4ff")
    # three hanging spikes of different lengths, wide at the top
    for cx, ln in ((4, 11), (8, 15), (12, 8)):
        for y in range(ln):
            w = max(0, 2 - (y * 3) // ln)
            for x in range(cx - w, cx + w + 1):
                put(img, x, y, mid)
            put(img, cx - w, y, edge)
            put(img, cx + w, y, edge)
            if w > 0:
                put(img, cx, y, hi)
    return img


BLOCKS["blue_ice"] = blue_ice
BLOCKS["icicle"] = icicle
