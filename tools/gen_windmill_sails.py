#!/usr/bin/env python3
"""Paint the windmill sails' skin (packs/vanilla/textures/entity/windmill_sails.png).

Weathered oak spars and boards, a hub of end grain with an iron pin, and sails of off-white
canvas. Over the canvas (in the relief half of the skin, rows 64+) a wooden lattice that the
game stands out in 3D. The boxes are the "windmill_sails" model in packs/vanilla/mobs.ron.

This writes the 64x128 design; tools/tex64/build.py grows it to 4x like every other skin.

Usage: python3 tools/gen_windmill_sails.py && python3 tools/tex64/build.py
"""

import os
import zlib

from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "packs", "vanilla", "textures", "entity", "windmill_sails.png")

# (size w, h, d), uv: must match mobs.ron.
HUB = ((6, 6, 4), (0, 0))
SPAR = ((2, 34, 2), (0, 12))
SAIL = ((10, 26, 1), (8, 12))
BOARD = ((3, 26, 1), (30, 12))

WOOD = [(58, 40, 22), (84, 58, 32), (110, 78, 44), (134, 98, 58), (160, 122, 76)]
END_GRAIN = [(92, 66, 38), (126, 92, 54), (150, 112, 68)]
CANVAS = [(196, 188, 168), (214, 207, 188), (228, 222, 204), (238, 233, 218)]
IRON = (70, 72, 78)


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


def noise(*k):
    return (zlib.crc32(",".join(map(str, k)).encode()) & 0xFFFF) / 0xFFFF


def wood(px, size, uv, seed, along_y=True):
    """Planks with grain running along the arm; the lit side (top, left) a step lighter."""
    for face, (x0, y0, fw, fh) in faces(size, uv).items():
        for j in range(fh):
            for i in range(fw):
                # Grain: streaks along the length, broken now and then.
                streak = noise(seed, face, i if along_y else j)
                n = noise(seed, face, i, j // 3)
                k = 2 + (1 if streak > 0.72 else 0) - (1 if streak < 0.22 else 0)
                if n > 0.9:
                    k -= 1
                if face in ("top", "right"):
                    k += 1
                if face in ("bottom", "left"):
                    k -= 1
                # Dark edges where the board meets the next face.
                if face in ("front", "back") and (i == 0 or i == fw - 1) and fw > 2:
                    k -= 1
                px[x0 + i, y0 + j] = (*WOOD[max(0, min(4, k))], 255)


def main():
    img = Image.new("RGBA", (64, 128), (0, 0, 0, 0))
    px = img.load()

    # The hub: wood all round, end grain with growth rings and an iron pin in front.
    wood(px, *HUB, seed="hub")
    x0, y0, fw, fh = faces(*HUB)["front"]
    for j in range(fh):
        for i in range(fw):
            dx, dy = i - (fw - 1) / 2, j - (fh - 1) / 2
            r = (dx * dx + dy * dy) ** 0.5
            ring = int(r * 1.3) % 2
            c = END_GRAIN[1 + ring] if r < 2.6 else END_GRAIN[0]
            if r < 0.8:
                c = IRON
            px[x0 + i, y0 + j] = (*c, 255)

    wood(px, *SPAR, seed="spar")
    wood(px, *BOARD, seed="board")
    # Iron straps across the spar where the cloth is tied on.
    x0, y0, fw, fh = faces(*SPAR)["front"]
    for j in (8, 20, 32):
        for i in range(fw):
            px[x0 + i, y0 + j] = (*IRON, 255)

    # The sail: canvas with a faint weave, a little darker near the spar and the hem.
    for face, (x0, y0, fw, fh) in faces(*SAIL).items():
        for j in range(fh):
            for i in range(fw):
                weave = ((i + j) % 2) * 0.5 + noise("sail", face, i, j) * 0.5
                k = 2 if weave > 0.45 else 1
                if noise("sail-light", i // 3, j // 4) > 0.8:
                    k += 1
                if face in ("front", "back") and (i == 0 or j == fh - 1):
                    k -= 1
                if face in ("bottom", "left"):
                    k -= 1
                px[x0 + i, y0 + j] = (*CANVAS[max(0, min(3, k))], 255)

    # The lattice over the canvas (relief rows): a frame and cross slats every few pixels,
    # on both faces, so it stands out whichever side you look at.
    for face in ("front", "back"):
        x0, y0, fw, fh = faces(*SAIL)[face]
        for j in range(fh):
            for i in range(fw):
                edge = i == 0 or i == fw - 1 or j == 0 or j == fh - 1
                slat = j % 5 == 2
                rail = i == fw // 2
                if edge or slat or rail:
                    k = 2 if edge else 3
                    if noise("lat", face, i, j) > 0.8:
                        k -= 1
                    px[x0 + i, y0 + 64 + j] = (*WOOD[k], 255)
    img.save(OUT)
    print("wrote", os.path.relpath(OUT, ROOT))


if __name__ == "__main__":
    main()
