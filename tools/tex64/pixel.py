"""Item icons as real pixel art (HP, 2026-10-28: "mais pixelizados como no Terraria e no Minecraft").

Items are drawn at 64x64 by items.py, arms.py and upscale.py, which on a 64-pixel inventory slot
reads as smooth, rounded vector art. This step turns each one into chunky pixel art: the picture is
reduced to a GRID x GRID sprite (each 2x2 or 4x4 block becomes one pixel, by majority, so no
blended in-between colours appear), its colours are cut to a small palette, the silhouette gets a
crisp one-pixel outline in the material's own dark colour, and it is grown back to 64x64 with
nearest neighbour. The engine still sees 64x64 textures; they just have big square pixels.
"""

import numpy as np
from PIL import Image

from style import from_oklab, to_oklab

GRID = 32       # art pixels across an item (Terraria-like: 2x2 texels per pixel)
COLORS = 14     # palette size per item


def _reduce(img, k):
    n = img.shape[0] // k
    blk = img.reshape(n, k, n, k, 4).transpose(0, 2, 1, 3, 4).reshape(n, n, k * k, 4)
    opaque = blk[..., 3] > 127
    out = np.zeros((n, n, 4))
    keep = opaque.sum(-1) * 2 >= k * k
    lum = blk[..., :3] @ np.array([0.3, 0.59, 0.11])
    for y in range(n):
        for x in range(n):
            if not keep[y, x]:
                continue
            o = opaque[y, x]
            cols = blk[y, x][o, :3]
            # The texel closest to the block's median brightness: a real colour of the art.
            l = lum[y, x][o]
            i = np.argmin(np.abs(l - np.median(l)))
            out[y, x, :3] = cols[i]
            out[y, x, 3] = 255
    return out


def _palette(small, colors):
    a = small[..., 3] > 0
    if a.sum() <= colors:
        return small
    px = small[a, :3].astype(np.uint8)
    im = Image.fromarray(px.reshape(1, -1, 3), "RGB").quantize(colors=colors, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
    q = np.asarray(im.convert("RGB"), dtype=np.float64).reshape(-1, 3)
    out = small.copy()
    out[a, :3] = q
    return out


def _outline(small):
    """Darken the silhouette's rim pixels to the colour next to them, much darker and cooler."""
    a = small[..., 3] > 0
    p = np.pad(a, 1)
    edge = a & ~(p[:-2, 1:-1] & p[2:, 1:-1] & p[1:-1, :-2] & p[1:-1, 2:])
    inner = a & ~edge
    lab = to_oklab(small[..., :3])
    out = small.copy()
    n = a.shape[0]
    for y, x in zip(*np.nonzero(edge)):
        best = None
        for dy, dx in ((0, 1), (0, -1), (1, 0), (-1, 0), (1, 1), (-1, -1), (1, -1), (-1, 1)):
            yy, xx = y + dy, x + dx
            if 0 <= yy < n and 0 <= xx < n and inner[yy, xx]:
                best = lab[yy, xx]
                break
        ref = lab[y, x] if best is None else best
        L = ref[0] * 0.5
        if lab[y, x][0] <= L + 0.02:
            continue
        c = np.array([L, ref[1] * 0.8, ref[2] * 0.8 - 0.015])
        out[y, x, :3] = from_oklab(c)
    return out


def pixelate(img, grid=GRID, colors=COLORS):
    k = img.shape[0] // grid
    small = _reduce(img, k)
    small = _palette(small, colors)
    small = _outline(small)
    return np.repeat(np.repeat(small, k, 0), k, 1)
