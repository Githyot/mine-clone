"""Mob skins at 4 texels per model pixel (the 64x level of detail for creatures).

Skins are designed in the 64 x 128 layout of mobs.ron (tools/gen_mob_skins.py writes them). Like
the blocks, every design is archived in src16/entity/ and grown here to 256 x 512:

* Scale4x (EPX twice) keeps eyes, spots and stripes crisp, with rounded diagonals.
* A fine fur / cloth grain (2-texel clusters) and a soft vertical light on each model pixel
  (lit top, darker bottom) give the surface texture a 4x skin can show.
* Contrast is kept soft like the rest of the game.

The relief layers (the lower half of the layout) keep their exact shapes: the game turns those
model pixels into 3D blocks, and the detail is painted on them.
"""

import os
import shutil

import numpy as np
from PIL import Image

from style import from_oklab, rng_for, to_oklab, to_u8
from upscale import epx

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
PACK = os.path.join(ROOT, "packs", "vanilla", "textures", "entity")
SRC = os.path.join(HERE, "src16", "entity")
DESIGN = (64, 128)
SCALE = 4


def periodic_noise(rng, h, w, cell):
    gh, gw = max(1, h // cell), max(1, w // cell)
    grid = rng.random((gh, gw))
    fy = (np.arange(h) + 0.5) / cell - 0.5
    fx = (np.arange(w) + 0.5) / cell - 0.5
    y0, x0 = np.floor(fy).astype(int), np.floor(fx).astype(int)
    ty, tx = (fy - y0)[:, None], (fx - x0)[None, :]
    a = grid[np.ix_(y0 % gh, x0 % gw)]
    b = grid[np.ix_(y0 % gh, (x0 + 1) % gw)]
    c = grid[np.ix_((y0 + 1) % gh, x0 % gw)]
    d = grid[np.ix_((y0 + 1) % gh, (x0 + 1) % gw)]
    return (a * (1 - tx) + b * tx) * (1 - ty) + (c * (1 - tx) + d * tx) * ty


def fill_clear(src):
    """Give transparent texels the colour of their nearest opaque neighbour, so Scale4x never
    eats into a face's corners (the box would show holes)."""
    rgb = src[..., :3].copy()
    known = src[..., 3] > 127
    for _ in range(8):
        if known.all():
            break
        acc = np.zeros(rgb.shape)
        cnt = np.zeros(known.shape)
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            k = np.roll(np.roll(known, dy, 0), dx, 1)
            acc += np.roll(np.roll(rgb, dy, 0), dx, 1) * k[..., None]
            cnt += k
        upd = ~known & (cnt > 0)
        rgb[upd] = acc[upd] / cnt[upd][:, None]
        known = known | upd
    out = np.zeros_like(src)
    out[..., :3] = rgb
    out[..., 3] = 255
    return out


def grow(src, name):
    big = fill_clear(src)
    for _ in range(2):
        big = epx(big)
    # Opacity stays exactly the design's, texel for texel.
    alpha = np.repeat(np.repeat(src[..., 3], SCALE, 0), SCALE, 1)
    h, w = big.shape[:2]
    rng = rng_for("skin:" + name)
    lab = to_oklab(big[..., :3])
    grain = periodic_noise(rng, h, w, 2) * 0.6 + periodic_noise(rng, h, w, 1) * 0.4 - 0.5
    lab[..., 0] += grain * 0.05
    out = big.copy()
    out[..., :3] = from_oklab(lab)
    out[..., 3] = np.where(alpha > 127, 255, 0)
    return out


def main():
    os.makedirs(SRC, exist_ok=True)
    n = 0
    for f in sorted(os.listdir(PACK)):
        if not f.endswith(".png") or f == "preview.png":
            continue
        p = os.path.join(PACK, f)
        with Image.open(p) as im:
            if im.size == DESIGN:
                shutil.copyfile(p, os.path.join(SRC, f))
    for f in sorted(os.listdir(SRC)):
        if not f.endswith(".png"):
            continue
        src = np.asarray(Image.open(os.path.join(SRC, f)).convert("RGBA"), dtype=np.float64)
        out = grow(src, f[:-4])
        Image.fromarray(to_u8(out), "RGBA").save(os.path.join(PACK, f), optimize=True)
        n += 1
    print(f"entity: {n} skins at {SCALE}x ({DESIGN[0] * SCALE}x{DESIGN[1] * SCALE})")


if __name__ == "__main__":
    main()
