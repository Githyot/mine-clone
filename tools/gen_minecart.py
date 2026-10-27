#!/usr/bin/env python3
"""Paint the minecart's skin (packs/vanilla/textures/entity/minecart.png).

Dark riveted iron. The boxes are the "minecart" model in packs/vanilla/mobs.ron; the
unwrap is the one described at the top of that file. The bottom half holds the relief
layer: rivet heads on the side walls that stand off the metal in 3D.

Usage: python3 tools/gen_minecart.py
"""

import os
import zlib

from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "packs", "vanilla", "textures", "entity", "minecart.png")

# (size w, h, d), uv: must match mobs.ron.
BOXES = [
    ((14, 2, 18), (0, 0)),
    ((2, 8, 20), (0, 20)),
    ((12, 8, 2), (0, 48)),
    ((16, 2, 2), (28, 48)),
]
IRON = [(38, 38, 44), (58, 58, 66), (76, 76, 86), (96, 96, 108), (122, 122, 134)]


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
                    n = noise(x0 + i, y0 + j)
                    k = 2 + (1 if n > 0.7 else 0) - (1 if n < 0.2 else 0)
                    # Edges of every face are darker, the top rim catches the light.
                    if i == 0 or j == 0 or i == fw - 1 or j == fh - 1:
                        k = 1
                    if face == "top" and size[1] == 8:
                        k = 4
                    if face == "bottom":
                        k = max(0, k - 1)
                    px[x0 + i, y0 + j] = (*IRON[k], 255)
            # Rivets on the outside of the walls: in the relief layer, every 4 pixels.
            if size[1] == 8 and face in ("right", "left", "front", "back"):
                for j in (1, fh - 2):
                    for i in range(1, fw - 1, 4):
                        px[x0 + i, y0 + j + 64] = (*IRON[4], 255)
    img.save(OUT)
    print("wrote", os.path.relpath(OUT, ROOT))


if __name__ == "__main__":
    main()
