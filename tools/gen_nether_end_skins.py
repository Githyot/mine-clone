#!/usr/bin/env python3
"""Skins for the Nether and End creatures added in roadmap phase 5 (ghast, blaze, magma
cubes, piglins, hoglin, strider, wither skeleton, the Wither, shulker, endermite and the
dragon). Same 64x128 layout, box unwrap and helpers as tools/gen_mob_skins.py, which this
script borrows; that script skips these mobs.

Usage: python3 tools/gen_nether_end_skins.py   (deterministic)
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gen_mob_skins as g  # noqa: E402
from gen_mob_skins import fbm, pal, pick, mix  # noqa: E402

# Mobs this script paints, with the role of each part (in mobs.ron order).
ROLES = {
    "ghast": ["body"] + ["tentacle"] * 9,
    "blaze": ["head"] + ["rod"] * 8 + ["core"],
    "magma_cube": ["body", "core"],
    "magma_cube_small": ["body", "core"],
    "piglin": ["head", "snout", "ear", "ear", "body", "arm", "arm", "leg", "leg"],
    "piglin_brute": ["head", "snout", "ear", "ear", "body", "arm", "arm", "leg", "leg"],
    "strider": ["body", "leg", "leg"],
    "wither": ["head", "side_head", "side_head", "shoulders", "spine", "rib", "rib", "rib", "tail"],
    "shulker": ["shell", "lid", "head"],
    "endermite": ["body", "head"],
    "ender_dragon": ["body", "neck", "head", "horn", "horn", "wing", "wing", "tail", "tail",
                     "leg", "leg", "leg", "leg"],
}
# Recoloured copies of a model gen_mob_skins.py already paints.
RECOLORED = {
    "wither_skeleton": ("skeleton", dict(BONE=pal("#141216", "#1c1a1e", "#262428", "#302e32", "#3c3a3e"))),
    "hoglin": ("bear", dict(
        BEAR=pal("#7a4a3a", "#8c5644", "#9e644e", "#b07258", "#c08264", "#cc9070"),
        BEAR_MUZZLE=pal("#c49a86", "#d2aa96", "#dcb8a4", "#e6c6b2"))),
}

GHAST = pal("#c8c6c4", "#d6d4d2", "#e2e0de", "#ecebe9", "#f6f5f3")
BLAZE = pal("#b85a10", "#d0741a", "#e49024", "#f2ac34", "#fcd050")
BLAZE_ROD = pal("#c46a10", "#dc8a18", "#f0aa24", "#fcc838")
MAGMA = pal("#2a0e08", "#3c140a", "#4e1c0e", "#602412")
MAGMA_GLOW = pal("#c84a10", "#e86a18", "#ff9a28", "#ffc848")
PIG_SKIN = pal("#c27c74", "#d08c82", "#dc9c90", "#e6ac9e", "#eebcac")
BRUTE_SKIN = pal("#a8665e", "#b6766c", "#c4867a", "#d09688")
GOLD = pal("#9a6c10", "#b88618", "#d4a222", "#ecc032", "#f8d84e")
LOINCLOTH = pal("#5a3a1e", "#6c4624", "#7e522a", "#905e32")
BRUTE_CLOTH = pal("#2a2a30", "#34343c", "#3e3e48", "#4a4a54")
STRIDER = pal("#7a1e1e", "#8e2424", "#a22c2c", "#b63636", "#c84444")
STRIDER_LEG = pal("#4a3a44", "#5a4852", "#6a5662")
WITHER = pal("#121014", "#1a181c", "#222024", "#2c2a2e", "#36343a")
SHULKER = pal("#7c4e7c", "#8e5e8e", "#a06ea0", "#b07eb0", "#c092c0")
SHULKER_HEAD = pal("#d8d8b0", "#e4e2be", "#eeecca", "#f6f4d8")
MITE = pal("#24182e", "#30203c", "#3c284a", "#4a3258")
DRAGON = pal("#0e0a12", "#16101c", "#1e1626", "#281e32", "#32263e")
DRAGON_BELLY = pal("#2a2232", "#362c40", "#42364e")
DRAGON_WING = pal("#2a2234", "#342a40", "#3e324c", "#4a3c5a")
EYE_PURPLE = pal("#a03cd0", "#c060f0", "#e0a0ff")


def n3(c, scale=3.0, salt=0):
    return fbm(c.seed + salt, c.gx, c.gy, c.gz, scale, 3)


def shade(c):
    return {"top": 0.08, "bottom": -0.18, "back": -0.08, "right": -0.04}.get(c.face, 0.0)


def paint_ghast(c):
    t = 0.35 + 0.45 * n3(c, 4.0) + 0.2 * (c.jit - 0.5) + shade(c)
    col = pick(GHAST, t)
    if c.role == "body" and c.face == "front":
        # Two sad closed eyes and a small mouth, low on the face.
        if c.j == 6 and (3 <= c.i <= 5 or 10 <= c.i <= 12):
            return (60, 58, 62)
        if c.j == 5 and c.i in (3, 12):
            return (90, 88, 92)
        if c.j in (10, 11) and 6 <= c.i <= 9:
            return (54, 50, 56) if c.j == 10 else (80, 74, 80)
    if c.role == "tentacle" and c.j >= c.fh - 2:
        return pick(GHAST, t - 0.25)
    return col


def paint_blaze(c):
    if c.role == "head":
        t = 0.3 + 0.5 * n3(c, 2.5) + shade(c)
        if c.face == "front":
            if c.j == 3 and c.i in (1, 2, 5, 6):
                return (40, 20, 6)
            if c.j == 4 and c.i in (1, 2, 5, 6):
                return (255, 236, 120)
            if c.j == 6 and 2 <= c.i <= 5:
                return (90, 40, 8)
        return pick(BLAZE, t)
    if c.role == "core":
        return pick(pal("#3a3036", "#4a3e44", "#5a4c52"), c.jit)
    t = 0.3 + 0.55 * (c.j / max(1, c.fh)) * 0 + 0.6 * c.jit + shade(c)
    return pick(BLAZE_ROD, t)


def paint_magma(c):
    if c.role == "core":
        return pick(MAGMA_GLOW, 0.4 + 0.5 * c.jit)
    # Dark crust in bands with glowing cracks between them.
    band = (c.gy + 0.5) % 2.0 < 0.6
    crack = n3(c, 2.0, 7) > 0.62
    if c.face == "front" and c.j in (2, 3) and c.i in (1, 2, 5, 6):
        return (255, 200, 80) if c.j == 2 else (230, 120, 30)
    if band or crack:
        return pick(MAGMA_GLOW, 0.2 + 0.6 * c.jit)
    return pick(MAGMA, 0.2 + 0.6 * n3(c, 2.0) + shade(c))


def piglin_painter(skin, cloth, brute):
    def paint(c):
        t = 0.3 + 0.45 * n3(c, 3.0) + 0.25 * (c.jit - 0.5) + shade(c)
        if c.role in ("head", "snout", "ear"):
            if c.role == "head" and c.face == "front":
                if c.j == 3 and c.i in (2, 7):
                    return (240, 232, 220)
                if c.j == 3 and c.i in (3, 6):
                    return (30, 20, 16)
                if c.j == 2 and 1 <= c.i <= 8:
                    return pick(skin, t - 0.35)  # heavy brow
                if c.j >= 6 and c.i in (1, 8):
                    return (236, 226, 200)  # tusks
            if c.role == "snout" and c.face == "front" and c.j == 1 and c.i in (1, 2):
                return (60, 30, 28)
            if c.role == "ear" and c.j >= 3:
                return pick(skin, t - 0.2)
            if not brute and c.role == "head" and c.face in ("left", "right") and c.j == 2 and c.i in (3, 4):
                return pick(GOLD, 0.7)  # a gold earring
            return pick(skin, t)
        if c.role == "body":
            if c.j >= 8:
                return pick(cloth, t)
            if not brute and c.j == 7:
                return pick(GOLD, 0.4 + 0.5 * c.jit)  # a gold belt
            if brute and c.face in ("front", "back") and c.j in (1, 2):
                return pick(GOLD, 0.6)
            return pick(skin, t)
        if c.role == "arm":
            if c.j >= 10:
                return pick(skin, t - 0.15)
            if brute and c.j in (6, 7):
                return pick(GOLD, 0.5 + 0.4 * c.jit)  # gold bands
            return pick(skin, t)
        # legs: trousers and dark hooves
        if c.j >= c.fh - 2:
            return (46, 30, 28)
        return pick(cloth, t)
    return paint


def paint_strider(c):
    if c.role == "leg":
        t = 0.3 + 0.5 * c.jit + shade(c)
        if c.j >= c.fh - 2:
            return (40, 28, 32)
        return pick(STRIDER_LEG, t)
    t = 0.3 + 0.45 * n3(c, 3.0) + 0.25 * (c.jit - 0.5) + shade(c)
    if c.face == "front":
        if c.j in (3, 4) and c.i in (3, 4, 9, 10):
            return (30, 16, 16) if c.j == 4 else (250, 236, 210)
        if c.j == 8 and 3 <= c.i <= 10:
            return (60, 18, 18)
    if c.face == "top" and (c.i + c.j) % 3 == 0 and c.jit > 0.5:
        return pick(STRIDER, t + 0.3)
    return pick(STRIDER, t)


def paint_wither(c):
    t = 0.25 + 0.5 * n3(c, 2.5) + 0.25 * (c.jit - 0.5) + shade(c)
    if c.role in ("head", "side_head") and c.face == "front":
        w = c.fw
        eye_row = 2 if c.role == "side_head" else 3
        if c.j == eye_row and c.i in (1, 2, w - 3, w - 2):
            return (6, 4, 8)
        if c.j == eye_row + 1 and c.i in (1, w - 2):
            return EYE_PURPLE[1]
        if c.j == c.fh - 2 and 1 <= c.i <= w - 2 and c.i % 2:
            return (8, 6, 10)
    if c.role == "rib" and c.face in ("front", "back"):
        return pick(WITHER, t + 0.2)
    return pick(WITHER, t)


def paint_shulker(c):
    if c.role == "head":
        t = 0.4 + 0.4 * c.jit + shade(c)
        if c.face == "front" and c.j in (2, 3) and c.i in (1, 4):
            return (40, 30, 40)
        return pick(SHULKER_HEAD, t)
    t = 0.3 + 0.45 * n3(c, 3.0) + 0.25 * (c.jit - 0.5) + shade(c)
    # Shell plates with dark seams.
    if c.face in ("front", "back", "left", "right") and (c.i % 8 == 0 or (c.role == "lid" and c.j == 0)):
        return pick(SHULKER, t - 0.3)
    if c.face in ("top", "bottom") and (c.i % 8 == 0 or c.j % 8 == 0):
        return pick(SHULKER, t - 0.3)
    return pick(SHULKER, t)


def paint_endermite(c):
    t = 0.3 + 0.4 * n3(c, 1.5) + 0.3 * (c.jit - 0.5) + shade(c)
    if c.role == "body" and c.face in ("left", "right", "top") and c.i % 2 == 0:
        return pick(MITE, t - 0.2)
    if c.role == "head" and c.face == "front":
        return EYE_PURPLE[0]
    return pick(MITE, t)


def paint_dragon(c):
    t = 0.25 + 0.45 * n3(c, 3.0) + 0.3 * (c.jit - 0.5) + shade(c)
    if c.role == "wing":
        # Membrane between darker bones.
        if c.face in ("top", "bottom") and (c.i % 5 == 0 or c.j == 0):
            return pick(DRAGON, t - 0.1)
        return pick(DRAGON_WING, t)
    if c.role == "head" and c.face == "front":
        if c.j == 1 and c.i in (1, 2, 5, 6):
            return EYE_PURPLE[1] if c.i in (1, 6) else EYE_PURPLE[2]
        if c.j == 4 and 1 <= c.i <= 6 and c.i % 2:
            return (200, 194, 180)  # teeth
    if c.role == "head" and c.face in ("left", "right") and c.j == 1 and c.i in (5, 6):
        return EYE_PURPLE[1]
    if c.face == "bottom" and c.role in ("body", "neck", "tail"):
        return pick(DRAGON_BELLY, t)
    if c.role in ("body", "neck", "tail") and c.face == "top" and c.i in (c.fw // 2 - 1, c.fw // 2) and c.j % 3 == 0:
        return (150, 140, 160)  # spine ridge
    if c.role == "horn":
        return pick(pal("#9a92a0", "#b4acba", "#ccc4d0"), c.jit)
    if c.role == "leg" and c.j >= c.fh - 2:
        return (120, 112, 126)  # claws
    return pick(DRAGON, t)


def no_overlay(c):
    return None


PAINTERS = {
    "ghast": paint_ghast,
    "blaze": paint_blaze,
    "magma_cube": paint_magma,
    "magma_cube_small": paint_magma,
    "piglin": piglin_painter(PIG_SKIN, LOINCLOTH, False),
    "piglin_brute": piglin_painter(BRUTE_SKIN, BRUTE_CLOTH, True),
    "strider": paint_strider,
    "wither": paint_wither,
    "shulker": paint_shulker,
    "endermite": paint_endermite,
    "ender_dragon": paint_dragon,
}

# The names gen_mob_skins.py leaves to this script.
NAMES = list(ROLES) + list(RECOLORED)


def main():
    parsed, parsed_layers = g.parse_mobs(g.MOBS_RON)
    for name, roles in ROLES.items():
        g.ROLES[name] = roles
        g.PAINTERS[name] = PAINTERS[name]
        g.OVERLAYS[name] = no_overlay
        assert len(parsed[name]) == len(roles), f"{name}: {len(parsed[name])} parts in mobs.ron"
    for name, (base, colours) in RECOLORED.items():
        assert parsed[name] == g.HARDCODED[base], f"{name}: model differs from {base}"
        assert parsed_layers[name] == g.HARDCODED_LAYERS[base], f"{name}: layers differ from {base}"
        g.ROLES[name] = g.ROLES[base]
        g.PAINTERS[name] = g.townsfolk(name, g.PAINTERS[base], colours)
        g.OVERLAYS[name] = g.townsfolk(name, g.OVERLAYS[base], colours)
    for name in NAMES:
        img = g.render_skin(name, parsed[name], parsed_layers[name])
        path = os.path.join(g.OUT_DIR, f"{name}.png")
        img.save(path)
        print("wrote", os.path.relpath(path, g.ROOT))


if __name__ == "__main__":
    main()
