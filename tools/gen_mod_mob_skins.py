#!/usr/bin/env python3
"""Skins (64x128) and spawn egg sprites for the creatures of the mod content pack.

Reads packs/vanilla/mods/mobs.ron for the geometry (checked against tools/mod_creatures.py,
which also says what each box is and how the creature is coloured) and paints every face
pixel by pixel with the helpers of tools/gen_mob_skins.py: the same noise, palettes, box
unwrap and relief layers as the base mobs, so they keep the same low-res look. Everything is
our own art; nothing is read from any mod.

Mobs in mods/mobs.ron that this script has no painter for (other mods' creatures) are skipped.

Usage:
    python3 tools/gen_mod_mob_skins.py [preview.png]

Writes packs/vanilla/textures/entity/<mob>.png and packs/vanilla/textures/item/<mob>_spawn_egg.png.
"""

import math
import os
import sys
from types import SimpleNamespace

from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gen_mob_skins as G  # noqa: E402
import mod_creatures as MC  # noqa: E402

ROOT = G.ROOT
MODS_MOBS = MC.MODS_MOBS
ITEM_DIR = os.path.join(ROOT, "packs", "vanilla", "textures", "item")
SIDES = G.SIDES
pick, mul, mix, fbm, hash01 = G.pick, G.mul, G.mix, G.fbm, G.hash01


def P(hexes):
    return [G.hexrgb(h) for h in hexes] if hexes else None


EYE_DARK = (16, 12, 12)
EYE_WHITE = (236, 232, 224)


# --------------------------------------------------------------------------------------------
# Pattern helpers
# --------------------------------------------------------------------------------------------

def voronoi(seed, x, y, z, cell):
    """Distances to the nearest and second nearest feature point (3D cells of `cell` pixels)."""
    fx, fy, fz = x / cell, y / cell, z / cell
    ix, iy, iz = math.floor(fx), math.floor(fy), math.floor(fz)
    d1 = d2 = 9e9
    for a in (-1, 0, 1):
        for b in (-1, 0, 1):
            for e in (-1, 0, 1):
                cx, cy, cz = ix + a, iy + b, iz + e
                px = cx + hash01(seed, cx, cy, cz)
                py = cy + hash01(seed + 1, cx, cy, cz)
                pz = cz + hash01(seed + 2, cx, cy, cz)
                d = (px - fx) ** 2 + (py - fy) ** 2 + (pz - fz) ** 2
                if d < d1:
                    d1, d2 = d, d1
                elif d < d2:
                    d2 = d
    return math.sqrt(d1) * cell, math.sqrt(d2) * cell


BODYLIKE = {"body", "neck", "hump", "seg", "abdomen", "pouch", "head", "tail", "jaw", "flipper", "tentacle"}


class Painter:
    def __init__(self, name, look):
        self.name = name
        self.look = look
        self.base = P(look["base"])
        self.belly = P(look.get("belly"))
        self.pat = look.get("pattern")
        self.pat_cols = P(self.pat[1]) if self.pat else None

    def pal(self, key, default=None):
        v = self.look.get(key)
        if v is None:
            return default
        if isinstance(v, tuple):
            return [v]
        return P(v)

    # ---- coat --------------------------------------------------------------------------------
    def coat(self, c, pal=None, bias=0.0):
        pal = pal or self.base
        n = fbm(c.seed, c.gx, c.gy, c.gz, 4.0, 2)
        t = 0.2 + 0.6 * n + 0.25 * (c.jit - 0.5) + bias
        if c.face == "top":
            t += 0.08
        if c.face == "bottom":
            t -= 0.12
        return pick(pal, t)

    def belly_here(self, c):
        if not self.belly or c.role not in BODYLIKE - {"head"}:
            return False
        if c.face == "bottom":
            return True
        if c.role == "seg":
            return False
        return c.face in SIDES and c.y < c.h * 0.3 and c.role in ("body", "neck", "pouch", "jaw")

    def pattern(self, c):
        if not self.pat or c.face == "bottom":
            return None
        kind, cols, scale, thr = self.pat[0], self.pat_cols, self.pat[2], self.pat[3]
        s = c.seed + 31
        if kind == "spots":
            n = fbm(s, c.gx, c.gy, c.gz, scale, 2)
            if n > thr:
                return pick(cols, c.jit)
        elif kind == "patches":
            d1, d2 = voronoi(s, c.gx, c.gy * 0.7, c.gz, 3.4)
            if d2 - d1 > 0.55:
                return pick(cols, c.jit)
            return pick(self.belly or self.base, 0.5 + 0.4 * c.jit)
        elif kind == "cracks":
            d1, d2 = voronoi(s, c.gx, c.gy, c.gz, scale)
            if d2 - d1 < thr:
                return pick(cols, c.jit)
        elif kind == "plates":
            if c.role in ("body",) and (int(c.x) % scale == 0 or int(c.z) % scale == 0 or int(c.y) % scale == 0):
                return mul(self.base[0], 0.8)
            if c.role == "body" and c.face == "front" and c.j > 1 and (c.i + c.j) % 5 == 0:
                return pick(cols, c.jit)
        elif kind == "bands":
            k = int(math.floor(c.gz / max(1, scale))) % len(cols)
            return mul(cols[k], 0.92 + 0.16 * c.jit)
        elif kind == "diamonds":
            u = (c.gz % scale) - scale / 2
            if c.face == "top" or (c.face in SIDES and c.y > c.h * 0.5):
                if abs(u) + abs(c.gx) * 0.9 < scale * 0.38:
                    return pick(cols, c.jit)
                if abs(abs(u) + abs(c.gx) * 0.9 - scale * 0.5) < 0.4:
                    return mul(self.base[-1], 1.05)
        elif kind == "ribs":
            if c.role == "seg" and c.face in SIDES + ("top",) and int(c.z) % scale == 0:
                return pick(cols, c.jit)
        elif kind == "segments":
            if c.role == "body" and int(c.z) % scale == 0:
                return pick(cols, c.jit)
        elif kind == "streaks":
            h = hash01(s, int(c.gx), int(c.gy) // 2, int(c.gz))
            if h > 0.78:
                return cols[0]
            if h < 0.08:
                return cols[1]
        elif kind == "lateral":
            if c.role == "body" and c.face in ("left", "right") and abs(c.y - c.h * 0.55) < 0.8 and c.jit > 0.25:
                return cols[0]
        return None

    # ---- eyes --------------------------------------------------------------------------------
    def eye_row(self, c):
        return max(0, min(c.fh - 1, round(c.fh * 0.35) - (1 if c.fh >= 6 else 0)))

    def eyes(self, c):
        lk = self.look
        style = lk.get("eyes", "front")
        col = lk.get("eye")
        if c.role == "head" and style in ("front", "glow", "void", "pumpkin", "stalker") and c.face == "front":
            r = self.eye_row(c)
            fw = c.fw
            if style == "stalker":
                # A frightful face of dark hollows (our own layout), scaled to the head.
                u, v = c.i / fw, c.j / c.fh
                if (0.15 <= u < 0.4 or 0.6 <= u < 0.85) and 0.25 <= v < 0.45:
                    return (14, 18, 12)
                if 0.35 <= u < 0.65 and 0.45 <= v < 0.65:
                    return (20, 26, 16)
                if (0.25 <= u < 0.4 or 0.6 <= u < 0.75) and 0.6 <= v < 0.85:
                    return (20, 26, 16)
                return None
            if style == "void":
                if c.j in (r, r + 1) and (1 <= c.i <= 3 or fw - 4 <= c.i <= fw - 2):
                    return col if c.j == r else mul(col, 0.6)
                if c.j == c.fh - 3 and 2 <= c.i <= fw - 3 and c.jit > 0.3:
                    return (60, 20, 80)
                return None
            if style == "pumpkin":
                if c.j in (r, r + 1) and c.i in (2, 3, fw - 4, fw - 3):
                    return (20, 20, 24)
                if c.j == c.fh - 3 and 2 <= c.i <= fw - 3 and c.i % 2 == 0:
                    return (24, 24, 28)
                if c.j == c.fh - 4 and c.i in (1, fw - 2):
                    return (24, 24, 28)
                return None
            if style == "glow":
                if c.j == r and (c.i in (1, 2, fw - 3, fw - 2) if fw >= 7 else c.i in (0, fw - 1) if fw <= 4 else c.i in (1, fw - 2)):
                    return col or (255, 220, 120)
                return None
            if c.j == r:
                if fw >= 6:
                    if c.i in (1, fw - 2):
                        return EYE_WHITE
                    if c.i in (2, fw - 3):
                        return col or EYE_DARK
                elif fw >= 5:
                    if c.i in (1, fw - 2):
                        return col or EYE_DARK
                elif c.i in (0, fw - 1):
                    return col or EYE_DARK
            if lk.get("eye_ring") and c.j == r - 1 and fw >= 6 and c.i in (1, 2, fw - 3, fw - 2):
                return lk["eye_ring"]
            return None
        if c.role == "head" and style in ("side", "glossy", "bigred") and c.face in ("left", "right"):
            r = self.eye_row(c)
            front = (c.fw - 1 if c.face == "right" else 0)
            col_i = front + (-1 if c.face == "right" else 1) if c.fw >= 5 else front
            if style == "bigred":
                half = c.fw // 2
                near = (c.i >= c.fw - half) if c.face == "right" else (c.i < half)
                if near and c.j <= max(1, c.fh // 2):
                    return mul(col or (170, 30, 30), 0.85 + 0.3 * c.jit)
                return None
            if c.j == r and c.i == col_i:
                return (8, 8, 10) if style == "glossy" else (col or EYE_DARK)
            if lk.get("eye_ring") and abs(c.j - r) <= 1 and abs(c.i - col_i) <= 1:
                return lk["eye_ring"]
            if style == "glossy" and c.j == r and abs(c.i - col_i) == 1:
                return (30, 30, 40)
            return None
        if c.role == "eyestalk" and (c.face == "top" or (c.face in SIDES and c.j == 0)):
            return EYE_DARK
        if c.role == "eyebump" and style == "bump" and c.face in SIDES + ("top",):
            if c.face in ("front", "left", "right") and c.j == 0:
                return EYE_DARK
            return col or (240, 160, 40)
        return None

    # ---- whole painter -----------------------------------------------------------------------
    def __call__(self, c):
        lk = self.look
        e = self.eyes(c)
        if e is not None:
            return e
        role = c.role
        f = getattr(self, "role_" + role, None)
        if f:
            v = f(c)
            if v is not None:
                return v
        return self.coat_with_pattern(c)

    def coat_with_pattern(self, c, pal=None):
        lk = self.look
        if self.belly_here(c):
            col = self.coat(c, self.belly)
        else:
            p = self.pattern(c) if c.role in BODYLIKE or c.role in ("leg", "arm", "ear") else None
            col = p if p is not None else self.coat(c, pal)
        if lk.get("sheen") and c.face == "top" and c.jit > 0.7:
            col = mix(col, lk["sheen"], 0.5)
        if lk.get("shell_sheen") and c.face == "top" and (c.i + c.j) % 4 == 0:
            col = mul(col, 1.35)
        if lk.get("slime") and c.face in ("top",) + SIDES and c.jit > 0.9:
            col = mul(col, 1.3)
        if lk.get("stars") and c.jit > 0.94 and c.face != "bottom":
            col = (250, 245, 220)
        if lk.get("scars") and c.role in ("body", "head") and c.face in SIDES + ("top",):
            if hash01(c.seed + 77, int(c.gx), int(c.gy) // 3, int(c.gz) // 4) > 0.93 and c.jit > 0.3:
                col = (150, 150, 158)
        if lk.get("rainbow") and c.face != "bottom":
            k = (c.gz * 0.18 + c.gy * 0.1) % 1.0
            hues = [(40, 170, 140), (60, 120, 200), (200, 80, 120), (230, 160, 40), (80, 190, 80)]
            col = mix(col, hues[int(k * len(hues))], 0.55)
        return col

    def accent(self, c, key, default, bias=0.0):
        pal = self.pal(key) or default
        return pick(pal, 0.25 + 0.6 * c.jit + bias)

    def tipped(self, c, key, default):
        """Horns, tusks and teeth: lighter towards the tip."""
        pal = self.pal(key) or default
        t = 0.3 + 0.4 * c.jit
        if c.face == "top" or c.j == 0:
            t += 0.3
        return pick(pal, t)

    # ---- roles -------------------------------------------------------------------------------
    def role_head(self, c):
        lk = self.look
        hc = self.pal("head_col")
        if lk.get("silverback") or lk.get("face_mask"):
            skin = self.pal("skin")
            if c.face == "front" and 1 <= c.i <= c.fw - 2 and c.j >= 2:
                return pick(skin, 0.3 + 0.5 * c.jit)
        if lk.get("mask") and c.face in ("front", "left", "right") and abs(c.j - self.eye_row(c)) <= 1:
            if c.face == "front" and c.fw // 2 - 1 <= c.i <= c.fw // 2:
                return pick(WHITE, 0.6)
            return pick(P(["#141414", "#1e1e1e"]), c.jit)
        if lk.get("face_black") and c.face in ("front", "left", "right") and c.j >= c.fh // 2:
            return pick(P(["#141414", "#1e1e1e"]), c.jit)
        if lk.get("skunk") and c.face in ("front", "top") and c.fw // 2 - 1 <= c.i <= c.fw // 2:
            return pick(WHITE, 0.4 + 0.5 * c.jit)
        if lk.get("orca"):
            if c.face in ("left", "right") and c.j <= c.fh // 2 - 1:
                zz = c.z
                if c.d * 0.25 <= zz <= c.d * 0.75 and c.j >= 1:
                    return pick(WHITE, 0.6)
                if zz > c.d * 0.75 and c.j == 2:
                    return EYE_DARK
            if c.face in ("front",) and c.j >= c.fh * 0.6:
                return pick(WHITE, 0.5)
        if lk.get("throat") and c.face == "front" and c.j >= c.fh // 2:
            return lk["throat"]
        if self.pal("skin") and (lk.get("overlay") == "ruff" or lk.get("silverback") is None and "skin" in lk and "neck" in self.roles):
            return pick(self.pal("skin"), 0.3 + 0.5 * c.jit)
        if hc:
            return pick(hc, 0.3 + 0.5 * c.jit)
        if lk.get("collar") and c.face == "back":
            return lk["collar"]
        if lk.get("walker"):
            return None
        return None

    def role_neck(self, c):
        if self.pal("skin") and self.look.get("overlay") == "ruff":
            return pick(self.pal("skin"), 0.3 + 0.5 * c.jit)
        if self.look.get("skin") and self.look.get("beak") and "emu" in self.name:
            return pick(self.pal("skin"), 0.3 + 0.5 * c.jit)
        return None

    def role_body(self, c):
        lk = self.look
        if lk.get("silverback") and c.face in ("top", "back") and c.y > c.h * 0.4:
            return pick(P(["#7a7a80", "#8a8a90", "#9a9aa0"]), c.jit)
        if lk.get("skunk") and c.face in ("top", "back", "front") and abs(c.gx) < 1.6:
            if c.face == "top" or c.j <= 1:
                return pick(WHITE, 0.4 + 0.5 * c.jit)
        if lk.get("flank_stripe") and c.face in ("left", "right") and abs(c.y - c.h * 0.32) < 0.8:
            return pick(P(["#2a1a0e", "#3a2414"]), c.jit)
        if lk.get("orca") and c.face in ("left", "right") and c.y < c.h * 0.5 and c.z < c.d * 0.45 and c.y > c.h * 0.2:
            return pick(WHITE, 0.6)
        if lk.get("orca") and c.face == "top" and c.z < c.d * 0.5 and abs(c.x - c.w / 2) < 2.5 and c.z > c.d * 0.2:
            return pick(P(["#6a6a70", "#7a7a80"]), c.jit)
        if lk.get("throat") and c.face == "front" and c.j < c.fh // 2:
            return lk["throat"]
        if lk.get("abdomen_glow") and self.name == "firefly":
            return None
        return None

    def role_abdomen(self, c):
        g = self.look.get("abdomen_glow")
        if g:
            if self.name == "crimson_mosquito":
                return mul(g, 0.75 + 0.35 * c.jit) if c.face != "top" or c.jit > 0.4 else mul(g, 0.5)
            return mul(g, 0.85 + 0.2 * c.jit)
        return None

    def role_seg(self, c):
        lk = self.look
        if lk.get("rattle") and c.gz < -10 and c.w <= 2:
            return pick(P(["#8a7a5a", "#c8b890", "#e0d4b0"]), (int(c.z) % 2) * 0.6 + 0.2)
        return None

    def role_tail(self, c):
        lk = self.look
        if lk.get("tail_rings"):
            return pick(BLACK_P, c.jit) if int(c.z) % 3 == 0 else None
        if lk.get("skunk"):
            return pick(WHITE, 0.4 + 0.5 * c.jit) if abs(c.x - c.w / 2) < 1 else None
        if lk.get("orca"):
            return None
        if lk.get("toucan"):
            return None
        return None

    def role_snout(self, c):
        lk = self.look
        pal = self.pal("snout")
        col = pick(pal, 0.25 + 0.6 * c.jit) if pal else self.coat(c, None, 0.15)
        if c.face == "front":
            if c.j == 0 and c.fw >= 3 and c.i in (0, c.fw - 1):
                return (24, 18, 18)
            if c.j == c.fh - 1 and c.fh >= 3:
                return mul(col, 0.6)
        if self.name in ("crocodile", "alligator") and c.face in ("left", "right") and c.j == c.fh - 1 and c.jit > 0.4:
            return pick(self.pal("tooth") or WHITE, 0.6)
        return col

    def role_beak(self, c):
        pal = self.pal("beak") or P(["#6a6a5a", "#7a7a6a"])
        lk = self.look
        if lk.get("beak_tip") and (c.face == "front" or c.z > c.d - 1.2):
            return lk["beak_tip"]
        if lk.get("toucan") and c.face != "bottom" and c.y > c.h * 0.6:
            return mix(pick(pal, 0.6), (240, 200, 40), 0.5)
        if self.name == "flamingo":
            return pal[2] if c.z < c.d * 0.5 else pal[0]
        if c.face == "front" and c.j == 0 and c.fw >= 2 and c.i in (0, c.fw - 1):
            return mul(pal[0], 0.7)
        return pick(pal, 0.25 + 0.6 * c.jit)

    def role_bill(self, c):
        pal = self.pal("bill") or BLACK_P
        if c.face == "top" and c.j == c.fh - 2 and c.i in (1, c.fw - 2):
            return (10, 10, 10)
        return pick(pal, 0.25 + 0.6 * c.jit)

    def role_tooth(self, c):
        return self.tipped(c, "tooth", WHITE)

    def role_tusk(self, c):
        return self.tipped(c, "tusk", P(MC.TUSK))

    def role_horn(self, c):
        return self.tipped(c, "horn", P(MC.HORN))

    def role_antler(self, c):
        return self.tipped(c, "antler", P(MC.HORN))

    def role_spike(self, c):
        return self.tipped(c, "horn", P(["#1c2a16", "#2a3c20", "#3a5028"]))

    def role_ear(self, c):
        inner = self.pal("ear_inner")
        if inner and c.face == "front" and 0 < c.i < c.fw - 1 or (inner and c.face == "front" and c.fw <= 2 and c.j > 0):
            return pick(inner, 0.3 + 0.5 * c.jit)
        return None

    def role_brow(self, c):
        return self.coat(c, None, -0.25)

    def role_leg(self, c):
        lk = self.look
        pal = self.pal("legs")
        if lk.get("walker"):
            return None
        if c.face == "bottom" or (c.face in SIDES and c.j >= c.fh - 1 and c.fh >= 3):
            if lk.get("claws") and c.face == "front" and c.i % 2 == 0:
                return (220, 210, 190)
            base = pal or self.base
            return mul(pick(base, 0.2 * c.jit), 0.6)
        if pal:
            return pick(pal, 0.25 + 0.6 * c.jit)
        if lk.get("orca") or lk.get("butterfly") or self.pat:
            return None
        return self.coat(c, None, -0.1)

    def role_arm(self, c):
        pal = self.pal("arm_col")
        if self.look.get("walker"):
            return None
        if pal:
            return pick(pal, 0.25 + 0.6 * c.jit)
        if self.look.get("silverback") and c.face == "bottom":
            return pick(self.pal("skin"), 0.5)
        return None

    def role_wing(self, c):
        lk = self.look
        pal = self.pal("wing_col") or [mul(x, 0.85) for x in self.base]
        if lk.get("butterfly"):
            if c.face in ("top", "bottom"):
                edge = c.i in (0, c.fw - 1) or c.j in (0, c.fh - 1)
                if edge:
                    if c.jit > 0.75:
                        return (240, 240, 230)
                    return (20, 16, 14)
                if (c.i + c.j) % 3 == 0 and c.jit > 0.4:
                    return (30, 20, 16)
                return pick(pal, 0.3 + 0.6 * c.jit)
            return (20, 16, 14)
        if c.h == 1 and c.face in ("top", "bottom"):
            # insect wings: pale panes with dark veins
            if c.i % 3 == 0 or c.j == 0:
                return mul(pick(pal, 0.2), 0.6)
            return pick(pal, 0.4 + 0.5 * c.jit)
        tip = self.pal("wing_tip")
        if tip and c.z < 2.0:
            return pick(tip, c.jit)
        if c.face in ("left", "right") and c.j == c.fh - 1 and c.i % 2 == 0:
            return mul(pick(pal, 0.3), 0.8)
        if c.face in ("left", "right") and c.z < c.d * 0.5 and (c.i + c.j) % 3 == 0:
            return mul(pick(pal, 0.4), 0.85)
        if lk.get("finch") and c.jit > 0.7:
            return (230, 230, 230)
        return pick(pal, 0.25 + 0.6 * c.jit)

    def role_fin(self, c):
        pal = self.pal("fin") or [mul(x, 0.8) for x in self.base]
        if self.look.get("orca"):
            return pick(BLACK_P, c.jit)
        return pick(pal, 0.25 + 0.6 * c.jit)

    def role_tongue(self, c):
        return self.accent(c, "tongue", PINK_P)

    def role_whisker(self, c):
        return self.accent(c, "whisker", BLACK_P)

    def role_claw(self, c):
        return self.tipped(c, "claw_col", [mul(x, 0.6) for x in self.base])

    def role_stinger(self, c):
        return pick(BLACK_P, c.jit)

    def role_antenna(self, c):
        return pick(BLACK_P, c.jit) if self.name != "lobster" else self.coat(c)

    def role_proboscis(self, c):
        return pick(P(["#1a0a0a", "#2a1010"]), c.jit)

    def role_frill(self, c):
        return self.accent(c, "frill", self.base)

    def role_shell(self, c):
        pal = self.pal("shell") or self.base
        d1, d2 = voronoi(c.seed + 9, c.gx, c.gy, c.gz, 3.2)
        if self.name == "snail":
            r = math.hypot(c.y - c.h / 2, c.z - c.d / 2) if c.face in ("left", "right") else 99
            if r < 99 and int(r * 1.3) % 2 == 0:
                return pick(pal, 0.15 + 0.2 * c.jit)
            return pick(pal, 0.55 + 0.4 * c.jit)
        if self.name == "rocky_roller":
            return pick(pal, 0.2 + 0.7 * fbm(c.seed, c.gx, c.gy, c.gz, 2.0, 2))
        if d2 - d1 < 0.5:
            return mul(pal[0], 0.8)
        return pick(pal, 0.3 + 0.6 * c.jit)

    def role_tuft(self, c):
        pal = self.pal("tuft")
        if self.name == "jerboa":
            return pal[0] if c.z < c.d / 2 else pal[1]
        return pick(pal, c.jit) if pal else None

    def role_mane(self, c):
        return self.accent(c, "mane", self.base, -0.2)

    def role_carrot(self, c):
        return self.accent(c, "carrot", P(MC.ORANGE_BEAK))

    def role_crest(self, c):
        return self.coat(c, None, -0.2)

    def role_jaw(self, c):
        if self.name == "cachalot_whale":
            if c.face == "top" and c.jit > 0.5:
                return (230, 228, 220)
        return None

    def role_tentacle(self, c):
        if c.face == "bottom" and c.i % 2 == 0:
            return (240, 220, 210)
        return None

    def role_hump(self, c):
        return None

    def role_eyebump(self, c):
        return None


BLACK_P = P(MC.BLACK)
WHITE = P(MC.WHITE)
PINK_P = P(MC.PINK)


# --------------------------------------------------------------------------------------------
# Relief layers
# --------------------------------------------------------------------------------------------

def overlay_for(p):
    lk = p.look
    style = lk.get("overlay")

    def fur(c, density=0.55, pal=None):
        if c.face == "bottom":
            return None
        if c.role == "head" and c.face == "front":
            return None
        if fbm(c.seed + 3, c.gx * 1.3, c.gy * 1.3, c.gz * 1.3, 2.0, 2) + 0.25 * (c.jit - 0.5) < density:
            return None
        return pick(pal or p.base, 0.15 + 0.7 * c.jit)

    def f(c):
        if lk.get("walker"):
            return walker_overlay(p, c)
        if style == "fur":
            return fur(c, 0.56)
        if style == "shaggy":
            return fur(c, 0.42, p.pal("fur_col"))
        if style == "mane":
            if c.role == "body":
                return fur(c, 0.5, p.pal("mane")) if c.z > c.d * 0.55 else None
            if c.role == "head":
                if c.face == "front" and c.j > 1:
                    return None
                return fur(c, 0.3, p.pal("mane"))
            return fur(c, 0.35, p.pal("mane"))
        if style == "scutes":
            if c.face == "top" and int(c.z) % 2 == 0 and (abs(c.x - c.w / 2) < 0.8 or abs(c.x - c.w / 2) > c.w / 2 - 2):
                return pick(p.base, 0.1 + 0.3 * c.jit)
            if c.role == "tail" and c.face == "top" and int(c.z) % 2 == 0:
                return pick(p.base, 0.2)
            return None
        if style == "ridges":
            if c.role != "shell" or c.face == "bottom":
                return None
            d1, d2 = voronoi(c.seed + 9, c.gx, c.gy, c.gz, 3.2)
            if d1 < 0.9:
                return pick(p.pal("shell"), 0.5 + 0.4 * c.jit)
            return None
        if style == "spiral":
            if c.face in ("left", "right"):
                r = math.hypot(c.y - c.h / 2, c.z - c.d / 2)
                if int(r * 1.3) % 2 == 1:
                    return pick(p.pal("shell"), 0.6 + 0.3 * c.jit)
            return None
        if style == "ruff":
            if c.role == "body" and c.z > c.d - 3 and c.face != "bottom":
                return pick(p.pal("ruff"), c.jit)
            return None
        if style == "fluff_white":
            if c.role == "tail" and c.face != "bottom":
                if abs(c.x - c.w / 2) < 1.1:
                    return pick(WHITE, 0.5 + 0.4 * c.jit)
                return pick(BLACK_P, c.jit) if c.jit > 0.35 else None
            return None
        if style == "moss":
            if c.face == "bottom":
                return None
            if fbm(c.seed + 5, c.gx, c.gy, c.gz, 2.5, 2) > 0.55:
                return pick(P(["#1c3214", "#28461c", "#365c24", "#467030"]), c.jit)
            return None
        if style == "motes":
            if c.face != "bottom" and c.jit > 0.95:
                return mul(lk["eye"], 0.5 + 0.3 * c.jit)
            if c.face != "bottom" and c.jit < 0.3:
                return pick(p.base, 0.6 + 0.4 * c.jit)
            return None
        if style == "snow":
            if c.face == "bottom":
                return None
            if fbm(c.seed + 5, c.gx, c.gy, c.gz, 2.2, 2) > 0.5 or c.face == "top":
                return pick(p.base, 0.6 + 0.4 * c.jit)
            return None
        if style == "rocks":
            if c.face == "bottom":
                return None
            d1, d2 = voronoi(c.seed + 4, c.gx, c.gy, c.gz, 3.0)
            if d1 < 1.1:
                return pick(p.pal("shell"), 0.3 + 0.6 * c.jit)
            return None
        if style == "spines":
            if c.face == "bottom" or (c.face == "front" and c.j > c.fh // 2):
                return None
            if (c.i + c.j * 2) % 3 == 0:
                return None
            return (220, 210, 190) if c.jit > 0.7 else pick(P(["#2a2018", "#3a2e22", "#4a3c2e"]), c.jit)
        return None
    return f


WALKER_STYLES = {
    "vines": 0.52, "stones": 0.58, "coral": 0.56, "flowers": 0.6, "frost": 0.5, "warped": 0.54,
}


def walker_feature(p, c, thr_shift=0.0):
    """The land a void walker lives in, growing on it: vines, stones, coral, flowers, frost."""
    lk = p.look
    acc = P(lk["accent"])
    style = lk["style"]
    if style == "frost":
        if c.face == "top" or (c.face in SIDES and c.j == 0):
            return pick(acc[:2], c.jit)
        if c.jit > 0.9 and c.face != "bottom":
            return acc[2]
        return None
    n = fbm(c.seed + 11, c.gx, c.gy * 0.8, c.gz, 2.4, 2)
    if n < WALKER_STYLES[style] + thr_shift:
        return None
    if style == "vines":
        return acc[0] if (int(c.gx) + int(c.gy)) % 3 else acc[2]
    if style == "stones":
        return pick(acc[:2], c.jit) if c.jit > 0.15 else acc[2]
    if style == "coral":
        return acc[int(c.jit * 3) % 3]
    if style == "flowers":
        if c.jit > 0.8:
            return acc[2]
        return pick(acc[:2], c.jit)
    if style == "warped":
        return acc[2] if c.jit > 0.85 else pick(acc[:2], c.jit)
    return None


def walker_overlay(p, c):
    if c.face == "bottom":
        return None
    return walker_feature(p, c, -0.06)


def walker_painter(p):
    base = G.townsfolk(p.name, G.PAINTERS["skeleton"], dict(BONE=p.base))

    def f(c):
        v = walker_feature(p, c, 0.08) if c.role != "head" or c.face != "front" else None
        if v is not None:
            return v
        if c.role == "head" and c.face == "front" and c.j == 4 and c.i in (1, 2, 5, 6):
            # the walker's glowing eyes, tinted by its land
            return mix((220, 120, 255), P(p.look["accent"])[0], 0.4)
        return base(c)
    return f


# --------------------------------------------------------------------------------------------
# Eggs (same egg outline as tools/gen_eggs.py's style, our two colours per creature)
# --------------------------------------------------------------------------------------------

ROWS = [(6, 9), (5, 10), (4, 11), (4, 11), (3, 12), (3, 12), (3, 12), (2, 13), (2, 13), (2, 13), (2, 13), (3, 12), (3, 12), (4, 11)]
SPOTS = [(6, 4), (9, 6), (5, 8), (10, 10), (7, 11), (4, 11), (11, 7), (8, 8)]


def egg_colors(look):
    base = P(look["base"])
    b = base[len(base) // 2]
    for key in ("accent", "belly", "wing_col", "beak", "shell", "head_col", "skin", "mane", "frill", "claw_col"):
        if look.get(key):
            s = P(look[key])[-1] if key != "beak" else P(look[key])[1]
            break
    else:
        if look.get("pattern"):
            s = P(look["pattern"][1])[0]
        elif look.get("eye"):
            s = look["eye"]
        else:
            s = mul(b, 1.6)
    if sum(abs(x - y) for x, y in zip(b, s)) < 60:
        s = mul(s, 1.8) if sum(s) < 300 else mul(s, 0.4)
    return b, s


def egg(look):
    base, spot = egg_colors(look)
    img = Image.new("RGBA", (16, 16), (0, 0, 0, 0))
    for i, (a, b) in enumerate(ROWS):
        y = i + 1
        for x in range(a, b + 1):
            edge = x in (a, b) or i in (0, len(ROWS) - 1)
            shade = 0.6 if edge else (1.15 if x < 7 and y < 8 else 1.0)
            img.putpixel((x, y), mul(base, shade) + (255,))
    for x, y in SPOTS:
        img.putpixel((x, y), tuple(spot) + (255,))
        img.putpixel((x + 1, y), tuple(spot) + (255,))
    return img


# --------------------------------------------------------------------------------------------
# Driver
# --------------------------------------------------------------------------------------------

def main():
    parsed, parsed_layers = G.parse_mobs(MODS_MOBS)
    specs = {c["name"]: c for c in MC.CREATURES}
    done = []
    for name, parts in parsed.items():
        spec = specs.get(name)
        if spec is None:
            print("skip", name, "(no painter here)")
            continue
        model = spec["model"]
        want = [(p["size"], p["origin"], p["uv"]) for p in model.parts]
        assert parts == want, f"{name}: parts in mods/mobs.ron differ from tools/mod_creatures.py (run it)"
        assert parsed_layers[name] == [(i, float(f)) for i, f in model.layers], f"{name}: layers differ"
        p = Painter(name, spec["look"])
        p.roles = {q["role"] for q in model.parts}
        G.ROLES[name] = [q["role"] for q in model.parts]
        G.PAINTERS[name] = walker_painter(p) if spec["look"].get("walker") else p
        G.OVERLAYS[name] = overlay_for(p)
        img = G.render_skin(name, parts, parsed_layers[name])
        img.save(os.path.join(G.OUT_DIR, f"{name}.png"))
        egg(spec["look"]).save(os.path.join(ITEM_DIR, f"{name}_spawn_egg.png"))
        done.append((name, img))
    print(f"wrote {len(done)} skins and eggs")
    if len(sys.argv) > 1:
        scale, gap, cols = 3, 6, 10
        rows = (len(done) + cols - 1) // cols
        sheet = Image.new("RGBA", (cols * (64 * scale + gap) + gap, rows * (128 * scale + gap) + gap), (24, 24, 28, 255))
        for k, (name, img) in enumerate(done):
            big = img.resize((64 * scale, 128 * scale), Image.NEAREST)
            tile = G.checker(big.size)
            tile.alpha_composite(big)
            sheet.paste(tile, (gap + (k % cols) * (64 * scale + gap), gap + (k // cols) * (128 * scale + gap)))
        sheet.save(sys.argv[1])


if __name__ == "__main__":
    main()
