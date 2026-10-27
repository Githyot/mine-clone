#!/usr/bin/env python3
"""Wagons, carts and the draft ox: models and skins (our own art).

Writes the skins straight at the 64x level of detail (256 x 512, 4 texels per model pixel) to
packs/vanilla/textures/entity/, and the models into packs/vanilla/mobs.ron between the
"wagons" markers. A wagon is drawn from several models so each one fits the 64-pixel-wide
skin layout and can move on its own (see crates/game/src/wagons.rs):

* wagon         the bed, side boards, rear axle, seat and brake (scale 2: 1 px = 1/8 block)
* wagon_cover   the canvas over its hoops, the cargo you see from the back, a lantern
* wagon_hay     a load of hay with a pitchfork, for the farm cart
* wagon_shafts  front axle and two shafts, for one animal; turns with the animal
* wagon_pole    front axle, a pole, an evener and a neck yoke with bows, for a pair
* wagon_wheel   one spoked wheel (scale 0.5, so the spokes are fine); spins as it rolls
* harness       a girth band and a collar, drawn in the horse's own model space
* ox            the draft ox (zebu-like hump, wide horns, dewlap)

Depth comes from relief layers: iron straps and nail heads on the boards, the hoops pressing
through the canvas, straw sticking out of the hay, and the whole wheel (rim, felloes, spokes)
is relief, so it is a real 3D wheel with gaps between the spokes.

Usage: python3 tools/gen_wagons.py
"""

import math
import os
import re
import zlib

import numpy as np
from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ENTITY = os.path.join(ROOT, "packs", "vanilla", "textures", "entity")
MOBS = os.path.join(ROOT, "packs", "vanilla", "mobs.ron")
S = 4  # texels per model pixel
W, H = 64, 128

BEGIN = "    // == wagons (tools/gen_wagons.py) =="
END = "    // == end wagons =="


def rng(name):
    return np.random.default_rng(zlib.crc32(name.encode()))


def hexc(h):
    h = h.lstrip("#")
    return np.array([int(h[i:i + 2], 16) for i in (0, 2, 4)], dtype=np.float64)


def ramp(*cols):
    return [hexc(c) for c in cols]


def pick(r, t):
    """Colour at t (0..1) along a ramp, blended between steps."""
    t = min(max(t, 0.0), 1.0) * (len(r) - 1)
    k = int(t)
    if k >= len(r) - 1:
        return r[-1]
    f = t - k
    return r[k] * (1 - f) + r[k + 1] * f


# Ramps: shadows lean cool, lights lean warm (like the block textures).
OAK = ramp("#3b2616", "#5a3a20", "#77502c", "#8f6538", "#a87c48", "#c09458")
OAK_DARK = ramp("#2a1b12", "#3d2819", "#523622", "#66452c", "#7a5536")
IRON = ramp("#24252b", "#36373f", "#4a4c55", "#62646d", "#7c7e87")
CANVAS = ramp("#8f8672", "#aaa18a", "#c2b9a0", "#d6ceb5", "#e6dfc8")
HAY = ramp("#7a5a1c", "#9c7826", "#b99331", "#d2ad44", "#e6c763")
BURLAP = ramp("#5e4a2e", "#7a6240", "#937a52", "#a99063")
LEATHER = ramp("#2e1c14", "#43291c", "#5a3826", "#6e4630")
BRASS = ramp("#6a5222", "#957636", "#c0a052", "#e0c878")
OX = ramp("#3e2316", "#56301d", "#6e3f25", "#874f2e", "#9c6239", "#b07548")
OX_LIGHT = ramp("#8a7466", "#a48d7c", "#bca592", "#d0bba6")
HORN = ramp("#3a3128", "#6e6150", "#a89a80", "#d6cbb0", "#ece4cc")
LANTERN = ramp("#ff9a3a", "#ffc25a", "#ffe08a")


# ------------------------------------------------------------------ the layout


def unwrap(size, uv):
    """Face rectangles of a box (x, y, w, h) in model pixels, as the game reads them."""
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


# Which model axis runs along a face's i (across) and j (down) directions.
FACE_AXES = {
    "top": ("x", "z"), "bottom": ("x", "z"), "right": ("z", "y"), "left": ("z", "y"),
    "front": ("x", "y"), "back": ("x", "y"),
}


def pack(parts):
    """Shelf-pack the unwraps into the 64 x 64 top half. Parts with the same `share` key use
    one unwrap (both side boards, say)."""
    x = y = shelf = 0
    placed = {}
    # Tallest unwraps first, so the shelves waste little.
    for p in sorted(parts, key=lambda p: -(p["size"][2] + p["size"][1])):
        key = p.get("share", id(p))
        if "at" in p:
            # Placed by hand (the wheel's hub sits in the rows its zero-width disc leaves free).
            p["uv"] = p["at"]
            continue
        if key in placed:
            p["uv"] = placed[key]
            continue
        w, h, d = p["size"]
        uw, uh = 2 * d + 2 * w, d + h
        if x + uw > 64:
            x, y, shelf = 0, y + shelf, 0
        if uw > 64 or y + uh > 64:
            raise SystemExit(f"skin full at part {p}")
        p["uv"] = (x, y)
        placed[key] = (x, y)
        x += uw
        shelf = max(shelf, uh)


class Skin:
    def __init__(self):
        self.px = np.zeros((H * S, W * S, 4), dtype=np.float64)

    def put(self, x, y, c, a=255):
        self.px[y, x, :3] = c
        self.px[y, x, 3] = a

    def save(self, name):
        img = Image.fromarray(np.clip(self.px, 0, 255).astype(np.uint8), "RGBA")
        img.save(os.path.join(ENTITY, name + ".png"), optimize=True)


# ------------------------------------------------------------------ materials
# A painter gets the texel position in the face (tx, ty in texels, 0..w*S, 0..h*S), the face
# size in texels, the face's axes and the part, and returns a colour (or None: transparent).


def noise2(r, h, w, cell):
    gh, gw = max(2, h // cell + 2), max(2, w // cell + 2)
    g = r.random((gh, gw))
    fy = np.arange(h) / cell
    fx = np.arange(w) / cell
    y0, x0 = np.floor(fy).astype(int), np.floor(fx).astype(int)
    ty, tx = (fy - y0)[:, None], (fx - x0)[None, :]
    a, b = g[np.ix_(y0, x0)], g[np.ix_(y0, x0 + 1)]
    c, d = g[np.ix_(y0 + 1, x0)], g[np.ix_(y0 + 1, x0 + 1)]
    return (a * (1 - tx) + b * tx) * (1 - ty) + (c * (1 - tx) + d * tx) * ty


def wood(face_w, face_h, along, r, pal=OAK, plank=2, nails=True):
    """Planks with the grain along `along` ("i", "j" or None for end grain)."""
    tw, th = face_w * S, face_h * S
    out = np.zeros((th, tw, 3))
    n1 = noise2(r, th, tw, 3)
    n2 = noise2(r, th, tw, 7)
    pw = plank * S
    for ty in range(th):
        for tx in range(tw):
            if along is None:
                # End grain: growth rings around a centre.
                cx, cy = tw / 2, th / 2
                d = math.hypot(tx - cx, ty - cy)
                t = 0.55 + 0.12 * math.sin(d * 1.4) + (n1[ty, tx] - 0.5) * 0.2
                if tx in (0, tw - 1) or ty in (0, th - 1):
                    t -= 0.18
                out[ty, tx] = pick(pal, t)
                continue
            a, c = (tx, ty) if along == "i" else (ty, tx)  # a along the grain, c across
            k = c // pw
            cc = c % pw
            # Each plank its own shade, a stretched grain, a dark seam, a lit top edge.
            pr = zlib.crc32(f"{k}".encode()) % 1000 / 1000.0
            grain = math.sin((a * 0.18) + n2[ty, tx] * 6 + k * 2.3) * 0.06
            t = 0.55 + (pr - 0.5) * 0.18 + (n1[ty, tx] - 0.5) * 0.18 + grain
            if cc == 0:
                t = 0.08
            elif cc == 1:
                t += 0.08
            elif cc == pw - 1:
                t -= 0.1
            out[ty, tx] = pick(pal, t)
            # A nail head near each plank end.
            if nails and cc in (pw // 2 - 1, pw // 2) and (a % (24 * S // 4)) in (3, 4):
                out[ty, tx] = pick(IRON, 0.75 if cc == pw // 2 - 1 else 0.35)
    return out


def iron(face_w, face_h, r):
    tw, th = face_w * S, face_h * S
    n = noise2(r, th, tw, 2)
    out = np.zeros((th, tw, 3))
    for ty in range(th):
        for tx in range(tw):
            t = 0.5 + (n[ty, tx] - 0.5) * 0.35 - (ty / max(1, th)) * 0.15
            if ty == 0:
                t += 0.25
            out[ty, tx] = pick(IRON, t)
    return out


def cloth(face_w, face_h, r, pal=CANVAS, folds="i"):
    """Canvas: a fine weave, soft folds across the cloth, darker where it meets the boards."""
    tw, th = face_w * S, face_h * S
    n = noise2(r, th, tw, 5)
    out = np.zeros((th, tw, 3))
    for ty in range(th):
        for tx in range(tw):
            f = tx if folds == "i" else ty
            fold = math.sin(f * 0.21 + n[ty, tx] * 2.5) * 0.09
            weave = 0.03 if (tx + ty) % 2 == 0 else -0.03
            t = 0.58 + fold + weave + (n[ty, tx] - 0.5) * 0.12
            out[ty, tx] = pick(pal, t)
    return out


def straw(face_w, face_h, r, along="i"):
    tw, th = face_w * S, face_h * S
    out = np.zeros((th, tw, 3))
    n = noise2(r, th, tw, 2)
    phase = r.random(tw + th) * 6.28
    for ty in range(th):
        for tx in range(tw):
            a, c = (tx, ty) if along == "i" else (ty, tx)
            s = math.sin(c * 1.7 + phase[a % len(phase)] * 0.3 + a * 0.15)
            t = 0.5 + s * 0.22 + (n[ty, tx] - 0.5) * 0.35
            out[ty, tx] = pick(HAY, t)
    return out


def fur(face_w, face_h, r, pal=OX, base=0.55, shade_down=True):
    tw, th = face_w * S, face_h * S
    n = noise2(r, th, tw, 2)
    m = noise2(r, th, tw, 6)
    out = np.zeros((th, tw, 3))
    for ty in range(th):
        for tx in range(tw):
            t = base + (n[ty, tx] - 0.5) * 0.25 + (m[ty, tx] - 0.5) * 0.2
            if shade_down:
                t -= (ty / max(1, th)) * 0.15
            out[ty, tx] = pick(pal, t)
    return out


def paint_face(sk, rect, img, alpha=None):
    x, y, w, h = rect
    for ty in range(h * S):
        for tx in range(w * S):
            a = 255 if alpha is None else alpha[ty // S, tx // S]
            if a:
                sk.put(x * S + tx, y * S + ty, img[ty, tx], 255)


def relief(sk, rect, mask, img):
    """Fill the relief layer (bottom half) for a face: mask is per model pixel."""
    x, y, w, h = rect
    for j in range(h):
        for i in range(w):
            if not mask[j][i]:
                continue
            for ty in range(S):
                for tx in range(S):
                    c = img(i, j, tx, ty)
                    sk.put((x + i) * S + tx, (y + 64 + j) * S + ty, c, 255)


def grain_axis(part, face):
    """Which face direction the wood grain runs along: the part's longest axis."""
    w, h, d = part["size"]
    longest = max((("x", w), ("y", h), ("z", d)), key=lambda t: t[1])[0]
    ia, ja = FACE_AXES[face]
    if ia == longest:
        return "i"
    if ja == longest:
        return "j"
    return None


def paint_part(sk, part, name):
    r = rng(name + str(part["uv"]) + str(part["size"]))
    w, h, d = part["size"]
    mat = part["mat"]
    for face, rect in unwrap(part["size"], part["uv"]).items():
        fw, fh = rect[2], rect[3]
        if fw <= 0 or fh <= 0:
            continue
        if mat == "clear":
            continue
        if mat in ("oak", "oak_dark", "crate"):
            pal = OAK_DARK if mat == "oak_dark" else OAK
            along = grain_axis(part, face)
            img = wood(fw, fh, along, r, pal, plank=part.get("plank", 2), nails=part.get("nails", True))
            if mat == "crate":
                # A dark frame round each face of the crate.
                for ty in range(fh * S):
                    for tx in range(fw * S):
                        if tx < 3 or ty < 3 or tx >= fw * S - 3 or ty >= fh * S - 3:
                            img[ty, tx] = pick(OAK_DARK, 0.45 + 0.1 * ((tx + ty) % 3 == 0))
        elif mat == "barrel":
            img = wood(fw, fh, "j" if face not in ("top", "bottom") else None, r, OAK, plank=1, nails=False)
            if face not in ("top", "bottom"):
                for ty in range(fh * S):
                    if ty in (3, 4, fh * S - 5, fh * S - 4):
                        img[ty, :] = [pick(IRON, 0.45 + (0.2 if ty in (3, fh * S - 5) else 0))] * (fw * S)
        elif mat == "iron":
            img = iron(fw, fh, r)
        elif mat == "canvas":
            img = cloth(fw, fh, r, folds="i" if face in ("right", "left") else "j")
            if face in ("right", "left"):
                # Grime toward the bottom edge.
                for ty in range(fh * S):
                    k = max(0.0, ty / (fh * S) - 0.75) * 1.2
                    img[ty] = img[ty] * (1 - k * 0.35)
        elif mat == "hay":
            img = straw(fw, fh, r, "i" if face in ("top", "bottom", "front", "back") else "j")
        elif mat == "burlap":
            img = cloth(fw, fh, r, BURLAP)
        elif mat == "leather":
            img = fur(fw, fh, r, LEATHER, 0.55, False)
            for ty in range(fh * S):
                for tx in range(fw * S):
                    if ty in (1, fh * S - 2):
                        img[ty, tx] = pick(LEATHER, 0.85)
        elif mat == "lantern":
            img = iron(fw, fh, r)
            if face not in ("top", "bottom"):
                for ty in range(2, fh * S - 2):
                    for tx in range(2, fw * S - 2):
                        img[ty, tx] = pick(LANTERN, 0.3 + 0.7 * (1 - abs(tx - fw * S / 2) / (fw * S / 2)))
        elif mat == "horn":
            img = np.zeros((fh * S, fw * S, 3))
            for ty in range(fh * S):
                for tx in range(fw * S):
                    img[ty, tx] = pick(HORN, 0.75 - ty / (fh * S) * 0.25)
        elif mat.startswith("ox"):
            img = paint_ox(part, face, fw, fh, r)
        else:
            raise SystemExit(f"unknown material {mat}")
        paint_face(sk, rect, img)
    if "relief" in part:
        part["relief"](sk, part, r)


# ------------------------------------------------------------------ relief designs


def faces_of(part):
    return unwrap(part["size"], part["uv"])


def straps_relief(sk, part, r):
    """Iron straps across the side boards and a nail head on each end."""
    for face in ("right", "left"):
        x, y, w, h = faces_of(part)[face]
        mask = [[0] * w for _ in range(h)]
        for i in range(w):
            if i % 5 == 2:
                for j in range(h):
                    mask[j][i] = 1
        relief(sk, (x, y, w, h), mask, lambda i, j, tx, ty: pick(IRON, 0.55 + (0.25 if ty == 0 else 0) - ty * 0.05 + (0.3 if (j == 0 or j == h - 1) and tx in (1, 2) and ty in (1, 2) else 0)))


def board_relief(sk, part, r):
    """A top rail along the front and back boards."""
    for face in ("front", "back"):
        x, y, w, h = faces_of(part)[face]
        mask = [[1 if j == 0 or i in (0, w - 1) else 0 for i in range(w)] for j in range(h)]
        relief(sk, (x, y, w, h), mask, lambda i, j, tx, ty: pick(OAK_DARK, 0.6 + (0.2 if ty == 0 else 0)))


def hoops_relief(sk, part, r):
    """Hoops under the canvas: raised ribs across the cloth, and the rope lacing at the bottom."""
    sides = ("right", "left") if part["size"][0] == 1 else ("top",)
    for face in sides:
        x, y, w, h = faces_of(part)[face]
        mask = [[0] * w for _ in range(h)]
        rib = lambda a: a % 5 == 1
        for j in range(h):
            for i in range(w):
                if face in ("right", "left") and rib(i):
                    mask[j][i] = 1
                if face == "top" and rib(j):
                    mask[j][i] = 1
        if face in ("right", "left"):
            for i in range(w):
                if i % 2 == 0:
                    mask[h - 1][i] = 1
        relief(sk, (x, y, w, h), mask, lambda i, j, tx, ty: pick(CANVAS, 0.72 + 0.12 * (ty == 0) - 0.04 * ty) if not (j == h - 1 and face in ("right", "left") and not rib(i)) else pick(BURLAP, 0.6 + 0.1 * (tx % 2)))
    # The front and back edges of the cover: a hoop showing its wood at the opening.
    for face in ("front", "back"):
        x, y, w, h = faces_of(part)[face]
        if w <= 0 or h <= 0:
            continue
        mask = [[1] * w for _ in range(h)]
        relief(sk, (x, y, w, h), mask, lambda i, j, tx, ty: pick(OAK, 0.5 + 0.15 * (ty == 0)))


def tufts_relief(sk, part, r):
    """Straw poking out of the load."""
    for face in ("top", "right", "left", "front", "back"):
        x, y, w, h = faces_of(part)[face]
        mask = [[1 if r.random() < (0.45 if face == "top" else 0.3) else 0 for i in range(w)] for j in range(h)]
        relief(sk, (x, y, w, h), mask, lambda i, j, tx, ty: pick(HAY, 0.45 + r.random() * 0.5))


def wheel_relief(sk, part, r):
    """The whole wheel: iron tyre, wooden felloes, twelve spokes and the hub."""
    for face in ("right", "left"):
        x, y, w, h = faces_of(part)[face]
        cx, cy = (w - 1) / 2, (h - 1) / 2
        R = w / 2
        mask = [[0] * w for _ in range(h)]
        kind = [[None] * w for _ in range(h)]
        for j in range(h):
            for i in range(w):
                dx, dy = i - cx, j - cy
                d = math.hypot(dx, dy)
                a = math.atan2(dy, dx)
                if d > R - 0.2:
                    continue
                if d > R - 1.4:
                    kind[j][i] = "tyre"
                elif d > R - 3.6:
                    kind[j][i] = "felloe"
                elif d < 3.2:
                    kind[j][i] = "hub"
                else:
                    # Twelve spokes, a little wider near the hub.
                    sa = (a / (2 * math.pi) * 12) % 1.0
                    width = 0.11 + 0.9 / max(d, 1.0)
                    if sa < width or sa > 1 - width:
                        kind[j][i] = "spoke"
                if kind[j][i]:
                    mask[j][i] = 1

        def col(i, j, tx, ty):
            k = kind[j][i]
            lit = 0.08 * (ty == 0) - 0.06 * (ty == S - 1)
            if k == "tyre":
                return pick(IRON, 0.45 + lit + 0.15 * ((i * 7 + j * 3) % 11 == 0))
            if k == "felloe":
                return pick(OAK, 0.45 + lit + 0.05 * math.sin(i + j))
            if k == "hub":
                d = math.hypot(i - cx, j - cy)
                return pick(IRON, 0.6 + lit) if d < 1.3 else pick(OAK_DARK, 0.6 + lit)
            return pick(OAK, 0.62 + lit + 0.06 * ((tx + ty) % 2))

        relief(sk, (x, y, w, h), mask, col)


def ox_fur_relief(sk, part, r):
    """Shaggy patches on the ox's shoulders and hump."""
    for face in ("top", "right", "left"):
        x, y, w, h = faces_of(part)[face]
        mask = [[1 if r.random() < 0.18 else 0 for i in range(w)] for j in range(h)]
        relief(sk, (x, y, w, h), mask, lambda i, j, tx, ty: pick(OX, 0.4 + r.random() * 0.3))


def collar_relief(sk, part, r):
    """Brass studs round the collar."""
    for face in ("right", "left", "front", "top"):
        x, y, w, h = faces_of(part)[face]
        mask = [[1 if (i + j) % 3 == 0 and j in (1, h - 2) else 0 for i in range(w)] for j in range(h)]
        relief(sk, (x, y, w, h), mask, lambda i, j, tx, ty: pick(BRASS, 0.7 - 0.12 * ty + 0.2 * (tx < 2)))


def paint_ox(part, face, fw, fh, r):
    mat = part["mat"]
    if mat == "ox_body":
        img = fur(fw, fh, r)
        if face == "bottom":
            img = fur(fw, fh, r, OX_LIGHT, 0.45, False)
        return img
    if mat == "ox_leg":
        img = fur(fw, fh, r, OX, 0.45)
        for ty in range(fh * S - 6, fh * S):
            for tx in range(fw * S):
                img[ty, tx] = pick(HORN, 0.15 + 0.1 * (ty % 2))  # hooves
        return img
    if mat == "ox_head":
        img = fur(fw, fh, r, OX, 0.5, False)
        if face == "front":
            # Eyes: dark with a small highlight, on the upper half; a pale blaze between.
            for ey, ex in ((2, 1), (2, fw - 2)):
                for ty in range(S):
                    for tx in range(S):
                        img[ey * S + ty, ex * S + tx] = hexc("#141010") if not (tx == 1 and ty == 1) else hexc("#cfc6b8")
            for ty in range(0, fh * S):
                for tx in range(fw * S // 2 - 3, fw * S // 2 + 3):
                    if ty < 2 * S:
                        img[ty, tx] = pick(OX_LIGHT, 0.55)
        return img
    if mat == "ox_muzzle":
        img = fur(fw, fh, r, OX_LIGHT, 0.4, False)
        if face == "front":
            for (ny, nx) in ((1, 1), (1, fw - 2)):
                for ty in range(S):
                    for tx in range(S):
                        img[ny * S + ty, nx * S + tx] = hexc("#2a2020")
        return img
    if mat == "ox_tail":
        img = fur(fw, fh, r, OX, 0.45)
        for ty in range(fh * S - 8, fh * S):
            img[ty] = [pick(OX, 0.1)] * (fw * S)
        return img
    raise SystemExit(mat)


# ------------------------------------------------------------------ models


def P(size, origin, mat, **kw):
    d = {"size": size, "origin": origin, "mat": mat}
    d.update(kw)
    return d


MODELS = {
    "wagon": dict(display="Carroça", scale=2.0, width=1.6, height=1.4, parts=[
        P((12, 1, 20), (-6, 4, -10), "oak", plank=2),
        P((1, 3, 20), (-6, 5, -10), "oak", share="side", relief=straps_relief, inflate=0.2),
        P((1, 3, 20), (5, 5, -10), "oak", share="side"),
        P((10, 3, 1), (-5, 5, 9), "oak", share="board", relief=board_relief, inflate=0.2),
        P((10, 3, 1), (-5, 5, -10), "oak", share="board"),
        P((14, 1, 1), (-7, 3, -6), "oak_dark", nails=False),
        P((10, 1, 2), (-5, 3, 4), "oak_dark", nails=False),
        P((10, 1, 3), (-5, 8, 6), "oak", plank=1, nails=False),
        P((10, 2, 1), (-5, 9, 5), "oak", plank=1, nails=False),
        P((10, 1, 3), (-5, 6, 10), "oak_dark", plank=1, nails=False),
        P((1, 4, 1), (6, 7, 7), "iron"),
    ]),
    # The tall canvas (HP: the covered wagon stands 4 blocks high and 2 wide with its wheels):
    # coarser pixels (scale 3) so the high hoops fit the skin; the game draws the whole wagon
    # 1.1 times this size (wagons.rs SIZE).
    "wagon_cover": dict(display="Lona da carroça", scale=3.0, width=2.0, height=4.0, parts=[
        P((1, 9, 11), (-4, 5, -7), "canvas", share="wall", relief=hoops_relief, inflate=0.25),
        P((1, 9, 11), (3, 5, -7), "canvas", share="wall"),
        P((1, 3, 11), (-3, 14, -7), "canvas", share="shoulder", relief=hoops_relief, inflate=0.25),
        P((1, 3, 11), (2, 14, -7), "canvas", share="shoulder"),
        P((4, 2, 11), (-2, 17, -7), "canvas", relief=hoops_relief, inflate=0.25),
    ]),
    # What you see inside from the back, and the lantern at the front.
    "wagon_cargo": dict(display="Carga da carroça", scale=2.0, width=1.6, height=1.9, parts=[
        P((5, 4, 5), (-5, 5, -9), "crate"),
        P((4, 5, 4), (1, 5, -9), "barrel"),
        P((4, 2, 6), (0, 5, -3), "burlap"),
        P((2, 3, 2), (-8, 10, 5), "lantern"),
        P((2, 1, 1), (-7, 13, 5), "iron"),
    ]),
    "wagon_hay": dict(display="Feno da carroça", scale=2.0, width=1.4, height=1.5, parts=[
        P((10, 4, 18), (-5, 5, -9), "hay", relief=tufts_relief, inflate=0.35),
        P((8, 2, 14), (-4, 9, -7), "hay", relief=tufts_relief, inflate=0.35),
        P((1, 1, 14), (2, 11, -8), "oak", plank=1, nails=False),
        P((3, 1, 2), (1, 11, 6), "iron"),
    ]),
    "wagon_shafts": dict(display="Varais", scale=2.0, width=1.2, height=0.8, parts=[
        P((14, 1, 1), (-7, 3, 0), "oak_dark", nails=False, share="axle"),
        P((1, 1, 16), (-4, 4, 1), "oak", plank=1, nails=False, share="shaft"),
        P((1, 1, 16), (3, 4, 1), "oak", plank=1, nails=False, share="shaft"),
        P((8, 1, 1), (-4, 4, 4), "oak_dark", nails=False),
        P((2, 2, 2), (-1, 3, 0), "iron"),
    ]),
    "wagon_pole": dict(display="Cabeçalho", scale=2.0, width=1.2, height=0.8, parts=[
        P((14, 1, 1), (-7, 3, 0), "oak_dark", nails=False),
        P((2, 1, 22), (-1, 4, 1), "oak", plank=1, nails=False),
        P((12, 1, 1), (-6, 4, 3), "oak_dark", nails=False),
        P((16, 1, 1), (-8, 5, 21), "oak", plank=1, nails=False),
        P((1, 3, 1), (-6, 2, 21), "oak_dark", nails=False, share="bow"),
        P((1, 3, 1), (-3, 2, 21), "oak_dark", nails=False, share="bow"),
        P((1, 3, 1), (2, 2, 21), "oak_dark", nails=False, share="bow"),
        P((1, 3, 1), (5, 2, 21), "oak_dark", nails=False, share="bow"),
        P((2, 2, 2), (-1, 3, 0), "iron"),
    ]),
    "wagon_wheel": dict(display="Roda", scale=0.5, width=1.0, height=1.0, parts=[
        P((0, 28, 28), (0, -14, -14), "clear", relief=wheel_relief, inflate=2.0, at=(0, 0)),
        P((8, 6, 6), (-4, -3, -3), "oak_dark", nails=False, at=(0, 0)),
    ]),
    "harness": dict(display="Arreio", scale=1.0, width=1.0, height=1.0, parts=[
        P((12, 12, 2), (-6, 10, -3), "leather"),
        P((6, 4, 8), (-3, 17, 7), "leather", relief=collar_relief, inflate=0.3),
    ]),
}

# The draft ox: a big working animal with a hump, wide horns and a dewlap. It pulls the
# wagons in pairs under a yoke.
OX_MODEL = dict(display="Boi", scale=1.1, width=1.0, height=1.6, parts=[
    P((12, 12, 20), (-6, 12, -10), "ox_body", relief=ox_fur_relief, inflate=0.4),
    P((8, 8, 7), (-4, 15, 9), "ox_head", anim="Head", pivot=(0, 19, 10)),
    P((6, 4, 2), (-3, 15, 16), "ox_muzzle", anim="Head", pivot=(0, 19, 10)),
    P((5, 2, 2), (-9, 21, 11), "horn", anim="Head", pivot=(0, 19, 10), share="horn"),
    P((5, 2, 2), (4, 21, 11), "horn", anim="Head", pivot=(0, 19, 10), share="horn"),
    P((2, 3, 2), (-10, 23, 11), "horn", anim="Head", pivot=(0, 19, 10), share="tip"),
    P((2, 3, 2), (8, 23, 11), "horn", anim="Head", pivot=(0, 19, 10), share="tip"),
    P((6, 4, 6), (-3, 24, 3), "ox_body"),
    P((2, 5, 6), (-1, 10, 8), "ox_body"),
    P((4, 12, 4), (-6, 0, -9), "ox_leg", anim="Leg(0)", pivot=(-4, 12, -7), share="leg"),
    P((4, 12, 4), (2, 0, -9), "ox_leg", anim="Leg(1)", pivot=(4, 12, -7), share="leg"),
    P((4, 12, 4), (-6, 0, 5), "ox_leg", anim="Leg(1)", pivot=(-4, 12, 7), share="leg"),
    P((4, 12, 4), (2, 0, 5), "ox_leg", anim="Leg(0)", pivot=(4, 12, 7), share="leg"),
    P((1, 10, 1), (0, 13, -11), "ox_tail"),
])


def ron_model(name, m, extra=""):
    # Parts sharing an unwrap share its relief layer too.
    for p in m["parts"]:
        for q in m["parts"]:
            if p.get("share") and q.get("share") == p.get("share") and "relief" in q and "relief" not in p:
                p["relief"], p["inflate"] = q["relief"], q["inflate"]
    parts = []
    layers = []
    for k, p in enumerate(m["parts"]):
        s = f"            (size: {tuple(p['size'])}, origin: {tuple(p['origin'])}, uv: {tuple(p['uv'])}"
        if "anim" in p:
            s += f", anim: {p['anim']}, pivot: {tuple(p['pivot'])}"
        parts.append(s + "),")
        if "relief" in p:
            layers.append(f"(part: {k}, inflate: {p['inflate']})")
    lines = [
        "    (",
        f'        name: "{name}", display_name: "{m["display"]}", kind: Passive, health: {m.get("health", 6)}, speed: {m.get("speed", 0.0)},{extra}',
        f'        width: {m["width"]}, height: {m["height"]}, spawn: Never, scale: {m["scale"]},',
        "        parts: [",
        *parts,
        "        ],",
        "        layers: [" + ", ".join(layers) + "],",
        "    ),",
    ]
    return "\n".join(lines)


def main():
    out = [BEGIN,
           "    // Not creatures: the parts of the wagons and carts (crates/game/src/wagons.rs), and the",
           "    // draft ox that pulls them. Generated by tools/gen_wagons.py: edit that script."]
    for name, m in MODELS.items():
        pack(m["parts"])
        sk = Skin()
        done = set()
        for p in m["parts"]:
            key = p.get("share")
            if key and key in done:
                continue
            done.add(key)
            paint_part(sk, p, name)
        sk.save(name)
        out.append(ron_model(name, m))
    pack(OX_MODEL["parts"])
    sk = Skin()
    done = set()
    for p in OX_MODEL["parts"]:
        key = p.get("share")
        if key and key in done:
            continue
        done.add(key)
        paint_part(sk, p, "ox")
    sk.save("ox")
    OX_MODEL["health"] = 30
    OX_MODEL["speed"] = 1.6
    out.append(ron_model("ox", OX_MODEL, ' drops: [(item: "beef", min: 2, max: 4), (item: "leather", min: 1, max: 3)], xp: 3,'))
    out.append(END)
    block = "\n".join(out)
    text = open(MOBS, encoding="utf-8").read()
    if BEGIN in text:
        text = re.sub(re.escape(BEGIN) + r".*?" + re.escape(END), lambda _: block, text, flags=re.S)
    else:
        # Before the minecart, with the other models that are not creatures.
        anchor = "    // Not a creature: the minecart's model"
        k = text.index(anchor)
        text = text[:k] + block + "\n" + text[k:]
    open(MOBS, "w", encoding="utf-8").write(text)
    print("wrote", len(MODELS) + 1, "models and skins")


if __name__ == "__main__":
    main()
