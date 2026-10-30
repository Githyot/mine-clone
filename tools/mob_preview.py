#!/usr/bin/env python3
"""Draws mobs from their box models and 64x128 skin designs, without the game: a quick look at
shape and proportion for reviewing models (tools/gen_fauna.py, the mob audit).

    python3 tools/mob_preview.py out.png name [name ...]      # one row of views per mob
    python3 tools/mob_preview.py --sheet out.png name ...      # one 3/4 view per mob, in a grid

Every face pixel is drawn as a small quad, far to near (painter's order), lit from above-front.
Relief layers (the lower half of the skin) are drawn on boxes grown by their `inflate`.
"""

import math
import os
import re
import sys

from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
PACK = os.path.join(ROOT, "packs", "vanilla")
SRC = os.path.join(HERE, "tex64", "src16", "entity")
FILES = ["mobs.ron", "mods/mobs.ron", "etapa5/mobs.ron", "fase6/mobs.ron", "tr/mobs.ron", "aguas/mobs.ron",
         "vulcao/mobs.ron", "tr3/mobs.ron", "mods/dims_mobs.ron"]

PART = re.compile(r"\(size:\s*\((\d+),\s*(\d+),\s*(\d+)\),\s*origin:\s*\((-?\d+),\s*(-?\d+),\s*(-?\d+)\),\s*uv:\s*\((\d+),\s*(\d+)\)")
LAYER = re.compile(r"\(part:\s*(\d+),\s*inflate:\s*([0-9.]+)\)")


def load_mobs():
    mobs = {}
    for f in FILES:
        p = os.path.join(PACK, f)
        if not os.path.exists(p):
            continue
        text = open(p, encoding="utf-8").read()
        for m in re.finditer(r'name:\s*"([a-z0-9_]+)",\s*display_name:\s*"([^"]*)"', text):
            name = m.group(1)
            start = m.end()
            pm = re.search(r"parts:\s*\[", text[start:])
            if not pm:
                continue
            i = start + pm.end()
            depth, j = 1, i
            while depth:
                c = text[j]
                depth += c == "["
                depth -= c == "]"
                j += 1
            parts = [tuple(int(v) for v in g.groups()) for g in PART.finditer(text, i, j)]
            rest = text[j:j + 400]
            lm = re.match(r"\s*,\s*layers:\s*\[(.*?)\]", rest, re.S)
            layers = [(int(a), float(b)) for a, b in LAYER.findall(lm.group(1))] if lm else []
            sc = re.search(r"scale:\s*([0-9.]+)", text[m.start():j])
            mobs.setdefault(name, dict(name=name, display=m.group(2), parts=parts, layers=layers, file=f,
                                       scale=float(sc.group(1)) if sc else 1.0))
    # Real animals' own models (tools/gen_fauna.py) replace the boxes their file gave them.
    fp = os.path.join(PACK, "fauna", "models.ron")
    if os.path.exists(fp):
        text = open(fp, encoding="utf-8").read()
        for m in re.finditer(r'name:\s*"([a-z0-9_]+)",\s*skin:\s*"([a-z0-9_]+)"', text):
            name, skin = m.group(1), m.group(2)
            if name not in mobs:
                continue
            i = text.index("parts:", m.end()) + len("parts:")
            j = text.index("        ],", i)
            parts = [tuple(int(v) for v in g.groups()) for g in PART.finditer(text, i, j)]
            end = text.find("\n    ),", j)
            lm = re.search(r"layers:\s*\[(.*?)\]", text[j:end])
            sc = re.search(r"scale:\s*Some\(([0-9.]+)\)", text[m.start():i])
            mobs[name].update(parts=parts, skin=skin, layers=[(int(a), float(b)) for a, b in LAYER.findall(lm.group(1))] if lm else [])
            if sc:
                mobs[name]["scale"] = float(sc.group(1))
    return mobs


def skin_for(name):
    for d in (SRC, os.path.join(PACK, "textures", "entity")):
        p = os.path.join(d, name + ".png")
        if os.path.exists(p):
            im = Image.open(p).convert("RGBA")
            if im.size != (64, 128):
                im = im.resize((64, 128), Image.NEAREST)
            return im
    return None


def faces(size, uv):
    w, h, d = size
    u, v = uv
    # (rect in skin, function face-pixel -> corner points in box space, normal)
    return [
        ("top", (u + d, v, w, d), lambda i, j: [(i, h, j), (i + 1, h, j), (i + 1, h, j + 1), (i, h, j + 1)], (0, 1, 0)),
        ("bottom", (u + d + w, v, w, d), lambda i, j: [(i, 0, d - j), (i + 1, 0, d - j), (i + 1, 0, d - j - 1), (i, 0, d - j - 1)], (0, -1, 0)),
        ("right", (u, v + d, d, h), lambda i, j: [(0, h - j, i), (0, h - j, i + 1), (0, h - j - 1, i + 1), (0, h - j - 1, i)], (-1, 0, 0)),
        ("front", (u + d, v + d, w, h), lambda i, j: [(i, h - j, d), (i + 1, h - j, d), (i + 1, h - j - 1, d), (i, h - j - 1, d)], (0, 0, 1)),
        ("left", (u + d + w, v + d, d, h), lambda i, j: [(w, h - j, d - i), (w, h - j, d - i - 1), (w, h - j - 1, d - i - 1), (w, h - j - 1, d - i)], (1, 0, 0)),
        ("back", (u + 2 * d + w, v + d, w, h), lambda i, j: [(w - i, h - j, 0), (w - i - 1, h - j, 0), (w - i - 1, h - j - 1, 0), (w - i, h - j - 1, 0)], (0, 0, -1)),
    ]


def quads(mob, skin):
    out = []
    px = skin.load()
    boxes = [(p, 0.0, 0) for p in mob["parts"]]
    for idx, inf in mob["layers"]:
        if idx < len(mob["parts"]):
            boxes.append((mob["parts"][idx], inf, 64))
    for p, inf, dv in boxes:
        w, h, d, ox, oy, oz, u, v = p
        sx, sy, sz = ((w + 2 * inf) / w if w else 1), ((h + 2 * inf) / h if h else 1), ((d + 2 * inf) / d if d else 1)
        for name, (x0, y0, fw, fh), corner, n in faces((w, h, d), (u, v + dv)):
            for j in range(fh):
                for i in range(fw):
                    X, Y = x0 + i, y0 + j
                    if not (0 <= X < 64 and 0 <= Y < 128):
                        continue
                    c = px[X, Y]
                    if c[3] < 128:
                        continue
                    pts = [(ox - inf + a * sx, oy - inf + b * sy, oz - inf + e * sz) for a, b, e in corner(i, j)]
                    out.append((pts, c[:3], n))
    return out


def project(pt, yaw, pitch):
    x, y, z = pt
    cy, sy = math.cos(yaw), math.sin(yaw)
    x, z = x * cy + z * sy, -x * sy + z * cy
    cp, sp = math.cos(pitch), math.sin(pitch)
    y, z = y * cp - z * sp, y * sp + z * cp
    return x, -y, z


def render(mob, skin, yaw, pitch, size=360, light=(0.4, 0.8, 0.6)):
    qs = quads(mob, skin)
    img = Image.new("RGB", (size, size), (52, 58, 66))
    if not qs:
        return img
    proj = []
    for pts, col, n in qs:
        pp = [project(p, yaw, pitch) for p in pts]
        nn = project(n, yaw, pitch)
        depth = sum(p[2] for p in pp) / 4
        k = 0.55 + 0.45 * max(0.0, n[0] * light[0] + n[1] * light[1] + n[2] * light[2])
        proj.append((depth, pp, tuple(int(c * k) for c in col)))
    xs = [p[0] for _, pp, _ in proj for p in pp]
    ys = [p[1] for _, pp, _ in proj for p in pp]
    span = max(max(xs) - min(xs), max(ys) - min(ys), 1)
    s = (size - 24) / span
    cx, cy = (max(xs) + min(xs)) / 2, (max(ys) + min(ys)) / 2
    dr = ImageDraw.Draw(img)
    for depth, pp, col in sorted(proj, key=lambda t: t[0]):
        dr.polygon([(size / 2 + (x - cx) * s, size / 2 + (y - cy) * s) for x, y, _ in pp], fill=col, outline=col)
    return img


VIEWS = [(math.radians(-35), math.radians(-20)), (math.radians(-90), 0.0), (math.radians(0), 0.0), (math.radians(150), math.radians(-25))]


def label(img, text):
    d = ImageDraw.Draw(img)
    d.rectangle((0, 0, img.width, 16), fill=(20, 20, 24))
    d.text((4, 2), text, fill=(240, 240, 240))
    return img


def main():
    args = sys.argv[1:]
    sheet = args and args[0] == "--sheet"
    if sheet:
        args = args[1:]
    out, names = args[0], args[1:]
    mobs = load_mobs()
    if names == ["all"]:
        names = sorted(mobs)
    tiles = []
    for n in names:
        m = mobs.get(n)
        sk = skin_for(m.get("skin", n)) if m else None
        if not m or sk is None:
            print("skip", n)
            continue
        views = VIEWS[:1] if sheet else VIEWS
        row = [label(render(m, sk, y, p, 300 if sheet else 360), f"{n} ({m['display']})") for y, p in views]
        tiles.append(row)
    if not tiles:
        return
    if sheet:
        cols = 6
        W = 300
        img = Image.new("RGB", (cols * W, ((len(tiles) + cols - 1) // cols) * W), (30, 30, 34))
        for k, row in enumerate(tiles):
            img.paste(row[0], ((k % cols) * W, (k // cols) * W))
    else:
        W = 360
        img = Image.new("RGB", (len(VIEWS) * W, len(tiles) * W), (30, 30, 34))
        for r, row in enumerate(tiles):
            for c, t in enumerate(row):
                img.paste(t, (c * W, r * W))
    img.save(out)
    print("wrote", out, len(tiles))


if __name__ == "__main__":
    main()
