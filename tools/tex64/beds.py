"""Beds at 64x64: the wooden frame, sheet, pillow and a quilted blanket for each of the 16
colours, and the item icons, which are the bed's own 3D model (read from
crates/voxel/src/bed.rs) drawn from above front-right with these same textures.

    python3 tools/tex64/beds.py      # writes the textures, their 16x16 designs and the icons

build.py also rebuilds them (blocks.py and items.py take the generators from here).
"""

import os
import re
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.dirname(HERE))
ROOT = os.path.dirname(os.path.dirname(HERE))

from style import (N, XX, YY, blur, fbm, from_oklab, light, normalize, occlusion, quantize, ramp, rgba,  # noqa: E402
                   rng_for, soften, stretched_noise, to_oklab, value_noise)
import gen_textures as g16  # noqa: E402  (wool colours)

COLORS = ["white", "orange", "magenta", "light_blue", "yellow", "lime", "pink", "gray", "light_gray",
          "cyan", "purple", "blue", "brown", "green", "red", "black"]

WOOD = "#8a5634"
LINEN = "#e9e3d6"


# ---------------------------------------------------------------- materials


def weave(rng, amount=1.0):
    """Fine cloth: threads over and under every two texels, slightly uneven."""
    over = ((XX // 2 + YY // 2) % 2).astype(float)
    thread_x = 0.5 + 0.5 * np.cos((YY % 2) * np.pi)
    thread_y = 0.5 + 0.5 * np.cos((XX % 2) * np.pi)
    w = np.where(over > 0.5, thread_x, thread_y)
    slub = stretched_noise(rng, 16, 1) * 0.5 + stretched_noise(rng, 1, 16) * 0.5
    return (w - 0.5) * 0.35 * amount + (slub - 0.5) * 0.25 * amount


def bed_frame():
    """Varnished hardwood: long fine grain, a few darker streaks, a soft sheen."""
    rng = rng_for("bed_frame")
    pal = ramp(WOOD, 12, 0.36, 0.14, sat=1.05)
    g1 = stretched_noise(rng, 32, 2)
    g2 = stretched_noise(rng, 16, 1)
    g3 = stretched_noise(rng, 64, 4)
    grain = g1 * 0.45 + g2 * 0.35 + g3 * 0.2
    # Growth rings seen edge-on: thin wavy lines.
    wave = value_noise(rng, 16) * 6
    rings = np.abs(np.sin((YY + wave) * np.pi / 5.0)) ** 6
    t = 0.52 + (grain - 0.5) * 0.5 - rings * 0.16
    h = grain * 1.4 - rings * 0.6
    # Varnish: a broad soft highlight band.
    sheen = np.exp(-((YY % 32 - 9) / 6.0) ** 2) * 0.06
    t = t + light(h) * 0.8 + sheen
    return soften(rgba(quantize(np.clip(t, 0, 1), pal, 0.25, rng)), "built")


def bed_sheet():
    """Linen for the sheet and the mattress: a fine weave with soft creases."""
    rng = rng_for("bed_sheet")
    pal = ramp(LINEN, 10, 0.2, 0.1, sat=0.9)
    folds = fbm(rng, ((32, 0.5), (16, 0.35), (8, 0.15)))
    crease = np.abs(stretched_noise(rng, 32, 8) - 0.5) < 0.04
    h = folds * 3.0
    t = 0.56 + light(h) * 1.2 - occlusion(h, 2) * 0.3 + weave(rng) - crease * 0.08
    return soften(rgba(quantize(np.clip(t, 0, 1), pal, 0.3, rng)), "natural")


def bed_pillow():
    """The pillow from above: a plump cushion with piped edges and a dent in the middle."""
    rng = rng_for("bed_pillow")
    pal = ramp("#f1ede4", 10, 0.26, 0.1, sat=0.9)
    u = (XX + 0.5) / N * 2 - 1
    v = (YY + 0.5) / N * 2 - 1
    # A rounded cushion: high in the middle, falling off to the seams.
    r = (np.abs(u) ** 4 + np.abs(v) ** 4) ** 0.25
    dome = np.clip(1 - r * r, 0, 1) * 6
    dent = np.exp(-((u / 0.45) ** 2 + (v / 0.35) ** 2)) * 1.6
    puckers = (fbm(rng, ((16, 0.6), (8, 0.4))) - 0.5) * 1.2 * np.clip(r * 1.6 - 0.6, 0, 1)
    h = dome - dent + puckers
    t = 0.52 + light(blur(h, 1, wrap=False), wrap=False) * 1.8 + weave(rng, 0.3)
    # Piping round the edge, two texels in.
    edge = (np.minimum(np.minimum(XX, YY), np.minimum(N - 1 - XX, N - 1 - YY)))
    t = np.where(edge < 2, t - 0.12, t)
    t = np.where(edge == 2, t + 0.08, t)
    return rgba(quantize(np.clip(t, 0, 1), pal, 0.3, rng))


def blanket(color):
    """A quilted blanket: puffy diamonds sewn along the diagonals, a soft weave and folds."""
    rng = rng_for(f"bed_blanket_{color}")
    base = g16.WOOL[color]
    dark = color in ("black", "gray", "brown", "blue", "purple")
    pal = ramp(base, 12, 0.3 if not dark else 0.24, 0.12, lift=0.04 if dark else 0.0)
    # Diamonds 16 texels across (4 model pixels), seams along both diagonals.
    a = ((XX + YY) % 16) / 16.0
    b = ((XX - YY) % 16) / 16.0
    da = np.abs(a - 0.5) * 2
    db = np.abs(b - 0.5) * 2
    puff = (1 - da ** 2) * (1 - db ** 2)
    seam = (np.minimum(np.minimum(a, 1 - a), np.minimum(b, 1 - b)) * 16) < 0.9
    # Stitches: dashes along the seams.
    dash = ((XX + YY) // 2 % 2 == 0) | ((XX - YY) // 2 % 2 == 0)
    folds = fbm(rng, ((32, 0.6), (16, 0.4)))
    h = puff * 3.2 + folds * 2.0
    t = 0.55 + light(h) * 1.1 - occlusion(h, 2) * 0.35 + weave(rng, 0.8) * 0.8
    t = np.where(seam, t - 0.2, t)
    t = np.where(seam & dash, t - 0.06, t)
    return soften(rgba(quantize(np.clip(t, 0, 1), pal, 0.3, rng)), "built")


BLOCKS = {"bed_frame": bed_frame, "bed_sheet": bed_sheet, "bed_pillow": bed_pillow}
for _c in COLORS:
    BLOCKS[f"bed_blanket_{_c}"] = lambda c=_c: blanket(c)


# ---------------------------------------------------------------- the icon: the 3D bed


def bed_parts():
    """The head and foot boxes from bed.rs: (from, to, material, top uv) facing north."""
    src = open(os.path.join(ROOT, "crates", "voxel", "src", "bed.rs")).read()
    out = {}
    for which in ("HEAD_PARTS", "FOOT_PARTS"):
        body = src.split(f"const {which}: &[Part] = &[")[1].split("];")[0]
        parts = []
        for line in body.splitlines():
            m = re.search(r"\[(\d+), (\d+), (\d+)\]\D+\[(\d+), (\d+), (\d+)\].*Mat::(\w+)", line)
            if not m:
                continue
            uv = re.search(r"top_uv: Some\(\[([\d.]+), ([\d.]+), ([\d.]+), ([\d.]+)\]\)", line)
            v = [int(x) for x in m.groups()[:6]]
            parts.append((v[:3], v[3:], m.group(7), [float(x) for x in uv.groups()] if uv else None))
        out[which] = parts
    return out


MAT_TEX = {"Wood": "bed_frame", "Sheet": "bed_sheet", "Pillow": "bed_pillow"}


def bed_icon(color, textures=None):
    """Draw both halves of the bed (head to the back) the way the inventory shows blocks."""
    tex = textures or {k: f() for k, f in BLOCKS.items()}
    parts = bed_parts()
    boxes = [(f, t, m, uv, 0) for f, t, m, uv in parts["HEAD_PARTS"]] + [(f, t, m, uv, 16) for f, t, m, uv in parts["FOOT_PARTS"]]
    S = 4 * N  # drawn large, then shrunk
    col = np.zeros((S, S, 3))
    depth = np.full((S, S), -1e9)
    hit = np.zeros((S, S), bool)
    # Seen from front-right above: the bed's long side runs to the lower left.
    ang = np.deg2rad(38)
    ca, sa = np.cos(ang), np.sin(ang)
    tilt = 0.52
    scale = S / 46.0

    def project(x, y, z):
        # Turn round y so the foot comes toward the viewer, then look down at `tilt`.
        rx = x * ca + z * sa
        rz = -x * sa + z * ca
        sx = rx
        sy = rz * tilt - y * np.sqrt(1 - tilt * tilt)
        d = rz * np.sqrt(1 - tilt * tilt) + y * tilt
        return sx, sy, d

    # Screen offset so the bed sits in the middle.
    corners = np.array([[x, y, z] for x in (0, 16) for y in (0, 15) for z in (0, 32)], float)
    sx, sy, _ = project(corners[:, 0], corners[:, 1], corners[:, 2])
    ox = S / 2 - (sx.min() + sx.max()) / 2 * scale
    oy = S / 2 - (sy.min() + sy.max()) / 2 * scale
    shade = {0: 0.74, 2: 1.0, 4: 0.86, 1: 0.6, 5: 0.6, 3: 0.4}
    step = 0.1
    for f, t, m, uv, z0 in boxes:
        name = MAT_TEX.get(m, f"bed_blanket_{color}")
        img = tex[name]
        lo = np.array(f, float) + [0, 0, z0]
        hi = np.array(t, float) + [0, 0, z0]
        for face in range(6):
            axis = face >> 1
            pos = hi[axis] if face % 2 == 0 else lo[axis]
            others = [a for a in range(3) if a != axis]
            a0 = np.arange(lo[others[0]] + step / 2, hi[others[0]], step)
            a1 = np.arange(lo[others[1]] + step / 2, hi[others[1]], step)
            if len(a0) == 0 or len(a1) == 0:
                continue
            g0, g1 = np.meshgrid(a0, a1, indexing="ij")
            p = np.zeros(g0.shape + (3,))
            p[..., axis] = pos
            p[..., others[0]] = g0
            p[..., others[1]] = g1
            x, y, z = p[..., 0], p[..., 1], p[..., 2]
            zl = z - z0
            # Texture coordinates as the mesher projects them (bed.rs -> mesh::model_quads).
            if face == 2:
                u, v = x, zl
                if uv is not None:
                    u = uv[0] + (x - f[0]) / (t[0] - f[0]) * (uv[2] - uv[0])
                    v = uv[1] + (zl - f[2]) / (t[2] - f[2]) * (uv[3] - uv[1])
            elif face == 3:
                u, v = x, 16 - zl
            elif face == 0:
                u, v = 16 - zl, 16 - y
            elif face == 1:
                u, v = zl, 16 - y
            elif face == 4:
                u, v = x, 16 - y
            else:
                u, v = 16 - x, 16 - y
            tu = np.clip((u * 4).astype(int), 0, N - 1)
            tv = np.clip((v * 4).astype(int), 0, N - 1)
            c = img[tv, tu, :3] * shade[face]
            sx, sy, d = project(x, y, z)
            px = np.clip((sx * scale + ox).astype(int), 0, S - 1)
            py = np.clip((sy * scale + oy).astype(int), 0, S - 1)
            px, py, d, c = px.ravel(), py.ravel(), d.ravel(), c.reshape(-1, 3)
            order = np.argsort(d)  # nearest written last wins
            px, py, d, c = px[order], py[order], d[order], c[order]
            nearer = d > depth[py, px]
            px, py, d, c = px[nearer], py[nearer], d[nearer], c[nearer]
            depth[py, px] = d
            col[py, px] = c
            hit[py, px] = True
    # Shrink 4x4 -> 1 with coverage as alpha, then a soft outline.
    cov = hit.reshape(N, 4, N, 4).mean((1, 3))
    rgb = (col * hit[..., None]).reshape(N, 4, N, 4, 3).sum((1, 3)) / np.maximum(hit.reshape(N, 4, N, 4).sum((1, 3)), 1)[..., None]
    out = np.zeros((N, N, 4))
    inside = cov >= 0.4
    out[..., :3] = rgb
    out[..., 3] = np.where(inside, 255.0, 0.0)
    ring = blur(inside.astype(float), 1, wrap=False) > 0.05
    ring &= ~inside
    wood = ramp(WOOD, 8, 0.4, 0.14)[0]
    lab = to_oklab(wood)
    lab[0] *= 0.6
    out[ring, :3] = from_oklab(lab)
    out[ring, 3] = 255.0
    return out


ITEMS = {f"{c}_bed": (lambda c=c: bed_icon(c)) for c in COLORS}


def main():
    from PIL import Image
    from style import to_u8
    pack = os.path.join(ROOT, "packs", "vanilla", "textures")
    src = os.path.join(HERE, "src16")
    tex = {k: f() for k, f in BLOCKS.items()}

    def write(kind, name, img):
        big = Image.fromarray(to_u8(img), "RGBA")
        # The 16x16 design keeps the name in build.py's list; the 64x64 is the real texture.
        big.resize((16, 16), Image.BOX).save(os.path.join(src, kind, name + ".png"), optimize=True)
        big.save(os.path.join(pack, kind, name + ".png"), optimize=True)

    for k, img in tex.items():
        write("block", k, img)
    for c in COLORS:
        # Items are pixel art like the others (pixel.py, as build.py does).
        from pixel import pixelate
        write("item", f"{c}_bed", pixelate(bed_icon(c, tex)))
    # The old one-block bed's pictures.
    for c in ("red", "white"):
        for part in ("top", "side", "end"):
            for d in (os.path.join(pack, "block"), os.path.join(src, "block")):
                p = os.path.join(d, f"{c}_bed_{part}.png")
                if os.path.exists(p):
                    os.remove(p)
    print(f"{len(tex)} bed textures and {len(COLORS)} icons written")


if __name__ == "__main__":
    main()
