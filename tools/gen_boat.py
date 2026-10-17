#!/usr/bin/env python3
"""Paint the boat's skin (packs/vanilla/textures/entity/boat.png).

Oak planks running along the hull, darker at the seams, a lighter rim. The boxes are the
"boat" model in packs/vanilla/mobs.ron; the unwrap is the one in tools/gen_minecart.py.

Usage: python3 tools/gen_boat.py
"""

import os
import zlib

from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "packs", "vanilla", "textures", "entity", "boat.png")

# (size w, h, d), uv: must match mobs.ron.
BOXES = [
    ((12, 2, 20), (0, 0)),
    ((2, 6, 22), (0, 22)),
    ((12, 6, 2), (0, 50)),
    ((12, 1, 3), (28, 50)),
]
WOOD = [(70, 46, 22), (100, 68, 34), (126, 88, 46), (150, 108, 60), (176, 132, 78)]


def faces(size, uv):
    w, h, d = size
    u, v = uv
    return {
        "top": (u + d, v, w, d),
        "bottom": (u + d + w, v, w, d),
        "right": (u, v + d, d, h),
        "front": (u + d, v + d, w, h),
        "left": (u + d + w, v + d, d, h),
        "back": (u + 2 * d + w, v + d, w, h),
    }


def noise(x, y):
    return (zlib.crc32(f"{x},{y}".encode()) & 0xFFFF) / 0xFFFF


def main():
    img = Image.new("RGBA", (64, 128), (0, 0, 0, 0))
    px = img.load()
    for size, uv in BOXES:
        for face, (x0, y0, fw, fh) in faces(size, uv).items():
            for j in range(fh):
                for i in range(fw):
                    n = noise(x0 + i // 3, y0 + j)
                    k = 2 + (1 if n > 0.7 else 0) - (1 if n < 0.25 else 0)
                    # Plank seams every 3 pixels across the grain.
                    if (j % 3 == 2 and face in ("right", "left", "front", "back")) or (i % 4 == 3 and face in ("top", "bottom")):
                        k = 1
                    if face == "top" and size[1] == 6:
                        k = 4
                    if face == "bottom":
                        k = max(0, k - 1)
                    px[x0 + i, y0 + j] = (*WOOD[k], 255)
    img.save(OUT)
    print("wrote", os.path.relpath(OUT, ROOT))


if __name__ == "__main__":
    main()
