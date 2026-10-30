#!/usr/bin/env python3
"""The long cloth of tall banners (Shape::TallBanner): 64x128, twice as tall as wide like the
cloth in the game (20 by 40 pixels), saved as two 64x64 halves: NAME_cloth_top.png and
NAME_cloth_bottom.png next to the banner's own 64x64 picture.

    python3 tools/tex64/banners.py [--preview DIR]

Each cloth is drawn from the banner's picture: its cloth colour, and its emblem (the diamond, or
the Hunting Banner's monster head) lifted out whole and set in the upper part. Around it: a woven
cloth in chunky 2x2 texels with soft folds, darker borders down the sides, a darker hem where it
wraps the bar, a band of the emblem's colour low down and a fringe of tassels at the bottom.
Deterministic: the same picture always gives the same cloth.
"""

import argparse
import os
import zlib

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
BLOCKS = os.path.join(ROOT, "packs", "vanilla", "textures", "block")

COLOURS = ["white", "orange", "magenta", "light_blue", "yellow", "lime", "pink", "gray", "light_gray",
           "cyan", "purple", "blue", "brown", "green", "red", "black"]
NAMES = [f"{c}_banner" for c in COLOURS] + ["hunt_banner"]

W, H = 64, 128


def cloth_colour(src):
    """The plain cloth: the most common colour of the picture's middle (median of a patch)."""
    patch = src[8:14, 18:46, :3].reshape(-1, 3)
    return np.median(patch, axis=0)


def emblem(src, base):
    """The emblem's pixels (RGBA, 64x64, clear elsewhere): everything that stands out from the
    cloth and touches the picture's centre, between the bar at the top and the pole below."""
    d = np.sqrt(((src[..., :3] - base) ** 2).sum(-1))
    thr = max(18.0, 0.18 * float(base.mean()))
    mask = (d > thr) & (src[..., 3] > 127)
    mask[:12] = False
    mask[51:] = False
    mask[:, :10] = False
    mask[:, 55:] = False
    # Flood from the middle so borders and bands that don't touch the emblem stay out.
    seen = np.zeros_like(mask)
    starts = [(y, x) for y in range(24, 36) for x in range(28, 36) if mask[y, x]]
    stack = starts[:]
    for s in starts:
        seen[s] = True
    while stack:
        y, x = stack.pop()
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)):
            ny, nx = y + dy, x + dx
            if 0 <= ny < 64 and 0 <= nx < 64 and mask[ny, nx] and not seen[ny, nx]:
                seen[ny, nx] = True
                stack.append((ny, nx))
    out = np.zeros_like(src)
    out[seen] = src[seen]
    return out


def noise(name, shape):
    rng = np.random.default_rng(zlib.crc32(name.encode()))
    return rng.random(shape)


def make(name):
    src = np.asarray(Image.open(os.path.join(BLOCKS, f"{name}.png")).convert("RGBA"), dtype=np.float64)
    base = cloth_colour(src)
    em = emblem(src, base)
    img = np.zeros((H, W, 4))
    # Woven cloth in 2x2 texels: a little light and dark per cell, soft vertical folds.
    n = noise(name, (H // 2, W // 2)).repeat(2, 0).repeat(2, 1)
    x = np.arange(W)[None, :]
    y = np.arange(H)[:, None]
    fold = 1.0 + 0.05 * np.sin(x / W * np.pi * 3.0 + 0.6) * np.ones((H, 1))
    shade = (0.95 + 0.08 * n) * fold
    img[..., :3] = base * shade[..., None]
    img[..., 3] = 255
    dark = base * 0.7
    # Borders down the sides, the hem round the bar at the top.
    for x0 in (4, 56):
        img[:, x0:x0 + 4, :3] = dark * (0.95 + 0.08 * n[:, x0:x0 + 4, None])
    img[0:6, :, :3] = base * 0.78 * (0.95 + 0.08 * n[0:6, :, None])
    img[6:8, :, :3] = np.minimum(base * 1.12, 255)
    # The emblem, a little lower than in the square picture so it sits in the upper third.
    ys, xs = np.nonzero(em[..., 3] > 0)
    accent = base * 1.25
    if len(ys):
        top = ys.min()
        shift = 22 - top
        for yy, xx in zip(ys, xs):
            ty = yy + shift
            if 0 <= ty < H:
                img[ty, xx] = em[yy, xx]
        accent = em[ys, xs, :3].mean(0)
    # A band low down in the emblem's colour, edged dark.
    img[90:92, 8:56, :3] = dark
    img[92:98, 8:56, :3] = accent * (0.95 + 0.08 * n[92:98, 8:56, None])
    img[98:100, 8:56, :3] = dark
    # The hem and a fringe of tassels: every other 4-texel column hangs on, the rest is clear.
    img[110:114, :, :3] = dark * (0.95 + 0.08 * n[110:114, :, None])
    for x0 in range(0, W, 4):
        tassel = (x0 // 4) % 2 == 0
        if tassel:
            img[114:H, x0:x0 + 4, :3] = base * 0.85 * (0.95 + 0.08 * n[114:H, x0:x0 + 4, None])
            img[H - 2:H, x0:x0 + 4, :3] = dark
        else:
            img[118:H, x0:x0 + 4, 3] = 0
    img[..., :3] = np.clip(img[..., :3], 0, 255)
    return img.astype(np.uint8)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--preview")
    a = ap.parse_args()
    sheet = []
    for name in NAMES:
        img = make(name)
        Image.fromarray(img[:64], "RGBA").save(os.path.join(BLOCKS, f"{name}_cloth_top.png"), optimize=True)
        Image.fromarray(img[64:], "RGBA").save(os.path.join(BLOCKS, f"{name}_cloth_bottom.png"), optimize=True)
        sheet.append(img)
    if a.preview:
        os.makedirs(a.preview, exist_ok=True)
        s = np.concatenate([np.pad(i, ((4, 4), (4, 4), (0, 0))) for i in sheet], axis=1)
        Image.fromarray(s, "RGBA").resize((s.shape[1] * 2, s.shape[0] * 2), Image.NEAREST).save(os.path.join(a.preview, "banners.png"))
    print(f"{len(NAMES)} banner cloths")


if __name__ == "__main__":
    main()
