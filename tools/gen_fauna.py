#!/usr/bin/env python3
"""Real animals with their own anatomy. Many animals were first made as recolours of another
mob's boxes (the zebra on the deer, the elephant on the cow, the lion on the bear, the frog on the
rabbit); this script gives each of them a body of its own, built from how the real animal looks,
with relief layers for manes, wool and fur.

Writes packs/vanilla/fauna/models.ron, which the game lays over the mob lists after loading them
(MobRegistry::apply_models: only the boxes, layers, animation hints and skin change; stats and
behaviour stay where they are defined), and one 64x128 skin per model,
packs/vanilla/textures/entity/fauna_<name>.png, grown to 4x like every other skin. The other mob
generators can run again without undoing this; run this one after them.

    python3 tools/gen_fauna.py                   # models.ron + skins
    python3 tools/mob_preview.py out.png zebra   # look at one

Everything is drawn here from short palettes and noise.
"""

import math
import os
import sys
import zlib

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "tex64"))
from gen_mob_skins import face_rects, fbm, hash01, hexrgb, local_pos, mix  # noqa: E402

ROOT = os.path.dirname(HERE)
PACK = os.path.join(ROOT, "packs", "vanilla")
OUT_RON = os.path.join(PACK, "fauna", "models.ron")
ENTITY = os.path.join(PACK, "textures", "entity")
SRC16 = os.path.join(HERE, "tex64", "src16", "entity")
W, H = 64, 64


# ---------------------------------------------------------------- colours


def pal(*hexes):
    return [hexrgb(h) for h in hexes]


def ramp(p, t):
    """Smooth pick along a dark-to-light palette."""
    t = max(0.0, min(0.999, t))
    k = t * (len(p) - 1)
    i = int(k)
    return mix(p[i], p[min(i + 1, len(p) - 1)], k - i)


def dim(rgb, k):
    return tuple(max(0, min(255, int(round(c * k)))) for c in rgb)


EYE = (18, 14, 14)
SHINE = (236, 232, 222)


# ---------------------------------------------------------------- painting context


class C:
    """One face pixel: face, (i, j) in the face, local centre (lx, ly, lz) in the box of size
    (w, h, d), model-space centre (gx, gy, gz), and noise."""

    __slots__ = ("face", "i", "j", "fw", "fh", "lx", "ly", "lz", "w", "h", "d", "gx", "gy", "gz", "seed", "jit", "n", "layer")

    def side(self):
        return self.face in ("left", "right")

    def vert(self):
        return self.face not in ("top", "bottom")

    def up(self):
        """0 at the bottom of the box, 1 at its top."""
        return self.ly / self.h

    def fwd(self):
        """0 at the back of the box, 1 at its front."""
        return self.lz / self.d

    def out(self):
        """-1 at the box's right (-X) side .. 1 at its left (+X)."""
        return (self.lx / self.w) * 2 - 1


def coat(c, p, scale=4.0, k=0.55, base=0.35):
    """Fur or skin: soft noise, a little lighter on top and darker underneath, like the rest."""
    t = base + k * (fbm(c.seed, c.gx, c.gy, c.gz, scale) - 0.5) * 2 * 0.5 + 0.18 * (c.jit - 0.5)
    if c.face == "top":
        t += 0.12
    elif c.face == "bottom":
        t -= 0.18
    elif c.h > 3:
        t += 0.1 * (c.up() - 0.5)
    return ramp(p, t + 0.25)


def hair(c, p, streak=1.0):
    """Long hair (manes, tails): streaks running down the strands."""
    s = hash01(c.seed + 5, int(c.gx * 2) if c.face in ("front", "back") else int(c.gz * 2), int(c.gy / 3)) - 0.5
    t = 0.45 + s * 0.5 * streak + 0.2 * (c.jit - 0.5)
    if c.face == "top":
        t += 0.1
    return ramp(p, t)


def scales(c, p):
    a = (int(c.gx + 64) + int(c.gy + 64)) % 2
    return ramp(p, 0.3 + 0.35 * a + 0.2 * (c.jit - 0.5) + 0.2 * (fbm(c.seed, c.gx, c.gy, c.gz, 3.0) - 0.5))


def feathers(c, p):
    row = int((c.gz + 64) / 2) if c.face in ("top", "left", "right") else int((c.gy + 64) / 2)
    t = 0.35 + 0.3 * ((row % 2) == 0) + 0.25 * (c.jit - 0.5) + 0.2 * (fbm(c.seed, c.gx, c.gy, c.gz, 3.0) - 0.5)
    if c.face == "bottom":
        t -= 0.15
    return ramp(p, t)


def wrinkles(c, p, every=2):
    """Thick grey skin: fine horizontal creases and a few deep folds."""
    t = 0.45 + 0.35 * (fbm(c.seed, c.gx, c.gy * 1.6, c.gz, 3.0) - 0.5) + 0.15 * (c.jit - 0.5)
    if c.vert() and int(c.gy * 2 + 64) % (every * 2) == 0:
        t -= 0.18
    if c.face == "top":
        t += 0.08
    elif c.face == "bottom":
        t -= 0.15
    return ramp(p, t)


def hoof(c, p, rows=2, col=None):
    """Bottom rows of a leg as hoof or paw."""
    if c.face == "bottom" or (c.vert() and c.ly < rows):
        return col or ramp(p, 0.2 + 0.4 * c.jit)
    return None


# ---------------------------------------------------------------- models


class Look:
    def __init__(self, key, paint, relief=None):
        self.key = key
        self.paint = paint      # C -> rgb or None (see-through)
        self.relief = relief    # C -> rgb or None, the 3D layer


class Model:
    def __init__(self, name, skin=None, scale=None):
        self.name = name
        self.skin = skin or "fauna_" + name
        self.scale = scale
        self.parts = []
        self.layers = []
        self.motion = {}

    def box(self, size, origin, look, anim=None, pivot=(0, 0, 0), layer=None):
        self.parts.append(dict(size=tuple(int(v) for v in size), origin=tuple(int(v) for v in origin), look=look, anim=anim, pivot=tuple(int(v) for v in pivot)))
        i = len(self.parts) - 1
        if layer is not None:
            self.layers.append((i, layer))
        return i

    def pack(self):
        """First fit on a grid, biggest unwraps first; same size and look share pixels."""
        used = np.zeros((H, W), bool)
        seen = {}
        order = sorted(self.parts, key=lambda p: -(2 * (p["size"][0] + p["size"][2]) * (p["size"][2] + p["size"][1])))
        for p in order:
            w, h, d = p["size"]
            key = (p["size"], p["look"].key)
            if key in seen:
                p["uv"] = seen[key]
                continue
            pw, ph = 2 * (w + d), d + h
            spot = None
            for y in range(0, H - ph + 1):
                for x in range(0, W - pw + 1):
                    if not used[y:y + ph, x:x + pw].any():
                        spot = (x, y)
                        break
                if spot:
                    break
            if spot is None:
                raise AssertionError("skin full: " + self.name)
            x, y = spot
            used[y:y + ph, x:x + pw] = True
            p["uv"] = spot
            seen[key] = spot

    def render(self):
        self.pack()
        img = Image.new("RGBA", (64, 128), (0, 0, 0, 0))
        px = img.load()
        seed = zlib.crc32(self.name.encode())
        layered = {i for i, _ in self.layers}
        base_done, relief_done = set(), set()
        for k, p in enumerate(self.parts):
            do_base = p["uv"] not in base_done
            do_relief = k in layered and p["look"].relief and p["uv"] not in relief_done
            if not (do_base or do_relief):
                continue
            base_done.add(p["uv"])
            if do_relief:
                relief_done.add(p["uv"])
            for face, (x0, y0, fw, fh) in face_rects(p["size"], p["uv"]).items():
                for j in range(fh):
                    for i in range(fw):
                        c = C()
                        c.face, c.i, c.j, c.fw, c.fh = face, i, j, fw, fh
                        c.lx, c.ly, c.lz = local_pos(face, i, j, p["size"])
                        c.w, c.h, c.d = p["size"]
                        o = p["origin"]
                        c.gx, c.gy, c.gz = o[0] + c.lx, o[1] + c.ly, o[2] + c.lz
                        c.seed = seed
                        c.jit = hash01(seed, int(c.gx * 4), int(c.gy * 4), int(c.gz * 4), hash(face) & 0xFF)
                        c.layer = False
                        rgb = p["look"].paint(c) if do_base else None
                        if rgb is not None and 0 <= x0 + i < 64 and 0 <= y0 + j < 64:
                            px[x0 + i, y0 + j] = (*rgb, 255)
                        if do_relief:
                            c.layer = True
                            rgb = p["look"].relief(c)
                            if rgb is not None:
                                px[x0 + i, y0 + j + 64] = (*rgb, 255)
        return img

    def ron(self):
        out = f'    (\n        name: "{self.name}", skin: "{self.skin}",'
        if self.scale:
            out += f" scale: Some({self.scale}),"
        out += "\n        parts: [\n"
        for p in self.parts:
            s = f"            (size: {p['size']}, origin: {p['origin']}, uv: {p['uv']}"
            if p["anim"]:
                s += f", anim: {p['anim']}, pivot: {p['pivot']}"
            out += s + "),\n"
        out += "        ],\n"
        if self.layers:
            out += "        layers: [" + ", ".join(f"(part: {a}, inflate: {b})" for a, b in self.layers) + "],\n"
        if self.motion:
            fs = []
            for k, v in self.motion.items():
                if k in ("style",):
                    fs.append(f'{k}: Some("{v}")')
                elif k in ("heavy", "wag", "bounce", "flap", "waddle"):
                    fs.append(f"{k}: Some({float(v)})")
                elif k == "graze":
                    fs.append(f"{k}: Some({'true' if v else 'false'})")
                elif k == "follow":
                    fs.append("follow: [" + ", ".join(f"({a}, {b})" for a, b in v) + "]")
                else:
                    fs.append(f"{k}: [" + ", ".join(str(x) for x in v) + "]")
            out += "        motion: (" + ", ".join(fs) + "),\n"
        return out + "    ),\n"


MODELS = []


def add(m):
    assert len(m.parts) <= 15, (m.name, len(m.parts))
    MODELS.append(m)
    return m


def legs4(m, look, size, xs, zs, top=None, pivot_y=None):
    """Four legs: front pair at zs[1], hind pair at zs[0], diagonal pairs swing together."""
    w, h, d = size
    top = h if top is None else top
    out = []
    for k, (x, z) in enumerate([(xs[0], zs[0]), (xs[1], zs[0]), (xs[0], zs[1]), (xs[1], zs[1])]):
        swing = (k + (k // 2)) % 2
        out.append(m.box(size, (x, 0, z), look, f"Leg({swing})", (x + w // 2, pivot_y or top, z + d // 2)))
    return out


# ================================================================ the animals

# ---------------------------------------------------------------- equines: zebra

ZW = pal("#d8d4c8", "#e6e2d6", "#f0ede4", "#f8f6f0")   # zebra white
ZB = pal("#0e0e10", "#18181c", "#222226", "#2c2c30")   # zebra black
ZMUZ = pal("#141416", "#222226", "#303034")


def zebra_stripe(c, freq, warp, along):
    """True where a black stripe is. `along` is the coordinate the stripes count along."""
    w = (fbm(c.seed + 41, c.gx * 0.5, c.gy * 0.5, c.gz * 0.5, 6.0) - 0.5) * warp
    v = math.sin((along + w) * freq)
    return v > 0.15


def zebra_body(c):
    if c.face == "bottom" and abs(c.gx) < c.w / 2 - 1:
        return coat(c, ZW, k=0.3)   # pale belly
    # Body stripes run up and down, bending back over the rump.
    along = c.gz + (c.gy - 15) * (0.6 if c.gz < -3 else 0.15)
    if c.face == "top":
        along = c.gz
    if zebra_stripe(c, 1.15, 5.0, along):
        return coat(c, ZB, k=0.25)
    return coat(c, ZW, k=0.3)


def zebra_neck(c):
    if zebra_stripe(c, 1.25, 2.5, c.gy):
        return coat(c, ZB, k=0.25)
    return coat(c, ZW, k=0.3)


def zebra_head(c):
    # Eyes on the sides, a dark muzzle at the front, thin stripes down the face.
    if c.side() and 2.5 <= c.lz <= 4 and c.h - 2.5 <= c.ly < c.h - 1:
        return EYE if c.lz > 3 else (60, 44, 34)
    if c.face == "front" and c.ly < c.h - 2:
        return coat(c, ZMUZ, k=0.2)
    along = c.gx * 2.2 if c.face in ("top", "front") else c.gy + c.gz * 0.3
    if math.sin((along + (fbm(c.seed + 41, c.gx, c.gy, c.gz, 6.0) - 0.5)) * 1.9) > 0.5:
        return coat(c, ZB, k=0.2)
    return coat(c, ZW, k=0.25)


def zebra_muzzle(c):
    if c.face == "front" and c.ly > c.h - 2 and c.lx in (0.5, c.w - 0.5):
        return (6, 6, 8)   # nostrils
    if c.face == "front" and c.ly < 1:
        return (40, 36, 36)
    return coat(c, ZMUZ, k=0.25)


def zebra_ear(c):
    if c.face == "front":
        return coat(c, ZW, k=0.2) if c.ly < c.h - 1 else ZB[1]
    if c.ly > c.h - 1.2:
        return ZB[1]
    return coat(c, ZW, k=0.2) if int(c.ly) % 2 else coat(c, ZB, k=0.2)


def zebra_mane(c):
    # Short, standing straight up: black and white bands, dark tips.
    if c.face == "top" or c.lz > c.d - 0.6 and c.face != "bottom":
        return ZB[2]
    return ZB[1] if zebra_stripe(c, 1.25, 2.5, c.gy) else ZW[1]


def zebra_mane_relief(c):
    if c.face == "top" or (c.side() and c.lz < 1):
        return ZB[0] if (c.jit > 0.35) else None
    return None


def zebra_leg(c):
    h = hoof(c, ZB, rows=1.5, col=(20, 18, 18))
    if h:
        return h
    if zebra_stripe(c, 1.5, 1.0, c.gy):
        return coat(c, ZB, k=0.2)
    return coat(c, ZW, k=0.25)


def zebra_tail(c):
    return ZB[1] if zebra_stripe(c, 1.6, 0.5, c.gy) else ZW[1]


def tuft(p):
    return Look("tuft" + str(id(p)), lambda c: hair(c, p))


def build_zebra():
    m = Model("zebra")
    head = m.box((6, 6, 9), (-3, 24, 9), Look("zhead", zebra_head), "Head", (0, 20, 8))
    m.box((4, 9, 5), (-2, 17, 6), Look("zneck", zebra_neck), "Head", (0, 20, 8))
    m.box((4, 4, 3), (-2, 24, 18), Look("zmuz", zebra_muzzle), "Head", (0, 20, 8))
    m.box((2, 4, 1), (-3, 30, 11), Look("zear", zebra_ear), "Head", (0, 20, 8))
    m.box((2, 4, 1), (1, 30, 11), Look("zear", zebra_ear), "Head", (0, 20, 8))
    m.box((2, 13, 2), (-1, 18, 4), Look("zmane", zebra_mane, zebra_mane_relief), "Head", (0, 20, 8), layer=0.5)
    m.box((10, 9, 18), (-5, 11, -9), Look("zbody", zebra_body))
    tail = m.box((2, 8, 2), (-1, 11, -11), Look("ztail", zebra_tail))
    tip = m.box((2, 4, 2), (-1, 7, -11), tuft(ZB))
    legs4(m, Look("zleg", zebra_leg), (3, 11, 3), (-5, 2), (-8, 5))
    m.motion = dict(follow=[(tip, tail)], graze=True)
    _ = head
    return add(m)


# ---------------------------------------------------------------- elephant

EL = pal("#4e4a4c", "#5e5a5c", "#6e6a6a", "#7e7a7a", "#8e8a88")
EL_EAR = pal("#7a6a6a", "#8c7a78", "#9c8886")
IVORY = pal("#bdb39a", "#d6ceb4", "#ebe4cf", "#f6f1e2")
NAIL = (196, 188, 168)


def el_body(c):
    return wrinkles(c, EL)


def el_head(c):
    if c.face == "front" and 2 <= c.ly <= 3.5 and (abs(c.lx - c.w / 2) > 2.5) and abs(c.lx - c.w / 2) < 4:
        return EYE if c.ly < 3 else (40, 34, 32)
    if c.side() and c.lz > c.d - 2.5 and 4 <= c.ly <= 5.5:
        return EYE   # eye seen from the side
    if c.face == "top" and c.lz < 3:
        return wrinkles(c, EL[1:], every=3)
    return wrinkles(c, EL, every=3)


def el_trunk(c):
    # Rings of wrinkles round the trunk, the tip pink-grey with two nostrils.
    t = wrinkles(c, EL[1:], every=1)
    if c.vert() and int(c.gy * 2) % 3 == 0:
        t = dim(t, 0.78)
    if c.face == "bottom" and c.h <= 5 and c.w <= 2:
        return (60, 46, 46) if c.lz > 0.8 else (92, 76, 74)
    return t


def el_tusk(c):
    return ramp(IVORY, 0.35 + 0.5 * c.fwd() + 0.1 * (c.jit - 0.5))


def el_ear(c):
    # Flat ear: grey outside, pinker veined inside, a darker ragged rim.
    edge = c.ly < 1 or c.lz < 1 or c.ly > c.h - 1
    if edge:
        return ramp(EL, 0.15 + 0.2 * c.jit)
    inner = (c.face == "right" and c.gx > 0) or (c.face == "left" and c.gx < 0)
    if inner:
        vein = abs(fbm(c.seed + 9, c.gx, c.gy, c.gz, 2.2) - 0.5) < 0.04
        if c.lz > c.d - 3 and c.ly > 2:
            return dim(ramp(EL_EAR, 0.4 + 0.3 * c.jit), 0.85) if vein else ramp(EL_EAR, 0.45 + 0.3 * c.jit)
        return wrinkles(c, EL)
    return wrinkles(c, EL)


def el_leg(c):
    if c.face == "bottom":
        return (58, 52, 50)
    if c.ly < 1.5 and c.face in ("front", "left", "right") and int(c.lx if c.face == "front" else c.lz) % 2 == 0:
        return NAIL   # toenails
    return wrinkles(c, EL)


def build_elephant():
    m = Model("elephant", scale=2.0)
    piv = (0, 19, 9)
    head = m.box((10, 10, 7), (-5, 14, 8), Look("ehead", el_head), "Head", piv)
    t1 = m.box((4, 5, 3), (-2, 10, 14), Look("etrunk", el_trunk), "Head", piv)
    t2 = m.box((4, 5, 3), (-2, 5, 14), Look("etrunk", el_trunk), "Head", piv)
    t3 = m.box((2, 5, 2), (-1, 0, 15), Look("etip", el_trunk), "Head", piv)
    m.box((1, 2, 5), (-3, 11, 14), Look("etusk", el_tusk), "Head", piv)
    m.box((1, 2, 5), (2, 11, 14), Look("etusk", el_tusk), "Head", piv)
    el = m.box((1, 12, 9), (-6, 11, 5), Look("eear", el_ear), "Head", piv)
    er = m.box((1, 12, 9), (5, 11, 5), Look("eear", el_ear), "Head", piv)
    m.box((14, 11, 18), (-7, 9, -9), Look("ebody", el_body))
    tail = m.box((1, 7, 1), (0, 10, -10), Look("etail", lambda c: wrinkles(c, EL)))
    tip = m.box((2, 2, 2), (-1, 8, -11), tuft(pal("#1e1a1a", "#2c2626", "#3a3232")))
    legs4(m, Look("eleg", el_leg), (5, 9, 5), (-7, 2), (-8, 3))
    m.motion = dict(trunk=[t1, t2, t3], ears=[el, er], flap=0.75, follow=[(tip, tail)], heavy=1.0, graze=False)
    _ = head
    return add(m)


# ---------------------------------------------------------------- big cats

LION = pal("#8a6630", "#a47c3c", "#bc9450", "#cfaa66", "#dcbd80")
LION_PALE = pal("#d2b88c", "#dcc6a0", "#e8d6b6")
MANE = pal("#3a2212", "#52301a", "#6c4224", "#86562e", "#9c6a3a")
NOSE = (70, 40, 36)


def cat_face(p, pale, nose=NOSE, eye=(150, 120, 40), whiskers=True):
    def f(c):
        if c.face == "front":
            cx = c.lx - c.w / 2
            if c.h - 3.5 <= c.ly < c.h - 2 and 1 <= abs(cx) <= 2.5:
                return EYE if abs(cx) < 1.6 else eye
            if c.ly < 2.5 and abs(cx) < 2:
                return coat(c, pale, k=0.2)
        if c.face == "bottom":
            return coat(c, pale, k=0.2)
        return coat(c, p, k=0.4)
    return f


def cat_muzzle(pale, nose=NOSE):
    def f(c):
        if c.face == "front" and c.ly > c.h - 1.2 and abs(c.lx - c.w / 2) < 1 and c.w > 2:
            return nose
        if c.face == "top" and c.lz > c.d - 0.6 and abs(c.lx - c.w / 2) < 1:
            return nose
        return coat(c, pale, k=0.25)
    return f


def lion_body(c):
    if c.face == "bottom":
        return coat(c, LION_PALE, k=0.25)
    return coat(c, LION, k=0.35)


def lion_mane(c):
    # Darker toward the outside and the back, lighter round the face.
    t = 0.3 + 0.5 * c.fwd() + 0.3 * (fbm(c.seed + 3, c.gx, c.gy, c.gz, 2.0) - 0.5)
    if c.face == "front":
        r = math.hypot(c.lx - c.w / 2, c.ly - c.h / 2) / (c.w / 2)
        t = 0.75 - 0.4 * r
    return ramp(MANE, t + 0.2 * (c.jit - 0.5))


def lion_mane_relief(c):
    # Tufts stand out all round except where the face is.
    if c.face == "front":
        r = max(abs(c.lx - c.w / 2) / (c.w / 2), abs(c.ly - c.h / 2 - 0.5) / (c.h / 2))
        if r < 0.62:
            return None
    if c.face == "bottom" and c.lz > c.d - 2:
        return None
    if fbm(c.seed + 21, c.gx * 1.3, c.gy * 1.3, c.gz * 1.3, 1.6) < 0.42:
        return None
    return ramp(MANE, 0.25 + 0.5 * c.jit)


def lion_leg(p):
    def f(c):
        if c.face == "bottom":
            return ramp(p, 0.15)
        if c.ly < 1.5 and c.face == "front":
            return ramp(p, 0.75) if int(c.lx * 2) % 2 else ramp(p, 0.45)
        return coat(c, p, k=0.35)
    return f


def feline(name, body, head, leg_h, p, pale, mane=False, tail_len=10, tail_tuft=True, scale=None, eye=(150, 120, 40), ears=(2, 2, 1), legw=3, extra=None):
    """A cat-family body: long, low, deep chest, small round head, long tail."""
    m = Model(name, scale=scale)
    bw, bh, bd = body
    hw, hh, hd = head
    hy = leg_h + bh - hh // 2 - 1
    hz = bd // 2 - 1
    piv = (0, hy + hh // 2, hz)
    m.box(head, (-hw // 2, hy, hz), Look(name + "head", cat_face(p, pale, eye=eye)), "Head", piv)
    mw = max(2, hw // 2)
    m.box((mw, 3, 2), (-mw // 2, hy, hz + hd), Look(name + "muz", cat_muzzle(pale)), "Head", piv)
    ew, eh, ed = ears
    m.box(ears, (-hw // 2, hy + hh, hz + 1), Look(name + "ear", lambda c: coat(c, p, k=0.2) if c.face != "front" else ramp(pale, 0.3)), "Head", piv)
    m.box(ears, (hw // 2 - ew, hy + hh, hz + 1), Look(name + "ear", lambda c: coat(c, p, k=0.2) if c.face != "front" else ramp(pale, 0.3)), "Head", piv)
    if mane:
        mw2 = hw + 6
        m.box((mw2, hh + 6, 6), (-mw2 // 2, hy - 3, hz - 4), Look(name + "mane", lion_mane, lion_mane_relief), "Head", piv, layer=1.0)
    m.box(body, (-bw // 2, leg_h, -bd // 2), Look(name + "body", lambda c: coat(c, pale, k=0.25) if c.face == "bottom" else coat(c, p, k=0.35)))
    ty = leg_h + bh - 3
    tail = m.box((1, 1, tail_len), (0, ty, -bd // 2 - tail_len), Look(name + "tail", lambda c: coat(c, p, k=0.3)))
    if tail_tuft:
        tip = m.box((2, 2, 3), (-1, ty - 1, -bd // 2 - tail_len - 2), tuft(MANE))
        m.motion["follow"] = [(tip, tail)]
    lg = Look(name + "leg", lion_leg(p))
    legs4(m, lg, (legw, leg_h, legw), (-bw // 2, bw // 2 - legw), (-bd // 2 + 1, bd // 2 - legw - 1))
    m.motion["tails"] = [tail]
    if extra:
        extra(m)
    return add(m)


# ---------------------------------------------------------------- pigs: boar

BOAR = pal("#2e2018", "#3e2c20", "#4e382a", "#5e4634", "#6e5440")
BOAR_SNOUT = pal("#5a4038", "#6e5048", "#80605a")
TUSK = pal("#cfc4a8", "#e2d9c0", "#f2ecda")
CRIM = pal("#4a0c12", "#6a121a", "#8a1a22", "#a8262c", "#c43a3a")
CRIM_CRUST = pal("#2a0a0c", "#3c1014", "#561a1c")
CRIM_EYE = (250, 210, 70)


def boar_body(p, crust=None):
    def f(c):
        if crust and fbm(c.seed + 13, c.gx, c.gy, c.gz, 2.6) > 0.6:
            return ramp(crust, 0.3 + 0.5 * c.jit)
        t = coat(c, p, k=0.5)
        return t
    return f


def boar_crest_relief(p):
    def f(c):
        if c.face in ("top", "left", "right", "back") and c.jit > 0.25:
            return ramp(p, 0.05 + 0.3 * c.jit)
        return None
    return f


def boar_head(p, eye=EYE, glow=False):
    def f(c):
        if c.side() and c.lz > c.d - 2.2 and c.h - 3 <= c.ly < c.h - 2:
            return eye
        if c.face == "front" and c.h - 3 <= c.ly < c.h - 2 and abs(c.lx - c.w / 2) > 1.5 and abs(c.lx - c.w / 2) < 2.6:
            return eye
        return coat(c, p, k=0.45)
    return f


def boar_snout(sp):
    def f(c):
        if c.face == "front":
            cx = c.lx - c.w / 2
            if 1.5 <= c.ly < 2.5 and abs(cx) < 1:
                return (30, 18, 16)
            return ramp(sp, 0.55 + 0.3 * c.jit)
        return coat(c, sp, k=0.3)
    return f


def build_boar(name="boar", p=BOAR, sp=BOAR_SNOUT, crust=None, eye=EYE, spikes=False):
    m = Model(name)
    piv = (0, 11, 7)
    m.box((6, 7, 6), (-3, 6, 6), Look(name + "head", boar_head(p, eye)), "Head", piv)
    m.box((4, 4, 4), (-2, 6, 12), Look(name + "snout", boar_snout(sp)), "Head", piv)
    m.box((1, 4, 1), (-3, 8, 13), Look(name + "tusk", lambda c: ramp(TUSK, 0.3 + 0.6 * c.up())), "Head", piv)
    m.box((1, 4, 1), (2, 8, 13), Look(name + "tusk", lambda c: ramp(TUSK, 0.3 + 0.6 * c.up())), "Head", piv)
    m.box((2, 3, 1), (-3, 13, 9), Look(name + "ear", lambda c: coat(c, p, k=0.3)), "Head", piv)
    m.box((2, 3, 1), (1, 13, 9), Look(name + "ear", lambda c: coat(c, p, k=0.3)), "Head", piv)
    # A deep chest and shoulders that slope down to a small rump.
    m.box((8, 8, 14), (-4, 6, -7), Look(name + "body", boar_body(p, crust)), layer=0.35)
    m.parts[-1]["look"].relief = lambda c: ramp(p, 0.1 + 0.3 * c.jit) if c.jit > 0.55 and c.face != "bottom" else None
    crest_relief = (lambda c: ramp(CRIM_EYE and pal("#d8c8a0", "#efe3c4"), 0.3 + 0.6 * c.jit) if c.jit > 0.55 else None) if spikes else boar_crest_relief(p)
    m.box((2, 3, 13), (-1, 14, -6), Look(name + "crest", lambda c: ramp(p, 0.1 + 0.3 * c.jit), crest_relief), layer=0.6)
    m.box((1, 4, 1), (0, 9, -8), Look(name + "tail", lambda c: ramp(p, 0.2 + 0.3 * c.jit)))
    legs4(m, Look(name + "leg", lambda c: hoof(c, pal("#1a1410", "#262018"), rows=1) or coat(c, p, k=0.4)), (3, 6, 3), (-4, 1), (-6, 3))
    m.motion = dict(graze=True)
    return add(m)


# ---------------------------------------------------------------- penguin

PENG_BLACK = pal("#121418", "#1a1d22", "#23272e", "#2c3038")
PENG_WHITE = pal("#d8dade", "#e6e8ea", "#f1f2f3", "#fafafa")
PENG_ORANGE = pal("#d06a10", "#e88a1c", "#f4a832")


def peng_body(c):
    # White front and belly, black back and sides, a yellow-orange wash at the top of the chest.
    if c.face == "front" or (c.side() and c.lz > c.d - 1.5 and c.ly < c.h - 2) or c.face == "bottom":
        if c.face == "front" and c.ly > c.h - 2.5 and abs(c.lx - c.w / 2) < c.w / 2 - 1:
            return ramp(PENG_ORANGE, 0.5 + 0.4 * c.jit)
        if c.face == "front" and (c.lx < 1 or c.lx > c.w - 1):
            return coat(c, PENG_BLACK, k=0.2)
        return coat(c, PENG_WHITE, k=0.25)
    return coat(c, PENG_BLACK, k=0.3)


def peng_head(c):
    if c.face == "front":
        cx = c.lx - c.w / 2
        if c.ly < 2.5 and abs(cx) < 2:
            return ramp(PENG_WHITE, 0.6)
        if 3 <= c.ly < 4 and 1 <= abs(cx) < 2:
            return EYE
    if c.side() and c.ly < 3 and c.lz > c.d - 3:
        return ramp(PENG_ORANGE, 0.6 + 0.3 * c.jit)   # ear patch
    return coat(c, PENG_BLACK, k=0.25)


def peng_beak(c):
    if c.lz > c.d - 1.2:
        return (30, 26, 24)
    return ramp(PENG_ORANGE, 0.3 + 0.5 * c.jit) if c.face != "top" else (36, 30, 28)


def build_penguin():
    m = Model("penguin")
    piv = (0, 15, 0)
    m.box((6, 6, 6), (-3, 15, -3), Look("phead", peng_head), "Head", piv)
    m.box((2, 2, 3), (-1, 16, 3), Look("pbeak", peng_beak), "Head", piv)
    m.box((8, 13, 7), (-4, 2, -4), Look("pbody", peng_body))
    m.box((1, 8, 4), (-5, 6, -3), Look("pflip", lambda c: coat(c, PENG_BLACK, k=0.2) if c.face != "right" or c.gx > 0 else coat(c, PENG_WHITE, k=0.2)), "Wing(1)", (-4, 14, -1))
    m.box((1, 8, 4), (4, 6, -3), Look("pflip", lambda c: coat(c, PENG_BLACK, k=0.2)), "Wing(0)", (4, 14, -1))
    m.box((4, 1, 3), (-2, 2, -7), Look("ptail", lambda c: coat(c, PENG_BLACK, k=0.2)))
    foot = Look("pfoot", lambda c: ramp(PENG_ORANGE, 0.2 + 0.4 * c.jit))
    m.box((3, 2, 4), (-4, 0, -1), foot, "Leg(0)", (-2, 2, 0))
    m.box((3, 2, 4), (1, 0, -1), foot, "Leg(1)", (2, 2, 0))
    m.motion = dict(style="biped", waddle=0.22, bounce=0.6)
    return add(m)


# ---------------------------------------------------------------- bears: panda

PANDA_W = pal("#cfcbc0", "#dcd8ce", "#e8e5dc", "#f3f1ea")
PANDA_K = pal("#121212", "#1c1c1c", "#262626", "#303030")


def panda_body(c):
    # Black shoulder band round the front of the body, white elsewhere.
    if c.gz > 2 and c.gz < 7:
        return coat(c, PANDA_K, k=0.25)
    return coat(c, PANDA_W, k=0.3)


def panda_head(c):
    if c.face == "front":
        cx = c.lx - c.w / 2
        # Tear-drop eye patches slanting down and out.
        if 2.5 <= c.ly <= 5.5 and 1 <= abs(cx) <= 3.5 and (abs(cx) - 1) * 0.6 + (5.5 - c.ly) * 0.5 < 2.4:
            if 4 <= c.ly < 5 and 1.6 <= abs(cx) < 2.6:
                return (230, 228, 220) if abs(cx) < 2 else EYE
            return coat(c, PANDA_K, k=0.2)
    return coat(c, PANDA_W, k=0.25)


def build_panda():
    m = Model("panda")
    piv = (0, 18, 10)
    m.box((9, 8, 7), (-5, 14, 9), Look("pdhead", panda_head), "Head", piv)
    m.box((5, 3, 3), (-3, 14, 16), Look("pdsnout", lambda c: (24, 22, 22) if c.face == "front" and c.ly > 1.5 and abs(c.lx - 2.5) < 1 else coat(c, PANDA_W, k=0.2)), "Head", piv)
    m.box((3, 3, 2), (-6, 21, 11), Look("pdear", lambda c: coat(c, PANDA_K, k=0.2)), "Head", piv)
    m.box((3, 3, 2), (3, 21, 11), Look("pdear", lambda c: coat(c, PANDA_K, k=0.2)), "Head", piv)
    m.box((12, 12, 20), (-6, 9, -10), Look("pdbody", panda_body), layer=0.5)
    m.parts[-1]["look"].relief = lambda c: (coat(c, PANDA_K, k=0.2) if 2 < c.gz < 7 else coat(c, PANDA_W, k=0.2)) if c.jit > 0.6 and c.face not in ("bottom",) else None
    m.box((3, 3, 2), (-1, 15, -12), Look("pdtail", lambda c: coat(c, PANDA_W, k=0.2)))
    legs4(m, Look("pdleg", lambda c: coat(c, PANDA_K, k=0.25) if c.face != "bottom" else (40, 34, 30)), (5, 9, 5), (-6, 1), (-9, 4))
    m.motion = dict(heavy=0.8)
    return add(m)


# ---------------------------------------------------------------- goat

GOAT = pal("#a89c88", "#bcb09c", "#cec4b0", "#ddd5c4", "#ebe5d8")
GOAT_HORN = pal("#4a4236", "#5e5446", "#746858", "#8a7e6c")


def goat_head(c):
    if c.side() and c.lz > c.d - 3 and c.h - 3 <= c.ly < c.h - 2:
        return (190, 150, 60) if c.lz < c.d - 2 else EYE   # sideways pupil
    if c.face == "front" and c.ly < 2 and abs(c.lx - c.w / 2) < 1.5:
        return (70, 58, 52)
    return coat(c, GOAT, k=0.35)


def build_goat():
    m = Model("goat")
    piv = (0, 21, 6)
    m.box((5, 6, 8), (-3, 20, 7), Look("ghead", goat_head), "Head", piv)
    m.box((4, 7, 4), (-2, 16, 5), Look("gneck", lambda c: coat(c, GOAT, k=0.3)), "Head", piv)
    m.box((2, 4, 1), (-1, 17, 13), Look("gbeard", lambda c: hair(c, GOAT[:3])), "Head", piv)
    hl = Look("ghorn", lambda c: ramp(GOAT_HORN, 0.25 + 0.5 * c.up() + 0.15 * (int(c.gy + c.gz) % 2)))
    m.box((1, 3, 2), (-2, 26, 9), hl, "Head", piv)
    m.box((1, 3, 2), (1, 26, 9), hl, "Head", piv)
    hl2 = Look("ghorn2", lambda c: ramp(GOAT_HORN, 0.5 + 0.3 * c.fwd() + 0.1 * (int(c.gz) % 2)))
    m.box((1, 2, 4), (-2, 28, 6), hl2, "Head", piv)
    m.box((1, 2, 4), (1, 28, 6), hl2, "Head", piv)
    m.box((3, 1, 2), (-6, 24, 10), Look("gear", lambda c: coat(c, GOAT, k=0.2)), "Head", piv)
    m.box((3, 1, 2), (3, 24, 10), Look("gear", lambda c: coat(c, GOAT, k=0.2)), "Head", piv)
    m.box((8, 8, 14), (-4, 11, -7), Look("gbody", lambda c: coat(c, GOAT, k=0.35)), layer=0.4)
    m.parts[-1]["look"].relief = lambda c: coat(c, GOAT, k=0.3) if c.face in ("bottom", "left", "right") and c.ly < 3 and c.jit > 0.4 else None
    m.box((2, 3, 1), (-1, 17, -8), Look("gtail", lambda c: coat(c, GOAT, k=0.3)))
    legs4(m, Look("gleg", lambda c: hoof(c, pal("#2a2420", "#3a322c"), rows=1.5) or coat(c, GOAT, k=0.3)), (2, 11, 2), (-4, 2), (-6, 4))
    m.motion = dict(graze=True, bounce=1.2)
    return add(m)


# ---------------------------------------------------------------- cats and dogs

def tabby(base, stripe, belly):
    def f(c):
        if c.face == "bottom":
            return coat(c, belly, k=0.2)
        along = c.gz if c.face == "top" or c.side() else c.gy
        if c.h <= c.d and math.sin((along + (fbm(c.seed + 7, c.gx, c.gy, c.gz, 4) - 0.5) * 3) * 1.4) > 0.55:
            return coat(c, stripe, k=0.2)
        return coat(c, base, k=0.3)
    return f


def cat(name, base, stripe, belly, collar=None, eye=(110, 170, 70)):
    m = Model(name, scale=0.85)
    piv = (0, 12, 7)
    head = Look(name + "head", cat_face(base, belly, eye=eye, nose=(150, 90, 90)))
    m.box((6, 5, 5), (-3, 10, 6), head, "Head", piv)
    m.box((2, 2, 1), (-1, 10, 11), Look(name + "muz", cat_muzzle(belly, nose=(170, 100, 100))), "Head", piv)
    ear = Look(name + "ear", lambda c: (200, 140, 140) if c.face == "front" and c.ly < 1 else coat(c, stripe, k=0.2))
    m.box((2, 2, 1), (-3, 15, 7), ear, "Head", piv)
    m.box((2, 2, 1), (1, 15, 7), ear, "Head", piv)
    m.box((4, 5, 12), (-2, 7, -6), Look(name + "body", tabby(base, stripe, belly)))
    if collar:
        m.box((5, 1, 2), (-2, 10, 5), Look(name + "collar", lambda c: collar))
    t1 = m.box((1, 1, 6), (0, 11, -12), Look(name + "tail", lambda c: coat(c, stripe, k=0.2) if int(c.gz) % 2 else coat(c, base, k=0.2)))
    t2 = m.box((1, 5, 1), (0, 11, -13), Look(name + "tail2", lambda c: coat(c, stripe, k=0.2) if int(c.gy) % 2 else coat(c, base, k=0.2)))
    legs4(m, Look(name + "leg", lambda c: coat(c, belly, k=0.2) if c.ly < 1.5 else coat(c, base, k=0.3)), (2, 7, 2), (-2, 0), (-5, 3))
    m.motion = dict(follow=[(t2, t1)], tails=[t1], wag=0.35)
    return add(m)


WOLF = pal("#5a5a5c", "#727274", "#8a8a8a", "#a2a09c", "#b8b6b0")
WOLF_PALE = pal("#c4c0b6", "#d4d0c6", "#e2ded6")
FOX = pal("#8a3a10", "#a44a16", "#bc5c1e", "#d2702a", "#e08a3e")
FOX_PALE = pal("#d8d2c4", "#e6e0d4", "#f2eee6")


def canid(name, body, leg_h, base, pale, muzzle_dark=False, tail_tip=None, scale=None, collar=None, ear_h=3, tail_len=9):
    m = Model(name, scale=scale)
    bw, bh, bd = body
    hy = leg_h + bh - 3
    hz = bd // 2
    piv = (0, hy + 2, hz)

    def head(c):
        if c.face == "front":
            cx = c.lx - c.w / 2
            if c.h - 3 <= c.ly < c.h - 2 and 1 <= abs(cx) <= 2.5:
                return EYE if abs(cx) < 1.6 else (200, 160, 60)
            if c.ly < 2:
                return coat(c, pale, k=0.2)
        if c.side() and c.ly < 2 and c.lz > 2:
            return coat(c, pale, k=0.2)
        return coat(c, base, k=0.4)

    def muz(c):
        if c.face == "front" and c.ly > c.h - 1.5 and abs(c.lx - c.w / 2) < 1:
            return (22, 20, 20)
        if c.face == "top" and c.lz > c.d - 1 and abs(c.lx - c.w / 2) < 1:
            return (22, 20, 20)
        if c.face == "top":
            return coat(c, base, k=0.3)
        return coat(c, pale, k=0.2)

    m.box((6, 6, 5), (-3, hy, hz), Look(name + "head", head), "Head", piv)
    m.box((4, 3, 4), (-2, hy, hz + 5), Look(name + "muz", muz), "Head", piv)
    ear = Look(name + "ear", lambda c: (40, 34, 34) if c.face == "front" else coat(c, base, k=0.2))
    m.box((2, ear_h, 1), (-3, hy + 6, hz + 1), ear, "Head", piv)
    m.box((2, ear_h, 1), (1, hy + 6, hz + 1), ear, "Head", piv)
    m.box(body, (-bw // 2, leg_h, -bd // 2), Look(name + "body", lambda c: coat(c, pale, k=0.2) if c.face == "bottom" else coat(c, base, k=0.4)))
    # A thick ruff of fur round the neck and chest.
    ruff = Look(name + "ruff", lambda c: coat(c, pale if c.face in ("front", "bottom") else base, k=0.3),
                lambda c: coat(c, pale if c.face in ("front", "bottom") else base, k=0.3) if c.jit > 0.4 else None)
    m.box((bw + 2, bh + 1, 5), (-(bw + 2) // 2, leg_h - 1, bd // 2 - 6), ruff, layer=0.6)
    if collar:
        m.box((bw + 3, 2, 2), (-(bw + 3) // 2, hy - 1, hz - 1), Look(name + "collar", lambda c: collar))

    def tail(c):
        if tail_tip and c.lz < 2:
            return coat(c, tail_tip, k=0.2)
        return coat(c, base, k=0.4)
    t = m.box((4, 4, tail_len), (-2, leg_h + bh - 5, -bd // 2 - tail_len + 1), Look(name + "tail", tail, lambda c: tail(c) if c.jit > 0.45 else None), layer=0.5)
    legs4(m, Look(name + "leg", lambda c: (40, 34, 30) if c.face == "bottom" else coat(c, base if c.ly > 3 else pale, k=0.3)), (2, leg_h, 2), (-bw // 2, bw // 2 - 2), (-bd // 2 + 1, bd // 2 - 3))
    m.motion = dict(tails=[t])
    return add(m)


# ---------------------------------------------------------------- camel

CAMEL = pal("#8e6a3c", "#a47e4c", "#b8925c", "#c8a46e", "#d6b682")


def build_camel():
    m = Model("camel", scale=1.5)
    piv = (0, 22, 8)

    def head(c):
        if c.side() and c.lz > c.d - 5 and c.lz < c.d - 3.5 and c.h - 2.5 <= c.ly < c.h - 1.5:
            return EYE
        if c.face == "front" and c.ly < 2.5 and abs(c.lx - c.w / 2) > 0.8 and abs(c.lx - c.w / 2) < 2:
            return (60, 40, 26)
        return coat(c, CAMEL, k=0.35)
    m.box((5, 5, 8), (-3, 29, 13), Look("chead", head), "Head", piv)
    m.box((4, 4, 6), (-2, 19, 8), Look("cneck", lambda c: coat(c, CAMEL, k=0.35)), "Head", piv)
    m.box((4, 10, 4), (-2, 21, 12), Look("cneck2", lambda c: coat(c, CAMEL, k=0.35)), "Head", piv)
    ear = Look("cear", lambda c: coat(c, CAMEL, k=0.2))
    m.box((1, 2, 1), (-3, 34, 15), ear, "Head", piv)
    m.box((1, 2, 1), (1, 34, 15), ear, "Head", piv)
    m.box((10, 9, 18), (-5, 15, -9), Look("cbody", lambda c: coat(c, CAMEL, k=0.35)))
    hump = Look("chump", lambda c: coat(c, CAMEL, k=0.35), lambda c: coat(c, CAMEL[:3], k=0.3) if c.face == "top" and c.jit > 0.35 else None)
    m.box((8, 6, 8), (-4, 24, -4), hump, layer=0.5)
    m.box((2, 7, 1), (-1, 13, -10), Look("ctail", lambda c: hair(c, CAMEL[:3])))
    # Long thin legs with knobbly knees and wide padded feet.
    legs4(m, Look("cleg", lambda c: (70, 52, 36) if c.face == "bottom" else (coat(c, CAMEL[:3], k=0.3) if 7 <= c.ly < 9 else coat(c, CAMEL, k=0.3))), (3, 15, 3), (-5, 2), (-8, 5))
    m.motion = dict(graze=True, heavy=0.6)
    return add(m)


# ---------------------------------------------------------------- skeleton horse

BONE = pal("#8e8a7e", "#a8a498", "#c2beb2", "#d8d4c8", "#e8e6dc")


def bone(c):
    return ramp(BONE, 0.35 + 0.45 * (fbm(c.seed, c.gx, c.gy, c.gz, 3.0) - 0.5) * 2 * 0.5 + 0.2 * c.jit + (0.12 if c.face == "top" else 0))


def ribs(c):
    """A cage of ribs: bars across the sides with gaps between, a breastbone underneath."""
    if c.face == "bottom":
        return bone(c) if abs(c.lx - c.w / 2) < 1.5 or int(c.lz) % 3 == 0 else None
    if c.face == "top":
        return bone(c) if abs(c.lx - c.w / 2) < 1 or int(c.lz) % 3 == 0 else None
    if c.side():
        return bone(c) if int(c.lz) % 3 == 0 else None
    return bone(c) if c.lx < 1 or c.lx > c.w - 1 or c.ly < 1 or c.ly > c.h - 1 else None


def skull(c):
    if c.side() and c.lz > c.d - 5 and c.lz < c.d - 2.5 and c.h - 3 <= c.ly < c.h - 1:
        return (14, 12, 12)   # eye socket
    if c.face == "front" and c.ly > c.h - 2 and abs(c.lx - c.w / 2) < 1.5:
        return (30, 26, 24)
    if c.side() and c.ly < 1.2 and int(c.lz) % 2 == 0:
        return (40, 36, 32)   # teeth gaps
    return bone(c)


def build_skeleton_horse():
    m = Model("skeleton_horse")
    piv = (0, 20, 8)
    m.box((6, 5, 10), (-3, 25, 8), Look("shskull", skull), "Head", piv)
    neck = Look("shneck", lambda c: bone(c) if int(c.gy) % 2 == 0 or abs(c.lx - c.w / 2) < 1 else None)
    m.box((2, 10, 3), (-1, 17, 6), neck, "Head", piv)
    ear = Look("shear", bone)
    m.box((1, 3, 1), (-3, 30, 10), ear, "Head", piv)
    m.box((1, 3, 1), (2, 30, 10), ear, "Head", piv)
    m.box((2, 2, 22), (-1, 20, -11), Look("shspine", lambda c: bone(c) if int(c.gz) % 2 == 0 or c.face in ("top", "bottom") else dim(bone(c), 0.8)))
    m.box((10, 9, 12), (-5, 11, -1), Look("shribs", ribs))
    m.box((8, 6, 6), (-4, 13, -11), Look("shpelvis", lambda c: bone(c) if c.face in ("top", "back") or c.ly > c.h - 2 or c.lz < 1 else None))
    m.box((1, 10, 1), (0, 10, -12), Look("shtail", lambda c: bone(c) if int(c.gy) % 2 else None))

    def leg(c):
        if 5 <= c.ly < 6.5 and c.vert():
            return ramp(BONE, 0.8)   # knee knob
        if c.ly < 1 or c.face == "bottom":
            return ramp(BONE, 0.2)
        return bone(c)
    legs4(m, Look("shleg", leg), (2, 11, 2), (-4, 2), (-9, 6))
    m.motion = dict(graze=False)
    return add(m)


# ---------------------------------------------------------------- bats

def bat(name, fur, wing, ear_inner=(150, 110, 110), eye=(20, 14, 14), scale=None):
    m = Model(name, scale=scale)
    piv = (0, 10, 1)

    def head(c):
        if c.face == "front":
            cx = c.lx - c.w / 2
            if 1.5 <= c.ly < 2.5 and 0.6 < abs(cx) < 1.6:
                return eye
            if c.ly < 1.2 and abs(cx) < 1:
                return dim(fur[0], 0.7)
        return coat(c, fur, k=0.4)
    m.box((4, 4, 4), (-2, 9, -1), Look(name + "head", head), "Head", piv)
    ear = Look(name + "ear", lambda c: ear_inner if c.face == "front" and 0.5 < c.ly < c.h - 0.5 else coat(c, fur[:3], k=0.2))
    m.box((2, 4, 1), (-3, 13, 1), ear, "Head", piv)
    m.box((2, 4, 1), (1, 13, 1), ear, "Head", piv)
    m.box((4, 6, 3), (-2, 3, -2), Look(name + "body", lambda c: coat(c, fur, k=0.4)), layer=0.4)
    m.parts[-1]["look"].relief = lambda c: coat(c, fur, k=0.3) if c.jit > 0.5 and c.face in ("front", "left", "right", "back") and c.ly > 2 else None

    def membrane(c):
        # Thin skin stretched between finger bones; a scalloped trailing edge.
        x = abs(c.gx) - 2
        if c.face in ("top", "bottom"):
            fingers = int(x) in (3, 6, 9) or c.lz > c.d - 1
            edge = c.lz < 1 + 1.2 * abs(math.sin(x * 1.05))
            if edge and not fingers:
                return None
            if fingers:
                return dim(wing[1], 0.8)
            return ramp(wing, 0.45 + 0.25 * (fbm(c.seed + 2, c.gx, c.gy, c.gz, 2.0) - 0.5) + 0.1 * c.jit)
        return dim(wing[1], 0.8)
    wl = Look(name + "wing", membrane)
    m.box((11, 1, 6), (-13, 7, -2), wl, "Wing(0)", (-2, 7, 1))
    m.box((11, 1, 6), (2, 7, -2), wl, "Wing(1)", (2, 7, 1))
    feet = Look(name + "feet", lambda c: dim(fur[0], 0.8))
    m.box((1, 2, 1), (-2, 1, -1), feet, "Leg(0)", (-2, 3, -1))
    m.box((1, 2, 1), (1, 1, -1), feet, "Leg(1)", (1, 3, -1))
    return add(m)


# ---------------------------------------------------------------- swan

SWAN = pal("#c8c8c4", "#d8d8d4", "#e6e6e2", "#f2f2ee", "#fbfbf8")
SWAN_BILL = pal("#c85a10", "#e07018", "#f08a26")


def build_swan():
    m = Model("swan")
    piv = (0, 8, 4)

    def head(c):
        if c.side() and c.lz > c.d - 2 and 1.5 <= c.ly < 2.5:
            return EYE
        return coat(c, SWAN, k=0.2)

    def bill(c):
        if c.lz < 1.2:
            return (24, 20, 20)   # the black knob at the base
        if c.lz > c.d - 0.8:
            return (40, 30, 24)
        return ramp(SWAN_BILL, 0.35 + 0.4 * c.jit)
    # An S-curved neck: rising from the chest, leaning back, then forward to the head.
    neck = Look("swneck", lambda c: coat(c, SWAN, k=0.2))
    m.box((3, 4, 3), (-1, 7, 4), neck, "Head", piv)
    m.box((3, 5, 3), (-1, 11, 3), neck, "Head", piv)
    m.box((3, 4, 3), (-1, 16, 4), neck, "Head", piv)
    m.box((3, 3, 5), (-1, 19, 5), Look("swhead", head), "Head", piv)
    m.box((2, 2, 3), (-1, 19, 10), Look("swbill", bill), "Head", piv)
    m.box((8, 6, 13), (-4, 2, -7), Look("swbody", lambda c: feathers(c, SWAN)), layer=0.4)
    m.parts[-1]["look"].relief = lambda c: feathers(c, SWAN) if c.face == "top" and c.lz < c.d - 3 and c.jit > 0.35 else None
    wing = Look("swwing", lambda c: feathers(c, SWAN), lambda c: feathers(c, SWAN) if c.jit > 0.5 and c.face != "bottom" else None)
    m.box((2, 4, 10), (-5, 5, -6), wing, "Wing(1)", (-4, 9, 0), layer=0.4)
    m.box((2, 4, 10), (3, 5, -6), wing, "Wing(0)", (4, 9, 0), layer=0.4)
    m.box((4, 3, 3), (-2, 6, -10), Look("swtail", lambda c: feathers(c, SWAN)))
    foot = Look("swfoot", lambda c: (30, 30, 34))
    m.box((2, 2, 3), (-3, 0, -1), foot, "Leg(0)", (-2, 2, 0))
    m.box((2, 2, 3), (1, 0, -1), foot, "Leg(1)", (2, 2, 0))
    m.motion = dict(style="bird")
    return add(m)


# ---------------------------------------------------------------- llama

LLAMA = pal("#b8a684", "#c8b694", "#d6c6a6", "#e2d4b8", "#ece2cc")


def build_llama():
    m = Model("llama", scale=0.95)
    piv = (0, 19, 6)

    def head(c):
        if c.side() and c.lz > c.d - 3 and c.lz < c.d - 1.5 and c.h - 3 <= c.ly < c.h - 2:
            return EYE
        if c.face == "front" and c.ly < 2 and abs(c.lx - c.w / 2) < 1.2:
            return (70, 56, 46)
        return coat(c, LLAMA, k=0.3)
    wool = Look("llwool", lambda c: coat(c, LLAMA, k=0.3), lambda c: coat(c, LLAMA, k=0.35) if c.jit > 0.25 and c.face != "bottom" else None)
    m.box((5, 5, 7), (-3, 30, 6), Look("llhead", head), "Head", piv)
    m.box((4, 13, 4), (-2, 18, 4), wool, "Head", piv, layer=0.8)
    ear = Look("llear", lambda c: coat(c, LLAMA[:3], k=0.2))
    m.box((1, 4, 1), (-3, 35, 7), ear, "Head", piv)
    m.box((1, 4, 1), (1, 35, 7), ear, "Head", piv)
    m.box((10, 9, 16), (-5, 13, -8), Look("llbody", lambda c: coat(c, LLAMA, k=0.3), lambda c: coat(c, LLAMA, k=0.35) if c.jit > 0.2 else None), layer=1.0)
    m.box((3, 4, 2), (-1, 18, -10), Look("lltail", lambda c: coat(c, LLAMA, k=0.3)))
    legs4(m, Look("llleg", lambda c: hoof(c, pal("#3a3028", "#4a3e34"), rows=1) or coat(c, LLAMA, k=0.3)), (3, 13, 3), (-5, 2), (-7, 4))
    m.motion = dict(graze=True)
    return add(m)


# ---------------------------------------------------------------- birds


def bird(name, colors, body=(4, 4, 6), head=(3, 3, 3), leg_h=2, beak=(1, 1, 2), neck=0, tail=(2, 1, 4), crest=None,
         scale=None, head_paint=None, body_paint=None, upright=False, wing_relief=False, leg_w=1, tail_down=False, style=None):
    """A bird: round body, head on top at the front, folded wings along the sides (they open
    when it flies), a fan of tail feathers, thin legs. `colors` holds palettes for back, belly,
    head, wing, tail, beak and legs."""
    m = Model(name, scale=scale)
    bw, bh, bd = body
    hw, hh, hd = head
    back, belly = colors["back"], colors.get("belly", colors["back"])
    hp = colors.get("head", back)
    if upright:
        # Perching birds that sit up (parrots, crows on a branch): a taller body, head right on top.
        hy = leg_h + bh - 1 + neck
        hz = -hd // 2 + 1
    else:
        hy = leg_h + bh - 2 + neck
        hz = bd // 2 - hd + 2
    piv = (0, hy, hz + hd // 2)

    def hpaint(c):
        if head_paint:
            r = head_paint(c)
            if r is not None:
                return r
        if c.side() and c.d - 2 < c.lz < c.d - 1 and c.h - 2 <= c.ly < c.h - 1:
            return EYE
        if c.face == "bottom" or (c.vert() and c.ly < 1 and c.face != "back"):
            return feathers(c, colors.get("throat", belly))
        return feathers(c, hp)
    m.box(head, (-hw // 2, hy, hz), Look(name + "head", hpaint), "Head", piv)
    bk = colors.get("beak", pal("#c08a2a", "#d8a43a"))
    bw_, bh_, bd_ = beak
    m.box(beak, (-bw_ // 2, hy + max(0, hh // 2 - bh_), hz + hd), Look(name + "beak", lambda c: ramp(bk, 0.3 + 0.4 * c.fwd() + 0.2 * c.jit)), "Head", piv)
    if neck:
        m.box((max(2, hw - 2), neck + 2, max(2, hd - 2)), (-max(2, hw - 2) // 2, hy - neck - 1, hz + 1), Look(name + "neck", lambda c: feathers(c, colors.get("neck", hp))), "Head", piv)
    if crest:
        cw, ch, cd = crest
        m.box(crest, (-cw // 2, hy + hh, hz), Look(name + "crest", lambda c: feathers(c, colors.get("crest", hp))), "Head", piv)

    def bpaint(c):
        if body_paint:
            r = body_paint(c)
            if r is not None:
                return r
        if c.face == "bottom" or c.face == "front" or (c.side() and c.ly < c.h * 0.45):
            return feathers(c, belly)
        return feathers(c, back)
    m.box(body, (-bw // 2, leg_h, -bd // 2), Look(name + "body", bpaint, (lambda c: feathers(c, back) if c.face == "top" and c.jit > 0.5 else None) if wing_relief else None), layer=0.3 if wing_relief else None)
    wp = colors.get("wing", back)
    wing = Look(name + "wing", lambda c: feathers(c, wp) if not (c.ly < 1.5 and c.lz < c.d * 0.6) else feathers(c, colors.get("primaries", wp[:2] or wp)))
    m.box((1, bh - 1, bd - 1), (-bw // 2 - 1, leg_h + 1, -bd // 2), wing, "Wing(1)", (-bw // 2, leg_h + bh, 0))
    m.box((1, bh - 1, bd - 1), (bw // 2, leg_h + 1, -bd // 2), wing, "Wing(0)", (bw // 2, leg_h + bh, 0))
    tw, th, td = tail
    tp = colors.get("tail", wp)
    if tail_down:
        m.box((tw, td, th), (-tw // 2, leg_h - td + 2, -bd // 2 - th), Look(name + "tail", lambda c: feathers(c, tp)))
    else:
        m.box(tail, (-tw // 2, leg_h + bh - 2, -bd // 2 - td + 1), Look(name + "tail", lambda c: feathers(c, tp)))
    lp = colors.get("leg", pal("#3a3430", "#4a443e"))
    leg = Look(name + "leg", lambda c: ramp(lp, 0.3 + 0.4 * c.jit))
    m.box((leg_w, leg_h, leg_w), (-1 - leg_w + 1 - (1 if bw > 4 else 0), 0, 0), leg, "Leg(0)", (-1, leg_h, 0))
    m.box((leg_w, leg_h, leg_w), ((1 if bw > 4 else 0), 0, 0), leg, "Leg(1)", (1, leg_h, 0))
    if style:
        m.motion["style"] = style
    return add(m)


def mask(col, rows=(1.2, 2.8)):
    """A dark band through the eyes (cardinals, jays)."""
    def f(c):
        if c.vert() and c.face != "back" and rows[0] <= c.ly < rows[1] and (c.face == "front" or c.lz > c.d - 3):
            if c.side() and c.d - 2 < c.lz < c.d - 1 and c.h - 2 <= c.ly < c.h - 1:
                return None
            return col
        return None
    return f


GRAY = pal("#6a6a6e", "#7e7e82", "#929296", "#a6a6aa")
WHITE_F = pal("#d6d6d2", "#e4e4e0", "#f0f0ec", "#fafaf6")
BLACK_F = pal("#121214", "#1c1c20", "#26262c", "#30303a")


def build_birds():
    bird("cardinal", dict(back=pal("#8a1414", "#a01a1a", "#b42222", "#c82a2a"), belly=pal("#b42222", "#c82a2a", "#d83434"),
                          head=pal("#a01a1a", "#b42222", "#c82a2a"), wing=pal("#701010", "#861616", "#9a1c1c"),
                          beak=pal("#c86a1c", "#e0822a"), crest=pal("#a01a1a", "#c42828")),
         body=(4, 4, 6), crest=(2, 2, 2), head_paint=mask((22, 12, 12)), scale=0.6)
    bird("bluejay", dict(back=pal("#24468e", "#2c54a2", "#3462b4", "#3e70c4"), belly=pal("#c8ccd4", "#d6d9e0", "#e2e5ea"),
                         head=pal("#2c54a2", "#3462b4", "#3e70c4"), wing=pal("#1e3a7a", "#2a50a0", "#e8ecf2"),
                         beak=pal("#1a1a1e", "#2a2a2e"), crest=pal("#2c54a2", "#3e70c4")),
         body=(4, 4, 6), crest=(2, 2, 2), head_paint=mask((20, 20, 26), rows=(0.5, 1.5)), scale=0.6)
    bird("robin", dict(back=pal("#4a4440", "#5a544e", "#6a645e"), belly=pal("#c4561c", "#d66a28", "#e27e36"),
                       head=pal("#262222", "#302c2a", "#3a3634"), beak=pal("#d0a020", "#e8bc30")), scale=0.6)
    bird("sparrow", dict(back=pal("#5a4028", "#6e5034", "#846442", "#987650"), belly=pal("#b8ae9c", "#c8beac", "#d6cebe"),
                         head=pal("#6e6660", "#827a72"), wing=pal("#4a3420", "#6a4c30", "#9a7a54"), throat=pal("#26201c", "#302824"),
                         beak=pal("#5a5048", "#6e645a")), scale=0.6)
    bird("finch", dict(back=pal("#c8a018", "#dcb424", "#ecc838"), belly=pal("#e0c440", "#ecd458", "#f4e070"),
                       head=pal("#c8a018", "#dcb424"), wing=pal("#141414", "#202020", "#e8e8e8"), tail=pal("#141414", "#262626"),
                       beak=pal("#d8b090", "#e8c4a8")), head_paint=lambda c: (20, 20, 20) if c.face == "top" or (c.vert() and c.ly > c.h - 1.2 and c.face != "back") else None, scale=0.6)
    bird("canary", dict(back=pal("#d8b418", "#e8c628", "#f4d63c"), belly=pal("#ecd040", "#f4dc58", "#faea78"),
                        head=pal("#e8c628", "#f4d63c"), wing=pal("#b89a14", "#ccae20", "#dcc030"), beak=pal("#d8a878", "#e8bc90")), scale=0.6)
    bird("crow", dict(back=BLACK_F, belly=BLACK_F, head=BLACK_F, wing=pal("#0e0e14", "#181822", "#24243a"), beak=pal("#141416", "#222226"),
                      leg=pal("#141416", "#202024")), body=(4, 5, 7), head=(4, 4, 4), beak=(2, 2, 3), tail=(2, 1, 5), scale=0.8)
    bird("vulture", dict(back=pal("#1e1a18", "#2a2420", "#362e28"), belly=pal("#2a2420", "#362e28"), head=pal("#b06a5a", "#c47c6a", "#d4907c"),
                         neck=pal("#b06a5a", "#c47c6a"), throat=pal("#b06a5a", "#c47c6a"), wing=pal("#16120f", "#221c18", "#2e2620"),
                         beak=pal("#b8b0a0", "#d0c8b8"), leg=pal("#8a8478", "#9a9488")),
         body=(6, 7, 8), head=(3, 3, 4), beak=(1, 2, 2), neck=3, tail=(4, 1, 4), leg_h=3, scale=1.0, upright=True, wing_relief=True,
         body_paint=lambda c: ramp(pal("#e0dcd4", "#ece8e0"), 0.4 + 0.4 * c.jit) if c.vert() and c.ly > c.h - 1.5 and c.face != "bottom" else None)
    bird("seagull", dict(back=pal("#8a9098", "#9aa0a8", "#aab0b8"), belly=WHITE_F, head=WHITE_F, wing=pal("#8a9098", "#9aa0a8", "#1a1a1e"),
                         tail=WHITE_F, beak=pal("#e0b81c", "#f0cc30"), leg=pal("#d8908a", "#e8a49c")),
         body=(4, 5, 8), head=(4, 4, 4), beak=(1, 1, 3), tail=(4, 1, 3), leg_h=3, scale=0.9)
    bird("hummingbird", dict(back=pal("#1e6a3a", "#24844a", "#2c9a58", "#36b066"), belly=pal("#b8c4b0", "#ccd4c4"),
                             head=pal("#1e6a3a", "#2c9a58"), throat=pal("#a8102a", "#c81c3a", "#e02a4a"), wing=pal("#2a3a32", "#3a4a42"),
                             beak=pal("#141414", "#222222")),
         body=(2, 3, 4), head=(2, 2, 3), beak=(1, 1, 3), tail=(2, 1, 2), leg_h=1, scale=0.5)
    bird("green_jay", dict(back=pal("#2a8a2a", "#36a034", "#44b440"), belly=pal("#c8c840", "#d8d850", "#e6e464"), head=pal("#1e5ac8", "#2a6ee0", "#4a88f0"),
                           throat=pal("#141414", "#222222"), wing=pal("#1e7a24", "#2a902e"), tail=pal("#2a8a2a", "#d8c840"), beak=BLACK_F),
         head_paint=mask((16, 16, 20)), scale=0.7)
    bird("gralha_azul", dict(back=pal("#1e4ab0", "#2a5ccc", "#3a70e0"), belly=pal("#2a5ccc", "#3a70e0"), head=BLACK_F, throat=BLACK_F,
                             wing=pal("#1a40a0", "#2654be", "#3466d4"), tail=pal("#1e4ab0", "#2a5ccc"), beak=BLACK_F, leg=BLACK_F, crest=BLACK_F),
         body=(4, 5, 7), crest=(2, 2, 3), tail=(2, 1, 5))
    bird("asa_branca", dict(back=pal("#7a7480", "#8a8490", "#9a94a0"), belly=pal("#a8a0a8", "#b8b0b8"), head=pal("#8a8490", "#9a94a0"),
                            wing=pal("#6a6470", "#7a7480", "#ecece8"), beak=BLACK_F, leg=pal("#b0646a", "#c4767c")),
         body=(4, 5, 7), head=(3, 3, 4), scale=0.8)
    bird("parrot", dict(back=pal("#a8141c", "#c41e26", "#d82c32"), belly=pal("#c41e26", "#d82c32"), head=pal("#c41e26", "#d82c32", "#e43a3e"),
                        wing=pal("#1e4ab8", "#e8c020", "#2a8a3a"), tail=pal("#a8141c", "#1e4ab8"), beak=pal("#e8e2d6", "#1a1a1a"),
                        leg=pal("#4a4440", "#5a544e")),
         body=(4, 6, 4), head=(4, 4, 4), beak=(2, 3, 2), tail=(2, 1, 7), tail_down=True, upright=True, leg_h=2, scale=0.8,
         head_paint=lambda c: (236, 232, 222) if c.side() and c.lz > c.d - 2 and c.ly < c.h - 1 and not (c.d - 2 < c.lz < c.d - 1 and c.h - 2 <= c.ly) else None)
    bird("emu", dict(back=pal("#3a3026", "#4a3e32", "#5a4c3e", "#6a5a4a"), belly=pal("#4a3e32", "#5a4c3e"), head=pal("#4a5a6a", "#5a6a7a"),
                     neck=pal("#5a6a7a", "#6a7a8a", "#3a3026"), throat=pal("#6a7a8a", "#7a8a9a"), beak=BLACK_F, leg=pal("#5a5a5a", "#6a6a6a")),
         body=(8, 8, 11), head=(3, 3, 4), beak=(1, 1, 2), neck=10, tail=(4, 2, 3), leg_h=12, leg_w=2, scale=1.0, wing_relief=True, style="biped")
    bird("tuiuiu", dict(back=WHITE_F, belly=WHITE_F, head=BLACK_F, neck=pal("#141416", "#1e1e22", "#c42a2a"), throat=pal("#c42a2a", "#d83838"),
                        wing=WHITE_F, tail=WHITE_F, beak=BLACK_F, leg=BLACK_F),
         body=(6, 6, 10), head=(3, 3, 4), beak=(1, 2, 7), neck=9, tail=(4, 1, 3), leg_h=13, scale=1.0, style="biped")


# ---------------------------------------------------------------- savanna and river giants

RHINO = pal("#5a5654", "#6a6664", "#7a7672", "#8a8682", "#9a9690")
HIPPO = pal("#5a4a52", "#6a5862", "#7a6670", "#8a747e", "#9a8490")
HIPPO_PINK = pal("#b07880", "#c48a90", "#d49ca0")
HORN = pal("#6a6052", "#82786a", "#9a9082", "#b0a698")


def build_rhino():
    m = Model("rhino", scale=1.35)
    piv = (0, 13, 9)

    def head(c):
        if c.side() and 3 <= c.lz < 4.5 and c.h - 3 <= c.ly < c.h - 2:
            return EYE
        if c.face == "front" and c.ly < 2.5 and abs(c.lx - c.w / 2) > 1 and abs(c.lx - c.w / 2) < 2.5 and c.ly > 1:
            return (30, 26, 24)
        return wrinkles(c, RHINO, every=3)
    m.box((7, 6, 10), (-4, 8, 9), Look("rhead", head), "Head", piv)
    m.box((2, 6, 2), (-1, 14, 16), Look("rhorn", lambda c: ramp(HORN, 0.3 + 0.6 * c.up())), "Head", piv)
    m.box((2, 3, 2), (-1, 14, 12), Look("rhorn2", lambda c: ramp(HORN, 0.3 + 0.5 * c.up())), "Head", piv)
    ear = Look("rear", lambda c: wrinkles(c, RHINO))
    m.box((2, 3, 1), (-4, 14, 10), ear, "Head", piv)
    m.box((2, 3, 1), (2, 14, 10), ear, "Head", piv)

    def body(c):
        t = wrinkles(c, RHINO, every=3)
        # Folds of armour-like skin at the shoulder and the hip.
        if c.vert() and c.face != "front" and (abs(c.gz - 4) < 0.6 or abs(c.gz + 5) < 0.6):
            return dim(t, 0.75)
        return t
    m.box((12, 10, 20), (-6, 7, -10), Look("rbody", body))
    m.box((1, 6, 1), (0, 10, -11), Look("rtail", lambda c: wrinkles(c, RHINO)))
    legs4(m, Look("rleg", lambda c: (190, 182, 168) if c.ly < 1 and c.face == "front" and int(c.lx) % 2 == 0 else wrinkles(c, RHINO)), (4, 7, 4), (-6, 2), (-9, 5))
    m.motion = dict(heavy=0.9, graze=True)
    return add(m)


def build_hippo():
    m = Model("hippo", scale=1.8)
    piv = (0, 10, 7)

    def head(c):
        if c.face == "top" and c.lz < 3 and c.lz > 1.5 and abs(c.lx - c.w / 2) > 1.5 and abs(c.lx - c.w / 2) < 3:
            return EYE   # eyes high on the head, to see over the water
        return wrinkles(c, HIPPO, every=4)

    def muzzle(c):
        if c.face == "top" and c.lz > c.d - 1.5 and 0.8 < abs(c.lx - c.w / 2) < 2:
            return (40, 28, 32)   # nostrils on top
        if c.face == "front" and c.ly < 2:
            return ramp(HIPPO_PINK, 0.3 + 0.4 * c.jit)   # pink lips
        if c.face == "bottom":
            return ramp(HIPPO_PINK, 0.2 + 0.3 * c.jit)
        return wrinkles(c, HIPPO, every=4)
    m.box((8, 6, 6), (-4, 7, 7), Look("hhead", head), "Head", piv)
    m.box((10, 5, 5), (-5, 6, 13), Look("hmuz", muzzle), "Head", piv)
    m.box((1, 2, 1), (-3, 13, 8), Look("hear", lambda c: wrinkles(c, HIPPO)), "Head", piv)
    m.box((1, 2, 1), (2, 13, 8), Look("hear", lambda c: wrinkles(c, HIPPO)), "Head", piv)
    m.box((2, 2, 1), (-3, 6, 17), Look("htusk", lambda c: ramp(TUSK, 0.5)), "Head", piv)
    m.box((2, 2, 1), (1, 6, 17), Look("htusk", lambda c: ramp(TUSK, 0.5)), "Head", piv)
    m.box((12, 9, 18), (-6, 4, -9), Look("hbody", lambda c: ramp(HIPPO_PINK, 0.3 + 0.3 * c.jit) if c.face == "bottom" else wrinkles(c, HIPPO, every=4)))
    m.box((2, 3, 1), (-1, 9, -10), Look("htail", lambda c: wrinkles(c, HIPPO)))
    legs4(m, Look("hleg", lambda c: wrinkles(c, HIPPO)), (4, 4, 4), (-6, 2), (-8, 4))
    m.motion = dict(heavy=1.0, graze=True)
    return add(m)


# ---------------------------------------------------------------- small creatures

FROG = pal("#2e5a1e", "#3a7026", "#488430", "#56983a", "#66ac46")
FROG_BELLY = pal("#c8c890", "#d6d6a0", "#e2e2b2")


def build_frog():
    m = Model("frog")
    piv = (0, 3, 2)

    def head(c):
        if c.face == "bottom":
            return coat(c, FROG_BELLY, k=0.2)
        if c.face == "front" and c.ly < 1.2:
            return dim(FROG[0], 0.8)   # the wide mouth line
        return coat(c, FROG, k=0.4)
    m.box((6, 3, 4), (-3, 2, 2), Look("fhead", head), "Head", piv)

    def eye(c):
        if c.face in ("front", "left", "right", "top") and c.ly > 0.6:
            return (200, 160, 40) if c.jit > 0.35 or c.face == "top" else EYE
        return coat(c, FROG, k=0.3)
    m.box((2, 2, 2), (-3, 5, 3), Look("feye", eye), "Head", piv)
    m.box((2, 2, 2), (1, 5, 3), Look("feye", eye), "Head", piv)

    def body(c):
        if c.face == "bottom" or (c.face == "front" and c.ly < 1.5):
            return coat(c, FROG_BELLY, k=0.2)
        if c.face == "top" and fbm(c.seed + 4, c.gx, c.gy, c.gz, 1.5) > 0.6:
            return ramp(FROG, 0.05)   # dark blotches
        return coat(c, FROG, k=0.4)
    m.box((6, 3, 6), (-3, 1, -4), Look("fbody", body))
    leg = Look("fleg", lambda c: coat(c, FROG, k=0.3) if c.face != "bottom" else coat(c, FROG_BELLY, k=0.2))
    # Big folded hind legs at the sides, thin front legs.
    m.box((2, 2, 6), (-5, 0, -4), leg, "Leg(0)", (-4, 2, -1))
    m.box((2, 2, 6), (3, 0, -4), leg, "Leg(0)", (4, 2, -1))
    m.box((1, 2, 1), (-3, 0, 1), leg, "Leg(1)", (-3, 2, 1))
    m.box((1, 2, 1), (2, 0, 1), leg, "Leg(1)", (2, 2, 1))
    m.motion = dict(bounce=1.6, graze=False)
    return add(m)


SQ = pal("#7a4422", "#8e522a", "#a26434", "#b47640")
SQ_BELLY = pal("#d8c8b0", "#e4d6c0", "#eee4d2")
GOLD = pal("#a07a18", "#c49a24", "#dcb434", "#f0d050")


def squirrel(name, p, belly, scale=None):
    m = Model(name, scale=scale)
    piv = (0, 7, 2)

    def head(c):
        if c.side() and c.lz > c.d - 2.2 and c.lz < c.d - 0.8 and 2 <= c.ly < 3:
            return EYE
        if c.face == "front" and 2 <= c.ly < 3 and (c.lx < 1 or c.lx > c.w - 1):
            return EYE
        if c.face == "bottom" or (c.face == "front" and c.ly < 1.2):
            return coat(c, belly, k=0.2)
        return coat(c, p, k=0.35)
    m.box((4, 4, 4), (-2, 6, 2), Look(name + "head", head), "Head", piv)
    ear = Look(name + "ear", lambda c: coat(c, p, k=0.2))
    m.box((1, 2, 1), (-2, 10, 3), ear, "Head", piv)
    m.box((1, 2, 1), (1, 10, 3), ear, "Head", piv)
    m.box((4, 5, 6), (-2, 2, -3), Look(name + "body", lambda c: coat(c, belly, k=0.2) if c.face in ("bottom", "front") else coat(c, p, k=0.35)))
    # The big bushy tail curls up behind the back and over it.
    tl = Look(name + "tail", lambda c: coat(c, p, k=0.45), lambda c: coat(c, p, k=0.4) if c.jit > 0.4 else None)
    t1 = m.box((4, 3, 3), (-2, 2, -6), tl, layer=0.6)
    t2 = m.box((4, 7, 3), (-2, 5, -7), tl, layer=0.6)
    t3 = m.box((4, 3, 3), (-2, 11, -6), tl, layer=0.6)
    leg = Look(name + "leg", lambda c: coat(c, p, k=0.3))
    m.box((2, 2, 3), (-3, 0, -3), leg, "Leg(0)", (-2, 2, -2))
    m.box((2, 2, 3), (1, 0, -3), leg, "Leg(1)", (2, 2, -2))
    m.box((1, 3, 1), (-2, 0, 2), leg, "Leg(1)", (-2, 3, 2))
    m.box((1, 3, 1), (1, 0, 2), leg, "Leg(0)", (1, 3, 2))
    m.motion = dict(follow=[(t2, t1), (t3, t1)], tails=[t1], wag=0.6, bounce=1.4, graze=False)
    return add(m)


ARMA = pal("#6a5444", "#7e6652", "#927862", "#a68a72", "#b89c84")
TATU = pal("#8a6a3a", "#a07e48", "#b49258", "#c6a46a", "#d6b67e")


def armadillo(name, p, scale=None, round_=False):
    m = Model(name, scale=scale)
    piv = (0, 4, 4)

    def head(c):
        if c.side() and 1 <= c.lz < 2 and 1.5 <= c.ly < 2.5:
            return EYE
        return coat(c, p[1:] if c.face == "top" else p, k=0.3)
    m.box((3, 3, 3), (-2, 2, 4), Look(name + "head", head), "Head", piv)
    m.box((2, 2, 3), (-1, 2, 7), Look(name + "snout", lambda c: (40, 30, 26) if c.face == "front" else coat(c, p, k=0.3)), "Head", piv)
    ear = Look(name + "ear", lambda c: coat(c, p[:3], k=0.2))
    m.box((1, 2, 1), (-2, 5, 4), ear, "Head", piv)
    m.box((1, 2, 1), (0, 5, 4), ear, "Head", piv)

    def shell(c):
        # Hard plates front and back, banded in the middle.
        band = (int(c.gz + 64) % 2 == 0) and -2 < c.gz < 3
        t = 0.55 + 0.2 * (c.jit - 0.5) + (0.12 if c.face == "top" else 0)
        if band:
            t -= 0.3
        if c.face == "bottom":
            return coat(c, pal("#c8b8a8", "#d8c8b8"), k=0.2)
        if c.vert() and c.ly < 1:
            return ramp(p, 0.2)   # the shell's rim
        # Little scales.
        if (int(c.gx * 2 + c.gz * 2 + c.gy * 2)) % 3 == 0:
            t -= 0.12
        return ramp(p, t)
    m.box((6, 5, 8) if round_ else (6, 5, 9), (-3, 2, -4), Look(name + "shell", shell, lambda c: ramp(p, 0.45 + 0.2 * c.jit) if c.face == "top" and int(c.gz + 64) % 2 == 0 and -2 < c.gz < 3 else None), layer=0.3)
    tail = Look(name + "tail", lambda c: ramp(p, 0.3 if int(c.gz) % 2 else 0.6))
    m.box((1, 1, 5), (0, 2, -9), tail)
    leg = Look(name + "leg", lambda c: coat(c, p[:3], k=0.2))
    legs4(m, leg, (2, 2, 2), (-3, 1), (-4, 2))
    m.motion = dict(graze=True)
    return add(m)


HYENA = pal("#8a7454", "#9e8662", "#b29a72", "#c4ac84")


def build_hyena():
    m = Model("hyena")
    piv = (0, 16, 6)

    def coatf(c):
        if fbm(c.seed + 8, c.gx * 1.6, c.gy * 1.6, c.gz * 1.6, 1.4) > 0.62 and c.face != "bottom":
            return ramp(pal("#3a2e20", "#4a3a2a"), c.jit)   # spots
        return coat(c, HYENA, k=0.35)

    def head(c):
        if c.face == "front":
            cx = c.lx - c.w / 2
            if c.h - 3 <= c.ly < c.h - 2 and 1 <= abs(cx) < 2.5:
                return EYE if abs(cx) < 1.6 else (60, 40, 20)
        return coat(c, HYENA, k=0.3)
    m.box((6, 6, 5), (-3, 13, 6), Look("hyhead", head), "Head", piv)
    m.box((4, 3, 4), (-2, 13, 11), Look("hymuz", lambda c: (24, 20, 18) if c.face == "front" or c.lz > c.d - 1.5 else coat(c, pal("#5a4a38", "#6a5844"), k=0.3)), "Head", piv)
    ear = Look("hyear", lambda c: (40, 32, 26) if c.face == "front" else coat(c, HYENA, k=0.2))
    m.box((2, 3, 1), (-3, 19, 7), ear, "Head", piv)
    m.box((2, 3, 1), (1, 19, 7), ear, "Head", piv)
    # A heavy front and a sloping back: high shoulders, low hips.
    m.box((7, 8, 8), (-4, 10, -1), Look("hyfront", coatf))
    m.box((6, 6, 8), (-3, 8, -8), Look("hyrear", coatf))
    mane = Look("hymane", lambda c: ramp(pal("#2e241a", "#3e3226"), c.jit), lambda c: ramp(pal("#2e241a", "#3e3226"), c.jit) if c.jit > 0.3 else None)
    m.box((2, 2, 12), (-1, 18, -6), mane, layer=0.6)
    tail = m.box((2, 5, 2), (-1, 8, -10), Look("hytail", lambda c: hair(c, pal("#2e241a", "#4a3a2a"))))
    leg = Look("hyleg", lambda c: coatf(c) if c.ly > 1 else ramp(HYENA, 0.15))
    m.box((2, 10, 2), (-4, 0, 3), leg, "Leg(0)", (-3, 10, 4))
    m.box((2, 10, 2), (1, 0, 3), leg, "Leg(1)", (2, 10, 4))
    m.box((2, 8, 2), (-3, 0, -7), leg, "Leg(1)", (-2, 8, -6))
    m.box((2, 8, 2), (1, 0, -7), leg, "Leg(0)", (2, 8, -6))
    m.motion = dict(tails=[tail])
    return add(m)


def build_skunk():
    m = Model("skunk")
    K = pal("#121214", "#1c1c20", "#26262a")
    Wt = pal("#d6d4ce", "#e6e4de", "#f2f0ea")
    piv = (0, 6, 4)

    def stripe(c):
        return abs(c.gx) > 0.4 and abs(c.gx) < 2 and c.face in ("top", "left", "right", "back") and c.ly > c.h - 2.5

    def head(c):
        if c.face == "front" and abs(c.lx - c.w / 2) < 0.6 * 2 and c.ly > 1.5:
            return coat(c, Wt, k=0.2)   # white blaze
        if c.face == "front" and c.ly > c.h - 3 and c.ly < c.h - 2 and (c.lx < 1 or c.lx > c.w - 1):
            return (10, 8, 8)
        return coat(c, K, k=0.2)
    m.box((4, 4, 4), (-2, 4, 4), Look("skhead", head), "Head", piv)
    m.box((2, 2, 2), (-1, 4, 8), Look("sksnout", lambda c: (10, 8, 8) if c.face == "front" and c.ly > 1 else coat(c, K, k=0.2)), "Head", piv)
    m.box((6, 5, 10), (-3, 2, -5), Look("skbody", lambda c: coat(c, Wt, k=0.2) if stripe(c) else coat(c, K, k=0.25)))
    tl = Look("sktail", lambda c: coat(c, Wt if c.lx < 1.5 or c.lx > c.w - 1.5 else K, k=0.2), lambda c: coat(c, K if 1.5 < c.lx < c.w - 1.5 else Wt, k=0.2) if c.jit > 0.35 else None)
    t1 = m.box((4, 3, 3), (-2, 5, -8), tl, layer=0.6)
    t2 = m.box((4, 8, 3), (-2, 7, -9), tl, layer=0.6)
    legs4(m, Look("skleg", lambda c: coat(c, K, k=0.2)), (2, 2, 2), (-3, 1), (-4, 2))
    m.motion = dict(follow=[(t2, t1)], tails=[t1], wag=0.3)
    return add(m)


def build_jerboa():
    m = Model("jerboa", scale=0.6)
    J = pal("#b89462", "#c8a472", "#d6b484", "#e2c49a")
    Wt = pal("#e6e0d2", "#f0ece2")
    piv = (0, 7, 1)

    def head(c):
        if c.side() and c.lz > c.d - 2.5 and c.lz < c.d - 0.5 and 1.5 <= c.ly < 3:
            return EYE   # big night eyes
        return coat(c, Wt if c.face == "bottom" else J, k=0.3)
    m.box((4, 4, 4), (-2, 6, 0), Look("jhead", head), "Head", piv)
    ear = Look("jear", lambda c: (220, 170, 160) if c.face == "front" else coat(c, J, k=0.2))
    m.box((2, 5, 1), (-3, 10, 1), ear, "Head", piv)
    m.box((2, 5, 1), (1, 10, 1), ear, "Head", piv)
    m.box((4, 5, 5), (-2, 3, -4), Look("jbody", lambda c: coat(c, Wt if c.face in ("bottom", "front") else J, k=0.3)))
    tail = m.box((1, 1, 10), (0, 4, -14), Look("jtail", lambda c: coat(c, J, k=0.2)))
    tip = m.box((2, 2, 2), (-1, 3, -16), Look("jtip", lambda c: (24, 20, 18) if c.lz > 1 else (240, 236, 228)))
    hl = Look("jleg", lambda c: coat(c, Wt, k=0.2))
    m.box((1, 4, 3), (-2, 0, -2), hl, "Leg(0)", (-2, 4, -1))
    m.box((1, 4, 3), (1, 0, -2), hl, "Leg(0)", (1, 4, -1))
    m.box((1, 2, 1), (-1, 3, 0), hl, "Leg(1)", (-1, 5, 0))
    m.box((1, 2, 1), (0, 3, 0), hl, "Leg(1)", (0, 5, 0))
    m.motion = dict(follow=[(tip, tail)], tails=[tail], bounce=1.6)
    return add(m)


def build_quati():
    m = Model("quati")
    Q = pal("#5a3a1e", "#6e4a28", "#825a32", "#966c40")
    piv = (0, 7, 5)

    def head(c):
        if c.side() and c.lz > c.d - 2 and 2 <= c.ly < 3:
            return EYE
        if c.face == "front" and c.ly < 2:
            return coat(c, pal("#d8c8b0", "#e6d8c4"), k=0.2)
        return coat(c, Q, k=0.35)
    m.box((4, 4, 4), (-2, 5, 5), Look("qhead", head), "Head", piv)
    m.box((2, 2, 4), (-1, 5, 9), Look("qsnout", lambda c: (20, 16, 14) if c.face == "front" else coat(c, Q[:3], k=0.3)), "Head", piv)
    ear = Look("qear", lambda c: coat(c, Q, k=0.2))
    m.box((1, 1, 1), (-2, 9, 6), ear, "Head", piv)
    m.box((1, 1, 1), (1, 9, 6), ear, "Head", piv)
    m.box((5, 5, 11), (-3, 4, -6), Look("qbody", lambda c: coat(c, pal("#b89a70", "#c8aa80"), k=0.3) if c.face == "bottom" else coat(c, Q, k=0.35)))
    ring = Look("qtail", lambda c: coat(c, Q if int(c.gy + c.gz + 64) % 3 else pal("#1e140c", "#2a1c12"), k=0.2))
    t1 = m.box((2, 2, 4), (-1, 8, -9), ring)
    t2 = m.box((2, 9, 2), (-1, 9, -11), ring)
    legs4(m, Look("qleg", lambda c: coat(c, pal("#2a1c10", "#3a2818"), k=0.2)), (2, 4, 2), (-3, 1), (-5, 3))
    m.motion = dict(follow=[(t2, t1)], tails=[t1], wag=0.2)
    return add(m)


def build_mico():
    m = Model("mico_leao_dourado", scale=0.8)
    G = pal("#c86a10", "#dc7e18", "#ec9424", "#f6a836")
    piv = (0, 8, 3)

    def face(c):
        if c.face == "front":
            if 1 < c.lx < c.w - 1 and c.ly < c.h - 0.5:
                if c.h - 2.5 <= c.ly < c.h - 1.5 and c.lx in (1.5, c.w - 1.5):
                    return EYE
                return ramp(pal("#4a2a20", "#5e3a2c"), 0.5 + 0.3 * c.jit)   # bare dark face
        return coat(c, G, k=0.3)
    m.box((4, 4, 3), (-2, 7, 4), Look("mhead", face), "Head", piv)
    mane = Look("mmane", lambda c: coat(c, G, k=0.4), lambda c: None if c.face == "front" and 1 < c.lx < c.w - 1 and 1 < c.ly < c.h - 1 else (coat(c, G, k=0.4) if c.jit > 0.2 else None))
    m.box((6, 6, 2), (-3, 6, 3), mane, "Head", piv, layer=0.8)
    m.box((4, 5, 7), (-2, 4, -4), Look("mbody", lambda c: coat(c, G, k=0.3)))
    t = m.box((1, 1, 10), (0, 7, -14), Look("mtail", lambda c: coat(c, G, k=0.3)))
    legs4(m, Look("mleg", lambda c: coat(c, G, k=0.3)), (1, 4, 1), (-2, 1), (-3, 2))
    m.motion = dict(tails=[t], bounce=1.3)
    return add(m)


def build_batch2():
    build_birds()
    build_rhino()
    build_hippo()
    build_frog()
    squirrel("squirrel", SQ, SQ_BELLY, scale=0.6)
    squirrel("critter_squirrel", SQ, SQ_BELLY)
    squirrel("critter_gold_squirrel", GOLD, pal("#f0d878", "#f8e8a0"))
    armadillo("armadillo", ARMA)
    armadillo("tatu_bola", TATU, round_=True)
    build_hyena()
    build_skunk()
    build_jerboa()
    build_quati()
    build_mico()


# ---------------------------------------------------------------- monsters that had a plain human body

WERE = pal("#2e2620", "#3e342c", "#4e4238", "#5e5044", "#6e5e50")
WERE_PALE = pal("#8a7a68", "#9c8c78", "#ae9e8a")


def build_werewolf():
    m = Model("werewolf")
    piv = (0, 24, 1)

    def head(c):
        if c.face == "front":
            cx = c.lx - c.w / 2
            if c.h - 3 <= c.ly < c.h - 2 and 1 <= abs(cx) < 2.5:
                return (230, 200, 60) if abs(cx) < 1.6 else (60, 20, 10)
        return coat(c, WERE, k=0.4)

    def snout(c):
        if c.face == "front" and c.ly > c.h - 1.2 and abs(c.lx - c.w / 2) < 1:
            return (14, 10, 10)
        if c.face == "front" and c.ly < 1.2:
            return (230, 224, 210) if int(c.lx) % 2 == 0 else (60, 14, 14)   # fangs
        return coat(c, WERE_PALE if c.face == "bottom" else WERE, k=0.3)
    m.box((7, 6, 6), (-4, 24, -2), Look("wwhead", head), "Head", piv)
    m.box((4, 3, 5), (-2, 24, 4), Look("wwsnout", snout), "Head", piv)
    ear = Look("wwear", lambda c: (60, 30, 30) if c.face == "front" else coat(c, WERE, k=0.2))
    m.box((2, 4, 1), (-4, 30, 0), ear, "Head", piv)
    m.box((2, 4, 1), (1, 30, 0), ear, "Head", piv)
    fur = Look("wwbody", lambda c: coat(c, WERE_PALE if c.face == "front" and c.ly > 4 else WERE, k=0.4),
               lambda c: coat(c, WERE, k=0.4) if c.jit > 0.35 and (c.face != "front" or c.ly > c.h - 4) else None)
    # A hunched, broad chest over thinner hips.
    m.box((10, 12, 6), (-5, 12, -3), fur, layer=0.8)

    def arm(c):
        if c.ly < 2 and c.face in ("front", "bottom"):
            return (220, 212, 196) if c.face == "front" and int(c.lx) % 2 == 0 else (30, 24, 20)   # claws
        return coat(c, WERE, k=0.35)
    m.box((3, 14, 3), (-8, 9, -1), Look("wwarm", arm), "Arm(0)", (-6, 22, 0))
    m.box((3, 14, 3), (5, 9, -1), Look("wwarm", arm), "Arm(1)", (6, 22, 0))
    leg = Look("wwleg", lambda c: (30, 24, 20) if c.ly < 1 else coat(c, WERE, k=0.35))
    m.box((4, 12, 4), (-4, 0, -2), leg, "Leg(0)", (-2, 12, 0))
    m.box((4, 12, 4), (0, 0, -2), leg, "Leg(1)", (2, 12, 0))
    m.box((3, 3, 7), (-1, 12, -10), Look("wwtail", lambda c: coat(c, WERE, k=0.4), lambda c: coat(c, WERE, k=0.4) if c.jit > 0.4 else None), layer=0.5)
    return add(m)


SNOW = pal("#b8c4d4", "#cad4e0", "#dae2ec", "#e8eef4", "#f4f8fa")


def build_snowman_gunner():
    m = Model("snowman_gunner")
    piv = (0, 18, 0)
    snow = lambda c: ramp(SNOW, 0.45 + 0.4 * (fbm(c.seed, c.gx, c.gy, c.gz, 2.0) - 0.5) + 0.15 * c.jit + (0.1 if c.face == "top" else -0.1 if c.face == "bottom" else 0))

    def head(c):
        if c.face == "front":
            cx = c.lx - c.w / 2
            if 3 <= c.ly < 4 and 1 <= abs(cx) < 2:
                return (20, 20, 24)   # coal eyes
            if c.ly < 1.5 and int(c.lx) % 2 == 1 and abs(cx) < 2.5:
                return (30, 30, 34)   # coal smile
        return snow(c)
    m.box((6, 6, 6), (-3, 18, -3), Look("sghead", head), "Head", piv)
    m.box((1, 1, 3), (0, 20, 3), Look("sgnose", lambda c: ramp(pal("#c85a10", "#e07018"), c.jit)), "Head", piv)
    hat = Look("sghat", lambda c: (150, 30, 30) if 1 <= c.ly < 2 else ramp(pal("#141418", "#202026"), c.jit))
    m.box((4, 4, 4), (-2, 24, -2), hat, "Head", piv)
    m.box((6, 1, 6), (-3, 24, -3), Look("sgbrim", lambda c: ramp(pal("#141418", "#202026"), c.jit)), "Head", piv)
    m.box((8, 8, 8), (-4, 10, -4), Look("sgmid", lambda c: (30, 30, 34) if c.face == "front" and abs(c.lx - 4) < 0.6 and int(c.ly) % 3 == 1 else snow(c)))
    m.box((10, 10, 10), (-5, 0, -5), Look("sgbase", snow))
    m.box((9, 2, 9), (-4.5, 17, -4.5), Look("sgscarf", lambda c: (170, 30, 30) if int(c.gx + c.gz + 64) % 3 else (230, 220, 200)))
    stick = Look("sgarm", lambda c: ramp(pal("#3a2614", "#5a3c20", "#6e4c2a"), c.jit))
    m.box((2, 9, 2), (-6, 8, -1), stick, "ArmForward(0)", (-5, 16, 0))
    m.box((2, 9, 2), (4, 8, -1), stick, "ArmForward(1)", (5, 16, 0))
    # The snowball launcher in the right hand.
    m.box((3, 3, 7), (4, 6, -1), Look("sggun", lambda c: (20, 20, 24) if c.face == "front" and 1 <= c.lx < 2 and 1 <= c.ly < 2 else ramp(pal("#3a3e46", "#4e545e", "#626a76"), c.jit)), "ArmForward(1)", (5, 16, 0))
    m.motion = dict(style="biped")
    return add(m)


NIGHTMARE = pal("#121014", "#1a171c", "#231f26", "#2d2830", "#38323c")


def build_headless_rider():
    m = Model("headless_rider")
    # The black horse: eyes and nostrils glow like embers.
    def hhead(c):
        if c.side() and c.lz > c.d - 4 and c.lz < c.d - 2.5 and 3 <= c.ly < 4:
            return (255, 120, 30)
        if c.face == "front" and c.ly < 2 and abs(c.lx - c.w / 2) > 1:
            return (200, 60, 10)
        return coat(c, NIGHTMARE, k=0.35)
    hp = (0, 18, 10)
    m.box((4, 5, 8), (-2, 21, 10), Look("hrhhead", hhead), "Head", hp)
    mane = lambda c: (c.face == "back" or c.face == "top" or (c.side() and c.lz < 1.5))
    m.box((4, 10, 5), (-2, 15, 7), Look("hrhneck", lambda c: ramp(pal("#4a0a0e", "#6a121a", "#8a1c24"), c.jit) if mane(c) else coat(c, NIGHTMARE, k=0.35),
                                         lambda c: ramp(pal("#4a0a0e", "#6a121a"), c.jit) if mane(c) and c.jit > 0.3 else None), "Head", hp, layer=0.6)
    m.box((8, 8, 16), (-4, 10, -8), Look("hrhbody", lambda c: coat(c, NIGHTMARE, k=0.4)))
    m.box((2, 10, 2), (-1, 7, -10), Look("hrhtail", lambda c: ramp(pal("#4a0a0e", "#6a121a"), c.jit), lambda c: ramp(pal("#4a0a0e", "#6a121a"), c.jit) if c.jit > 0.4 else None), layer=0.5)
    legs4(m, Look("hrhleg", lambda c: ramp(pal("#2a2a2e", "#3a3a40"), c.jit) if c.ly < 2 else coat(c, NIGHTMARE, k=0.35)), (3, 10, 3), (-4, 1), (-7, 4))
    # The rider, sitting on the saddle with no head on the neck.
    cloth = pal("#121014", "#1c181e", "#262028", "#302834")
    m.box((12, 3, 4), (-6, 16, -1), Look("hrlegs", lambda c: ramp(pal("#1a1410", "#2a2018"), c.jit) if abs(c.gx) > 5 else coat(c, cloth, k=0.3)))
    m.box((8, 10, 3), (-4, 18, -1), Look("hrbody", lambda c: ramp(pal("#5a4a2a", "#7a6438"), c.jit) if c.ly < 2 else coat(c, cloth, k=0.4)))
    m.box((8, 2, 4), (-4, 28, -2), Look("hrcollar", lambda c: (6, 4, 6) if c.face == "top" and 1 < c.lx < c.w - 1 and 1 < c.lz < c.d - 1 else ramp(pal("#4a0e14", "#6a1820"), c.jit)))
    m.box((8, 12, 1), (-4, 17, -2), Look("hrcape", lambda c: coat(c, pal("#3a0a10", "#5a121a", "#7a1c24"), k=0.4)))
    arm = Look("hrarm", lambda c: coat(c, cloth, k=0.35) if c.ly > 2 else ramp(pal("#2a2420", "#3a322c"), c.jit))
    m.box((3, 10, 3), (-7, 18, -1), arm, "Arm(0)", (-5.5, 27, 0))
    m.box((3, 10, 3), (4, 18, -1), arm, "ArmForward(1)", (5.5, 27, 0))

    def lantern(c):
        if c.face == "front":
            cx = c.lx - c.w / 2
            if (3 <= c.ly < 4 and 0.5 < abs(cx) < 2) or (1 <= c.ly < 2 and abs(cx) < 2.5):
                return (255, 210, 80)   # the carved face glows
        return ramp(pal("#a04a08", "#c46010", "#e07a18"), 0.4 + 0.4 * c.jit)
    m.box((5, 5, 5), (3, 13, 0), Look("hrpumpkin", lantern), "ArmForward(1)", (5.5, 27, 0))
    return add(m)


VORTEX = pal("#0c3a3a", "#145050", "#1e6a68", "#2c8a84", "#44b0a6")


def build_vortex_hornet():
    m = Model("vortex_hornet")
    piv = (0, 10, 4)

    def head(c):
        if c.face == "front" or c.side():
            if c.ly > 1 and (c.face == "front" and (c.lx < 1.5 or c.lx > c.w - 1.5) or c.side() and c.lz > c.d - 2):
                return (120, 255, 230)   # glowing compound eyes
        return coat(c, VORTEX, k=0.3)
    m.box((4, 4, 4), (-2, 8, 4), Look("vhhead", head), "Head", piv)
    m.box((1, 1, 2), (-2, 8, 8), Look("vhjaw", lambda c: ramp(VORTEX, 0.2)), "Head", piv)
    m.box((1, 1, 2), (1, 8, 8), Look("vhjaw", lambda c: ramp(VORTEX, 0.2)), "Head", piv)
    m.box((4, 4, 6), (-2, 7, -2), Look("vhthorax", lambda c: coat(c, VORTEX, k=0.35)))
    m.box((5, 5, 8), (-3, 4, -10), Look("vhabdomen", lambda c: (120, 255, 230) if c.vert() and int(c.gz + 64) % 3 == 0 else coat(c, VORTEX, k=0.35)))
    m.box((1, 1, 3), (-1, 5, -13), Look("vhsting", lambda c: (200, 255, 240)))
    wing = Look("vhwing", lambda c: (150, 230, 220) if (int(c.gx + c.gz) % 3) else None)
    m.box((8, 1, 5), (-10, 11, -3), wing, "Wing(0)", (-2, 11, 0))
    m.box((8, 1, 5), (2, 11, -3), wing, "Wing(1)", (2, 11, 0))
    leg = Look("vhleg", lambda c: ramp(VORTEX, 0.15))
    for k, z in enumerate((2, -1)):
        m.box((1, 5, 1), (-3, 2, z), leg, f"Leg({k % 2})", (-3, 7, z))
        m.box((1, 5, 1), (2, 2, z), leg, f"Leg({(k + 1) % 2})", (2, 7, z))
    m.motion = dict(style="flyer")
    return add(m)


ETHER = pal("#3a1e5a", "#4e2a78", "#643894", "#7c4aae", "#9a64c8")


def build_ether_wyvern():
    m = Model("ether_wyvern")
    piv = (0, 12, 6)

    def head(c):
        if c.side() and c.lz > c.d - 3 and c.lz < c.d - 1.5 and 2 <= c.ly < 3:
            return (255, 220, 120)
        return scales(c, ETHER)
    m.box((4, 4, 6), (-2, 12, 8), Look("ewhead", head), "Head", piv)
    m.box((2, 6, 3), (-1, 9, 5), Look("ewneck", lambda c: scales(c, ETHER)), "Head", piv)
    horn = Look("ewhorn", lambda c: ramp(pal("#c8b8e8", "#e4dcf6"), c.jit))
    m.box((1, 1, 3), (-2, 16, 7), horn, "Head", piv)
    m.box((1, 1, 3), (1, 16, 7), horn, "Head", piv)
    m.box((6, 6, 12), (-3, 4, -6), Look("ewbody", lambda c: ramp(pal("#c8a8e0", "#dcc4ee"), c.jit) if c.face == "bottom" else scales(c, ETHER)))

    def wing(c):
        if c.face in ("top", "bottom"):
            x = abs(c.gx) - 3
            if c.lz < 1 + 1.5 * abs(math.sin(x * 0.8)) and int(x) not in (4, 8):
                return None
            return dim(ETHER[3], 0.8) if int(x) in (4, 8) or c.lz > c.d - 1 else ramp(pal("#7a5aa8", "#9a7ac4", "#b89ade"), 0.4 + 0.3 * c.jit)
        return ETHER[1]
    m.box((12, 1, 8), (-15, 10, -4), Look("ewwing", wing), "Wing(0)", (-3, 10, 0))
    m.box((12, 1, 8), (3, 10, -4), Look("ewwing", wing), "Wing(1)", (3, 10, 0))
    t1 = m.box((3, 3, 8), (-1, 5, -14), Look("ewtail", lambda c: scales(c, ETHER)))
    t2 = m.box((1, 1, 6), (0, 6, -20), Look("ewtail2", lambda c: scales(c, ETHER)))
    leg = Look("ewleg", lambda c: scales(c, ETHER[:3]))
    m.box((2, 4, 2), (-3, 0, 2), leg, "Leg(0)", (-2, 4, 3))
    m.box((2, 4, 2), (1, 0, 2), leg, "Leg(1)", (2, 4, 3))
    m.box((2, 4, 2), (-3, 0, -5), leg, "Leg(1)", (-2, 4, -4))
    m.box((2, 4, 2), (1, 0, -5), leg, "Leg(0)", (2, 4, -4))
    m.motion = dict(style="flyer", follow=[(t2, t1)], tails=[t1])
    return add(m)


RAV = pal("#3a3634", "#4a4542", "#5a5450", "#6a635e", "#7a726c")
HORN = pal("#8a7a5c", "#a8966e", "#c4b088", "#ddcca4")


def build_ravager():
    m = Model("ravager")
    piv = (0, 12, 8)

    def head(c):
        if c.side() and c.lz > c.d - 4 and c.lz < c.d - 2.5 and c.h - 3 <= c.ly < c.h - 2:
            return (230, 60, 40)   # small furious eyes
        if c.face == "front" and 2 <= c.ly < 3 and 1 < c.lx < c.w - 1:
            return (40, 16, 14)    # the wide mouth line
        if c.face == "front" and c.ly < 2 and int(c.lx) % 2 == 0 and 1 < c.lx < c.w - 1:
            return (220, 210, 186)  # teeth under the lip
        return wrinkles(c, RAV, every=3)
    m.box((9, 8, 8), (-4, 6, 9), Look("rvhead", head), "Head", piv)
    nose = lambda c: (20, 18, 18) if c.face == "front" and 1 <= c.ly < 2 and (c.lx < 1.5 or c.lx > c.w - 1.5) else wrinkles(c, RAV[:3], every=2)
    m.box((5, 4, 2), (-2, 9, 17), Look("rvnose", nose), "Head", piv)
    horn = Look("rvhorn", lambda c: ramp(HORN, 0.2 + 0.6 * c.ly / max(c.h, 1) + 0.1 * c.jit))
    # Thick horns curl back and up from the brow.
    m.box((3, 3, 5), (-7, 11, 10), horn, "Head", piv)
    m.box((3, 3, 5), (5, 11, 10), horn, "Head", piv)
    m.box((2, 5, 2), (-7, 14, 9), horn, "Head", piv)
    m.box((2, 5, 2), (6, 14, 9), horn, "Head", piv)
    hump = lambda c: wrinkles(c, RAV[1:], every=4) if c.face == "top" else wrinkles(c, RAV, every=4)
    m.box((12, 11, 18), (-6, 9, -9), Look("rvbody", hump, lambda c: wrinkles(c, RAV, every=4) if c.face == "top" and c.jit > 0.5 else None), layer=0.6)
    leg = Look("rvleg", lambda c: ramp(pal("#1e1c1a", "#2c2826"), c.jit) if c.ly < 2 else wrinkles(c, RAV, every=3))
    legs4(m, leg, (4, 10, 4), (-6, 2), (-8, 4))
    m.box((2, 6, 2), (-1, 12, -11), Look("rvtail", lambda c: wrinkles(c, RAV, every=2)))
    return add(m)


HOP = pal("#3a5a18", "#4a7020", "#5c8628", "#70a032", "#88b840")


def build_grasshopper():
    m = Model("critter_grasshopper")
    piv = (0, 2, 2)

    def head(c):
        if c.side() and c.lz > 0.5 and c.ly > 1:
            return (60, 40, 20)    # big brown side eyes
        return coat(c, HOP, k=0.3)
    m.box((2, 3, 2), (-1, 1, 2), Look("ghead", head), "Head", piv)
    ant = Look("gant", lambda c: ramp(HOP[:2], c.jit))
    m.box((1, 1, 4), (-1, 3, 3), ant, "Head", piv)
    m.box((1, 1, 4), (0, 3, 3), ant, "Head", piv)
    m.box((2, 2, 6), (-1, 1, -4), Look("gbody", lambda c: coat(c, HOP[1:], k=0.3) if c.face != "bottom" else ramp(pal("#a8b850", "#bccc60"), c.jit)))
    m.box((2, 1, 5), (-1, 3, -4), Look("gwing", lambda c: ramp(pal("#4a6020", "#5a7428", "#6a8430"), 0.3 + 0.4 * c.jit)))
    # The big jumping legs: thighs up high, shins down to the ground.
    hind = Look("ghind", lambda c: (150, 40, 30) if c.face == "bottom" else coat(c, HOP, k=0.35))
    m.box((1, 2, 4), (-2, 2, -4), hind, "Leg(0)", (-2, 3, -1))
    m.box((1, 2, 4), (1, 2, -4), hind, "Leg(0)", (1, 3, -1))
    shin = Look("gshin", lambda c: ramp(HOP[:3], c.jit))
    m.box((1, 2, 1), (-2, 0, -4), shin, "Leg(0)", (-2, 3, -1))
    m.box((1, 2, 1), (1, 0, -4), shin, "Leg(0)", (1, 3, -1))
    m.box((1, 1, 1), (-2, 0, 1), shin, "Leg(1)", (-2, 1, 1))
    m.box((1, 1, 1), (1, 0, 1), shin, "Leg(1)", (1, 1, 1))
    m.motion = dict(bounce=1.4, graze=False)
    return add(m)


TAD = pal("#1c1610", "#2a2018", "#382c20", "#463828")


def build_tadpole():
    m = Model("tadpole")
    piv = (0, 1, 0)

    def head(c):
        if c.side() and c.d - 2 <= c.lz < c.d - 1 and 1 <= c.ly < 2:
            return (10, 8, 6)
        if c.face == "bottom":
            return ramp(pal("#6a5a48", "#7a6a56"), c.jit)
        return ramp(TAD, 0.3 + 0.5 * c.jit + (0.15 if c.face == "top" else 0))
    # A round body that is mostly head, and a thin finned tail that wiggles.
    m.box((4, 3, 4), (-2, 0, -1), Look("tdhead", head), "Head", piv)
    t1 = m.box((1, 2, 3), (0, 0, -4), Look("tdtail", lambda c: ramp(TAD, 0.4 + 0.3 * c.jit)))
    fin = Look("tdfin", lambda c: ramp(pal("#4a3e30", "#5a4c3a"), c.jit) if c.side() or c.ly > 1 else None)
    t2 = m.box((1, 2, 3), (0, 0, -7), fin)
    m.motion = dict(follow=[(t2, t1)], tails=[t1])
    return add(m)


SEAL = pal("#4a4c50", "#5c5e62", "#6e7074", "#808286", "#94969a")
SEAL_PALE = pal("#9c9a94", "#b0aea6", "#c4c2ba")


def build_seal():
    m = Model("seal")
    piv = (0, 6, 6)

    def spots(c, p):
        if c.face != "bottom" and fbm(c.seed + 9, c.gx * 1.3, c.gy * 1.3, c.gz * 1.3, 1.0) > 0.66:
            return ramp(p, 0.08)
        return coat(c, p, k=0.3)

    def head(c):
        if c.face == "front":
            cx = c.lx - c.w / 2
            if 2 <= c.ly < 4 and 1 <= abs(cx) < 2.5:
                return (8, 8, 10) if c.ly >= 3 or abs(cx) < 2 else (60, 64, 70)   # big dark wet eyes
        return spots(c, SEAL)
    m.box((6, 5, 5), (-3, 3, 5), Look("shead", head), "Head", piv)

    def muz(c):
        if c.face == "front" and c.ly >= 1 and abs(c.lx - c.w / 2) < 1:
            return (20, 18, 18)
        if c.face in ("left", "right", "front") and c.ly < 1 and int(c.lx + c.lz) % 2 == 0:
            return (210, 206, 196)   # whisker pads
        return coat(c, SEAL_PALE, k=0.2)
    m.box((4, 2, 2), (-2, 3, 10), Look("smuz", muz), "Head", piv)
    # A smooth torpedo body lying on the ground, tapering to the hips.
    m.box((8, 6, 9), (-4, 0, -3), Look("sbody", lambda c: coat(c, SEAL_PALE, k=0.2) if c.face == "bottom" else spots(c, SEAL)))
    hip = m.box((6, 4, 5), (-3, 0, -8), Look("ship", lambda c: coat(c, SEAL_PALE, k=0.2) if c.face == "bottom" else spots(c, SEAL)))
    # Hind flippers fan out behind like a tail; front flippers are short paddles.
    hf = Look("shflip", lambda c: ramp(SEAL[:3], 0.3 + 0.3 * c.jit) if int(c.gx + 64) % 2 or c.lz > 1 else ramp(SEAL, 0.05))
    t = m.box((6, 1, 4), (-3, 0, -12), hf)
    ff = Look("sfflip", lambda c: ramp(SEAL[:3], 0.3 + 0.3 * c.jit))
    m.box((1, 1, 4), (-5, 0, 1), ff, "Wing(1)", (-4, 1, 3))
    m.box((1, 1, 4), (4, 0, 1), ff, "Wing(0)", (4, 1, 3))
    m.motion = dict(style="crawler", tails=[t], follow=[])
    _ = hip
    return add(m)


GAZ = pal("#9a6430", "#b0763a", "#c48a48", "#d49e5a", "#e2b06e")
GAZ_WHITE = pal("#d8d0c0", "#e6e0d2", "#f2eee4")


def build_gazelle():
    m = Model("gazelle")
    piv = (0, 17, 6)

    def head(c):
        if c.side() and c.lz > c.d - 4 and c.lz < c.d - 2.5 and c.h - 2 <= c.ly < c.h - 1:
            return EYE
        if c.face == "front" and c.ly < 1.5 and abs(c.lx - c.w / 2) < 1:
            return (30, 24, 20)
        if c.face == "front" and abs(c.lx - c.w / 2) >= 1:
            return coat(c, GAZ_WHITE, k=0.2)   # pale face stripes
        return coat(c, GAZ, k=0.3)
    m.box((4, 4, 6), (-2, 19, 7), Look("gzhead", head), "Head", piv)
    m.box((3, 7, 3), (-1, 14, 5), Look("gzneck", lambda c: coat(c, GAZ_WHITE, k=0.2) if c.face == "front" else coat(c, GAZ, k=0.3)), "Head", piv)
    horn = Look("gzhorn", lambda c: (24, 20, 18) if int(c.gy + c.gz) % 2 else (54, 46, 40))   # ringed black horns
    m.box((1, 5, 1), (-2, 23, 8), horn, "Head", piv)
    m.box((1, 5, 1), (1, 23, 8), horn, "Head", piv)
    m.box((1, 1, 2), (-2, 27, 6), horn, "Head", piv)
    m.box((1, 1, 2), (1, 27, 6), horn, "Head", piv)
    ear = Look("gzear", lambda c: (200, 170, 150) if c.face == "front" else coat(c, GAZ, k=0.2))
    m.box((2, 1, 1), (-4, 22, 8), ear, "Head", piv)
    m.box((2, 1, 1), (2, 22, 8), ear, "Head", piv)

    def body(c):
        if c.face == "bottom" or (c.side() and c.ly < 2):
            return coat(c, GAZ_WHITE, k=0.2)
        if c.side() and 2 <= c.ly < 3:
            return ramp(pal("#2a1c14", "#3a281c"), c.jit)   # the dark flank stripe
        if c.face == "back" and c.ly < 4:
            return coat(c, GAZ_WHITE, k=0.2)
        return coat(c, GAZ, k=0.35)
    m.box((5, 6, 12), (-3, 10, -6), Look("gzbody", body))
    m.box((1, 3, 1), (-1, 12, -7), Look("gztail", lambda c: (24, 20, 18) if c.ly < 1 else coat(c, GAZ, k=0.2)))
    leg = Look("gzleg", lambda c: (30, 24, 20) if c.ly < 1 else coat(c, GAZ_WHITE if c.face in ("front", "back") and c.ly < 5 else GAZ, k=0.3))
    legs4(m, leg, (2, 10, 2), (-3, 0), (-5, 3))
    m.motion = dict(graze=True)
    return add(m)


SPECT = pal("#5a7896", "#7090b0", "#88a8c8", "#a2c0dc", "#bcd6ec")
SPECT_PALE = pal("#c8dcee", "#d8e8f6", "#e8f2fc")


def build_batch3():
    build_seal()
    build_gazelle()
    canid("spectral_wolf", (6, 7, 14), 9, SPECT, SPECT_PALE, ear_h=3, tail_len=8)
    build_ravager()
    build_grasshopper()
    build_tadpole()
    build_werewolf()
    build_snowman_gunner()
    build_headless_rider()
    build_vortex_hornet()
    build_ether_wyvern()


# ================================================================ build list


def build_all():
    build_zebra()
    build_elephant()
    feline("lion", (8, 8, 18), (7, 7, 6), 10, LION, LION_PALE, mane=True, scale=0.95)
    feline("lioness", (8, 8, 18), (6, 6, 6), 9, LION, LION_PALE)
    build_boar()
    build_boar("crimson_boar", CRIM, pal("#7a1a22", "#9a2a30", "#b83c40"), crust=CRIM_CRUST, eye=CRIM_EYE, spikes=True)
    build_penguin()
    build_panda()
    build_goat()
    cat("cat", pal("#5c5a58", "#706e6a", "#86837e", "#9a968e"), pal("#2c2a28", "#3a3836"), pal("#c8c4bc", "#dad6ce", "#e8e4dc"))
    cat("cat_tame", pal("#a85a1c", "#c06e28", "#d48436", "#e29a4a"), pal("#7a3a10", "#8c4616"), pal("#e6d8c0", "#f0e6d4", "#f8f2e6"), collar=(170, 30, 30))
    canid("wolf", (6, 7, 15), 10, WOLF, WOLF_PALE, scale=1.1)
    canid("wolf_tame", (6, 7, 15), 10, WOLF, WOLF_PALE, scale=1.1, collar=(170, 30, 30))
    canid("fox", (5, 5, 11), 6, FOX, FOX_PALE, tail_tip=FOX_PALE, ear_h=3, tail_len=9)
    build_camel()
    build_skeleton_horse()
    bat("bat", pal("#2e2420", "#3e3028", "#4e3c32", "#5e4a3c"), pal("#2a201c", "#3a2c26", "#4a3a32", "#5a4840"))
    bat("cave_bat", pal("#3a3036", "#4a3e44", "#5a4c52"), pal("#2e262a", "#3e3238", "#4e4046"))
    bat("jungle_bat", pal("#1e3418", "#2a4620", "#36582a"), pal("#18280f", "#223816", "#2e4a20"))
    bat("ice_bat", pal("#6a8aa8", "#80a0bc", "#98b6ce"), pal("#5a7896", "#7090ae", "#88a8c4"), ear_inner=(170, 200, 220), eye=(30, 50, 90))
    bat("lava_bat", pal("#5a1a0a", "#7a2610", "#9a3416"), pal("#4a1408", "#6a200c", "#8a2e12"), ear_inner=(240, 140, 40), eye=(255, 200, 60))
    bat("giant_bat", pal("#2a1830", "#3a2240", "#4a2e52"), pal("#22142a", "#321e3c", "#42284e"), ear_inner=(160, 60, 70), eye=(220, 40, 40))
    build_swan()
    build_llama()
    build_batch2()
    build_batch3()


def grow(img, name):
    """The 4x skin, like tools/tex64/entities.py does for every design."""
    import entities as ent
    a = np.asarray(img.convert("RGBA"), dtype=np.float64)
    big = ent.grow(a, name)
    from style import to_u8
    return Image.fromarray(to_u8(big), "RGBA")


def main():
    build_all()
    os.makedirs(os.path.dirname(OUT_RON), exist_ok=True)
    os.makedirs(SRC16, exist_ok=True)
    text = ("// Real animals' own models (tools/gen_fauna.py writes this file: edit that script).\n"
            "// Laid over the mob lists after loading: only parts, layers, motion and skin change.\n"
            "(models: [\n")
    for m in MODELS:
        img = m.render()
        img.save(os.path.join(SRC16, m.skin + ".png"))
        grow(img, m.skin).save(os.path.join(ENTITY, m.skin + ".png"), optimize=True)
        text += m.ron()
    text += "])\n"
    with open(OUT_RON, "w", encoding="utf-8") as f:
        f.write(text)
    print(f"wrote {len(MODELS)} models")


if __name__ == "__main__":
    main()
