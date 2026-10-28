"""Coins (Terraria's idea, our own art): copper, silver, gold and platinum, drawn at 64x64.

A small pile of three coins: one lying flat behind, one leaning, one standing face-on in front
with its mark struck into it. Light from the top-left like everything else (tools/tex64/style.py),
2-pixel outline in the metal's own darkest, cooler colour, never black.

    python3 tools/gen_coins.py      # writes packs/vanilla/textures/item/<metal>_coin.png
"""

import os

import numpy as np
from PIL import Image

N = 64
OUT = os.path.join(os.path.dirname(__file__), "..", "packs", "vanilla", "textures", "item")

# Ramps from shadow (cooler) to light (warmer), 9 steps each.
RAMPS = {
    "copper": ["3b1a14", "5c2a1a", "7d3b20", "9c4f28", "b86632", "cf7f40", "e09a58", "eeb879", "f8d6a4"],
    "silver": ["2c2f3d", "474b5c", "636879", "7f8495", "9ba0ae", "b5b9c5", "cdd0d9", "e2e4ea", "f5f6f8"],
    "gold": ["4a2a10", "6e4512", "936214", "b8821a", "d5a124", "e8bb3a", "f4d35e", "fbe68e", "fff6c8"],
    "platinum": ["263642", "3d5463", "577282", "7391a0", "8fadb9", "aac6cf", "c4dbe1", "dcecef", "f2fafb"],
}

YY, XX = np.mgrid[0:N, 0:N].astype(np.float64) + 0.5


def hexrgb(h):
    return np.array([int(h[i:i + 2], 16) for i in (0, 2, 4)], dtype=np.float64)


def ellipse(cx, cy, rx, ry):
    return ((XX - cx) / rx) ** 2 + ((YY - cy) / ry) ** 2


def mark_mask(metal, cx, cy, r):
    """The stamp in the middle of the coin face: a different sign for each metal."""
    x, y = (XX - cx) / r, (YY - cy) / r
    if metal == "copper":
        # A square hole, like old cash coins.
        return (np.abs(x) < 0.22) & (np.abs(y) < 0.22)
    if metal == "silver":
        # A five-point star.
        a = np.arctan2(y, x) + np.pi / 2
        rr = np.hypot(x, y)
        k = 0.5 + 0.5 * np.cos(5 * a)
        return rr < 0.20 + 0.22 * k
    if metal == "gold":
        # A crown: three points over a band.
        band = (np.abs(y - 0.18) < 0.12) & (np.abs(x) < 0.42)
        frac = ((x + 0.42) / 0.28) % 1.0
        top = 0.06 - 0.44 * (1.0 - np.abs(2.0 * frac - 1.0))
        pts = (y < 0.1) & (y > top) & (np.abs(x) < 0.42)
        return band | pts
    # Platinum: a diamond.
    return np.abs(x) + np.abs(y) * 0.8 < 0.36


def coin(metal):
    ramp = np.array([hexrgb(c) for c in RAMPS[metal]])
    img = np.zeros((N, N, 4))
    shade = np.full((N, N), -1.0)

    def lay(mask, value):
        shade[mask] = value[mask] if isinstance(value, np.ndarray) else value

    # Back coin lying flat (just its rim shows), then one leaning, then the front coin.
    back = ellipse(42, 47, 15, 6.5) < 1
    lay(back, 2.6 + 2.0 * ((YY < 46).astype(float)) + (XX < 40) * 0.8)
    lean_d = ellipse(43, 34, 11, 15)
    lean = lean_d < 1
    lay(lean, np.clip(3.0 + (1 - lean_d) * 1.5 - (XX - 43) / 12.0, 1.5, 6.0))
    lay(lean & (lean_d > 0.68), 1.6)

    cx, cy, r = 25.0, 33.0, 17.0
    d = ellipse(cx, cy, r, r)
    face = d < 1
    # Domed face lit from the top-left.
    nx, ny = (XX - cx) / r, (YY - cy) / r
    lit = 4.4 - 2.4 * (nx * 0.62 + ny * 0.78)
    lay(face, lit)
    rim = face & (d > 0.70)
    # The rim: bright where it faces the light, dark on the far side.
    lay(rim, np.clip(4.2 - 3.4 * (nx * 0.62 + ny * 0.78), 1.0, 8.0))
    inner_edge = face & (d > 0.62) & (d <= 0.70)
    lay(inner_edge, np.clip(2.6 + 2.6 * (nx * 0.62 + ny * 0.78), 1.0, 6.0))
    # The stamp: struck in, so its top-left edge is in shadow and bottom-right catches light.
    m = mark_mask(metal, cx, cy, r) & face
    mo = np.roll(np.roll(m, 1, 0), 1, 1)
    lay(m, 2.2)
    lay(m & ~mo, 1.4)
    lay(mo & ~m & face & ~rim, 6.6)
    # A highlight glint on the face.
    glint = ellipse(cx - 7, cy - 8, 3.2, 2.2) < 1
    lay(glint & face & ~m, 8.0)

    q = np.clip(np.round(shade), 0, len(ramp) - 1).astype(int)
    filled = shade >= 0
    img[..., :3] = ramp[q]
    img[..., 3] = filled * 255.0
    # 2-pixel outline in the darkest colour.
    grown = filled.copy()
    for _ in range(2):
        g = grown.copy()
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            g |= np.roll(np.roll(grown, dy, 0), dx, 1)
        grown = g
    edge = grown & ~filled
    img[edge, :3] = ramp[0] * 0.8
    img[edge, 3] = 255.0
    # The front coin overlaps the others: a dark seam keeps them apart.
    seam = (~face) & filled & (np.abs(np.sqrt(d) - 1.0) < 0.07)
    img[seam, :3] = ramp[0]
    return Image.fromarray(img.astype(np.uint8), "RGBA")


def main():
    os.makedirs(OUT, exist_ok=True)
    for metal in RAMPS:
        coin(metal).save(os.path.join(OUT, f"{metal}_coin.png"))


if __name__ == "__main__":
    main()
