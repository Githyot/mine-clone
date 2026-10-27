#!/usr/bin/env python3
"""Draw the ten block-breaking overlays (destroy_stage_0..9): original 16x16 crack art.

Cracks start near the middle and branch outwards; each stage shows more of the same
cracks, so breaking a block looks like one crack spreading. Transparent background.

Usage: python3 tools/gen_cracks.py
"""
import os
import random

from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "packs", "vanilla", "textures", "block")

import math

rng = random.Random(7)
# Thin cracks: rays from near the middle that wander a little, plus a few side branches.
# Rays grow one step at a time together, so each stage is a longer version of the last.
rays = []
for k in range(5):
    a = k * 2 * math.pi / 5 + rng.uniform(-0.4, 0.4)
    rays.append([7.5 + rng.uniform(-1, 1), 7.5 + rng.uniform(-1, 1), a, 0])
order = []
seen = set()
for step in range(14):
    for r in list(rays):
        x, y, a, born = r
        a += rng.uniform(-0.45, 0.45)
        x += math.cos(a)
        y += math.sin(a)
        r[0], r[1], r[2] = x, y, a
        px, py = int(round(x)), int(round(y))
        if 0 <= px < 16 and 0 <= py < 16 and (px, py) not in seen:
            seen.add((px, py))
            order.append((px, py))
        if step in (3, 6) and rng.random() < 0.5 and len(rays) < 10:
            rays.append([x, y, a + rng.choice([-1, 1]) * rng.uniform(0.8, 1.3), step])

for stage in range(10):
    n = int(len(order) * (stage + 1) / 10)
    img = Image.new("RGBA", (16, 16), (0, 0, 0, 0))
    for k, (x, y) in enumerate(order[:n]):
        # Older parts of the crack are darker and wider-looking.
        shade = 25 if k < n * 0.6 else 50
        img.putpixel((x, y), (shade, shade, shade, 255))
    img.save(os.path.join(OUT, f"destroy_stage_{stage}.png"))
print("wrote 10 crack stages to", OUT)
