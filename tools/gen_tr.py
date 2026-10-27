#!/usr/bin/env python3
"""The rest of the Terraria side (plan/terraria-completo.md), lote by lote: its blocks, items,
creatures and recipes in packs/vanilla/tr/, the creatures' skins and the 16x16 designs of the
new items and blocks. Everything is drawn here from scratch; tools/tex64 grows the art to 64x64.

Usage:
    python3 tools/gen_tr.py [sheet.png]
    python3 tools/tex64/build.py --only <names it prints>
    python3 -c "import sys; sys.path.insert(0, 'tools/tex64'); import entities; entities.main()"
"""

import os
import sys
from pathlib import Path

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gen_etapa5_skins as e5s  # noqa: E402,F401  (registers the eye/skull painters)
import gen_etapa5_textures as e5  # noqa: E402
import gen_items as gi  # noqa: E402
import gen_mob_skins as g  # noqa: E402
import gen_textures as gt  # noqa: E402
from gen_items import outline, pmap  # noqa: E402
from gen_textures import pal, put  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
PACK = ROOT / "packs" / "vanilla"
OUT = PACK / "tr"
N = 16
from_map = e5.from_map


def known_items():
    import re
    names = set()
    for f in [PACK / "items.ron", PACK / "blocks.ron"] + sorted(PACK.glob("*/items.ron")) + sorted(PACK.glob("*/blocks.ron")):
        if f.parent.name == "tr":
            continue
        names |= set(re.findall(r'\(\s*name:\s*"([a-z0-9_]+)"', f.read_text()))
    return names


def icon(rows, p, prefix="12345"):
    return outline(from_map(rows, pmap(p, prefix)))


# ==============================================================================================
# Lote 1: the early bosses, the jellies, the hive
# ==============================================================================================

GEL = [
    "................",
    "................",
    "................",
    "................",
    "......1111......",
    "....11233211....",
    "...1233544321...",
    "..123345543321..",
    "..123334433321..",
    ".12233333332221.",
    ".12223333322221.",
    "..122222222221..",
    "...1111111111...",
    "................",
    "................",
    "................",
]

LENS = [
    "................",
    "................",
    "................",
    ".....111111.....",
    "....12333321....",
    "...1234553321...",
    "...1235543321...",
    "...1234433321...",
    "...1233333321...",
    "...1233333221...",
    "....12322221....",
    ".....111111.....",
    "................",
    "................",
    "................",
    "................",
]

CROWN = [
    "................",
    "................",
    "................",
    "..1.....1.....1.",
    ".141...151...141",
    ".1331.13331.1331",
    ".13331333331333.",
    ".13333333333331.",
    ".12333343333321.",
    ".12234454432221.",
    ".12223343322221.",
    ".1222222222222..",
    "..11111111111...",
    "................",
    "................",
    "................",
]

SPINE = [
    "................",
    "......11........",
    ".....1331.......",
    "....133331......",
    ".....1331.......",
    "......22........",
    ".....1331.......",
    "....133331......",
    ".....1331.......",
    "......22........",
    ".....1331.......",
    "....134431......",
    ".....1331.......",
    "......22........",
    ".......2........",
    "................",
]

COMB = [
    "................",
    "................",
    "....1111111.....",
    "...123232321....",
    "..12345454321...",
    "..13454545431...",
    ".1234545454321..",
    ".1345454545431..",
    ".1234545454321..",
    "..13454545431...",
    "..12345454321...",
    "...123232321....",
    "....1111111.....",
    "................",
    "................",
    "................",
]

HORN = [
    "................",
    "..1.............",
    ".141............",
    ".1431...........",
    "..1331..........",
    "..13331.........",
    "...13331........",
    "...123331.......",
    "....1233311.....",
    "....12233331....",
    ".....122233331..",
    "......1122222341",
    "........11122341",
    "...........1111.",
    "................",
    "................",
]

TISSUE = [
    "................",
    "................",
    "................",
    "......111.......",
    "....1123311.....",
    "...123344321....",
    "..12334443321...",
    "..12333433221...",
    "...1223332221...",
    "....122222211...",
    ".....1112211....",
    "........11......",
    "................",
    "................",
    "................",
    "................",
]

WAX = [
    "................",
    "................",
    "................",
    "................",
    "....11111111....",
    "...1344444431...",
    "..134555555431..",
    "..134444444431..",
    "..123333333321..",
    "..122222222221..",
    "...1111111111...",
    "................",
    "................",
    "................",
    "................",
    "................",
]

SHARD = [
    "................",
    "..........11....",
    ".........1451...",
    "........14531...",
    ".......145321...",
    "......145321....",
    ".....145321.....",
    "....145321......",
    "...144321.......",
    "..14321.........",
    "..1321..........",
    "...11...........",
    "................",
    "................",
    "................",
    "................",
]

ACC1 = {
    "royal_gel": ([
        "................",
        "................",
        "........1.......",
        ".......141......",
        "......13331.....",
        ".....1333331....",
        "....133454331...",
        "...13345543331..",
        "...13334433331..",
        "..1223333333221.",
        "..1222333332221.",
        "...12222222221..",
        "....111111111...",
        "................",
        "................",
        "................",
    ], pal("#0e2a5a", "#1e5ab0", "#4a8ae0", "#a8d4ff", "#ffffff")),
    "watcher_shield": ([
        "................",
        "..111111111111..",
        "..122222222221..",
        "..123333333321..",
        "..123311113321..",
        "..123145541321..",
        "..121455554121..",
        "..121455554121..",
        "..123145541321..",
        "..123311113321..",
        "...1233333321...",
        "...1223333221...",
        "....12233221....",
        ".....122221.....",
        "......1111......",
        "................",
    ], pal("#2a1a12", "#5a3a2a", "#8a6a4a", "#7a0a14", "#e04040")),
    "mind_ward": ([
        "................",
        "................",
        "....11111111....",
        "...1233243321...",
        "..123324433321..",
        "..133243324331..",
        ".1332433243331..",
        ".1324332433321..",
        ".1333243324331..",
        "..132433243321..",
        "..123324433221..",
        "...1222222221...",
        "....11155111....",
        "......1551......",
        ".......11.......",
        "................",
    ], pal("#3a0810", "#8a1a2a", "#c84a5a", "#f08a9a", "#ffd0a0")),
    "hive_pack": ([
        "................",
        "....11111111....",
        "...1222222221...",
        "..123434343421..",
        "..124343434321..",
        "..123434343421..",
        "..124343434321..",
        "..123434343421..",
        "..124343434321..",
        "..123434343421..",
        "..122222222221..",
        "..1.1111111.1...",
        "..1.........1...",
        "..11.......11...",
        "................",
        "................",
    ], pal("#2a1a08", "#6a4a14", "#c8901c", "#f0c040", "#fff0a0")),
    "fur_cloak": ([
        "................",
        "....11111111....",
        "...1455555541...",
        "..145444444541..",
        "..143333333341..",
        ".14333333333341.",
        ".13332333323331.",
        ".13323333332331.",
        ".13333333333331.",
        ".12333333333321.",
        ".12233322333221.",
        "..122221122221..",
        "...1111..1111...",
        "................",
        "................",
        "................",
    ], pal("#2a3440", "#5a6a7a", "#8a9aaa", "#c8d4e0", "#f4f8fc")),
}

JELLY_PALS = {
    "jelly_green": pal("#1a4a12", "#2e7a1e", "#4aa832", "#7ad458", "#c4f4a4"),
    "jelly_blue": pal("#0e2a5a", "#1e4aa0", "#3a7ad8", "#7ab0f4", "#c8e4ff"),
    "jelly_red": pal("#4a0a0a", "#8a1a1a", "#c43030", "#e86a5a", "#ffc0b0"),
    "jelly_ice": pal("#3a5a74", "#6a94b4", "#9ac4e0", "#c8e8f8", "#f4fcff"),
    "jelly_lava": pal("#3a0a02", "#8a2a04", "#d0560a", "#f89a2a", "#fff0a0"),
    "jelly_pink": pal("#5a1a3a", "#a03a6a", "#d86a9a", "#f4a4c8", "#ffe0f0"),
    "jelly_spiked": pal("#0a1e4a", "#16388a", "#2a5ac4", "#5a8ae8", "#a8ccff"),
    "king_jelly": pal("#0a1e4a", "#163a8e", "#2a62cc", "#5a94ec", "#b4d8ff"),
}

# (name, display, kind, health, speed, attack, extra fields, drops, xp)
JELLY_MOBS = [
    ("jelly_green", "Geleia Verde", 14, 2.2, 2, 'spawn: Grass, weight: 4, group: (1, 2), biomes: ["Plains", "Forest", "Meadow", "Birch Forest", "Flower Forest"]', '[(item: "gel", min: 1, max: 3)]', 2, 1.6),
    ("jelly_blue", "Geleia Azul", 20, 2.4, 3, 'spawn: Grass, weight: 3, group: (1, 2)', '[(item: "gel", min: 1, max: 3)]', 3, 1.8),
    ("jelly_red", "Geleia Vermelha", 30, 2.6, 4, "spawn: Never", '[(item: "gel", min: 2, max: 4)]', 5, 1.8),
    ("jelly_ice", "Geleia de Gelo", 34, 2.4, 4, 'spawn: Dark, weight: 4, biomes: ["Snowy Plains", "Ice Spikes", "Snowy Taiga", "Snowy Peaks", "Snowy Slopes", "Grove"]', '[(item: "gel", min: 1, max: 3), (item: "snowball", min: 0, max: 2)]', 5, 1.8),
    ("jelly_lava", "Geleia de Lava", 50, 2.8, 6, "spawn: Never", '[(item: "gel", min: 2, max: 4), (item: "magma_cream", chance: 0.3)]', 8, 1.8),
    ("jelly_pink", "Geleia Rosa", 60, 2.8, 5, 'spawn: Grass, weight: 1, group: (1, 1)', '[(item: "gel", min: 5, max: 9)]', 10, 1.0),
    ("jelly_spiked", "Geleia Espinhosa", 24, 2.8, 4, "spawn: Never", '[(item: "gel", min: 1, max: 2)]', 3, 1.6),
]

JELLY_PARTS = """[
            (size: (12, 10, 12), origin: (-6, 0, -6), uv: (0, 0)),
        ]"""
JELLY_LAYERS = "[(part: 0, inflate: 0.5)]"

KING_PARTS = """[
            (size: (12, 10, 12), origin: (-6, 0, -6), uv: (0, 0)),
            (size: (8, 3, 8), origin: (-4, 10, -4), uv: (0, 24)),
        ]"""

EYE_PARTS = """[
            (size: (16, 16, 16), origin: (-8, 0, -8), uv: (0, 0), anim: Head, pivot: (0, 8, 0)),
            (size: (2, 2, 8), origin: (-5, 7, -16), uv: (0, 32), anim: Head, pivot: (0, 8, 0)),
            (size: (2, 2, 8), origin: (3, 9, -16), uv: (0, 32), anim: Head, pivot: (0, 8, 0)),
            (size: (2, 2, 8), origin: (-1, 3, -16), uv: (0, 32), anim: Head, pivot: (0, 8, 0)),
        ]"""

BRAIN_PARTS = """[
            (size: (14, 10, 12), origin: (-7, 8, -6), uv: (0, 0), anim: Head, pivot: (0, 8, 0)),
            (size: (6, 8, 6), origin: (-3, 0, -3), uv: (0, 24), anim: Head, pivot: (0, 8, 0)),
        ]"""

CREEPER_PARTS = """[
            (size: (8, 8, 8), origin: (-4, 0, -4), uv: (0, 0), anim: Head, pivot: (0, 4, 0)),
            (size: (2, 2, 6), origin: (-1, 3, -10), uv: (0, 16), anim: Head, pivot: (0, 4, 0)),
        ]"""

BEE_PARTS = """[
            (size: (10, 10, 14), origin: (-5, 4, -14), uv: (0, 0)),
            (size: (8, 8, 8), origin: (-4, 6, 0), uv: (0, 24)),
            (size: (6, 6, 5), origin: (-3, 7, 8), uv: (32, 24), anim: Head, pivot: (0, 10, 8)),
            (size: (12, 1, 8), origin: (-14, 14, -2), uv: (0, 40), anim: Wing(0), pivot: (-3, 14, 2)),
            (size: (12, 1, 8), origin: (2, 14, -2), uv: (0, 40), anim: Wing(1), pivot: (3, 14, 2)),
            (size: (2, 2, 4), origin: (-1, 7, -18), uv: (40, 40)),
        ]"""

COLOSSUS_PARTS = """[
            (size: (12, 14, 8), origin: (-6, 14, -4), uv: (0, 0)),
            (size: (10, 8, 8), origin: (-5, 28, -3), uv: (0, 22), anim: Head, pivot: (0, 28, 0)),
            (size: (2, 8, 2), origin: (-7, 35, 0), uv: (36, 22), anim: Head, pivot: (0, 28, 0)),
            (size: (2, 8, 2), origin: (5, 35, 0), uv: (36, 22), anim: Head, pivot: (0, 28, 0)),
            (size: (4, 16, 4), origin: (-10, 12, -2), uv: (44, 22), anim: Arm(0), pivot: (-8, 26, 0)),
            (size: (4, 16, 4), origin: (6, 12, -2), uv: (44, 22), anim: Arm(1), pivot: (8, 26, 0)),
            (size: (5, 14, 5), origin: (-6, 0, -2), uv: (0, 40), anim: Leg(0), pivot: (-3, 14, 0)),
            (size: (5, 14, 5), origin: (1, 0, -2), uv: (0, 40), anim: Leg(1), pivot: (3, 14, 0)),
        ]"""


def mobs_l1():
    out = []
    for name, display, health, speed, attack, extra, drops, xp, scale in JELLY_MOBS:
        out.append(f"""    (
        name: "{name}", display_name: "{display}", kind: Hostile, health: {health}, speed: {speed}, attack: {attack},
        drops: {drops}, xp: {xp},
        width: {0.75 * scale:.2f}, height: {0.62 * scale:.2f}, {extra}, hops: true, scale: {scale},
        parts: {JELLY_PARTS},
        layers: {JELLY_LAYERS},
    ),""")
    out.append(f"""    // The Jelly King: a huge hopping jelly with a gold crown. Sheds jellies as it is hurt.
    (
        name: "king_jelly", display_name: "Rei Geleia", kind: Hostile, health: 420, speed: 2.0, attack: 6,
        drops: [(item: "gel", min: 20, max: 40), (item: "jelly_hook", chance: 0.5), (item: "jelly_staff", chance: 0.5), (item: "royal_gel"), (item: "lesser_healing_potion", min: 3, max: 6), (item: "ninja_helmet", chance: 0.34), (item: "ninja_chestplate", chance: 0.34), (item: "ninja_leggings", chance: 0.34), (item: "ninja_boots", chance: 0.34), (item: "jelly_saddle", chance: 0.25), (item: "jelly_bubble", chance: 0.2)], xp: 90,
        width: 3.2, height: 2.6, spawn: Never, hops: true, scale: 4.5, boss: true,
        parts: {KING_PARTS},
        layers: [(part: 0, inflate: 0.6), (part: 1, inflate: 0.4)],
    ),
    // The Great Eye: hovers, stares, dashes; at half health it opens a mouth of teeth.
    (
        name: "great_eye", display_name: "Grande Olho", kind: Hostile, health: 560, speed: 0.1, attack: 7,
        drops: [(item: "lens", min: 4, max: 8), (item: "watcher_shield"), (item: "emerald", min: 3, max: 6), (item: "life_crystal", chance: 0.3), (item: "eye_spring", chance: 0.2)], xp: 110,
        width: 2.2, height: 2.2, spawn: Never, scale: 2.4, boss: true, flies: true,
        parts: {EYE_PARTS},
        layers: [(part: 0, inflate: 0.5)],
    ),
    (
        name: "servant_eye", display_name: "Olho Servo", kind: Hostile, health: 10, speed: 0.1, attack: 3,
        drops: [(item: "lens", chance: 0.15)], xp: 2,
        width: 0.5, height: 0.5, spawn: Never, scale: 0.5, flies: true,
        parts: {EYE_PARTS},
        layers: [(part: 0, inflate: 0.5)],
    ),
    // The Blight Mind: a floating brain guarded by its creepers; when they are gone it hunts you.
    (
        name: "blight_mind", display_name: "Mente da Chaga", kind: Hostile, health: 600, speed: 0.1, attack: 6,
        drops: [(item: "blight_tissue", min: 10, max: 20), (item: "mind_ward"), (item: "emerald", min: 3, max: 6)], xp: 120,
        width: 2.2, height: 2.2, spawn: Never, scale: 2.4, boss: true, flies: true,
        parts: {BRAIN_PARTS},
        layers: [(part: 0, inflate: 0.6)],
    ),
    (
        name: "blight_creeper", display_name: "Rastejante da Chaga", kind: Hostile, health: 26, speed: 0.1, attack: 4,
        drops: [(item: "blight_tissue", chance: 0.25)], xp: 3,
        width: 0.7, height: 0.7, spawn: Never, scale: 1.0, flies: true,
        parts: {CREEPER_PARTS},
        layers: [(part: 0, inflate: 0.4)],
    ),
    // The Hive Queen: woken from her larva deep in the jungle; charges, stings, calls her drones.
    (
        name: "hive_queen", display_name: "Rainha da Colmeia", kind: Hostile, health: 680, speed: 0.1, attack: 7,
        drops: [(item: "beeswax", min: 10, max: 17), (item: "hive_staff", chance: 0.4), (item: "hive_blade", chance: 0.4), (item: "hive_pack"), (item: "honeycomb", min: 10, max: 20), (item: "honey_goggles", chance: 0.15), (item: "honey_jar", chance: 0.2)], xp: 140,
        width: 2.6, height: 2.0, spawn: Never, scale: 2.6, boss: true, flies: true,
        parts: {BEE_PARTS},
        layers: [(part: 0, inflate: 0.5), (part: 1, inflate: 0.5)],
    ),
    (
        name: "hive_drone", display_name: "Zangão", kind: Hostile, health: 12, speed: 0.1, attack: 3,
        drops: [(item: "honeycomb", chance: 0.1)], xp: 2,
        width: 0.6, height: 0.5, spawn: Never, scale: 0.6, flies: true,
        parts: {BEE_PARTS},
        layers: [],
    ),
    // The Frost Colossus: a one-eyed giant of the snow, with antlers. Ice spikes, frost nova, roars.
    (
        name: "frost_colossus", display_name: "Colosso do Gelo", kind: Hostile, health: 760, speed: 1.6, attack: 8,
        drops: [(item: "frost_shard", min: 6, max: 12), (item: "colossus_staff", chance: 0.4), (item: "fur_cloak"), (item: "packed_ice", min: 5, max: 10)], xp: 150,
        width: 2.4, height: 5.6, spawn: Never, scale: 2.2, boss: true,
        parts: {COLOSSUS_PARTS},
        layers: [(part: 0, inflate: 0.7), (part: 1, inflate: 0.5), (part: 4, inflate: 0.4), (part: 6, inflate: 0.4)],
    ),""")
    return out


def jelly_painter(p, crown=None):
    def paint(c):
        if c.role == "crown":
            k = 0.3 + 0.5 * c.jit + (0.2 if c.face == "top" else 0.0)
            if c.face in ("front", "back", "left", "right") and c.j == 0 and c.i % 3 == 1:
                return (220, 40, 60)
            return g.pick(crown, k)
        n = g.fbm(c.seed, c.gx, c.gy, c.gz, 5.0)
        t = 0.25 + 0.5 * n + 0.15 * (c.jit - 0.5)
        if c.face == "top":
            t += 0.25
        elif c.face == "bottom":
            t -= 0.3
        # A darker core shows through low on the sides.
        if c.face in ("front", "back", "left", "right") and c.j > c.fh * 0.55 and 0.25 < c.i / c.fw < 0.75:
            t -= 0.2
        col = g.pick(p, t)
        if c.face == "front":
            ey = c.fh // 3
            for ex in (c.fw // 2 - 3, c.fw // 2 + 2):
                if c.i == ex and ey <= c.j <= ey + 1:
                    return (14, 12, 18)
            # A highlight glint on the upper corner.
            if c.j == 1 and c.i in (1, 2):
                return g.pick(p, 1.0)
        return col
    return paint


def jelly_overlay(p, spikes=False):
    def overlay(c):
        if spikes and c.face in ("front", "back", "left", "right", "top") and (c.i + 2 * c.j) % 5 == 0 and c.jit > 0.3:
            return g.pick(pal("#c8d8f0", "#e8f0ff"), c.jit)
        if c.face == "top" and c.jit > 0.55:
            return g.pick(p, 0.85 + 0.15 * c.jit)
        if c.face in ("left", "right", "back") and c.j < 3 and c.jit > 0.7:
            return g.pick(p, 0.7)
        return None
    return overlay


def brain_painter(base, groove, eye=None):
    def paint(c):
        if c.role == "stem":
            return g.pick(groove, 0.3 + 0.5 * c.jit)
        # Folds: dark grooves winding over the surface.
        w = np.sin(c.gx * 0.9 + np.sin(c.gz * 0.7) * 2.0) + np.cos(c.gz * 0.8 + c.gy * 0.6)
        col = g.pick(base, 0.35 + 0.5 * c.jit)
        if abs(w) < 0.35:
            col = g.pick(groove, 0.4 + 0.4 * c.jit)
        if eye and c.face == "front":
            cx, cy = c.i - (c.fw - 1) / 2, c.j - (c.fh - 1) / 2
            r2 = cx * cx + cy * cy
            if r2 < 2.5:
                return (10, 6, 8)
            if r2 < 9:
                return eye
            if r2 < 13:
                return (230, 220, 210)
        return col
    return paint


def brain_overlay(base):
    def overlay(c):
        if c.face in ("top", "left", "right", "back") and c.jit > 0.62:
            return g.pick(base, 0.55 + 0.4 * c.jit)
        return None
    return overlay


QUEEN_BODY = pal("#3a2204", "#6a3e08", "#a0640e", "#d49a1e", "#f4c84a")
QUEEN_BAND = pal("#120a04", "#1e1208", "#2a1a0c", "#36220e")
QUEEN_WING = pal("#a8b8c4", "#c4d2dc", "#dce6ee", "#eef4f8")


def queen_painter(c):
    if c.role == "wing":
        if (c.i + c.j) % 5 == 0:
            return g.pick(QUEEN_WING, 0.1)
        return g.pick(QUEEN_WING, 0.4 + 0.5 * c.jit)
    if c.role == "stinger":
        return g.pick(QUEEN_BAND, 0.2 + 0.4 * c.jit)
    if c.role == "head":
        if c.face == "front" and c.j in (1, 2) and c.i in (0, 1, c.fw - 2, c.fw - 1):
            return (30, 10, 12)
        if c.face == "top" and c.i % 2 == 0 and c.jit > 0.4:
            return g.pick(pal("#c49418", "#fce47a"), c.jit)
        return g.pick(QUEEN_BAND, 0.4 + 0.5 * c.jit)
    if c.role == "thorax":
        return g.pick(QUEEN_BODY if c.jit > 0.35 else QUEEN_BAND, 0.4 + 0.4 * c.jit)
    # The abdomen: bands across its length.
    band = int(c.gz + 40) // 3 % 2 == 0 and c.face not in ("back", "front")
    t = 0.3 + 0.5 * c.jit + (0.15 if c.face == "top" else 0.0) - (0.25 if c.face == "bottom" else 0.0)
    return g.pick(QUEEN_BAND if band else QUEEN_BODY, t)


def queen_overlay(c):
    if c.face == "top" and c.jit > 0.5:
        return g.pick(pal("#d49a1e", "#f4c84a", "#ffe8a0"), c.jit)
    if c.face in ("left", "right") and c.j < 2 and c.jit > 0.6:
        return g.pick(QUEEN_BODY, 0.6)
    return None


FUR = pal("#5a6878", "#7a8a9a", "#9aaaba", "#bccad6", "#dfe8ef")
FUR_DARK = pal("#1e242c", "#2a323c", "#36404c", "#44505c")
ANTLER = pal("#5a4a3a", "#7a6a56", "#9a8a72", "#bcae94")


def colossus_painter(c):
    if c.role == "antler":
        return g.pick(ANTLER, 0.3 + 0.5 * c.jit)
    n = g.fbm(c.seed, c.gx, c.gy, c.gz, 3.0)
    t = 0.25 + 0.55 * n + 0.2 * (c.jit - 0.5)
    if c.face == "bottom":
        t -= 0.3
    if c.role == "head":
        if c.face == "front":
            # One great eye in a dark face.
            cx, cy = c.i - (c.fw - 1) / 2, c.j - 3
            r2 = cx * cx + cy * cy
            if r2 < 1.1:
                return (10, 8, 6)
            if r2 < 3.3:
                return (220, 60, 40)
            if r2 < 5.5:
                return (240, 220, 190)
            if c.j >= 6 and c.i % 2 == 0:
                return (226, 220, 206)
            return g.pick(FUR_DARK, 0.4 + 0.4 * c.jit)
        return g.pick(FUR, t)
    if c.role in ("arm", "leg") and c.j > c.fh - 3 and c.face != "top":
        return g.pick(FUR_DARK, 0.3 + 0.5 * c.jit)
    if c.role == "body" and c.face == "front" and 2 < c.i < c.fw - 3 and c.j > 3:
        t += 0.2
    return g.pick(FUR, t)


def colossus_overlay(c):
    # Shaggy tufts of fur.
    if c.face in ("front", "back", "left", "right") and (c.j % 3 == 2) and c.jit > 0.45:
        return g.pick(FUR, 0.5 + 0.5 * c.jit)
    if c.face == "top" and c.jit > 0.4:
        return g.pick(FUR, 0.8)
    return None


def skins_l1():
    """name -> (roles, painter, overlay)"""
    out = {}
    for name, p in JELLY_PALS.items():
        if name == "king_jelly":
            out[name] = (["jelly", "crown"], jelly_painter(p, pal("#7a5408", "#c49418", "#ecc434", "#fce47a")), jelly_overlay(p))
        else:
            out[name] = (["jelly"], jelly_painter(p), jelly_overlay(p, spikes=name == "jelly_spiked"))
    vein = pal("#6a0a12", "#8a1420", "#a8202c")
    out["great_eye"] = (["eye", "stalk", "stalk", "stalk"],
                        e5s.eye_painter(pal("#cfc6bb", "#ddd5cb", "#ece6de"), vein, pal("#2a6a2a", "#3a9a3a", "#6ac46a")),
                        e5s.eye_overlay(vein))
    out["servant_eye"] = (["eye", "stalk", "stalk", "stalk"],
                          e5s.eye_painter(pal("#c8c0b8", "#d8d0c8", "#e8e2da"), vein, pal("#6a3a0a", "#9a5a14", "#c4842a")),
                          e5s.eye_overlay(vein))
    flesh = pal("#7a1a2a", "#9a2a3a", "#b83e4e", "#d05a66", "#e47a82")
    dark = pal("#3a0810", "#4e0e18", "#621420")
    out["blight_mind"] = (["brain", "stem"], brain_painter(flesh, dark), brain_overlay(flesh))
    out["blight_creeper"] = (["brain", "stem"], brain_painter(flesh, dark, eye=(200, 40, 40)), brain_overlay(flesh))
    for name in ("hive_queen", "hive_drone"):
        out[name] = (["abdomen", "thorax", "head", "wing", "wing", "stinger"], queen_painter, queen_overlay)
    out["frost_colossus"] = (["body", "head", "antler", "antler", "arm", "arm", "leg", "leg"], colossus_painter, colossus_overlay)
    return out


def items_l1():
    """(name, display, icon, extra RON fields)"""
    jelly = JELLY_PALS["jelly_blue"]
    gi.MATERIALS["hive"] = pal("#3a2204", "#6a3e08", "#a0640e", "#d49a1e", "#f4c84a")
    gi.MATERIALS["frost"] = pal("#2a4a6a", "#4a7aa0", "#7aaed4", "#b4daf4", "#eef8ff")
    gi.WAND_GEMS["honey"] = pal("#6a3e08", "#a0640e", "#d49a1e", "#f4c84a", "#fff0a0")
    gi.WAND_GEMS["frost"] = pal("#2a4a6a", "#4a7aa0", "#7aaed4", "#b4daf4", "#eef8ff")
    gi.WAND_GEMS["jelly"] = jelly
    return [
        ("gel", "Gel", icon(GEL, jelly), ""),
        ("lens", "Lente", icon(LENS, pal("#3a2a2a", "#8a8080", "#c8c0c0", "#ece8e8", "#ffffff")), ""),
        ("jelly_crown", "Coroa Gelatinosa", icon(CROWN, pal("#5a3a04", "#a07010", "#e0b020", "#2a62cc", "#b4d8ff")), "max_stack: Some(16)"),
        ("eye_lure", "Olho Suspeito", e5.eye("#e8e2da", "#3a9a3a"), "max_stack: Some(16)"),
        ("blight_spine", "Espinha da Chaga", icon(SPINE, pal("#3a0810", "#7a1a2a", "#c84a5a", "#f08a9a")), "max_stack: Some(16)"),
        ("royal_comb", "Favo Real", icon(COMB, pal("#3a2204", "#8a5a0e", "#d49a1e", "#f4c84a", "#fff0a0")), "max_stack: Some(16)"),
        ("frost_horn", "Chifre Gelado", icon(HORN, pal("#2a3440", "#7a8a9a", "#c8d4e0", "#ffffff")), "max_stack: Some(16)"),
        ("blight_tissue", "Tecido da Chaga", icon(TISSUE, pal("#3a0810", "#7a1a2a", "#b83e4e", "#e47a82")), ""),
        ("beeswax", "Cera de Abelha", icon(WAX, pal("#3a2204", "#8a5a0e", "#c8901c", "#f0c040", "#fff0a0")), ""),
        ("frost_shard", "Lasca Gelada", icon(SHARD, pal("#2a4a6a", "#4a7aa0", "#7aaed4", "#b4daf4", "#ffffff")), ""),
        ("hive_staff", "Cajado da Colmeia", gi.staff("honey", "hive"), "max_stack: Some(1)"),
        ("colossus_staff", "Cajado do Colosso", gi.staff("frost", "frost"), "max_stack: Some(1)"),
        ("jelly_staff", "Cajado Gelatinoso", gi.staff("jelly", "titanium" if "titanium" in gi.MATERIALS else "iron"), "max_stack: Some(1)"),
        ("minion_jelly", "Geleinha", icon(GEL, JELLY_PALS["jelly_blue"]), "in_inventory: Some(false)"),
        ("hive_blade", "Lâmina da Colmeia", gi.tool("hive", "sword"), "max_stack: Some(1), tool: Some((kind: Sword, tier: 3, speed: 8.0, damage: 7.0, durability: 700))"),
        ("jelly_hook", "Gancho Gelatinoso", e5.hook("sapphire"), "max_stack: Some(1)"),
    ] + [(n, d, outline(from_map(ACC1[n][0], pmap(ACC1[n][1]))), "max_stack: Some(1)") for n, d in [
        ("royal_gel", "Gel Real"), ("watcher_shield", "Escudo do Vigia"), ("mind_ward", "Guarda da Mente"),
        ("hive_pack", "Mochila da Colmeia"), ("fur_cloak", "Manto de Pelo"),
    ]]


def block_noise(seed, base, p, spots=0.0):
    rng = np.random.default_rng(seed)
    img = gt.canvas()
    n = rng.random((N, N))
    for y in range(N):
        for x in range(N):
            t = base(x, y) + 0.25 * (n[y, x] - 0.5)
            k = int(np.clip(t, 0, 0.999) * len(p))
            put(img, x, y, p[k])
    return img


def blocks_l1():
    hive = pal("#4a2a06", "#7a4a0c", "#a86c14", "#d49a1e", "#f0c040")

    def hexes(x, y):
        # Honeycomb cells: a hexagonal lattice of rims and darker hollows.
        row = y // 4
        xx = x + (2 if row % 2 else 0)
        cx, cy = xx % 4 - 1.5, y % 4 - 1.5
        rim = abs(cx) > 1.0 or abs(cy) > 1.0
        return 0.75 if rim else 0.3

    larva = pal("#7a6a4a", "#b8a478", "#e0d0a4", "#f4ecd0", "#fffcf0")

    def grub(x, y):
        cx, cy = (x - 7.5) / 6.0, (y - 7.5) / 4.5
        r = cx * cx + cy * cy
        if r > 1.0:
            return hexes(x, y) * 0.6
        return 0.9 - 0.4 * r + (0.15 if x % 3 == 0 else 0.0)

    honey = pal("#8a4a04", "#c4780a", "#e8a018", "#f8c84a", "#fff0a0")

    def glossy(x, y):
        return 0.45 + 0.3 * np.sin((x + y * 0.6) * 0.7) * np.cos(y * 0.5) + (0.35 if (x - y) % 11 == 0 else 0.0)

    return {
        "hive_block": block_noise(11, hexes, hive),
        "hive_larva": block_noise(12, grub, larva),
        "hive_honey": block_noise(13, glossy, honey),
    }


BLOCKS_L1 = [
    '(name: "hive_block", display_name: Some("Bloco de Colmeia"), hardness: 0.8, tool: Some(Axe))',
    '(name: "hive_larva", display_name: Some("Larva Real"), light: 6, hardness: 0.4, drops: Some([]))',
    '(name: "hive_honey", display_name: Some("Mel da Colmeia"), light: 9, hardness: 0.5, tool: Some(Shovel), drops: Some([(item: "honeycomb", min: 1, max: 2)]))',
]

SHAPED_L1 = [
    (["GGG", "GCG", "GGG"], {"G": "gel", "C": "gold_ingot"}, "jelly_crown", 1),
    (["G", "S"], {"G": "gel", "S": "stick"}, "torch", 3),
    (["GGG", "GSG", "GGG"], {"G": "gel", "S": "slime_ball"}, "jelly_staff", 1),
]
SHAPELESS_L1 = [
    (["lens", "lens", "lens", "lens", "lens", "lens", "spider_eye"], "eye_lure", 1),
    (["rotten_flesh", "rotten_flesh", "rotten_flesh", "bone", "bone", "bone", "redstone", "redstone"], "blight_spine", 1),
    (["honeycomb", "honeycomb", "honeycomb", "honeycomb", "honeycomb", "hive_block", "gold_nugget"], "royal_comb", 1),
    (["packed_ice", "packed_ice", "packed_ice", "lens", "bone", "bone", "leather"], "frost_horn", 1),
    (["honeycomb", "honeycomb", "honeycomb", "honeycomb"], "hive_block", 2),
    (["beeswax", "beeswax", "string"], "candle", 2),
    (["gel", "gel", "gel", "lens"], "jelly_hook", 1),
]

# ==============================================================================================
# Lote 2: events (meteor, falling stars, jelly rain, sandstorm, lantern night, eclipse)
# ==============================================================================================

METEOR = pal("#2a1a14", "#4a2e22", "#6e4030", "#a8562c", "#f08a2a")

STAR = [
    "................",
    ".......1........",
    ".......1........",
    "......121.......",
    "......131.......",
    ".11111343111111.",
    "..1233454433321.",
    "...12345543321..",
    "....123443321...",
    "....12333321....",
    "...1233113321...",
    "...12311.13321..",
    "..1221....1221..",
    "..111......111..",
    "................",
    "................",
]

TABLET = [
    "................",
    "...1111111111...",
    "..122222222221..",
    "..123333333321..",
    "..123344443321..",
    "..123455554321..",
    "..134551155431..",
    "..134511115431..",
    "..134511115431..",
    "..134551155431..",
    "..123455554321..",
    "..123344443321..",
    "..123333333321..",
    "..122222222221..",
    "...1111111111...",
    "................",
]

DARK_SHARD = [
    "................",
    "................",
    ".........11.....",
    "........1441....",
    ".......14531....",
    "......145321....",
    ".....1453221....",
    "....14532221....",
    "...145322221....",
    "...14322221.....",
    "....1322221.....",
    ".....12221......",
    "......111.......",
    "................",
    "................",
    "................",
]

HEAD_PARTS = """[
            (size: (8, 8, 8), origin: (-4, 0, -4), uv: (0, 0), anim: Head, pivot: (0, 4, 0)),
        ]"""

WRAITH_PARTS = """[
            (size: (8, 8, 8), origin: (-4, 12, -4), uv: (0, 0), anim: Head, pivot: (0, 12, 0)),
            (size: (6, 12, 6), origin: (-3, 0, -3), uv: (0, 16)),
        ]"""

REAPER_PARTS = """[
            (size: (8, 8, 8), origin: (-4, 18, -4), uv: (0, 0), anim: Head, pivot: (0, 18, 0)),
            (size: (8, 18, 6), origin: (-4, 0, -3), uv: (0, 16)),
            (size: (2, 18, 2), origin: (5, 4, 2), uv: (32, 0), anim: Arm(0), pivot: (6, 16, 3)),
            (size: (8, 1, 2), origin: (5, 21, 0), uv: (40, 0), anim: Arm(0), pivot: (6, 16, 3)),
        ]"""

BRUTE_PARTS = """[
            (size: (12, 12, 8), origin: (-6, 10, -4), uv: (0, 0)),
            (size: (8, 8, 8), origin: (-4, 22, -3), uv: (0, 20), anim: Head, pivot: (0, 22, 0)),
            (size: (4, 14, 4), origin: (-10, 8, -2), uv: (32, 20), anim: Arm(0), pivot: (-8, 21, 0)),
            (size: (4, 14, 4), origin: (6, 8, -2), uv: (32, 20), anim: Arm(1), pivot: (8, 21, 0)),
            (size: (5, 10, 5), origin: (-6, 0, -2), uv: (0, 40), anim: Leg(0), pivot: (-3, 10, 0)),
            (size: (5, 10, 5), origin: (1, 0, -2), uv: (0, 40), anim: Leg(1), pivot: (3, 10, 0)),
        ]"""

MOTH_PARTS = """[
            (size: (8, 8, 16), origin: (-4, 4, -8), uv: (0, 0)),
            (size: (16, 1, 12), origin: (-20, 10, -4), uv: (0, 24), anim: Wing(0), pivot: (-4, 10, 2)),
            (size: (16, 1, 12), origin: (4, 10, -4), uv: (0, 24), anim: Wing(1), pivot: (4, 10, 2)),
            (size: (6, 6, 6), origin: (-3, 5, 8), uv: (0, 40), anim: Head, pivot: (0, 8, 8)),
        ]"""


def mobs_l2():
    return [f"""    // Lote 2: events.
    (
        name: "meteor_head", display_name: "Cabeça de Meteoro", kind: Hostile, health: 30, speed: 0.1, attack: 5,
        drops: [(item: "meteorite_ore", chance: 0.3)], xp: 4,
        width: 1.0, height: 1.0, spawn: Never, scale: 1.7, flies: true,
        parts: {HEAD_PARTS},
        layers: [(part: 0, inflate: 0.6)],
    ),
    (
        name: "dune_wraith", display_name: "Espírito da Areia", kind: Hostile, health: 44, speed: 0.1, attack: 6,
        drops: [(item: "sand", min: 1, max: 3), (item: "sandstorm_bottle", chance: 0.02)], xp: 6,
        width: 0.8, height: 1.6, spawn: Never, scale: 1.2, flies: true,
        parts: {WRAITH_PARTS},
        layers: [(part: 0, inflate: 0.5), (part: 1, inflate: 0.6)],
    ),
    (
        name: "reaper", display_name: "Ceifador", kind: Hostile, health: 80, speed: 0.1, attack: 10,
        drops: [(item: "dark_shard", chance: 0.5), (item: "eclipse_scythe", chance: 0.04)], xp: 14,
        width: 0.8, height: 1.7, spawn: Never, scale: 1.2, flies: true,
        parts: {REAPER_PARTS},
        layers: [(part: 0, inflate: 0.5), (part: 1, inflate: 0.4)],
    ),
    (
        name: "shade_brute", display_name: "Vulto Brutal", kind: Hostile, health: 110, speed: 3.0, attack: 9,
        drops: [(item: "dark_shard", chance: 0.6), (item: "rotten_flesh", min: 1, max: 3)], xp: 14,
        width: 1.2, height: 2.4, spawn: Never, scale: 1.3,
        parts: {BRUTE_PARTS},
        layers: [(part: 0, inflate: 0.6), (part: 1, inflate: 0.5)],
    ),
    (
        name: "eclipse_moth", display_name: "Mariposa do Eclipse", kind: Hostile, health: 600, speed: 0.1, attack: 10,
        drops: [(item: "dark_shard", min: 4, max: 8), (item: "moth_dust", min: 3, max: 6), (item: "solar_tablet", chance: 0.3)], xp: 80,
        width: 2.4, height: 1.4, spawn: Never, scale: 2.4, flies: true,
        parts: {MOTH_PARTS},
        layers: [(part: 0, inflate: 0.6)],
    ),"""]


ROCK = pal("#1e1410", "#36261e", "#4e382c", "#664a3a")
GLOW_ORANGE = pal("#c4500a", "#f08a2a", "#ffc860")
SAND_P = pal("#8a6a3a", "#b08a50", "#d4b070", "#ead09a", "#f8ecc8")
ROBE = pal("#0e0c12", "#18141e", "#221c2a", "#2c2436")
BONE = pal("#8a877d", "#b4b0a4", "#dcd8cc")
SHADE = pal("#16121e", "#221c2c", "#2e263a", "#3a3048", "#463a56")
MOTH = pal("#3a2a14", "#5a4220", "#7a5a2e", "#9a7840", "#c4a060")
MOTH_WING = pal("#4a3a2a", "#6a5238", "#8c6e4a", "#b89a6a", "#e0caa0")


def head_painter(c):
    n = g.fbm(c.seed, c.gx, c.gy, c.gz, 2.5)
    if n > 0.62:
        return g.pick(GLOW_ORANGE, (n - 0.62) * 2.6)
    if c.face == "front" and c.j in (3, 4) and c.i in (1, 2, 5, 6):
        return (255, 200, 80)
    return g.pick(ROCK, 0.2 + 0.6 * c.jit)


def head_overlay(c):
    if c.face in ("top", "back") and c.jit > 0.45:
        return g.pick(GLOW_ORANGE, c.jit)
    return None


def wraith_painter(c):
    n = g.fbm(c.seed, c.gx * 1.5, c.gy * 0.6, c.gz * 1.5, 3.0)
    t = 0.2 + 0.7 * n
    if c.role == "head" and c.face == "front" and c.j in (3, 4) and c.i in (1, 2, 5, 6):
        return (60, 30, 10)
    if c.role == "tail":
        t -= 0.15 * (1 - c.j / max(1, c.fh))
    return g.pick(SAND_P, t)


def wraith_overlay(c):
    if (c.i + c.j * 2) % 5 == 0 and c.jit > 0.3:
        return g.pick(SAND_P, 0.85)
    return None


def reaper_painter(c):
    if c.role == "staff":
        return g.pick(pal("#2a1a10", "#4a3020", "#6a4a30"), c.jit)
    if c.role == "blade":
        return g.pick(pal("#5a5e68", "#9aa0ae", "#dfe4ea"), 0.3 + 0.6 * c.jit)
    if c.role == "head" and c.face == "front":
        if 2 <= c.i <= 5 and 2 <= c.j <= 6:
            # A skull in the dark of the hood.
            if c.j in (3, 4) and c.i in (2, 5):
                return (220, 40, 30)
            return g.pick(BONE, 0.3 + 0.4 * c.jit)
    return g.pick(ROBE, 0.2 + 0.6 * c.jit)


def reaper_overlay(c):
    if c.role == "robe" and c.face in ("left", "right", "back", "front") and c.j > c.fh - 4 and c.i % 2 == 0:
        return g.pick(ROBE, 0.6)
    if c.role == "head" and c.face in ("top", "back", "left", "right") and c.jit > 0.3:
        return g.pick(ROBE, 0.5 * c.jit)
    return None


def brute_painter(c):
    n = g.fbm(c.seed, c.gx, c.gy, c.gz, 3.0)
    if c.role == "head" and c.face == "front" and c.j in (3, 4) and c.i in (1, 2, 5, 6):
        return (240, 120, 40)
    if c.role == "head" and c.face == "front" and c.j == 6 and 1 < c.i < 6:
        return (200, 190, 170)
    return g.pick(SHADE, 0.2 + 0.6 * n + 0.15 * (c.jit - 0.5))


def brute_overlay(c):
    if c.face in ("front", "back", "left", "right", "top") and c.jit > 0.6:
        return g.pick(SHADE, 0.8 + 0.2 * c.jit)
    return None


def moth_painter(c):
    if c.role == "wing":
        cx, cy = c.i - c.fw / 2, c.j - c.fh / 2
        if cx * cx + cy * cy < 6 and c.face in ("top", "bottom"):
            return (230, 120, 30) if cx * cx + cy * cy > 2 else (20, 10, 10)
        return g.pick(MOTH_WING, 0.2 + 0.6 * c.jit + (0.2 if (c.i // 3) % 2 else 0))
    if c.role == "head" and c.face == "front" and c.j in (1, 2) and c.i in (0, 1, 4, 5):
        return (240, 60, 30)
    band = int(c.gz + 40) // 3 % 2 == 0 and c.face not in ("front", "back")
    return g.pick(MOTH, (0.25 if band else 0.6) + 0.3 * c.jit)


def moth_overlay(c):
    if c.face == "top" and c.jit > 0.4:
        return g.pick(MOTH, 0.9)
    return None


def skins_l2():
    return {
        "meteor_head": (["head"], head_painter, head_overlay),
        "dune_wraith": (["head", "tail"], wraith_painter, wraith_overlay),
        "reaper": (["head", "robe", "staff", "blade"], reaper_painter, reaper_overlay),
        "shade_brute": (["body", "head", "arm", "arm", "leg", "leg"], brute_painter, brute_overlay),
        "eclipse_moth": (["body", "wing", "wing", "head"], moth_painter, moth_overlay),
    }


def items_l2():
    gi.MATERIALS["meteor"] = METEOR
    gi.ARMOR_MATERIALS["meteor"] = METEOR
    gi.MATERIALS["eclipse"] = pal("#0e0c12", "#2a2236", "#4a3a5e", "#8a7aa8", "#e0d4f4")
    gi.WAND_GEMS["meteor"] = pal("#6e4030", "#a8562c", "#f08a2a", "#ffc860", "#fff4c0")
    out = [
        ("meteorite_ingot", "Barra de Meteorito", gi_ingot(METEOR), ""),
        ("fallen_star", "Estrela Caída", icon(STAR, pal("#5a3a04", "#c49418", "#fce47a", "#fff4b0", "#ffffff")), "max_stack: Some(64)"),
        ("meteor_blaster", "Disparador de Meteoro", gi.staff("meteor", "meteor"), "max_stack: Some(1)"),
        ("solar_tablet", "Tábua Solar", icon(TABLET, pal("#3a2204", "#7a4a0c", "#c4841c", "#f4c040", "#fff4c0")), "max_stack: Some(16)"),
        ("dark_shard", "Lasca Sombria", icon(DARK_SHARD, pal("#0e0c12", "#2a2236", "#4a3a5e", "#8a7aa8", "#e0d4f4")), ""),
        ("moth_dust", "Pó de Mariposa", icon(GEL, pal("#3a2a14", "#7a5a2e", "#b89a6a", "#e0caa0", "#fff4dc")), ""),
        ("eclipse_scythe", "Foice do Eclipse", gi.tool("eclipse", "hoe"), "max_stack: Some(1), tool: Some((kind: Sword, tier: 6, speed: 9.0, damage: 12.0, durability: 2400))"),
    ]
    pts = {"helmet": (0, 2), "chestplate": (1, 5), "leggings": (2, 4), "boots": (3, 2)}
    names = {"helmet": "Capacete", "chestplate": "Peitoral", "leggings": "Calças", "boots": "Botas"}
    for k in gi.ARMOR_KINDS:
        slot, points = pts[k]
        out.append((f"meteor_{k}", f"{names[k]} de Meteoro", gi.armor("meteor", k), f"armor: Some((slot: {slot}, points: {points}, durability: {220 + 40 * points}))"))
    return out


def gi_ingot(p):
    gi.INGOT_COL["meteorite"] = p
    return gi.ingot("meteorite")


def blocks_l2():
    def crust(x, y):
        return 0.35 + 0.25 * np.sin(x * 1.3 + y * 0.7) * np.cos(y * 1.1 - x * 0.4)

    def ore(x, y):
        # Hot veins glowing through the dark stone.
        v = abs(np.sin(x * 0.9 + np.sin(y * 0.8) * 2.0))
        return 0.92 if v < 0.18 else crust(x, y) * 0.8

    return {"meteorite_ore": block_noise(21, ore, pal("#1e1410", "#36261e", "#4e382c", "#c4500a", "#ffc860"))}


BLOCKS_L2 = [
    '(name: "meteorite_ore", display_name: Some("Meteorito"), light: 6, hardness: 5.0, tool: Some(Pickaxe), tier: 2, xp: (1, 3))',
]

SHAPED_L2 = [
    (["III", "I I"], {"I": "meteorite_ingot"}, "meteor_helmet", 1),
    (["I I", "III", "III"], {"I": "meteorite_ingot"}, "meteor_chestplate", 1),
    (["III", "I I", "I I"], {"I": "meteorite_ingot"}, "meteor_leggings", 1),
    (["I I", "I I"], {"I": "meteorite_ingot"}, "meteor_boots", 1),
    (["III", "I G"], {"I": "meteorite_ingot", "G": "glowstone_dust"}, "meteor_blaster", 1),
    (["GGG", "GSG", "GGG"], {"G": "gold_ingot", "S": "soul_of_sight"}, "solar_tablet", 1),
]
SHAPELESS_L2 = [
    (["fallen_star", "fallen_star", "fallen_star", "fallen_star", "fallen_star"], "mana_crystal", 1),
    (["fallen_star", "glass_bottle"], "lesser_mana_potion", 1),
]
SMELTING_L2 = [("meteorite_ore", "meteorite_ingot", 1.0)]


# ==============================================================================================
# Lote 3: the evil lands (Ruína / Chaga), the Prism, sky islands, the pyramid, early armours
# ==============================================================================================

LANDS = {
    # name: (grass top, stone, sand)
    "ruin": (pal("#3a2450", "#4a2e64", "#5a3a78", "#6c4a8c", "#7e5ca0", "#9274b4"),
             pal("#2e2638", "#3a3046", "#463a54", "#524462", "#5e4e70", "#6c5a80"),
             pal("#3e3648", "#4a4256", "#564e64", "#625a70", "#6e667c")),
    "blight": (pal("#5a1414", "#6e1a1a", "#842222", "#9a2c2a", "#b03a34", "#c44a40"),
               pal("#3a1416", "#4c1a1c", "#5e2224", "#70302e", "#823c38", "#944a44"),
               pal("#5a2a28", "#6a3430", "#7a3e38", "#8a4a42", "#9a564c")),
    "prism": (pal("#5ab4c8", "#6ac4d6", "#7ad2e2", "#92dcea", "#b0e6f0", "#d0f2f8"),
              pal("#b8a8c0", "#c8b8d0", "#d6c8de", "#e2d6ea", "#eee4f4", "#faf4ff"),
              pal("#d8c4d8", "#e2d0e2", "#ecdcec", "#f4e8f4", "#fcf4fc")),
}
LAND_NAMES = {"ruin": "da Ruína", "blight": "da Chaga", "prism": "Prismática"}


def with_pals(fn, **pals):
    old = {k: getattr(gt, k) for k in pals}
    for k, v in pals.items():
        setattr(gt, k, v)
    try:
        return fn()
    finally:
        for k, v in old.items():
            setattr(gt, k, v)


def blocks_l3():
    out = {}
    for land, (grass, stone, sand) in LANDS.items():
        out[f"{land}_grass_top"] = with_pals(lambda: gt.grass_top(f"{land}_grass_top"), GRASS=grass)
        out[f"{land}_grass_side"] = gt.fringe_side(f"{land}_grass_side", grass)
        out[f"{land}_stone"] = with_pals(lambda: gt.stone(f"{land}_stone"), STONE=stone)
        out[f"{land}_sand"] = with_pals(lambda: gt.sand(f"{land}_sand"), SAND=sand)
    ruin_stone = LANDS["ruin"][1]
    blight_stone = LANDS["blight"][1]
    out["umbral_ore"] = gt.ore("umbral", bg=with_pals(lambda: gt.stone("umbral_bg"), STONE=ruin_stone), p=pal("#1a1028", "#5a2a8a", "#9a5ad4", "#d4a8ff"))
    out["sanguinite_ore"] = gt.ore("sanguinite", bg=with_pals(lambda: gt.stone("sanguinite_bg"), STONE=blight_stone), p=pal("#3a0408", "#a0141e", "#e8343a", "#ff9a8a"))

    def orb(x, y):
        cx, cy = (x - 7.5) / 7.0, (y - 7.5) / 7.0
        r = cx * cx + cy * cy
        return 0.98 - 0.7 * r + 0.25 * np.sin(x * 1.3 + y * 0.9) if r < 1 else 0.1

    out["ruin_orb"] = block_noise(31, orb, pal("#140a1e", "#3a1a5a", "#6a2a9a", "#a65ae0", "#e8c8ff"))
    out["blight_heart"] = block_noise(32, lambda x, y: orb(x, y) + (0.2 if (x * 3 + y) % 7 == 0 else 0), pal("#2a0406", "#6a0e14", "#a81c24", "#e04a4a", "#ffb0a0"))

    def altar_top(x, y):
        edge = min(x, y, 15 - x, 15 - y)
        return 0.85 if edge == 0 else (0.15 if 5 <= x <= 10 and 5 <= y <= 10 else 0.45)

    def altar_side(x, y):
        if y < 3:
            return 0.8
        if y > 12:
            return 0.5
        return 0.25 + (0.4 if (x in (2, 3, 12, 13)) else 0.0)

    out["ruin_altar_top"] = block_noise(33, altar_top, pal("#140a1e", "#2e2040", "#4a3464", "#7a5aa0", "#c8a0ff"))
    out["ruin_altar_side"] = block_noise(34, altar_side, pal("#140a1e", "#2e2040", "#4a3464", "#7a5aa0", "#c8a0ff"))
    out["blight_altar_top"] = block_noise(35, altar_top, pal("#200608", "#4a1014", "#7a1e22", "#b03a34", "#ff9a8a"))
    out["blight_altar_side"] = block_noise(36, altar_side, pal("#200608", "#4a1014", "#7a1e22", "#b03a34", "#ff9a8a"))

    def plates(x, y):
        rim = x % 8 == 0 or y % 8 == 0
        return 0.25 if rim else 0.55 + 0.3 * ((x % 8) + (y % 8) < 5)

    out["sky_brick"] = block_noise(37, plates, pal("#7a5a10", "#b08a1c", "#d8b030", "#f0d060", "#fff0b0"))
    out["cloud_block"] = block_noise(38, lambda x, y: 0.55 + 0.35 * np.sin(x * 0.8) * np.cos(y * 0.7), pal("#c8d2dc", "#dce4ec", "#eaf0f6", "#f6f9fc", "#ffffff"))
    return out


def land_blocks():
    out = []
    for land in LANDS:
        n = LAND_NAMES[land]
        out.append(f'(name: "{land}_grass", display_name: Some("Grama {n}"), hardness: 0.6, tool: Some(Shovel), textures: (top: Some("{land}_grass_top"), side: Some("{land}_grass_side"), bottom: Some("dirt")), drops: Some([(item: "dirt")]))')
        out.append(f'(name: "{land}_stone", display_name: Some("Pedra {n}"), hardness: 1.5, tool: Some(Pickaxe), tier: 1)')
        out.append(f'(name: "{land}_sand", display_name: Some("Areia {n}"), hardness: 0.5, tool: Some(Shovel), gravity: true)')
    return out


BLOCKS_L3 = land_blocks() + [
    '(name: "umbral_ore", display_name: Some("Minério de Sombraço"), hardness: 4.0, tool: Some(Pickaxe), tier: 2, xp: (1, 3))',
    '(name: "sanguinite_ore", display_name: Some("Minério de Sanguinita"), hardness: 4.0, tool: Some(Pickaxe), tier: 2, xp: (1, 3))',
    '(name: "ruin_orb", display_name: Some("Orbe da Ruína"), light: 9, hardness: 2.0, drops: Some([]))',
    '(name: "blight_heart", display_name: Some("Coração da Chaga"), light: 9, hardness: 2.0, drops: Some([]))',
    '(name: "ruin_altar", display_name: Some("Altar da Ruína"), light: 4, hardness: 40.0, tool: Some(Pickaxe), tier: 5, textures: (top: Some("ruin_altar_top"), side: Some("ruin_altar_side"), bottom: Some("ruin_altar_side")), drops: Some([]))',
    '(name: "blight_altar", display_name: Some("Altar da Chaga"), light: 4, hardness: 40.0, tool: Some(Pickaxe), tier: 5, textures: (top: Some("blight_altar_top"), side: Some("blight_altar_side"), bottom: Some("blight_altar_side")), drops: Some([]))',
    '(name: "sky_brick", display_name: Some("Tijolo Celeste"), light: 2, hardness: 2.0, tool: Some(Pickaxe))',
    '(name: "cloud_block", display_name: Some("Nuvem"), hardness: 0.3)',
]

POWDER = [
    "................",
    "................",
    "......1111......",
    ".....122221.....",
    "....12333321....",
    "....13444431....",
    "...1344554431...",
    "...1345555431...",
    "...1345555431...",
    "...1344554431...",
    "....13444431....",
    ".....133331.....",
    "......1111......",
    "................",
    "................",
    "................",
]

RING = [
    "................",
    "................",
    ".......55.......",
    "......5445......",
    ".......55.......",
    ".....111111.....",
    "....12222221....",
    "...1231..1321...",
    "...121....121...",
    "...121....121...",
    "...1231..1321...",
    "....12222221....",
    ".....111111.....",
    "................",
    "................",
    "................",
]

HORN2 = [
    "................",
    "..............5.",
    ".............54.",
    "............543.",
    "...........543..",
    "..........5431..",
    ".........5431...",
    "........5432....",
    ".......5432.....",
    "......5432......",
    ".....5432.......",
    "....4432........",
    "...3332.........",
    "...222..........",
    "................",
    "................",
]

FOSSIL = [
    "................",
    "................",
    "....11111111....",
    "...1222222221...",
    "..122333333221..",
    "..123344443321..",
    "..123455554321..",
    "..123451154321..",
    "..123451154321..",
    "..123455554321..",
    "..123344443321..",
    "..122333333221..",
    "...1222222221...",
    "....11111111....",
    "................",
    "................",
]

ARMOR_L3 = {
    # material: (display, palette, points per piece helmet/chest/legs/boots)
    "umbral": ("de Sombraço", pal("#1a1028", "#3a2458", "#5a3a84", "#8a64b8", "#c8a8f0"), (2, 5, 4, 2)),
    "sanguinite": ("de Sanguinita", pal("#2a0406", "#5a0e14", "#8a1c22", "#c03a38", "#f08a80"), (2, 5, 4, 2)),
    "hive": ("de Abelha", pal("#3a2204", "#6a3e08", "#a0640e", "#d49a1e", "#f4c84a"), (1, 3, 3, 1)),
    "jungle": ("da Selva", pal("#0e2a10", "#1e4a1c", "#2e6a2a", "#4e9a3a", "#8ad064"), (2, 4, 3, 1)),
    "fossil": ("Fóssil", pal("#4a3e2a", "#7a6a4a", "#a8966e", "#cfc09a", "#f0e6c8"), (2, 4, 3, 1)),
    "necro": ("Necro", pal("#2a2a24", "#5a584c", "#8a8676", "#b8b4a2", "#e4e0d0"), (2, 4, 3, 2)),
    "ninja": ("Ninja", pal("#0a0a0e", "#16161e", "#24242e", "#3a3a48", "#5a5a6e"), (1, 3, 2, 1)),
}


def items_l3():
    gi.INGOT_COL["umbral"] = ARMOR_L3["umbral"][1]
    gi.INGOT_COL["sanguinite"] = ARMOR_L3["sanguinite"][1]
    for m, (_, p, _) in ARMOR_L3.items():
        gi.ARMOR_MATERIALS[m] = p
        gi.MATERIALS[m] = p
    gi.MATERIALS["sky"] = pal("#6a7aa0", "#8a9ac4", "#b0c0e4", "#d8e4f8", "#ffffff")
    gi.WAND_GEMS["ruin"] = pal("#2a1040", "#5a2a8a", "#9a5ad4", "#d4a8ff", "#ffffff")
    gi.WAND_GEMS["blight"] = pal("#3a0408", "#a0141e", "#e8343a", "#ff9a8a", "#ffffff")
    out = [
        ("umbral_ingot", "Barra de Sombraço", gi.ingot("umbral"), ""),
        ("sanguinite_ingot", "Barra de Sanguinita", gi.ingot("sanguinite"), ""),
        ("purification_powder", "Pó Purificador", icon(POWDER, pal("#2a5a2a", "#4a8a3a", "#7ac45a", "#b4ea8a", "#f0fff0")), ""),
        ("vile_powder", "Pó Vil", icon(POWDER, pal("#2a1040", "#4a2a6a", "#7a4aa0", "#a87ad0", "#e0c8ff")), ""),
        ("star_band", "Anel Estelar", icon(RING, pal("#6a5008", "#c4941c", "#f0d060", "#7ab4ff", "#e0f0ff")), "max_stack: Some(1)"),
        ("ruin_thorn", "Espinho Vil", gi.wand("ruin"), "max_stack: Some(1)"),
        ("blight_rod", "Bastão da Chaga", gi.staff("blight", "sanguinite"), "max_stack: Some(1)"),
        ("pixie_dust", "Pó de Fada", icon(GEL, pal("#5a8aa0", "#8ac4d6", "#c0e6f0", "#f0d0f0", "#ffffff")), ""),
        ("unicorn_horn", "Chifre de Unicórnio", icon(HORN2, pal("#6a5a7a", "#b8a8c8", "#e8d8f0", "#fff0ff", "#ffffff")), ""),
        ("sturdy_fossil", "Fóssil Resistente", icon(FOSSIL, pal("#4a3e2a", "#7a6a4a", "#a8966e", "#cfc09a", "#f0e6c8")), ""),
        ("jungle_spore", "Esporo da Selva", icon(GEL, pal("#1e4a1c", "#3a8a2a", "#6ac44a", "#b4f08a", "#f0fff0")), ""),
        ("sky_blade", "Lâmina Celeste", gi.tool("sky", "sword"), "max_stack: Some(1), tool: Some((kind: Sword, tier: 2, speed: 8.0, damage: 6.5, durability: 600))"),
        ("umbral_pickaxe", "Picareta de Sombraço", gi.tool("umbral", "pickaxe"), "max_stack: Some(1), tool: Some((kind: Pickaxe, tier: 3, speed: 9.0, damage: 4.0, durability: 1100))"),
        ("sanguinite_pickaxe", "Picareta de Sanguinita", gi.tool("sanguinite", "pickaxe"), "max_stack: Some(1), tool: Some((kind: Pickaxe, tier: 3, speed: 9.0, damage: 4.0, durability: 1100))"),
        ("umbral_sword", "Espada de Sombraço", gi.tool("umbral", "sword"), "max_stack: Some(1), tool: Some((kind: Sword, tier: 3, speed: 8.0, damage: 7.5, durability: 1100))"),
        ("sanguinite_sword", "Espada de Sanguinita", gi.tool("sanguinite", "sword"), "max_stack: Some(1), tool: Some((kind: Sword, tier: 3, speed: 8.0, damage: 7.5, durability: 1100))"),
    ]
    names = {"helmet": "Capacete", "chestplate": "Peitoral", "leggings": "Calças", "boots": "Botas"}
    for m, (disp, _, pts) in ARMOR_L3.items():
        for slot, k in enumerate(gi.ARMOR_KINDS):
            out.append((f"{m}_{k}", f"{names[k]} {disp}", gi.armor(m, k), f"armor: Some((slot: {slot}, points: {pts[slot]}, durability: {200 + 30 * pts[slot]}))"))
    return out


def armor_recipes(mat, ing, extra=None):
    key = {"I": ing}
    if extra:
        key["E"] = extra
    e = "E" if extra else "I"
    return [
        (["III", f"I{e}I" if extra else "I I"], key if extra else {"I": ing}, f"{mat}_helmet", 1),
        (["I I", f"I{e}I" if extra else "III", "III"], key if extra else {"I": ing}, f"{mat}_chestplate", 1),
        (["III", "I I", "I I"], {"I": ing}, f"{mat}_leggings", 1),
        (["I I", "I I"], {"I": ing}, f"{mat}_boots", 1),
    ]


SHAPED_L3 = (
    armor_recipes("umbral", "umbral_ingot", "shadow_scale")
    + armor_recipes("sanguinite", "sanguinite_ingot", "blight_tissue")
    + armor_recipes("hive", "beeswax")
    + armor_recipes("jungle", "jungle_spore", "emerald")
    + armor_recipes("fossil", "sturdy_fossil")
    + armor_recipes("necro", "bone", "spider_eye")
    + [
        (["UUU", " S ", " S "], {"U": "umbral_ingot", "S": "stick"}, "umbral_pickaxe", 1),
        (["UUU", " S ", " S "], {"U": "sanguinite_ingot", "S": "stick"}, "sanguinite_pickaxe", 1),
        (["U", "U", "S"], {"U": "umbral_ingot", "S": "stick"}, "umbral_sword", 1),
        (["U", "U", "S"], {"U": "sanguinite_ingot", "S": "stick"}, "sanguinite_sword", 1),
    ]
)
SHAPELESS_L3 = [
    (["brown_mushroom", "glowstone_dust"], "purification_powder", 5),
    (["red_mushroom", "rotten_flesh"], "vile_powder", 5),
    (["vine", "vine", "vine", "cocoa_beans"], "jungle_spore", 1),
    (["bone", "bone", "bone", "sandstone"], "sturdy_fossil", 1),
    (["umbral_ore", "umbral_ore", "umbral_ore"], "umbral_ingot", 1),
    (["sanguinite_ore", "sanguinite_ore", "sanguinite_ore"], "sanguinite_ingot", 1),
]
SMELTING_L3 = [("umbral_ore", "umbral_ingot", 1.0), ("sanguinite_ore", "sanguinite_ingot", 1.0)]

EATER_PARTS = """[
            (size: (8, 8, 10), origin: (-4, 0, -2), uv: (0, 0), anim: Head, pivot: (0, 4, 0)),
            (size: (6, 6, 8), origin: (-3, 1, -10), uv: (0, 20)),
            (size: (8, 1, 6), origin: (-12, 6, -6), uv: (0, 36), anim: Wing(0), pivot: (-3, 6, -3)),
            (size: (8, 1, 6), origin: (4, 6, -6), uv: (0, 36), anim: Wing(1), pivot: (3, 6, -3)),
        ]"""

BRUTE2_PARTS = """[
            (size: (10, 12, 6), origin: (-5, 12, -3), uv: (0, 0)),
            (size: (10, 10, 8), origin: (-5, 24, -4), uv: (0, 20), anim: Head, pivot: (0, 24, 0)),
            (size: (4, 14, 4), origin: (-9, 10, -2), uv: (36, 0), anim: ArmForward(0), pivot: (-7, 23, 0)),
            (size: (4, 14, 4), origin: (5, 10, -2), uv: (36, 0), anim: ArmForward(1), pivot: (7, 23, 0)),
            (size: (4, 12, 4), origin: (-5, 0, -2), uv: (0, 40), anim: Leg(0), pivot: (-3, 12, 0)),
            (size: (4, 12, 4), origin: (1, 0, -2), uv: (0, 40), anim: Leg(1), pivot: (3, 12, 0)),
        ]"""

PIXIE_PARTS = """[
            (size: (4, 4, 4), origin: (-2, 6, -2), uv: (0, 0), anim: Head, pivot: (0, 6, 0)),
            (size: (4, 6, 2), origin: (-2, 0, -1), uv: (0, 10)),
            (size: (6, 8, 1), origin: (-8, 2, -1), uv: (16, 0), anim: Wing(0), pivot: (-2, 4, 0)),
            (size: (6, 8, 1), origin: (2, 2, -1), uv: (16, 0), anim: Wing(1), pivot: (2, 4, 0)),
        ]"""

UNICORN_PARTS = """[
            (size: (10, 10, 20), origin: (-5, 12, -10), uv: (0, 0)),
            (size: (6, 12, 6), origin: (-3, 18, 8), uv: (0, 30), anim: Head, pivot: (0, 20, 10)),
            (size: (6, 5, 8), origin: (-3, 26, 12), uv: (24, 30), anim: Head, pivot: (0, 20, 10)),
            (size: (2, 7, 2), origin: (-1, 31, 16), uv: (52, 30), anim: Head, pivot: (0, 20, 10)),
            (size: (4, 12, 4), origin: (-5, 0, 5), uv: (0, 48), anim: Leg(0), pivot: (-3, 12, 7)),
            (size: (4, 12, 4), origin: (1, 0, 5), uv: (0, 48), anim: Leg(1), pivot: (3, 12, 7)),
            (size: (4, 12, 4), origin: (-5, 0, -9), uv: (0, 48), anim: Leg(1), pivot: (-3, 12, -7)),
            (size: (4, 12, 4), origin: (1, 0, -9), uv: (0, 48), anim: Leg(0), pivot: (3, 12, -7)),
        ]"""


def mobs_l3():
    return [f"""    // Lote 3: the evil lands and the Prism.
    (
        name: "soul_eater", display_name: "Devorador de Almas", kind: Hostile, health: 34, speed: 0.1, attack: 5,
        drops: [(item: "rotten_flesh", chance: 0.5), (item: "shadow_scale", chance: 0.08), (item: "vile_powder", chance: 0.2)], xp: 6,
        width: 0.8, height: 0.7, spawn: Never, scale: 1.4, flies: true,
        parts: {EATER_PARTS},
        layers: [(part: 0, inflate: 0.5)],
    ),
    (
        name: "blight_flyer", display_name: "Voador da Chaga", kind: Hostile, health: 34, speed: 0.1, attack: 5,
        drops: [(item: "rotten_flesh", chance: 0.5), (item: "blight_tissue", chance: 0.08)], xp: 6,
        width: 0.8, height: 0.7, spawn: Never, scale: 1.4, flies: true,
        parts: {EATER_PARTS},
        layers: [(part: 0, inflate: 0.5)],
    ),
    (
        name: "blight_brute", display_name: "Rosto da Chaga", kind: Hostile, health: 70, speed: 2.4, attack: 7,
        drops: [(item: "rotten_flesh", min: 1, max: 2), (item: "blight_tissue", chance: 0.1)], xp: 8,
        width: 0.9, height: 2.2, spawn: Never, scale: 1.1,
        parts: {BRUTE2_PARTS},
        layers: [(part: 0, inflate: 0.4), (part: 1, inflate: 0.5)],
    ),
    (
        name: "prism_pixie", display_name: "Fada Prismática", kind: Hostile, health: 40, speed: 0.1, attack: 7,
        drops: [(item: "pixie_dust", min: 1, max: 3)], xp: 8,
        width: 0.6, height: 0.8, spawn: Never, scale: 1.3, flies: true,
        parts: {PIXIE_PARTS},
        layers: [(part: 0, inflate: 0.4)],
    ),
    (
        name: "prism_unicorn", display_name: "Unicórnio Prismático", kind: Hostile, health: 110, speed: 4.5, attack: 9,
        drops: [(item: "unicorn_horn", chance: 0.4), (item: "pixie_dust", min: 1, max: 2), (item: "blessed_apple", chance: 0.03)], xp: 14,
        width: 1.3, height: 1.9, spawn: Never, scale: 1.1,
        parts: {UNICORN_PARTS},
        layers: [(part: 0, inflate: 0.5), (part: 1, inflate: 0.4)],
    ),"""]


EATER = pal("#2a1a3a", "#3e2a52", "#54386a", "#6a4a82", "#82609a")
EATER_RED = pal("#3a0e10", "#561a1a", "#722626", "#8e3634", "#a84a44")
WING_SKIN = pal("#1a1020", "#2a1a32", "#3a2644")


def eater_painter(body):
    def f(c):
        if c.role == "wing":
            return g.pick(WING_SKIN if body is EATER else pal("#2a0808", "#3e1010", "#521818"), 0.3 + 0.5 * c.jit)
        if c.role == "head" and c.face == "front":
            # A round sucking mouth ringed with teeth.
            cx, cy = c.i - (c.fw - 1) / 2, c.j - (c.fh - 1) / 2
            r = cx * cx + cy * cy
            if r < 3:
                return (20, 6, 10)
            if r < 6:
                return (220, 210, 190) if (c.i + c.j) % 2 == 0 else (60, 20, 30)
        n = g.fbm(c.seed, c.gx, c.gy, c.gz, 2.5)
        return g.pick(body, 0.2 + 0.7 * n)
    return f


def eater_overlay(body):
    def f(c):
        if c.face in ("top", "left", "right") and c.jit > 0.55:
            return g.pick(body, 0.9)
        return None
    return f


def brute2_painter(c):
    n = g.fbm(c.seed, c.gx, c.gy, c.gz, 3.0)
    if c.role == "head" and c.face == "front":
        if c.j in (2, 3) and c.i in (1, 2, 7, 8):
            return (250, 220, 120)
        if 5 <= c.j <= 7 and 2 <= c.i <= 7:
            return (240, 230, 210) if (c.i % 2 == 0 and c.j != 6) else (40, 8, 10)
    return g.pick(EATER_RED, 0.2 + 0.7 * n)


def brute2_overlay(c):
    if c.face in ("front", "back", "left", "right") and (c.i * 3 + c.j) % 7 == 0:
        return g.pick(EATER_RED, 0.95)
    return None


PIXIE = pal("#c48ab4", "#dca8cc", "#f0c8e4", "#fce4f4", "#ffffff")
PIXIE_WING = pal("#7ac4e0", "#a8dcf0", "#d0eef8", "#f0faff")


def pixie_painter(c):
    if c.role == "wing":
        return g.pick(PIXIE_WING, 0.2 + 0.7 * c.jit)
    if c.role == "head" and c.face == "front" and c.j == 1 and c.i in (0, 3):
        return (60, 40, 120)
    return g.pick(PIXIE, 0.3 + 0.6 * c.jit)


UNI = pal("#c8c0d8", "#dcd6e8", "#ece8f4", "#f8f6fc", "#ffffff")
MANE = pal("#5ab4e0", "#8ad0f0", "#e08ad0", "#f0b4e4", "#fff0a0")


def unicorn_painter(c):
    if c.role == "horn":
        return g.pick(pal("#c49418", "#ecc434", "#fce47a", "#fff4c0"), 0.2 + 0.7 * (c.j / max(1, c.fh)))
    if c.role == "neck" and c.face in ("back", "top") or (c.role == "head" and c.face == "top"):
        return g.pick(MANE, (c.j * 0.2 + c.i * 0.07) % 1.0)
    if c.role == "head" and c.face in ("left", "right") and c.j == 2 and c.i in (4, 5):
        return (60, 40, 120)
    if c.role == "leg" and c.j > c.fh - 3:
        return g.pick(pal("#8a7aa0", "#a898bc"), c.jit)
    return g.pick(UNI, 0.2 + 0.7 * c.jit)


def unicorn_overlay(c):
    if c.role == "body" and c.face == "top" and c.jit > 0.6:
        return g.pick(UNI, 0.95)
    return None


def skins_l3():
    return {
        "soul_eater": (["head", "body", "wing", "wing"], eater_painter(EATER), eater_overlay(EATER)),
        "blight_flyer": (["head", "body", "wing", "wing"], eater_painter(EATER_RED), eater_overlay(EATER_RED)),
        "blight_brute": (["body", "head", "arm", "arm", "leg", "leg"], brute2_painter, brute2_overlay),
        "prism_pixie": (["head", "body", "wing", "wing"], pixie_painter, None),
        "prism_unicorn": (["body", "neck", "head", "horn", "leg", "leg", "leg", "leg"], unicorn_painter, unicorn_overlay),
    }


# The sky islands and the pyramid (tr/structures.ron).

def sky_island():
    """A floating island: a rounded clod of dirt and stone under a grass top, with a small
    golden-brick shrine and its chest, and clouds hanging around it."""
    R = 9
    size = 2 * R + 1
    layers = []
    depth = 7
    rng = np.random.default_rng(77)
    for y in range(depth + 7):
        rows = []
        for z in range(size):
            row = ""
            for x in range(size):
                dx, dz = x - R, z - R
                d = (dx * dx + dz * dz) ** 0.5
                c = "."
                if y < depth:
                    # The clod narrows towards the bottom.
                    r = R * (0.35 + 0.65 * (y + 1) / depth) - rng.random() * 0.8
                    if d <= r:
                        c = "S" if y < depth - 3 else "D"
                    elif d <= r + 1.5 and y >= depth - 2 and rng.random() < 0.35:
                        c = "c"
                    if y == depth - 1 and d <= r:
                        c = "G"
                else:
                    h = y - depth
                    if abs(dx) <= 3 and abs(dz) <= 3:
                        wall = abs(dx) == 3 or abs(dz) == 3
                        if h == 5:
                            c = "B" if abs(dx) <= 3 and abs(dz) <= 3 else "."
                        elif h == 6:
                            c = "B" if abs(dx) <= 1 and abs(dz) <= 1 else "."
                        elif wall:
                            door = dz == 3 and dx == 0 and h in (0, 1)
                            window = (dx == 0 or dz == 0) and h == 2 and not door
                            c = "." if door else ("g" if window else ("B" if (abs(dx) == 3 and abs(dz) == 3) else "b"))
                        elif h == 0 and dx == 0 and dz == -2:
                            c = "K"
                        elif h == 0 and dx == 2 and dz == -2:
                            c = "t"
                    elif h == 0 and d < R - 2 and rng.random() < 0.12:
                        c = "f"
                    elif h in (1, 2) and d > R and rng.random() < 0.05:
                        c = "c"
                row += c
            rows.append(row)
        layers.append(rows)
    return layers


def pyramid():
    """A sandstone pyramid in the desert with a corridor down to a treasure room under it."""
    R = 12
    size = 2 * R + 1
    sink = 8
    layers = []
    for y in range(sink + R + 1):
        rows = []
        h = y - sink
        for z in range(size):
            row = ""
            for x in range(size):
                dx, dz = x - R, z - R
                c = " "
                if h < 0:
                    # Under the ground: the treasure room and the shaft down to it.
                    room = abs(dx) <= 4 and abs(dz) <= 4 and -6 <= h <= -2
                    shell = abs(dx) <= 5 and abs(dz) <= 5 and -7 <= h <= -1
                    shaft = dx == 0 and dz == 3 and h >= -6
                    if room:
                        c = "."
                        if h == -6 and (dx, dz) in ((0, -3), (-3, -3), (3, -3)):
                            c = "K"
                        elif h == -4 and abs(dx) == 4 and dz == 0:
                            c = "t"
                    elif shaft:
                        c = "L"
                    elif shell:
                        c = "C"
                else:
                    r = R - h
                    if r < 0:
                        c = " "
                    elif max(abs(dx), abs(dz)) <= r:
                        inner = max(abs(dx), abs(dz)) < r - 1
                        hall = h in (0, 1, 2) and dx == 0 and dz > 0
                        hall2 = h in (0, 1, 2) and abs(dx) <= 1 and abs(dz) <= 1
                        if hall or hall2 or (inner and h < r - 2 and h < 3 and abs(dx) <= 2 and abs(dz) <= 2):
                            c = "."
                            if hall and dz == 3 and h == 0:
                                c = "L"
                        else:
                            c = "S" if max(abs(dx), abs(dz)) == r else "C"
                            if h == 1 and dz == r and dx == 0:
                                c = "."
                        if h == R:
                            c = "X"
                row += c
            rows.append(row)
        layers.append(rows)
    return layers, sink


def structures_l3():
    isl = sky_island()
    pyr, sink = pyramid()
    lines = ["// The rest of the Terraria side (plan/terraria-completo.md). Written by tools/gen_tr.py.", "(structures: ["]

    def emit(name, biomes, spacing, chance, palette, layers, extra, loot, mobs=None):
        lines.append("    (")
        lines.append(f'        name: "{name}",')
        lines.append(f"        biomes: [{', '.join(chr(34) + b + chr(34) for b in biomes)}],")
        lines.append(f"        spacing: {spacing},")
        lines.append(f"        chance: {chance},")
        lines.append("        palette: [" + ", ".join(f"('{k}', \"{v}\")" for k, v in palette) + "],")
        lines.append("        layers: [")
        for layer in layers:
            lines.append("            [" + ", ".join(f'"{r}"' for r in layer) + "],")
        lines.append("        ],")
        for e in extra:
            lines.append(f"        {e},")
        lines.append("        loot: [")
        for item, lo, hi, ch in loot:
            lines.append(f'            (item: "{item}", min: {lo}, max: {hi}, chance: {ch}),')
        lines.append("        ],")
        if mobs:
            lines.append("        mobs: [" + ", ".join(f'("{m}", {n})' for m, n in mobs) + "],")
        lines.append("    ),")

    emit("ilha_do_ceu", ["Plains", "Forest", "Birch Forest", "Taiga", "Savanna", "Meadow", "Flower Forest", "Dark Forest", "Jungle", "Desert", "Snowy Plains"], 22, 0.55,
         [("S", "stone"), ("D", "dirt"), ("G", "grass_block"), ("c", "cloud_block"), ("B", "sky_brick"), ("b", "sky_brick"), ("g", "glass"), ("K", "chest"), ("t", "torch"), ("f", "dandelion")],
         isl, ["place: Sky(150)"],
         [("lucky_horseshoe", 1, 1, 0.35), ("shiny_balloon", 1, 1, 0.3), ("cloud_bottle", 1, 1, 0.35), ("sky_blade", 1, 1, 0.3), ("fallen_star", 2, 5, 0.7), ("gold_ingot", 2, 6, 0.5)])
    emit("piramide", ["Desert"], 32, 0.6,
         [("S", "sandstone"), ("C", "cut_sandstone"), ("X", "chiseled_sandstone"), ("K", "chest"), ("t", "torch"), ("L", "ladder")],
         pyr, [f"sink: {sink}", "rough: 6"],
         [("sandstorm_bottle", 1, 1, 0.35), ("sturdy_fossil", 2, 6, 0.6), ("gold_ingot", 3, 9, 0.7), ("emerald", 1, 4, 0.5), ("lesser_healing_potion", 1, 3, 0.5), ("fallen_star", 1, 3, 0.4)],
         [("mummy", 3)])
    temple, tsink = structures_l7()
    emit("templo_escamado", ["Jungle", "Bamboo Jungle", "Sparse Jungle"], 40, 0.6,
         [("B", "scale_brick"), ("A", "scale_altar"), ("l", "scale_lamp"), ("K", "chest"), ("L", "ladder")],
         temple, [f"sink: {tsink}"],
         [("idol_core", 1, 1, 0.5), ("turtle_shell", 1, 2, 0.4), ("beetle_husk", 1, 3, 0.3), ("emerald", 3, 8, 0.7), ("super_healing_potion", 1, 3, 0.5), ("sun_stone_staff", 1, 1, 0.15)],
         None)
    (lake, lsink), oasis, grave = structures_l9()
    emit("lago_cintilante", ["Plains", "Forest", "Birch Forest", "Meadow", "Flower Forest", "Taiga", "Savanna"], 48, 0.5,
         [("S", "stone"), ("~", "shimmer"), ("a", "amethyst_block"), ("c", "shimmer_crystal"), ("K", "chest"), ("L", "ladder")],
         lake, [f"sink: {lsink}"],
         [("shimmer_crystal", 2, 6, 0.8), ("magic_mirror", 1, 1, 0.3), ("teleport_potion", 1, 2, 0.4), ("amethyst", 2, 6, 0.6), ("fallen_star", 2, 5, 0.5)],
         None)
    emit("oasis", ["Desert"], 20, 0.45,
         [("w", "water"), ("G", "grass_block"), ("f", "short_grass"), ("T", "jungle_log"), ("e", "jungle_leaves"), ("K", "chest")],
         oasis, ["sink: 1", "rough: 3"],
         [("magic_conch", 1, 1, 0.35), ("sandstorm_bottle", 1, 1, 0.2), ("javelin", 6, 16, 0.6), ("recall_potion", 1, 3, 0.5), ("gravity_potion", 1, 2, 0.3), ("gold_ingot", 2, 5, 0.5)],
         None)
    emit("cemiterio", ["Plains", "Forest", "Dark Forest", "Taiga", "Birch Forest"], 30, 0.4,
         [("P", "podzol"), ("F", "oak_fence"), ("t", "tombstone"), ("K", "chest"), ("d", "dead_bush")],
         grave, ["sink: 1", "rough: 3"],
         [("bone_javelin", 8, 16, 0.6), ("ectoplasm", 1, 3, 0.15), ("magic_mirror", 1, 1, 0.25), ("wormhole_potion", 1, 2, 0.4), ("bone", 4, 12, 0.8), ("tombstone", 1, 3, 0.5)],
         [("zombie", 3), ("skeleton", 2)])
    for k, (key, _, _, biomes) in enumerate(DUNGEON_COLOURS):
        dg, dsink = dungeon_layout(900 + k)
        emit(f"masmorra_{key}", biomes, 96, 0.9,
             [("B", f"dungeon_brick_{key}"), ("L", "ladder"), ("l", "dungeon_lantern"), ("v", "dungeon_spikes"), ("X", "locked_chest"), ("K", "chest"), ("b", "bookshelf"), ("w", "cobweb"), ("i", "iron_bars")],
             dg, [f"sink: {dsink}", "rough: 14"],
             [("golden_key", 1, 1, 0.5), ("blue_katana", 1, 1, 0.15), ("bastion_shield", 1, 1, 0.15), ("bone_flail", 1, 1, 0.15), ("water_bolt", 1, 1, 0.12), ("spirit_bell", 1, 1, 0.1), ("bone_whip", 1, 1, 0.1), ("lucky_horseshoe", 1, 1, 0.1), ("bone", 4, 12, 0.7), ("emerald", 2, 6, 0.6), ("healing_potion", 1, 3, 0.5), ("enchanted_book", 1, 1, 0.3)],
             [("skeleton", 3), ("mutant_skeleton", 1), ("invoker", 1)])
    lines += ["])", ""]
    (OUT / "structures.ron").write_text("\n".join(lines))


# ==============================================================================================
# Lote 4: weapons (yoyos, boomerangs, flails, guns and bullets, throwables, bombs, ore bows,
# sentries, swords that shoot, two more spells)
# ==============================================================================================

def icon2(rows, p1, p2):
    return outline(from_map(rows, {**pmap(p1), **pmap(p2, "abcde")}))


YOYO = [
    "................",
    "..a.............",
    "...a............",
    "....a...........",
    ".....a..........",
    "......a.........",
    ".......a1111....",
    "......112222....",
    ".....12233221...",
    ".....12344321...",
    ".....12355321...",
    ".....12344321...",
    ".....12233221...",
    "......1222211...",
    ".......1111.....",
    "................",
]

BOOMERANG = [
    "................",
    "................",
    "..1111..........",
    "..13321.........",
    "..123321........",
    "...123321.......",
    "....123321......",
    ".....12332111...",
    "......12333321..",
    ".......1234441..",
    ".......1233331..",
    "........11111...",
    "................",
    "................",
    "................",
    "................",
]

FLAIL = [
    "..........1111..",
    ".........134431.",
    "........1345531.",
    "........1344431.",
    "........1233321.",
    ".......b.11111..",
    "......b.........",
    ".....b..........",
    "....b...........",
    "...aa...........",
    "..aab...........",
    ".aab............",
    ".ab.............",
    "aa..............",
    "a...............",
    "................",
]

PISTOL = [
    "................",
    "................",
    "................",
    "....1111111111..",
    "...133333333331.",
    "...144444444441.",
    "...1333311111...",
    "...aaab1........",
    "..aabba.........",
    "..abba..........",
    ".aabba..........",
    ".abba...........",
    ".aaa............",
    "................",
    "................",
    "................",
]

MUSKET = [
    "................",
    "................",
    "................",
    "................",
    "...........11111",
    ".........1133331",
    "......11133441..",
    "....aa13341.....",
    "..aabbb11.......",
    ".aabbba.........",
    "aabba...........",
    "abba............",
    "aa..............",
    "................",
    "................",
    "................",
]

SHOTGUN = [
    "................",
    "................",
    "................",
    "...........1111.",
    ".........113331.",
    ".........134441.",
    "......1113331...",
    "....aa133431....",
    "..aabbb1111.....",
    ".aabbba.........",
    "aabba...........",
    "abba............",
    "aa..............",
    "................",
    "................",
    "................",
]

RAPID = [
    "................",
    "................",
    "................",
    "...........111..",
    "....1111111341..",
    "...13333333441..",
    "...14444444431..",
    "...1333113331...",
    "...aab1..1331...",
    "..aabb1...11....",
    "..abba..........",
    ".aabb...........",
    ".aaa............",
    "................",
    "................",
    "................",
]

BALL = [
    "................",
    "................",
    "................",
    "................",
    "...111....111...",
    "..13321..13321..",
    "..13421..13421..",
    "...111....111...",
    "......111.......",
    ".....13321......",
    ".....13421......",
    "......111.......",
    "................",
    "................",
    "................",
    "................",
]

SHURIKEN = [
    "................",
    "................",
    ".......1........",
    "......141.......",
    "......141.......",
    "......131.......",
    "..111.121.111...",
    "..14433233441...",
    "..111.121.111...",
    "......131.......",
    "......141.......",
    "......141.......",
    ".......1........",
    "................",
    "................",
    "................",
]

KNIFE = [
    "................",
    "..............1.",
    ".............141",
    "............1431",
    "...........1431.",
    "..........1431..",
    ".........1431...",
    "........1431....",
    ".......1431.....",
    "....b.1321......",
    ".....b121.......",
    ".....aab........",
    "....aab.b.......",
    "...aab..........",
    "...aa...........",
    "................",
]

BOMB = [
    "................",
    "..........a.....",
    ".........a.b....",
    "........aa......",
    ".......11.......",
    ".....111111.....",
    "....12222221....",
    "...1233222221...",
    "...1243222221...",
    "...1232222221...",
    "...1222222221...",
    "...1222222221...",
    "....12222221....",
    ".....111111.....",
    "................",
    "................",
]

GRENADE = [
    "................",
    "................",
    "......aaa.......",
    ".....a..ab......",
    "......111.......",
    ".....13331......",
    "....1344331.....",
    "....1232321.....",
    "....1323231.....",
    "....1232321.....",
    "....1323231.....",
    "....1232321.....",
    ".....12221......",
    "......111.......",
    "................",
    "................",
]

DYNAMITE = [
    "................",
    "........ab......",
    ".......a........",
    "......a.........",
    ".....1111.......",
    ".....1441.......",
    ".....1331.......",
    ".....1221.......",
    ".....1dd1.......",
    ".....1dd1.......",
    ".....1221.......",
    ".....1221.......",
    ".....1221.......",
    ".....1111.......",
    "................",
    "................",
]

BOW = [
    "................",
    ".........1111...",
    "........1a..21..",
    ".......1a....21.",
    "......1a......1.",
    ".....1a.......1.",
    "....1a........21",
    "...1a.........21",
    "..1a..........21",
    ".1a...........21",
    ".1............1.",
    ".1...........21.",
    ".12.........21..",
    "..12.......21...",
    "...12111121.....",
    "....1111........",
]

SENTRY = [
    "................",
    "........11111...",
    ".......1344431..",
    ".......1355531..",
    ".......1344431..",
    "........1aaa1...",
    ".........ab.....",
    "........ab......",
    ".......ab.......",
    "......ab........",
    ".....ab.........",
    "....ab..........",
    "...ab...........",
    "..ab............",
    "..a.............",
    "................",
]

SCYTHE = [
    "................",
    "....1111........",
    "...133331.......",
    "..13444431......",
    "..1431..1331....",
    "..131....1a1....",
    "..11.....a1.....",
    ".........a......",
    "........a.......",
    ".......a........",
    "......a.........",
    ".....a..........",
    "....a...........",
    "...a............",
    "................",
    "................",
]

WOOD = pal("#3a2410", "#6a4a24", "#8e6a3a", "#b08a52", "#d4b07a")
STEEL = pal("#2a2e36", "#5a606c", "#8a92a0", "#c0c8d4", "#eef2f8")
STRING = pal("#c8c0b0", "#e8e0d0")
FIRE = pal("#5a1404", "#b03a08", "#e86a10", "#f8a830", "#fff0a0")


def items_l4():
    gi.MATERIALS["enchanted"] = pal("#1a2a6a", "#2a4ab0", "#4a7ae8", "#8ab4ff", "#e0f0ff")
    gi.MATERIALS["star"] = pal("#5a3a04", "#c49418", "#ecc434", "#fce47a", "#ffffff")
    gi.WAND_GEMS["water"] = pal("#0a2a5a", "#1a5aa8", "#3a8ae8", "#8ac4ff", "#ffffff")
    umbral = ARMOR_L3["umbral"][1]
    sang = ARMOR_L3["sanguinite"][1]
    jungle = ARMOR_L3["jungle"][1]
    ice = pal("#2a4a6a", "#4a7aa0", "#7aaed4", "#b4daf4", "#eef8ff")
    copper = gi.MATERIALS["copper"]
    silver = gi.MATERIALS["silver"]
    plat = gi.MATERIALS["platinum"]
    gun = STEEL
    stack1 = "max_stack: Some(1)"
    out = [
        ("wood_yoyo", "Ioiô de Madeira", icon2(YOYO, WOOD, STRING), stack1),
        ("jungle_yoyo", "Ioiô da Selva", icon2(YOYO, jungle, STRING), stack1),
        ("umbral_yoyo", "Ioiô de Sombraço", icon2(YOYO, umbral, STRING), stack1),
        ("sanguinite_yoyo", "Ioiô de Sanguinita", icon2(YOYO, sang, STRING), stack1),
        ("magma_yoyo", "Ioiô de Magma", icon2(YOYO, FIRE, STRING), stack1),
        ("wood_boomerang", "Bumerangue de Madeira", icon(BOOMERANG, WOOD), stack1),
        ("enchanted_boomerang", "Bumerangue Encantado", icon(BOOMERANG, gi.MATERIALS["enchanted"]), stack1),
        ("ice_boomerang", "Bumerangue de Gelo", icon(BOOMERANG, ice), stack1),
        ("thorn_chakram", "Chakram de Espinhos", icon(SHURIKEN, jungle), stack1),
        ("flame_boomerang", "Bumerangue de Fogo", icon(BOOMERANG, FIRE), stack1),
        ("iron_flail", "Mangual de Ferro", icon2(FLAIL, STEEL, WOOD), stack1),
        ("umbral_flail", "Bola da Ruína", icon2(FLAIL, umbral, STEEL), stack1),
        ("sanguinite_flail", "Mangual de Sanguinita", icon2(FLAIL, sang, STEEL), stack1),
        ("sun_flail", "Fúria do Sol", icon2(FLAIL, FIRE, STEEL), stack1),
        ("flintlock_pistol", "Pistola de Pederneira", icon2(PISTOL, gun, WOOD), stack1),
        ("musket", "Mosquete", icon2(MUSKET, gun, WOOD), stack1),
        ("boomstick", "Espingarda", icon2(SHOTGUN, gun, WOOD), stack1),
        ("rapid_gun", "Metralhadora Leve", icon2(RAPID, pal("#2a3a4a", "#4a6a8a", "#7aa0c4", "#b4d4f0", "#eef8ff"), WOOD), stack1),
        ("musket_ball", "Bala de Mosquete", icon(BALL, pal("#2a2e36", "#5a606c", "#8a92a0", "#c0c8d4")), ""),
        ("meteor_shot", "Bala de Meteoro", icon(BALL, METEOR), ""),
        ("shuriken", "Shuriken", icon(SHURIKEN, STEEL), ""),
        ("throwing_knife", "Faca de Arremesso", icon2(KNIFE, STEEL, WOOD), ""),
        ("bomb", "Bomba", icon2(BOMB, pal("#0e0e14", "#2a2a36", "#5a5a6e", "#9a9ab0"), pal("#5a3a1a", "#f8a830")), "max_stack: Some(16)"),
        ("grenade", "Granada", icon2(GRENADE, pal("#1a2a14", "#2e4a24", "#4a6a3a", "#7a9a5a"), STEEL), "max_stack: Some(16)"),
        ("dynamite", "Dinamite", icon2(DYNAMITE, pal("#5a0e0e", "#a01c1c", "#d83a2a", "#f88a6a"), pal("#5a3a1a", "#f8a830", "#e8dcc0", "#e8dcc0")), "max_stack: Some(16)"),
        ("copper_bow", "Arco de Cobre", icon2(BOW, copper, STRING), "max_stack: Some(1), tool: Some((kind: Sword, tier: 1, speed: 1.0, damage: 1.0, durability: 300))"),
        ("silver_bow", "Arco de Prata", icon2(BOW, silver, STRING), "max_stack: Some(1), tool: Some((kind: Sword, tier: 1, speed: 1.0, damage: 1.0, durability: 450))"),
        ("platinum_bow", "Arco de Platina", icon2(BOW, plat, STRING), "max_stack: Some(1), tool: Some((kind: Sword, tier: 1, speed: 1.0, damage: 1.0, durability: 600))"),
        ("umbral_bow", "Arco de Sombraço", icon2(BOW, umbral, STRING), "max_stack: Some(1), tool: Some((kind: Sword, tier: 1, speed: 1.0, damage: 1.0, durability: 800))"),
        ("sanguinite_bow", "Arco de Sanguinita", icon2(BOW, sang, STRING), "max_stack: Some(1), tool: Some((kind: Sword, tier: 1, speed: 1.0, damage: 1.0, durability: 800))"),
        ("ballista_rod", "Cajado da Balista", icon2(SENTRY, pal("#3a2a14", "#7a5a2e", "#c4a060", "#f0d890", "#fff8e0"), WOOD), stack1),
        ("enchanted_sword", "Espada Encantada", gi.tool("enchanted", "sword"), "max_stack: Some(1), tool: Some((kind: Sword, tier: 2, speed: 8.0, damage: 6.0, durability: 900))"),
        ("star_saber", "Sabre Estelar", gi.tool("star", "sword"), "max_stack: Some(1), tool: Some((kind: Sword, tier: 3, speed: 8.0, damage: 7.0, durability: 1200))"),
        ("water_bolt", "Raio d'Água", gi.wand("water"), stack1),
        ("demon_scythe", "Foice Demoníaca", icon2(SCYTHE, umbral, WOOD), stack1),
    ]
    return out


SHAPED_L4 = [
    ([" PS", "PRS", " PS"], {"P": "oak_planks", "S": "string", "R": "stick"}, "wood_yoyo", 1),
    ([" P ", "PSP", " P "], {"P": "jungle_spore", "S": "string"}, "jungle_yoyo", 1),
    ([" P ", "PSP", " P "], {"P": "umbral_ingot", "S": "string"}, "umbral_yoyo", 1),
    ([" P ", "PSP", " P "], {"P": "sanguinite_ingot", "S": "string"}, "sanguinite_yoyo", 1),
    ([" P ", "PSP", " P "], {"P": "magmarite_ingot", "S": "string"}, "magma_yoyo", 1),
    (["PP ", " P ", " PP"], {"P": "oak_planks"}, "wood_boomerang", 1),
    (["PP ", " F ", " PP"], {"P": "wood_boomerang", "F": "fallen_star"}, "enchanted_boomerang", 1),
    (["PP ", " F ", " PP"], {"P": "ice", "F": "frost_shard"}, "ice_boomerang", 1),
    ([" J ", "JSJ", " J "], {"J": "jungle_spore", "S": "cactus"}, "thorn_chakram", 1),
    (["PP ", " F ", " PP"], {"P": "magmarite_ingot", "F": "blaze_powder"}, "flame_boomerang", 1),
    (["  I", " C ", "S  "], {"I": "iron_block", "C": "chain", "S": "stick"}, "iron_flail", 1),
    (["  I", " C ", "S  "], {"I": "umbral_ingot", "C": "chain", "S": "stick"}, "umbral_flail", 1),
    (["  I", " C ", "S  "], {"I": "sanguinite_ingot", "C": "chain", "S": "stick"}, "sanguinite_flail", 1),
    (["  I", " C ", "S  "], {"I": "magmarite_ingot", "C": "chain", "S": "stick"}, "sun_flail", 1),
    (["III", " WF"], {"I": "iron_ingot", "W": "oak_planks", "F": "flint"}, "flintlock_pistol", 1),
    (["  I", " I ", "WF "], {"I": "iron_ingot", "W": "oak_planks", "F": "flint"}, "musket", 1),
    (["III", "IWF"], {"I": "iron_ingot", "W": "oak_planks", "F": "gunpowder"}, "boomstick", 1),
    (["III", "IGI", " W "], {"I": "iron_ingot", "G": "gunpowder", "W": "lens"}, "rapid_gun", 1),
    ([" I ", "ISI", " I "], {"I": "iron_ingot", "S": "stick"}, "shuriken", 8),
    (["  I", " I ", "S  "], {"I": "iron_ingot", "S": "stick"}, "throwing_knife", 6),
    ([" S ", "GIG", " G "], {"S": "string", "G": "gunpowder", "I": "iron_nugget"}, "bomb", 2),
    ([" S ", "GIG"], {"S": "string", "G": "gunpowder", "I": "iron_ingot"}, "grenade", 3),
    (["S", "T", "T"], {"S": "string", "T": "tnt"}, "dynamite", 1),
    ([" IS", "I S", " IS"], {"I": "copper_ingot", "S": "string"}, "copper_bow", 1),
    ([" IS", "I S", " IS"], {"I": "silver_ingot", "S": "string"}, "silver_bow", 1),
    ([" IS", "I S", " IS"], {"I": "platinum_ingot", "S": "string"}, "platinum_bow", 1),
    ([" IS", "I S", " IS"], {"I": "umbral_ingot", "S": "string"}, "umbral_bow", 1),
    ([" IS", "I S", " IS"], {"I": "sanguinite_ingot", "S": "string"}, "sanguinite_bow", 1),
    (["GWG", " S ", " S "], {"G": "gold_ingot", "W": "crossbow", "S": "stick"}, "ballista_rod", 1),
    (["F", "F", "S"], {"F": "fallen_star", "S": "iron_sword"}, "enchanted_sword", 1),
    (["FFF", "FSF", "FFF"], {"F": "fallen_star", "S": "sky_blade"}, "star_saber", 1),
    (["WBW", "WWW"], {"W": "water_bucket", "B": "book"}, "water_bolt", 1),
    (["UUU", "U S", "  S"], {"U": "umbral_ingot", "S": "dark_shard"}, "demon_scythe", 1),
]
SHAPELESS_L4 = [
    (["lead_ingot"], "musket_ball", 50),
    (["iron_nugget", "iron_nugget", "gunpowder"], "musket_ball", 25),
    (["meteorite_ingot", "musket_ball", "musket_ball", "musket_ball", "musket_ball", "musket_ball"], "meteor_shot", 5),
]


# ==============================================================================================
# Lote 5: wings, item mounts, pets and light pets, souls, harpies of the high sky
# ==============================================================================================

WING = [
    "................",
    "..1.........1...",
    ".121.......121..",
    ".1231.....1321..",
    "12341.....14321.",
    "123451...154321.",
    "1234521.1254321.",
    ".123452.2543211.",
    ".12345222543221.",
    "..12345254321...",
    "..123451543211..",
    "...1234.4321....",
    "...123...321....",
    "....12...21.....",
    ".....1...1......",
    "................",
]

SOUL = [
    "................",
    "................",
    "......1111......",
    ".....123321.....",
    "....12344321....",
    "....13455431....",
    "....13455431....",
    "....12344321....",
    ".....123321.....",
    "......1221......",
    ".......12.......",
    "......12........",
    ".....12.........",
    "................",
    "................",
    "................",
]

SADDLE = [
    "................",
    "................",
    "................",
    "....11111111....",
    "...1222222221...",
    "..122333333221..",
    "..123344443321..",
    "..123344443321..",
    "..1aaaaaaaaaa1..",
    "...1a......a1...",
    "...1a......a1...",
    "....b......b....",
    "....b......b....",
    "................",
    "................",
    "................",
]

APPLE = [
    "................",
    "........ab......",
    ".......ab.......",
    ".....1111111....",
    "....122222221...",
    "...12333333221..",
    "...12344433221..",
    "...12345433221..",
    "...12344433221..",
    "...12333333221..",
    "....122222221...",
    ".....1111111....",
    "................",
    "................",
    "................",
    "................",
]

LAMP = [
    "................",
    "......aaaa......",
    ".....a....a.....",
    "......aaaa......",
    ".....bbbbbb.....",
    "....1111111b....",
    "....1234321.....",
    "....1345431.....",
    "....1355531.....",
    "....1345431.....",
    "....1234321.....",
    "....1111111.....",
    ".....bbbbbb.....",
    "................",
    "................",
    "................",
]

WINGS_L5 = [
    # name, display, palette, recipe material
    ("fairy_wings", "Asas de Fada", pal("#5a8aa0", "#8ac4d6", "#c0e6f0", "#f0d0f0", "#ffffff"), "pixie_dust"),
    ("demon_wings", "Asas Demoníacas", pal("#1a0a14", "#3a1428", "#5a2040", "#8a3a5a", "#c86a8a"), "soul_of_night"),
    ("leaf_wings", "Asas de Folha", pal("#0e2a10", "#1e4a1c", "#3a7a2a", "#6aaa4a", "#b4e08a"), "jungle_spore"),
    ("frost_wings", "Asas de Gelo", pal("#2a4a6a", "#4a7aa0", "#7aaed4", "#b4daf4", "#eef8ff"), "frost_shard"),
    ("flame_wings", "Asas de Chama", FIRE, "magmarite_ingot"),
    ("bee_wings", "Asas de Abelha", pal("#3a2204", "#8a5a0e", "#d49a1e", "#f4c84a", "#fff0a0"), "beeswax"),
]

MOUNT_PARTS_JELLY = """[
            (size: (12, 10, 12), origin: (-6, 0, -6), uv: (0, 0)),
            (size: (8, 2, 6), origin: (-4, 10, -3), uv: (0, 24)),
        ]"""

HARPY_PARTS = """[
            (size: (6, 10, 4), origin: (-3, 8, -2), uv: (0, 0)),
            (size: (6, 6, 6), origin: (-3, 18, -3), uv: (0, 16), anim: Head, pivot: (0, 18, 0)),
            (size: (14, 1, 8), origin: (-17, 16, -3), uv: (0, 30), anim: Wing(0), pivot: (-3, 16, 1)),
            (size: (14, 1, 8), origin: (3, 16, -3), uv: (0, 30), anim: Wing(1), pivot: (3, 16, 1)),
            (size: (2, 8, 2), origin: (-3, 0, -1), uv: (24, 0), anim: Leg(0), pivot: (-2, 8, 0)),
            (size: (2, 8, 2), origin: (1, 0, -1), uv: (24, 0), anim: Leg(1), pivot: (2, 8, 0)),
        ]"""

WISP_PARTS = """[
            (size: (4, 4, 4), origin: (-2, 0, -2), uv: (0, 0), anim: Head, pivot: (0, 2, 0)),
        ]"""


def mobs_l5():
    def pet(name, display, parts, scale, layers="[]", w=0.5, h=0.5):
        return f"""    (
        name: "{name}", display_name: "{display}", kind: Passive, health: 50, speed: 3.0, attack: 0,
        drops: [], xp: 0,
        width: {w}, height: {h}, spawn: Never, scale: {scale}, flies: true,
        parts: {parts},
        layers: {layers},
    ),"""

    def mount(name, display, parts, scale, speed, w, h, layers, extra=""):
        return f"""    (
        name: "{name}", display_name: "{display}", kind: Passive, health: 200, speed: {speed}, attack: 0,
        drops: [], xp: 0,
        width: {w}, height: {h}, spawn: Never, scale: {scale}{extra},
        parts: {parts},
        layers: {layers},
    ),"""

    return ["    // Lote 5: pets, light pets, item mounts and the harpies of the high sky.",
            pet("pet_eye", "Olhinho", EYE_PARTS, 0.45),
            pet("pet_bee", "Abelhinha", BEE_PARTS, 0.5),
            pet("pet_jelly", "Geleiazinha", JELLY_PARTS, 0.5, w=0.4, h=0.35),
            pet("pet_fairy", "Fada", PIXIE_PARTS, 0.9, "[(part: 0, inflate: 0.4)]"),
            pet("pet_orb", "Orbe Sombrio", WISP_PARTS, 1.6, w=0.4, h=0.4),
            pet("pet_heart", "Coração Pulsante", WISP_PARTS, 1.6, w=0.4, h=0.4),
            pet("pet_wisp", "Fogo-Fátuo", WISP_PARTS, 1.3, w=0.3, h=0.3),
            mount("mount_jelly", "Geleia de Montaria", MOUNT_PARTS_JELLY, 3.0, 3.2, 1.6, 1.4, "[(part: 0, inflate: 0.5)]", ", hops: true"),
            mount("mount_bee", "Abelha de Montaria", BEE_PARTS, 1.8, 3.6, 1.4, 1.2, "[(part: 0, inflate: 0.5)]"),
            mount("mount_unicorn", "Unicórnio de Montaria", UNICORN_PARTS, 1.1, 5.0, 1.3, 1.9, "[(part: 0, inflate: 0.5), (part: 1, inflate: 0.4)]"),
            f"""    (
        name: "harpy", display_name: "Harpia", kind: Hostile, health: 40, speed: 0.1, attack: 5,
        drops: [(item: "feather", min: 1, max: 3)], xp: 6,
        width: 0.7, height: 1.6, spawn: Never, scale: 1.3, flies: true,
        parts: {HARPY_PARTS},
        layers: [(part: 1, inflate: 0.5)],
    ),"""]


HARPY = pal("#6a4a8a", "#8a6aaa", "#aa8ac8", "#c8a8e0", "#e4d0f4")
HARPY_SKIN = pal("#b08a6a", "#c8a07a", "#dcb88e", "#ecd0a8")


def harpy_painter(c):
    if c.role == "wing":
        return g.pick(HARPY, 0.15 + 0.6 * c.jit + 0.2 * ((c.i // 2) % 2))
    if c.role == "head":
        if c.face == "front" and c.j >= 2:
            if c.j == 3 and c.i in (1, 4):
                return (40, 20, 60)
            return g.pick(HARPY_SKIN, 0.3 + 0.5 * c.jit)
        return g.pick(HARPY, 0.4 + 0.5 * c.jit)
    if c.role == "leg":
        return g.pick(pal("#c49418", "#ecc434", "#fce47a"), c.jit)
    return g.pick(HARPY, 0.2 + 0.6 * c.jit)


def wisp_painter(p):
    def f(c):
        cx, cy = c.i - (c.fw - 1) / 2, c.j - (c.fh - 1) / 2
        return g.pick(p, 0.95 - 0.25 * (cx * cx + cy * cy) + 0.1 * c.jit)
    return f


def skins_l5():
    base = skins_l1()
    l3 = skins_l3()
    vein = pal("#6a0a12", "#8a1420", "#a8202c")
    return {
        "pet_eye": (["eye", "stalk", "stalk", "stalk"], e5s.eye_painter(pal("#e8e2da", "#f0ece6", "#fbf8f4"), vein, pal("#2a4a9a", "#3a6ac4", "#6a9ae8")), e5s.eye_overlay(vein)),
        "pet_bee": base["hive_drone"],
        "pet_jelly": (["jelly"], jelly_painter(JELLY_PALS["jelly_pink"]), jelly_overlay(JELLY_PALS["jelly_pink"])),
        "pet_fairy": l3["prism_pixie"],
        "pet_orb": (["orb"], wisp_painter(pal("#2a1040", "#5a2a8a", "#9a5ad4", "#d4a8ff", "#ffffff")), None),
        "pet_heart": (["orb"], wisp_painter(pal("#3a0408", "#a0141e", "#e8343a", "#ff9a8a", "#ffffff")), None),
        "pet_wisp": (["orb"], wisp_painter(pal("#0a3a2a", "#1a8a6a", "#3ae0b0", "#a0ffe0", "#ffffff")), None),
        "mount_jelly": (["jelly", "saddle"], jelly_painter(JELLY_PALS["jelly_blue"], pal("#3a2010", "#5a3418", "#7a4a24", "#9a6232")), jelly_overlay(JELLY_PALS["jelly_blue"])),
        "mount_bee": base["hive_queen"],
        "mount_unicorn": l3["prism_unicorn"],
        "harpy": (["body", "head", "wing", "wing", "leg", "leg"], harpy_painter, None),
    }


def items_l5():
    stack1 = "max_stack: Some(1)"
    out = [
        ("soul_of_flight", "Alma do Voo", icon(SOUL, pal("#2a4a6a", "#4a8ac4", "#8ac4f0", "#c8e8ff", "#ffffff")), ""),
        ("soul_of_light", "Alma da Luz", icon(SOUL, pal("#6a3a5a", "#c47aa8", "#f0b4d8", "#ffe0f0", "#ffffff")), ""),
        ("soul_of_night", "Alma da Noite", icon(SOUL, pal("#1a0a2a", "#3a1a5a", "#6a3a9a", "#a87ad0", "#e0c8ff")), ""),
        ("jelly_saddle", "Sela Gelatinosa", icon2(SADDLE, JELLY_PALS["jelly_blue"], WOOD), stack1),
        ("honey_goggles", "Óculos de Mel", icon2(SADDLE, pal("#3a2204", "#8a5a0e", "#d49a1e", "#f4c84a", "#fff0a0"), STEEL), stack1),
        ("blessed_apple", "Maçã Abençoada", icon2(APPLE, pal("#7a5408", "#c49418", "#ecc434", "#fce47a", "#ffffff"), pal("#3a2a10", "#3a8a2a")), stack1),
        ("eye_spring", "Olho na Mola", e5.eye("#f0ece6", "#3a6ac4"), stack1),
        ("honey_jar", "Pote de Mel", icon2(LAMP, pal("#6a3e08", "#a0640e", "#d49a1e", "#f4c84a", "#fff0a0"), WOOD), stack1),
        ("jelly_bubble", "Bolha de Geleia", icon(GEL, JELLY_PALS["jelly_pink"]), stack1),
        ("fairy_bell", "Sino de Fada", icon2(LAMP, pal("#5a8aa0", "#8ac4d6", "#c0e6f0", "#f0d0f0", "#ffffff"), gi.MATERIALS["golden"]), stack1),
        ("shadow_orb_lamp", "Orbe Sombrio", icon2(LAMP, pal("#2a1040", "#5a2a8a", "#9a5ad4", "#d4a8ff", "#ffffff"), STEEL), stack1),
        ("blight_lamp", "Coração Pulsante", icon2(LAMP, pal("#3a0408", "#a0141e", "#e8343a", "#ff9a8a", "#ffffff"), STEEL), stack1),
        ("wisp_jar", "Pote de Fogo-Fátuo", icon2(LAMP, pal("#0a3a2a", "#1a8a6a", "#3ae0b0", "#a0ffe0", "#ffffff"), STEEL), stack1),
    ]
    for name, disp, p, _ in WINGS_L5:
        out.append((name, disp, icon(WING, p), stack1))
    return out


SHAPED_L5 = [(["W W", "WSW", "W W"], {"W": mat, "S": "soul_of_flight"}, name, 1) for name, _, _, mat in WINGS_L5] + [
    ([" G ", "PPP", "PPP"], {"G": "gold_nugget", "P": "pixie_dust"}, "fairy_bell", 1),
    ([" G ", "DSD", " B "], {"G": "glowstone_dust", "D": "dark_shard", "S": "soul_of_night", "B": "glass_bottle"}, "wisp_jar", 1),
]
SHAPELESS_L5 = []


# ==============================================================================================

def write_data(items, blocks, mobs, shaped, shapeless, smelting):
    OUT.mkdir(parents=True, exist_ok=True)
    known = known_items() | {i[0] for i in items} | {b.split('"')[1] for b in blocks}
    head = "// The rest of the Terraria side (plan/terraria-completo.md). Written by tools/gen_tr.py."
    lines = [head, "(items: ["]
    for name, display, _, extra in items:
        e = f", {extra}" if extra else ""
        lines.append(f'    (name: "{name}", display_name: Some("{display}"){e}),')
    lines += ["])", ""]
    (OUT / "items.ron").write_text("\n".join(lines))
    (OUT / "blocks.ron").write_text("\n".join([head, "(blocks: ["] + [f"    {b}," for b in blocks] + ["])", ""]))
    (OUT / "mobs.ron").write_text("\n".join([head, "(mobs: ["] + mobs + ["])", ""]))
    bad = []
    rl = [head, "(", "    shaped: ["]
    for pattern, key, result, count in shaped:
        used = set("".join(pattern)) - {" "}
        assert used == set(key), (result, used, set(key))
        for v in list(key.values()) + [result]:
            if v not in known:
                bad.append((result, v))
        pat = ", ".join(f'"{r}"' for r in pattern)
        k = ", ".join(f"'{c}': \"{v}\"" for c, v in key.items())
        c = f", count: {count}" if count != 1 else ""
        rl.append(f"        (pattern: [{pat}], key: {{{k}}}, result: \"{result}\"{c}),")
    rl += ["    ],", "    shapeless: ["]
    for ings, result, count in shapeless:
        for v in ings + [result]:
            if v not in known:
                bad.append((result, v))
        ing = ", ".join(f'"{i}"' for i in ings)
        c = f", count: {count}" if count != 1 else ""
        rl.append(f"        (ingredients: [{ing}], result: \"{result}\"{c}),")
    rl += ["    ],", "    smelting: ["]
    for inp, result, xp in smelting:
        for v in (inp, result):
            if v not in known:
                bad.append((result, v))
        rl.append(f"        (input: \"{inp}\", result: \"{result}\", xp: {xp}),")
    rl += ["    ],", ")", ""]
    if bad:
        raise SystemExit(f"unknown items in recipes: {bad}")
    (OUT / "recipes.ron").write_text("\n".join(rl))


def as_int(f):
    def h(c):
        r = f(c)
        return None if r is None else tuple(int(round(float(v))) for v in r[:3])
    return h


# ==============================================================================================
# Lote 6: the town grows: more town folk, the peddler, the party, pylons and town animals
# ==============================================================================================

PERSON_PARTS = """[
            (size: (8, 8, 8), origin: (-4, 24, -4), uv: (0, 0), anim: Head, pivot: (0, 24, 0)),
            (size: (8, 12, 4), origin: (-4, 12, -2), uv: (16, 16)),
            (size: (4, 12, 4), origin: (-8, 12, -2), uv: (40, 16), anim: Arm(0), pivot: (-6, 22, 0)),
            (size: (4, 12, 4), origin: (4, 12, -2), uv: (32, 48), anim: Arm(1), pivot: (6, 22, 0)),
            (size: (4, 12, 4), origin: (-4, 0, -2), uv: (0, 16), anim: Leg(0), pivot: (-2, 12, 0)),
            (size: (4, 12, 4), origin: (0, 0, -2), uv: (16, 48), anim: Leg(1), pivot: (2, 12, 0)),
        ]"""
PERSON_LAYERS = "[(part: 0, inflate: 0.5), (part: 1, inflate: 0.3), (part: 2, inflate: 0.3), (part: 3, inflate: 0.3), (part: 4, inflate: 0.3), (part: 5, inflate: 0.3)]"

SKIN_LIGHT = pal("#c49070", "#d4a080", "#e0b090", "#eac0a0", "#f2d0b4")
SKIN_TAN = pal("#a8694a", "#b97a58", "#c78a65", "#d29a74", "#dca984")
SKIN_DARK = pal("#4a2a1a", "#5a3422", "#6a3e2a", "#7a4a32", "#8a563a")
SKIN_DEEP = pal("#3a2014", "#46281a", "#523020", "#5e3826", "#6a402c")
SKIN_METAL = pal("#5a606c", "#6e7682", "#8a92a0", "#a4acb8", "#c0c8d4")

# name, display, skin, hair, tunic, trousers, leather, iris
TOWN_L6 = [
    ("npc_dryad", "Druida", SKIN_LIGHT, pal("#2a5a1a", "#3a7a22", "#4a9a2a", "#5aba34", "#7ad44a"), pal("#3a6a1e", "#4a8226", "#5a9a2e", "#6ab236", "#7aca40"), pal("#2a4a14", "#345a1a", "#3e6a20", "#487a26"), pal("#5a3a1a", "#6a4420", "#7a4e26", "#8a582c"), (90, 200, 90)),
    ("npc_demolitionist", "Demolidor", SKIN_TAN, pal("#6a6a6a", "#7a7a7a", "#8a8a8a", "#9a9a9a", "#aaaaaa"), pal("#5a4a1a", "#6a5820", "#7a6626", "#8a742c", "#9a8232"), pal("#3a2a1a", "#463220", "#523a26", "#5e422c"), pal("#2a2a2a", "#363636", "#424242", "#4e4e4e"), (60, 60, 60)),
    ("npc_arms_dealer", "Armeira", SKIN_DARK, pal("#141414", "#1c1c1c", "#242424", "#2c2c2c", "#343434"), pal("#2a2e36", "#343a44", "#3e4652", "#485260", "#525e6e"), pal("#1e1e24", "#26262e", "#2e2e38", "#363642"), pal("#5a3a1a", "#6a4420", "#7a4e26", "#8a582c"), (120, 80, 40)),
    ("npc_painter", "Pintor", SKIN_LIGHT, pal("#5a3a1a", "#6a4420", "#7a4e26", "#8a582c", "#9a6232"), pal("#c8c0b0", "#d4ccbc", "#e0d8c8", "#ece4d4", "#f8f0e0"), pal("#3a4a6a", "#44547a", "#4e5e8a", "#58689a"), pal("#a03030", "#b83a3a", "#d04444", "#e84e4e"), (70, 110, 170)),
    ("npc_dye_trader", "Tintureira", SKIN_DEEP, pal("#4a1a5a", "#5a226e", "#6a2a82", "#7a3296", "#8a3aaa"), pal("#a03a10", "#b84a14", "#d05a18", "#e86a1c", "#f87a24"), pal("#2a5a5a", "#326a6a", "#3a7a7a", "#428a8a"), pal("#c49a10", "#d4aa18", "#e4ba20", "#f4ca28"), (200, 120, 40)),
    ("npc_stylist", "Barbeira", SKIN_TAN, pal("#c43a6a", "#d44a7a", "#e45a8a", "#f06a9a", "#fa7aaa"), pal("#2a2a3a", "#343446", "#3e3e52", "#48485e", "#52526a"), pal("#1a1a24", "#22222e", "#2a2a38", "#323242"), pal("#c8c8d0", "#d4d4dc", "#e0e0e8", "#ececf4"), (150, 60, 120)),
    ("npc_angler", "Pescador", SKIN_LIGHT, pal("#c49a3a", "#d4aa4a", "#e4ba5a", "#f0ca6a", "#f8da7a"), pal("#c4a020", "#d4b02a", "#e4c034", "#f0d03e", "#f8e048"), pal("#2a4a6a", "#325478", "#3a5e86", "#426894"), pal("#5a3a1a", "#6a4420", "#7a4e26", "#8a582c"), (60, 140, 200)),
    ("npc_tailor", "Alfaiate", SKIN_TAN, pal("#8a8a8a", "#9a9a9a", "#aaaaaa", "#bababa", "#cacaca"), pal("#5a1a1a", "#6e2222", "#822a2a", "#963232", "#aa3a3a"), pal("#1e1a26", "#262030", "#2e263a", "#362c44"), pal("#c49a10", "#d4aa18", "#e4ba20", "#f4ca28"), (90, 70, 50)),
    ("npc_party_girl", "Festeira", SKIN_LIGHT, pal("#e04a9a", "#ec5aaa", "#f46aba", "#fa7aca", "#ff8ada"), pal("#3ab4e0", "#4ac4ec", "#5ad4f4", "#6ae0fa", "#7aecff"), pal("#c4308a", "#d43a9a", "#e444aa", "#f04eba"), pal("#f0d020", "#f8e030", "#fff040", "#fff860"), (230, 90, 170)),
    ("npc_witch_doctor", "Curandeiro da Selva", SKIN_DEEP, pal("#1a1410", "#241c16", "#2e241c", "#382c22", "#423428"), pal("#3a5a1a", "#466a20", "#527a26", "#5e8a2c", "#6a9a32"), pal("#5a3a1a", "#6a4420", "#7a4e26", "#8a582c"), pal("#c4a060", "#d4b070", "#e4c080", "#f0d090"), (230, 200, 60)),
    ("npc_pirate", "Pirata", SKIN_TAN, pal("#1a1410", "#241c16", "#2e241c", "#382c22", "#423428"), pal("#8a1a1a", "#a02020", "#b82828", "#cc3030", "#e03838"), pal("#1a1a24", "#22222e", "#2a2a38", "#323242"), pal("#c49a10", "#d4aa18", "#e4ba20", "#f4ca28"), (60, 40, 30)),
    ("npc_steampunker", "Engenheira a Vapor", SKIN_LIGHT, pal("#c46a1a", "#d47a22", "#e48a2a", "#f09a34", "#f8aa3e"), pal("#5a3a1a", "#6a4420", "#7a4e26", "#8a582c", "#9a6232"), pal("#3a2a1a", "#463220", "#523a26", "#5e422c"), pal("#b8862a", "#c89434", "#d8a23e", "#e8b048"), (140, 90, 40)),
    ("npc_cyborg", "Ciborgue", SKIN_METAL, pal("#2a2e36", "#343a44", "#3e4652", "#485260", "#525e6e"), pal("#3a3e48", "#4c515c", "#646a78", "#7e8492", "#9aa0ae"), pal("#2a2c32", "#383b42", "#484c54", "#5a5e68"), pal("#c41420", "#e02028", "#f03a3a", "#ff5a4a"), (255, 60, 50)),
    ("npc_zoologist", "Zoóloga", SKIN_DARK, pal("#c47a2a", "#d48a34", "#e49a3e", "#f0aa4a", "#f8ba56"), pal("#7a6a3a", "#8a7a44", "#9a8a4e", "#aa9a58", "#baaa62"), pal("#5a4a2a", "#665432", "#725e3a", "#7e6842"), pal("#5a3a1a", "#6a4420", "#7a4e26", "#8a582c"), (90, 150, 60)),
    ("npc_princess", "Princesa", SKIN_LIGHT, pal("#e4c040", "#ecca50", "#f4d460", "#f8de70", "#fce880"), pal("#e07aaa", "#ec8aba", "#f49aca", "#faaad8", "#ffbae4"), pal("#c46a9a", "#d07aaa", "#dc8aba", "#e89aca"), pal("#e4c040", "#ecca50", "#f4d460", "#f8de70"), (100, 160, 230)),
    ("npc_peddler", "Mascate", SKIN_TAN, pal("#5a5a5a", "#6a6a6a", "#7a7a7a", "#8a8a8a", "#9a9a9a"), pal("#4a2a5a", "#56326a", "#623a7a", "#6e428a", "#7a4a9a"), pal("#3a2a1a", "#463220", "#523a26", "#5e422c"), pal("#c49a10", "#d4aa18", "#e4ba20", "#f4ca28"), (120, 90, 160)),
]


def mobs_l6():
    out = ["    // Lote 6: town folk who move in when their wish is met, and the peddler who visits."]
    for name, display, *_ in TOWN_L6:
        out.append(f"""    (
        name: "{name}", display_name: "{display}", kind: Passive, health: 40, speed: 2.0,
        xp: 0,
        width: 0.6, height: 1.95, spawn: Never, attack: 0, town: true,
        parts: {PERSON_PARTS},
        layers: {PERSON_LAYERS},
    ),""")
    return out


def skins_l6():
    out = {}
    for name, _, skin, hair, tunic, trouser, leather, iris in TOWN_L6:
        colours = dict(PSKIN=skin, PHAIR=hair, PTUNIC=tunic, PTROUSER=trouser, PLEATHER=leather, PIRIS=iris)
        out[name] = (g.ROLES["player"], g.townsfolk(name, g.PAINTERS["player"], colours), g.townsfolk(name, g.OVERLAYS["player"], colours))
    return out


PYLONS_L6 = [
    ("forest", "Floresta", pal("#0e3a14", "#1e6a26", "#3aa040", "#7ad46a", "#d0ffc0")),
    ("desert", "Deserto", pal("#5a3a08", "#a0700e", "#e0aa20", "#f8d860", "#fff4c0")),
    ("snow", "Neve", pal("#2a4a6a", "#4a7aa0", "#7aaed4", "#b4daf4", "#ffffff")),
    ("jungle", "Selva", pal("#2a3a08", "#4a6a0e", "#7aa018", "#b4d440", "#ecff9a")),
    ("ocean", "Oceano", pal("#0a2a4a", "#14508a", "#2080c4", "#5ab4f0", "#d0f0ff")),
    ("cave", "Caverna", pal("#2a1a3a", "#4a2e6a", "#7a4aa0", "#b080d4", "#f0d8ff")),
]
BLOCKS_L6 = [
    f'(name: "pylon_{k}", display_name: Some("Pilone da {d}" ), light: 12, hardness: 1.5, tool: Some(Pickaxe))'.replace("da Oceano", "do Oceano").replace("da Deserto", "do Deserto").replace(" )", ")")
    for k, d, _ in PYLONS_L6
]


def blocks_l6():
    out = {}
    for k, (key, _, p) in enumerate(PYLONS_L6):
        def crystal(x, y):
            frame = x in (0, 15) or y in (0, 15)
            dx, dy = abs(x - 7.5), abs(y - 7.5)
            gem = dx * 1.6 + dy < 8.5
            if frame:
                return 0.3
            if gem:
                return 0.95 - (dx + dy) * 0.05 + (0.15 if (x + y) % 5 == 0 else 0)
            return 0.15
        out[f"pylon_{key}"] = block_noise(60 + k, crystal, p)
    return out


CONFETTI = [
    "................",
    "..1.....4.......",
    ".......a....2...",
    "....3........b..",
    "..a.....1.......",
    "......4.....3...",
    ".2...........a..",
    "......bbbbb.....",
    ".....b55555b....",
    "....b5444445b...",
    "....b4333334b...",
    "....b3222223b...",
    "....b2111112b...",
    ".....b11111b....",
    "......bbbbb.....",
    "................",
]
PARTY_KIT = [
    "................",
    ".......aa.......",
    "......a55a......",
    "......a54a......",
    ".......aa.......",
    ".......b........",
    "....11111111....",
    "....12222221....",
    "....12333321....",
    "....bbbbbbbb....",
    "....12333321....",
    "....12233221....",
    "....12222221....",
    "....11111111....",
    "................",
    "................",
]
LICENSE = [
    "................",
    "................",
    "..111111111111..",
    "..155555555551..",
    "..154444444451..",
    "..154aaa444451..",
    "..154aba444451..",
    "..154aaa444451..",
    "..154444333451..",
    "..154444444451..",
    "..154333333451..",
    "..155555555551..",
    "..111111111111..",
    "................",
    "................",
    "................",
]
PAPER = pal("#6a5a3a", "#a89470", "#d8c8a0", "#ece0c0", "#f8f0dc")


def items_l6():
    stack1 = "max_stack: Some(1)"
    return [
        ("confetti", "Confete", icon2(CONFETTI, pal("#c43a6a", "#e4c040", "#3ab4e0", "#7ad46a", "#f8f0dc"), pal("#c43a6a", "#3ab4e0")), ""),
        ("party_kit", "Kit de Festa", icon2(PARTY_KIT, pal("#8a1a5a", "#c43a8a", "#f07ab4", "#ffffff", "#ffffff"), pal("#c49a10", "#3ab4e0", "#f8e048", "#e04a4a", "#ffffff")), ""),
        ("cat_license", "Licença de Gato", icon2(LICENSE, PAPER, pal("#3a2a1a", "#c47a2a")), stack1),
        ("dog_license", "Licença de Cachorro", icon2(LICENSE, PAPER, pal("#3a2a1a", "#8a5a3a")), stack1),
        ("bunny_license", "Licença de Coelho", icon2(LICENSE, PAPER, pal("#3a2a1a", "#e8e0d8")), stack1),
    ]


# ==============================================================================================
# Lote 7: the last bosses (Jelly Queen, Scale Idol in its temple, Tide Duke, Prism Empress,
# Lunar Prophet and the four pillars), ectoplasm, sporite, fragments and the end-game sets
# ==============================================================================================

def shade(p, glow=None):
    """A plain painter for a palette: lighter on top, darker below, a little noise."""
    def f(c):
        t = 0.3 + 0.5 * c.jit
        if c.face == "top":
            t += 0.2
        elif c.face == "bottom":
            t -= 0.25
        return g.pick(p, t)
    return f


def zoned(parts):
    def paint(c):
        return parts.get(c.role, parts.get("*"))(c)
    return paint


def eyes_on(base, colour, rows=(2, 3)):
    def f(c):
        if c.face == "front" and c.j in rows and (c.i in (1, 2, c.fw - 3, c.fw - 2)):
            return colour
        return base(c)
    return f


def bands(p, every=4):
    def f(c):
        t = 0.35 + 0.4 * c.jit + (0.25 if c.j % every == 0 else 0.0)
        if c.face in ("front", "back", "left", "right") and c.i % 4 == 1 and c.j % every != 0:
            t += 0.3
        return g.pick(p, t)
    return f


QUEEN_PARTS_L7 = """[
            (size: (12, 10, 12), origin: (-6, 0, -6), uv: (0, 0)),
            (size: (8, 3, 8), origin: (-4, 10, -4), uv: (0, 24)),
            (size: (14, 1, 8), origin: (-20, 8, -4), uv: (0, 40), anim: Wing(0), pivot: (-6, 8, 0)),
            (size: (14, 1, 8), origin: (6, 8, -4), uv: (0, 40), anim: Wing(1), pivot: (6, 8, 0)),
        ]"""
IDOL_PARTS = """[
            (size: (16, 16, 12), origin: (-8, 4, -6), uv: (0, 0)),
            (size: (12, 10, 10), origin: (-6, 20, -5), uv: (0, 28), anim: Head, pivot: (0, 20, 0)),
            (size: (7, 7, 7), origin: (-16, 8, -4), uv: (0, 49), anim: Arm(0), pivot: (-9, 16, 0)),
            (size: (7, 7, 7), origin: (9, 8, -4), uv: (0, 49), anim: Arm(1), pivot: (9, 16, 0)),
            (size: (5, 4, 6), origin: (-7, 0, -3), uv: (28, 49), anim: Leg(0), pivot: (-4, 4, 0)),
            (size: (5, 4, 6), origin: (2, 0, -3), uv: (28, 49), anim: Leg(1), pivot: (4, 4, 0)),
        ]"""
DUKE_PARTS = """[
            (size: (10, 10, 20), origin: (-5, 2, -10), uv: (0, 0)),
            (size: (10, 8, 8), origin: (-5, 3, 10), uv: (0, 30), anim: Head, pivot: (0, 7, 10)),
            (size: (12, 1, 6), origin: (-17, 7, -2), uv: (0, 46), anim: Wing(0), pivot: (-5, 7, 1)),
            (size: (12, 1, 6), origin: (5, 7, -2), uv: (0, 46), anim: Wing(1), pivot: (5, 7, 1)),
            (size: (2, 10, 6), origin: (-1, 2, -16), uv: (36, 30)),
        ]"""
EMPRESS_PARTS = """[
            (size: (6, 12, 4), origin: (-3, 8, -2), uv: (0, 0)),
            (size: (6, 6, 6), origin: (-3, 20, -3), uv: (0, 16), anim: Head, pivot: (0, 20, 0)),
            (size: (18, 1, 14), origin: (-21, 14, -3), uv: (0, 28), anim: Wing(0), pivot: (-3, 16, 1)),
            (size: (18, 1, 14), origin: (3, 14, -3), uv: (0, 28), anim: Wing(1), pivot: (3, 16, 1)),
            (size: (8, 8, 6), origin: (-4, 0, -3), uv: (24, 0)),
        ]"""
PILLAR_PARTS = """[
            (size: (10, 36, 10), origin: (-5, 0, -5), uv: (0, 0)),
            (size: (14, 4, 14), origin: (-7, 36, -7), uv: (0, 46)),
        ]"""

PROPHET = pal("#0e2a4a", "#163a64", "#204c80", "#2c609c", "#3a74b4")
SOLAR = pal("#5a1404", "#a03008", "#e05a10", "#f89a30", "#ffe090")
VORTEX = pal("#06302a", "#0e5a4a", "#18887a", "#3ac4a8", "#a8ffe8")
NEBULA = pal("#2a0a3a", "#4a1466", "#7a2aa0", "#c45ad4", "#ffc0ff")
STARDUST = pal("#0a2a4a", "#14508a", "#2a80c4", "#6ac4f0", "#e0f8ff")
CRYSTAL = pal("#5a2a6a", "#a05aa8", "#d08ad0", "#f0c0f0", "#ffffff")
TEMPLE = pal("#3a2008", "#5a3410", "#7a4a1a", "#9a6224", "#b87a30")
TIDE = pal("#0a3a3a", "#145a5a", "#207a7a", "#3aa0a0", "#8ad8d0")
RAINBOW = [pal("#8a1a1a", "#e04a4a"), pal("#8a5a0a", "#f0b030"), pal("#2a6a1a", "#7ad44a"), pal("#14508a", "#5ab4f0"), pal("#4a1466", "#c45ad4")]


def empress_wing(c):
    band = RAINBOW[(c.i * 5 // max(c.fw, 1)) % 5]
    return g.pick(band, 0.3 + 0.6 * c.jit)


def mobs_l7():
    def boss(name, display, hp, atk, parts, scale, w, h, drops, xp, layers="[]", extra=", flies: true"):
        return f"""    (
        name: "{name}", display_name: "{display}", kind: Hostile, health: {hp}, speed: 0.1, attack: {atk},
        drops: [{drops}], xp: {xp},
        width: {w}, height: {h}, spawn: Never, scale: {scale}, boss: true{extra},
        parts: {parts},
        layers: {layers},
    ),"""

    def minion(name, display, hp, atk, parts, scale, w, h, drops, extra=", flies: true", layers="[]", spawn="spawn: Never"):
        return f"""    (
        name: "{name}", display_name: "{display}", kind: Hostile, health: {hp}, speed: 0.1, attack: {atk},
        drops: [{drops}], xp: 8,
        width: {w}, height: {h}, {spawn}, scale: {scale}{extra},
        parts: {parts},
        layers: {layers},
    ),"""

    out = ["    // Lote 7: the last bosses, the four pillars and their folk, the dungeon spirits."]
    out.append(boss("jelly_queen", "Rainha Geleia", 2400, 10, QUEEN_PARTS_L7, 4.0, 3.0, 2.6,
                    '(item: "gel", min: 30, max: 60), (item: "crystal_wings", chance: 0.4), (item: "crystal_yoyo", chance: 0.6), (item: "soul_of_light", min: 4, max: 8), (item: "jelly_saddle", chance: 0.2)', 250,
                    "[(part: 0, inflate: 0.6), (part: 1, inflate: 0.4)]", ", hops: true"))
    out.append(boss("scale_idol", "Ídolo Escamado", 5200, 14, IDOL_PARTS, 3.0, 3.2, 4.4,
                    '(item: "beetle_husk", min: 6, max: 12), (item: "turtle_shell", min: 1, max: 3), (item: "stone_fist", chance: 0.5), (item: "sun_stone_staff", chance: 0.5), (item: "emerald", min: 8, max: 14)', 250,
                    "[(part: 1, inflate: 0.5)]", ""))
    out.append(boss("tide_duke", "Duque das Marés", 6000, 16, DUKE_PARTS, 3.2, 3.0, 2.6,
                    '(item: "tide_wings", chance: 0.4), (item: "bubble_rifle", chance: 0.5), (item: "tide_boomerang", chance: 0.5), (item: "prismarine_shard", min: 8, max: 16)', 250,
                    "[(part: 1, inflate: 0.4)]"))
    out.append(boss("prism_empress", "Imperatriz Prismática", 7000, 18, EMPRESS_PARTS, 2.6, 1.6, 3.2,
                    '(item: "empress_wings", chance: 0.4), (item: "prism_saber", chance: 0.6), (item: "soul_of_light", min: 10, max: 20), (item: "pixie_dust", min: 10, max: 20)', 250,
                    "[(part: 1, inflate: 0.5)]"))
    out.append(boss("lunar_prophet", "Profeta Lunar", 4000, 12, PERSON_PARTS, 1.3, 0.8, 2.5,
                    '(item: "solar_fragment", min: 2, max: 4), (item: "vortex_fragment", min: 2, max: 4), (item: "nebula_fragment", min: 2, max: 4), (item: "stardust_fragment", min: 2, max: 4)', 250,
                    PERSON_LAYERS))
    for kind, disp, frag in [("solar", "Pilar Solar", "solar_fragment"), ("vortex", "Pilar do Vórtice", "vortex_fragment"), ("nebula", "Pilar da Nebulosa", "nebula_fragment"), ("stardust", "Pilar de Pó Estelar", "stardust_fragment")]:
        out.append(minion(f"pillar_{kind}", disp, 2000, 0, PILLAR_PARTS, 3.0, 3.6, 13.0, f'(item: "{frag}", min: 20, max: 32)', ", boss: true, flies: true", "[(part: 1, inflate: 0.5)]"))
    out.append(minion("solar_warrior", "Guerreiro Solar", 90, 12, PERSON_PARTS, 1.0, 0.6, 1.95, '(item: "solar_fragment", chance: 0.25)', "", PERSON_LAYERS))
    out.append(minion("vortex_hornet", "Vespão do Vórtice", 70, 10, HARPY_PARTS, 1.2, 0.7, 1.6, '(item: "vortex_fragment", chance: 0.25)'))
    out.append(minion("nebula_eye", "Olho da Nebulosa", 70, 10, EYE_PARTS, 0.9, 0.9, 0.9, '(item: "nebula_fragment", chance: 0.25)', layers="[(part: 0, inflate: 0.5)]"))
    out.append(minion("star_cell", "Célula Estelar", 60, 9, JELLY_PARTS, 1.2, 0.9, 0.8, '(item: "stardust_fragment", chance: 0.25)', ", hops: true", "[(part: 0, inflate: 0.5)]"))
    out.append(minion("dungeon_spirit", "Espírito da Masmorra", 60, 9, WISP_PARTS, 3.0, 0.7, 0.7, '(item: "ectoplasm", min: 1, max: 2)'))
    return out


def skins_l7():
    stone = shade(TEMPLE)
    prophet = dict(PSKIN=pal("#c49070", "#d4a080", "#e0b090", "#eac0a0", "#f2d0b4"), PHAIR=PROPHET, PTUNIC=PROPHET, PTROUSER=PROPHET, PLEATHER=pal("#c49a10", "#d4aa18", "#e4ba20", "#f4ca28"), PIRIS=(120, 200, 255))
    solar = dict(PSKIN=pal("#7a2a08", "#a03a0a", "#c44a10", "#e05a14", "#f07a20"), PHAIR=SOLAR, PTUNIC=SOLAR, PTROUSER=pal("#3a1004", "#5a1a06", "#7a2408", "#9a2e0a"), PLEATHER=pal("#f0b030", "#f8c040", "#ffd050", "#ffe070"), PIRIS=(255, 220, 60))
    person = lambda name, col: (g.ROLES["player"], g.townsfolk(name, g.PAINTERS["player"], col), g.townsfolk(name, g.OVERLAYS["player"], col))
    out = {
        "jelly_queen": (["jelly", "crown", "wing", "wing"], zoned({"jelly": jelly_painter(CRYSTAL, pal("#6a5008", "#c4941c", "#f0d060")), "crown": jelly_painter(CRYSTAL, pal("#6a5008", "#c4941c", "#f0d060")), "wing": shade(pal("#7ac4e0", "#a8dcf0", "#d0eef8", "#f0faff"))}), jelly_overlay(CRYSTAL)),
        "scale_idol": (["body", "head", "fist", "fist", "foot", "foot"], zoned({"head": eyes_on(stone, (255, 170, 40), (5, 6)), "*": bands(TEMPLE)}), None),
        "tide_duke": (["body", "head", "fin", "fin", "tail"], zoned({"head": eyes_on(shade(TIDE), (230, 230, 120)), "fin": shade(pal("#2a6a6a", "#3a8a8a", "#5ab0a8", "#8ad8d0")), "*": shade(TIDE)}), None),
        "prism_empress": (["body", "head", "wing", "wing", "skirt"], zoned({"wing": empress_wing, "head": eyes_on(shade(pal("#c49070", "#e0b090", "#f2d0b4")), (120, 60, 200), (3,)), "*": shade(CRYSTAL)}), None),
        "lunar_prophet": person("lunar_prophet", prophet),
        "pillar_solar": (["column", "cap"], bands(SOLAR), None),
        "pillar_vortex": (["column", "cap"], bands(VORTEX), None),
        "pillar_nebula": (["column", "cap"], bands(NEBULA), None),
        "pillar_stardust": (["column", "cap"], bands(STARDUST), None),
        "solar_warrior": person("solar_warrior", solar),
        "vortex_hornet": (["body", "head", "wing", "wing", "leg", "leg"], zoned({"head": eyes_on(shade(VORTEX), (200, 255, 230), (2,)), "*": shade(VORTEX)}), None),
        "nebula_eye": (["eye", "stalk", "stalk", "stalk"], e5s.eye_painter(pal("#c8a8e0", "#dcc4f0", "#f0e0ff"), pal("#4a1466", "#7a2aa0", "#a84ad4"), pal("#ffc0ff", "#ff80ff", "#ffffff")), e5s.eye_overlay(pal("#4a1466", "#7a2aa0", "#a84ad4"))),
        "star_cell": (["jelly"], jelly_painter(STARDUST), jelly_overlay(STARDUST)),
        "dungeon_spirit": (["orb"], wisp_painter(pal("#0a2a4a", "#2a6aa0", "#5ab4f0", "#b0e8ff", "#ffffff")), None),
    }
    return out


ARMOR_L7 = {
    # material: (display, palette, points helmet/chest/legs/boots, ingredient, extra)
    "spectral": ("Espectral", pal("#14304a", "#2a5a8a", "#4a8ac4", "#8ac4f0", "#e0f4ff"), (4, 8, 6, 3), "spectral_ingot", None),
    "sporite": ("de Esporita", pal("#0a1a4a", "#14307a", "#2a50b0", "#5a8ae0", "#b0d0ff"), (4, 8, 7, 3), "sporite_ingot", None),
    "beetle": ("Escamada", pal("#1a0a2a", "#3a1a4a", "#5a2a6a", "#8a4a9a", "#c48ad4"), (5, 10, 8, 4), "beetle_husk", "verdite_ingot"),
    "turtle": ("de Tartaruga", pal("#2a3a0a", "#4a5a14", "#6a8020", "#9ab03a", "#d0e07a"), (5, 9, 8, 4), "turtle_shell", "verdite_ingot"),
    "solar": ("Solar", SOLAR, (6, 11, 9, 4), "solar_fragment", "lumite_ingot"),
    "vortex": ("do Vórtice", VORTEX, (5, 10, 8, 4), "vortex_fragment", "lumite_ingot"),
    "nebula": ("da Nebulosa", NEBULA, (5, 9, 8, 4), "nebula_fragment", "lumite_ingot"),
    "stardust": ("de Pó Estelar", STARDUST, (4, 9, 7, 3), "stardust_fragment", "lumite_ingot"),
}

FRAGMENT = [
    "................",
    "........1.......",
    ".......121......",
    "......12321.....",
    ".....1234321....",
    "....123454321...",
    "...12345554321..",
    "....123454321...",
    ".....1234321....",
    "......12321.....",
    ".......121......",
    "........1.......",
    "................",
    "................",
    "................",
    "................",
]
ECTO = [
    "................",
    "................",
    "......1111......",
    ".....123321.....",
    "....12345321....",
    "....12355321....",
    "....12344321....",
    ".....123321.....",
    ".....12.121.....",
    "....12...121....",
    "....1.....1.....",
    "................",
    "................",
    "................",
    "................",
    "................",
]
HUSK = [
    "................",
    "................",
    ".....111111.....",
    "....12333321....",
    "...1234554321...",
    "...1234554321...",
    "...1233443321...",
    "...1223333221...",
    "...1222112221...",
    "....12211221....",
    ".....111111.....",
    "................",
    "................",
    "................",
    "................",
    "................",
]
CORE = [
    "................",
    "................",
    "....11111111....",
    "....1aaaaaa1....",
    "....1a2332a1....",
    "....1a3553a1....",
    "....1a3553a1....",
    "....1a2332a1....",
    "....1aaaaaa1....",
    "....11111111....",
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
]
BAIT = [
    "................",
    "................",
    "..........11....",
    ".........1221...",
    "........12331...",
    ".......12341....",
    "......12341.....",
    ".....12341......",
    "....12341.......",
    "...12331........",
    "..1221..........",
    "..11............",
    "................",
    "................",
    "................",
    "................",
]


def items_l7():
    stack1 = "max_stack: Some(1)"
    for m, (_, p, *_r) in ARMOR_L7.items():
        gi.ARMOR_MATERIALS[m] = p
        gi.MATERIALS[m] = p
    gi.INGOT_COL["spectral"] = ARMOR_L7["spectral"][1]
    gi.INGOT_COL["sporite"] = ARMOR_L7["sporite"][1]
    gi.MATERIALS["crystal"] = CRYSTAL
    gi.MATERIALS["tide"] = TIDE
    gi.WAND_GEMS["sun"] = SOLAR
    out = [
        ("gelatin_crystal", "Cristal Gelatinoso", icon(SHARD, CRYSTAL), ""),
        ("idol_core", "Núcleo do Ídolo", icon2(CORE, pal("#5a1404", "#a03008", "#e05a10", "#f89a30", "#ffe090"), TEMPLE), ""),
        ("tide_bait", "Isca das Marés", icon(BAIT, pal("#3a0a1a", "#8a2a4a", "#c45a7a", "#f09ab0", "#ffe0e8")), ""),
        ("prism_lamp", "Lâmpada Prismática", icon2(LAMP, CRYSTAL, pal("#6a5008", "#c4941c", "#f0d060")), ""),
        ("lunar_tablet", "Tábua Lunar", icon(TABLET, PROPHET), ""),
        ("ectoplasm", "Ectoplasma", icon(ECTO, pal("#0a2a4a", "#2a6aa0", "#5ab4f0", "#b0e8ff", "#ffffff")), ""),
        ("spectral_ingot", "Barra Espectral", gi.ingot("spectral"), ""),
        ("sporite_ingot", "Barra de Esporita", gi.ingot("sporite"), ""),
        ("beetle_husk", "Carapaça de Besouro", icon(HUSK, ARMOR_L7["beetle"][1]), ""),
        ("turtle_shell", "Casco de Tartaruga", icon(HUSK, ARMOR_L7["turtle"][1]), ""),
        ("solar_fragment", "Fragmento Solar", icon(FRAGMENT, SOLAR), ""),
        ("vortex_fragment", "Fragmento do Vórtice", icon(FRAGMENT, VORTEX), ""),
        ("nebula_fragment", "Fragmento da Nebulosa", icon(FRAGMENT, NEBULA), ""),
        ("stardust_fragment", "Fragmento de Pó Estelar", icon(FRAGMENT, STARDUST), ""),
        ("crystal_wings", "Asas de Cristal", icon(WING, CRYSTAL), stack1),
        ("tide_wings", "Asas das Marés", icon(WING, TIDE), stack1),
        ("empress_wings", "Asas da Imperatriz", icon(WING, pal("#8a1a5a", "#e04a9a", "#f0b030", "#7ad44a", "#5ab4f0")), stack1),
        ("crystal_yoyo", "Ioiô de Cristal", icon2(YOYO, CRYSTAL, STRING), stack1),
        ("stone_fist", "Punho de Pedra", icon2(FLAIL, TEMPLE, STEEL), stack1),
        ("sun_stone_staff", "Cajado da Pedra do Sol", gi.staff("sun", "golden"), stack1),
        ("tide_boomerang", "Bumerangue das Marés", icon2(BOOMERANG, TIDE, STEEL), stack1),
        ("bubble_rifle", "Rifle de Bolhas", icon2(RAPID, TIDE, STEEL), stack1),
        ("prism_saber", "Sabre Prismático", gi.tool("crystal", "sword"), "max_stack: Some(1), tool: Some((kind: Sword, tier: 5, speed: 9.0, damage: 12.0, durability: 3000))"),
        ("solar_eruption", "Erupção Solar", icon2(FLAIL, SOLAR, SOLAR), stack1),
        ("vortex_beater", "Batedor do Vórtice", icon2(RAPID, VORTEX, STEEL), stack1),
        ("nebula_blade", "Lâmina da Nebulosa", gi.tool("nebula", "sword"), "max_stack: Some(1), tool: Some((kind: Sword, tier: 5, speed: 9.0, damage: 13.0, durability: 3000))"),
        ("stardust_yoyo", "Ioiô de Pó Estelar", icon2(YOYO, STARDUST, STRING), stack1),
    ]
    names = {"helmet": "Capacete", "chestplate": "Peitoral", "leggings": "Calças", "boots": "Botas"}
    for m, (disp, _, pts, _i, _e) in ARMOR_L7.items():
        for slot, k in enumerate(gi.ARMOR_KINDS):
            out.append((f"{m}_{k}", f"{names[k]} {disp}", gi.armor(m, k), f"armor: Some((slot: {slot}, points: {pts[slot]}, durability: {400 + 40 * pts[slot]}))"))
    return out


BLOCKS_L7 = [
    '(name: "scale_brick", display_name: Some("Tijolo Escamado"), hardness: 30.0, tool: Some(Pickaxe), tier: 4)',
    '(name: "scale_altar", display_name: Some("Altar Escamado"), light: 6, hardness: 60.0, tool: Some(Pickaxe), tier: 5, textures: (top: Some("scale_altar_top"), side: Some("scale_altar_side"), bottom: Some("scale_brick")), drops: Some([]))',
    '(name: "scale_lamp", display_name: Some("Lâmpada Escamada"), light: 14, hardness: 1.0)',
]


def blocks_l7():
    def brick(x, y):
        row = y // 4
        off = 4 if row % 2 else 0
        mortar = y % 4 == 3 or (x + off) % 8 == 7
        return 0.15 if mortar else 0.5 + 0.3 * (((x + off) // 8 + row) % 2)

    def altar_top(x, y):
        dx, dy = abs(x - 7.5), abs(y - 7.5)
        return 0.95 if dx + dy < 4 else (0.2 if min(x, y, 15 - x, 15 - y) == 0 else 0.5)

    def lamp(x, y):
        dx, dy = abs(x - 7.5), abs(y - 7.5)
        return 0.15 if min(x, y, 15 - x, 15 - y) < 2 else 0.95 - 0.06 * (dx + dy)

    return {
        "scale_brick": block_noise(70, brick, TEMPLE),
        "scale_altar_top": block_noise(71, altar_top, pal("#3a2008", "#7a4a1a", "#b87a30", "#f89a30", "#ffe090")),
        "scale_altar_side": block_noise(72, lambda x, y: 0.85 if y < 3 else brick(x, y), TEMPLE),
        "scale_lamp": block_noise(73, lamp, pal("#3a2008", "#7a4a1a", "#e05a10", "#f8c040", "#fff0b0")),
    }


SHAPED_L7 = []
for _m, (_d, _p, _pts, _ing, _extra) in ARMOR_L7.items():
    SHAPED_L7 += armor_recipes(_m, _ing, _extra)
SHAPED_L7 += [
    (["CCC", "CEC", "CCC"], {"C": "pixie_dust", "E": "gel"}, "gelatin_crystal", 1),
    (["BBB", "BSB", "BBB"], {"B": "scale_brick", "S": "sun_flail"}, "idol_core", 1),
    ([" W ", "WSW", " W "], {"W": "glow_mushroom", "S": "spider_eye"}, "tide_bait", 1),
    (["LLL", "LGL", "LLL"], {"L": "soul_of_light", "G": "glowstone"}, "prism_lamp", 1),
    (["BBB", "BTB", "BBB"], {"B": "beetle_husk", "T": "ender_eye"}, "lunar_tablet", 1),
    (["FF", "FF"], {"F": "solar_fragment"}, "solar_eruption", 1),
    (["FF", "FF"], {"F": "vortex_fragment"}, "vortex_beater", 1),
    (["FF", "FF"], {"F": "nebula_fragment"}, "nebula_blade", 1),
    (["FF", "FF"], {"F": "stardust_fragment"}, "stardust_yoyo", 1),
]
SHAPELESS_L7 = [
    (["verdite_ingot", "ectoplasm"], "spectral_ingot", 1),
    (["verdite_ingot", "glow_mushroom", "glow_mushroom"], "sporite_ingot", 1),
    (["solar_fragment", "vortex_fragment", "nebula_fragment", "stardust_fragment"], "celestial_sigil", 1),
    (["scale_brick", "torch"], "scale_lamp", 1),
]


def structures_l7():
    """The Scale Temple, buried under the jungle: a big brick hall with pillars and lamps, the
    altar in the middle, chests in the corners and a ladder shaft up to the surface."""
    W, H, UP = 27, 11, 14
    layers = []
    for y in range(H + UP):
        rows = []
        for z in range(W):
            row = ""
            for x in range(W):
                if y >= H:
                    # Above the hall only the shaft: a ladder on a brick spine.
                    ch = "L" if (x, z) == (4, 4) else ("B" if (x, z) == (5, 4) else " ")
                    row += ch
                    continue
                edge = x in (0, W - 1) or z in (0, W - 1)
                pillar = x % 6 == 3 and z % 6 == 3 and not (12 <= x <= 14 and 12 <= z <= 14)
                if y == 0 or y == H - 1 or edge or pillar:
                    ch = "B"
                    if y == 5 and pillar:
                        ch = "l"
                else:
                    ch = "."
                    if y == 1 and (x, z) == (13, 13):
                        ch = "A"
                    if y == 1 and (x, z) in ((2, 2), (24, 2), (2, 24), (24, 24)):
                        ch = "K"
                if (x, z) == (4, 4) and y > 0:
                    ch = "L"
                if (x, z) == (5, 4) and y > 0:
                    ch = "B"
                row += ch
            rows.append(row)
        layers.append(rows)
    return layers, H + UP - 3


# ==============================================================================================
# Lote 8: fishing (power, baits, rods, crates, the angler's quests, lava fishing, potions) and
# the invasions (Harvest Moon, Winter Moon, sea raiders, star visitors, Crystal Defense)
# ==============================================================================================

CRATE = [
    "................",
    "................",
    "..111111111111..",
    "..1aaaaaaaaaa1..",
    "..1a23333332a1..",
    "..1a32222223a1..",
    "..1a3b2222b3a1..",
    "..1aaaaaaaaaa1..",
    "..1a3b2222b3a1..",
    "..1a32222223a1..",
    "..1a23333332a1..",
    "..1aaaaaaaaaa1..",
    "..111111111111..",
    "................",
    "................",
    "................",
]
FISH_ICON = [
    "................",
    "................",
    "................",
    "........111.....",
    "......1133311...",
    "11...123444321..",
    "121.12344445a31.",
    ".12123444444441.",
    ".12123444444441.",
    "121.12344444331.",
    "11...123333321..",
    "......1122211...",
    "........111.....",
    "................",
    "................",
    "................",
]
ROD = [
    "...............1",
    "..............12",
    ".............12.",
    "............12.a",
    "...........12..a",
    "..........12...a",
    ".........12....a",
    "........12.....a",
    ".......12......a",
    "......12.......a",
    ".....b3.......aa",
    "....b3........a.",
    "...33.........b.",
    "..33............",
    ".33.............",
    "33..............",
]
POTION = [
    "................",
    "......aaaa......",
    "......abba......",
    ".......bb.......",
    "......1111......",
    ".....1....1.....",
    "....1......1....",
    "...1........1...",
    "...1.222222.1...",
    "...1.233332.1...",
    "...1.234432.1...",
    "...1.233332.1...",
    "....1.2222.1....",
    ".....111111.....",
    "................",
    "................",
]
MEDAL = [
    "................",
    "......a..a......",
    ".....a....a.....",
    "......a..a......",
    ".......aa.......",
    ".....111111.....",
    "....12222221....",
    "...1233333321...",
    "...1234554321...",
    "...1234554321...",
    "...1233333321...",
    "....12222221....",
    ".....111111.....",
    "................",
    "................",
    "................",
]
MAP = [
    "................",
    "................",
    "..11111111111...",
    "..14444444441...",
    "..1444a44a441...",
    "..14444aa4441...",
    "..1444a44a441...",
    "..14444444441...",
    "..144b4444441...",
    "..1444b444441...",
    "..14444b44441...",
    "..1444444b441...",
    "..11111111111...",
    "................",
    "................",
    "................",
]

CRATES_L8 = [
    ("wooden_crate", "Caixa de Madeira", WOOD, pal("#2a1a0a", "#5a3a1a")),
    ("iron_crate", "Caixa de Ferro", STEEL, pal("#2a2e36", "#5a606c")),
    ("golden_crate", "Caixa Dourada", pal("#6a4a08", "#a07c16", "#d4aa18", "#f4ca28", "#fff0a0"), pal("#3a2a04", "#8a6a10")),
    ("jungle_crate", "Caixa da Selva", pal("#0e2a10", "#1e4a1c", "#3a7a2a", "#6aaa4a", "#b4e08a"), pal("#2a1a0a", "#5a3a1a")),
    ("frozen_crate", "Caixa Congelada", pal("#2a4a6a", "#4a7aa0", "#7aaed4", "#b4daf4", "#eef8ff"), pal("#1a2a3a", "#3a5a7a")),
    ("ocean_crate", "Caixa do Oceano", pal("#0a2a4a", "#14508a", "#2080c4", "#5ab4f0", "#d0f0ff"), pal("#5a3a1a", "#c4a060")),
    ("sandstone_crate", "Caixa de Arenito", pal("#8a6a3a", "#b08a50", "#d4b070", "#ead09a", "#f8ecc8"), pal("#5a3a1a", "#7a5a2e")),
    ("ruin_crate", "Caixa Corrompida", pal("#1a1028", "#3a2458", "#5a3a84", "#8a64b8", "#c8a8f0"), pal("#0a0a0e", "#2a1a3a")),
    ("prism_crate", "Caixa Prismática", CRYSTAL, pal("#6a5008", "#f0d060")),
    ("obsidian_crate", "Caixa de Obsidiana", pal("#0a0814", "#1a1428", "#2a2040", "#4a3a6a", "#8a6aa0"), pal("#5a1404", "#e05a10")),
]

# item, display, palette (the angler's quest fish first, then lava fish)
FISH_L8 = [
    ("mud_king", "Rei do Lodo", pal("#2a2010", "#4a3a1a", "#6a5a2a", "#8a7a44", "#c4b07a")),
    ("frost_minnow", "Piaba Gelada", pal("#2a4a6a", "#4a7aa0", "#7aaed4", "#b4daf4", "#eef8ff")),
    ("sand_eel", "Enguia da Areia", pal("#8a6a3a", "#b08a50", "#d4b070", "#ead09a", "#f8ecc8")),
    ("sky_trout", "Truta do Céu", pal("#6a7aa0", "#8a9ac4", "#b0c0e4", "#d8e4f8", "#ffffff")),
    ("lantern_fish", "Peixe-Lanterna", pal("#0a1a2a", "#1a3a5a", "#2a5a8a", "#f0d060", "#fff8c0")),
    ("vile_carp", "Carpa Vil", pal("#1a1028", "#3a2458", "#5a3a84", "#8a64b8", "#c8a8f0")),
    ("prism_guppy", "Barrigudinho Prismático", CRYSTAL),
    ("moon_sunfish", "Peixe-Lua", pal("#5a5a6a", "#8a8a9a", "#b4b4c4", "#dcdce8", "#ffffff")),
    ("lava_koi", "Carpa de Lava", SOLAR),
    ("obsidian_fish", "Peixe de Obsidiana", pal("#0a0814", "#1a1428", "#2a2040", "#4a3a6a", "#8a6aa0")),
]

PERSON_L8 = {
    # name: (display, hp, attack, scale, colours, boss)
    "scarecrow": ("Espantalho", 140, 10, 1.0, dict(PSKIN=pal("#8a6a2a", "#a8823a", "#c49a4a", "#d8b060", "#ecc87a"), PHAIR=pal("#c4a020", "#d4b02a", "#e4c034", "#f0d03e", "#f8e048"), PTUNIC=pal("#5a2a14", "#6c341a", "#7e3e20", "#904826", "#a2522c"), PTROUSER=pal("#3a4a6a", "#44547a", "#4e5e8a", "#58689a"), PLEATHER=pal("#5a3a1a", "#6a4420", "#7a4e26", "#8a582c"), PIRIS=(20, 10, 0)), False),
    "headless_rider": ("Cavaleiro Sem Cabeça", 400, 16, 1.3, dict(PSKIN=pal("#1a1a1a", "#242424", "#2e2e2e", "#383838", "#424242"), PHAIR=pal("#e05a10", "#f07a20", "#f89a30", "#ffb040", "#ffd060"), PTUNIC=pal("#1a1a24", "#22222e", "#2a2a38", "#323242", "#3a3a4c"), PTROUSER=pal("#1a1a24", "#22222e", "#2a2a38", "#323242"), PLEATHER=pal("#5a1a1a", "#6e2222", "#822a2a", "#963232"), PIRIS=(255, 140, 20)), False),
    "pumpkin_king": ("Rei Abóbora", 3000, 18, 2.4, dict(PSKIN=pal("#a04a08", "#c4600e", "#e07a18", "#f09a30", "#f8b84a"), PHAIR=pal("#2a5a1a", "#3a7a22", "#4a9a2a", "#5aba34", "#7ad44a"), PTUNIC=pal("#1a1014", "#24161c", "#2e1c24", "#38222c", "#42283a"), PTROUSER=pal("#1a1014", "#24161c", "#2e1c24", "#38222c"), PLEATHER=pal("#e05a10", "#f07a20", "#f89a30", "#ffb040"), PIRIS=(255, 200, 40)), True),
    "evil_elf": ("Elfo Malvado", 110, 9, 0.7, dict(PSKIN=pal("#c49070", "#d4a080", "#e0b090", "#eac0a0", "#f2d0b4"), PHAIR=pal("#8a1a1a", "#a02020", "#b82828", "#cc3030", "#e03838"), PTUNIC=pal("#1a5a1a", "#226a22", "#2a7a2a", "#328a32", "#3a9a3a"), PTROUSER=pal("#8a1a1a", "#a02020", "#b82828", "#cc3030"), PLEATHER=pal("#e4c040", "#ecca50", "#f4d460", "#f8de70"), PIRIS=(220, 30, 30)), False),
    "snowman_gunner": ("Boneco Atirador", 160, 10, 1.1, dict(PSKIN=pal("#c8d2dc", "#dce4ec", "#eaf0f6", "#f6f9fc", "#ffffff"), PHAIR=pal("#141414", "#1c1c1c", "#242424", "#2c2c2c", "#343434"), PTUNIC=pal("#c8d2dc", "#dce4ec", "#eaf0f6", "#f6f9fc", "#ffffff"), PTROUSER=pal("#c8d2dc", "#dce4ec", "#eaf0f6", "#f6f9fc"), PLEATHER=pal("#a03030", "#b83a3a", "#d04444", "#e84e4e"), PIRIS=(20, 20, 20)), False),
    "pirate_deckhand": ("Marujo Saqueador", 120, 10, 1.0, dict(PSKIN=pal("#a8694a", "#b97a58", "#c78a65", "#d29a74", "#dca984"), PHAIR=pal("#8a1a1a", "#a02020", "#b82828", "#cc3030", "#e03838"), PTUNIC=pal("#c8c0b0", "#d4ccbc", "#e0d8c8", "#ece4d4", "#f8f0e0"), PTROUSER=pal("#1a2e5a", "#22386c", "#2a427e", "#344e90"), PLEATHER=pal("#5a3a1a", "#6a4420", "#7a4e26", "#8a582c"), PIRIS=(60, 40, 30)), False),
    "pirate_gunner": ("Atirador Saqueador", 120, 10, 1.0, dict(PSKIN=pal("#6a3e2a", "#7a4a32", "#8a563a", "#9a6242", "#aa6e4a"), PHAIR=pal("#141414", "#1c1c1c", "#242424", "#2c2c2c", "#343434"), PTUNIC=pal("#1a2e5a", "#22386c", "#2a427e", "#344e90", "#3e5aa2"), PTROUSER=pal("#3a2a1a", "#463220", "#523a26", "#5e422c"), PLEATHER=pal("#c49a10", "#d4aa18", "#e4ba20", "#f4ca28"), PIRIS=(60, 40, 30)), False),
    "pirate_captain": ("Capitão Saqueador", 2200, 16, 1.6, dict(PSKIN=pal("#a8694a", "#b97a58", "#c78a65", "#d29a74", "#dca984"), PHAIR=pal("#141414", "#1c1c1c", "#242424", "#2c2c2c", "#343434"), PTUNIC=pal("#8a1a1a", "#a02020", "#b82828", "#cc3030", "#e03838"), PTROUSER=pal("#141414", "#1c1c1c", "#242424", "#2c2c2c"), PLEATHER=pal("#e4c040", "#ecca50", "#f4d460", "#f8de70"), PIRIS=(40, 30, 20)), True),
    "martian_soldier": ("Soldado Estelar", 180, 12, 1.0, dict(PSKIN=pal("#3a6a2a", "#4a8a34", "#5aa03e", "#6ab84a", "#7ad05a"), PHAIR=pal("#5a606c", "#6e7682", "#8a92a0", "#a4acb8", "#c0c8d4"), PTUNIC=pal("#5a606c", "#6e7682", "#8a92a0", "#a4acb8", "#c0c8d4"), PTROUSER=pal("#3a3e48", "#4c515c", "#646a78", "#7e8492"), PLEATHER=pal("#2a80c4", "#3a90d4", "#4aa0e4", "#5ab0f4"), PIRIS=(20, 20, 20)), False),
    "martian_officer": ("Oficial Estelar", 260, 14, 1.15, dict(PSKIN=pal("#3a6a2a", "#4a8a34", "#5aa03e", "#6ab84a", "#7ad05a"), PHAIR=pal("#c41420", "#e02028", "#f03a3a", "#ff5a4a", "#ff7a6a"), PTUNIC=pal("#3a3e48", "#4c515c", "#646a78", "#7e8492", "#9aa0ae"), PTROUSER=pal("#2a2c32", "#383b42", "#484c54", "#5a5e68"), PLEATHER=pal("#c41420", "#e02028", "#f03a3a", "#ff5a4a"), PIRIS=(20, 20, 20)), False),
    "ether_goblin": ("Goblin Etéreo", 150, 11, 0.9, dict(PSKIN=pal("#4a2a6a", "#5a347a", "#6a3e8a", "#7a489a", "#8a52aa"), PHAIR=pal("#141414", "#1c1c1c", "#242424", "#2c2c2c", "#343434"), PTUNIC=pal("#2a1436", "#341a44", "#3e2052", "#482660", "#522c6e"), PTROUSER=pal("#1e1a26", "#262030", "#2e263a", "#362c44"), PLEATHER=pal("#c45ad4", "#d46ae0", "#e47aec", "#f08af8"), PIRIS=(255, 120, 255)), False),
    "dark_mage": ("Mago Sombrio", 2400, 14, 1.5, dict(PSKIN=pal("#5a5a6a", "#6a6a7a", "#7a7a8a", "#8a8a9a", "#9a9aaa"), PHAIR=pal("#2a0a3a", "#3a1a4a", "#4a2a5a", "#5a3a6a", "#6a4a7a"), PTUNIC=pal("#2a0a3a", "#4a1466", "#7a2aa0", "#c45ad4", "#ffc0ff"), PTROUSER=pal("#1a0a2a", "#2a1a3a", "#3a2a4a", "#4a3a5a"), PLEATHER=pal("#c49a10", "#d4aa18", "#e4ba20", "#f4ca28"), PIRIS=(255, 120, 255)), True),
}

SAUCER_PARTS = """[
            (size: (14, 3, 14), origin: (-7, 0, -7), uv: (0, 0)),
            (size: (8, 5, 8), origin: (-4, 3, -4), uv: (0, 17)),
        ]"""
CRYSTAL_PARTS = """[
            (size: (6, 12, 6), origin: (-3, 0, -3), uv: (0, 0)),
        ]"""


def mobs_l8():
    out = ["    // Lote 8: the invasions' folk."]
    for name, (disp, hp, atk, scale, _, boss) in PERSON_L8.items():
        flag = ", boss: true" if boss else ""
        out.append(f"""    (
        name: "{name}", display_name: "{disp}", kind: Hostile, health: {hp}, speed: 2.4, attack: {atk},
        drops: [], xp: {min(250, hp // 10)},
        width: {round(0.6 * scale, 2)}, height: {round(1.95 * scale, 2)}, spawn: Never, scale: {scale}{flag},
        parts: {PERSON_PARTS},
        layers: {PERSON_LAYERS},
    ),""")
    flyers = [
        ("ice_queen", "Rainha do Gelo", 3200, 16, EMPRESS_PARTS, 2.4, 1.6, 3.2, ", boss: true", "[(part: 1, inflate: 0.5)]"),
        ("saucer", "Disco Voador", 3600, 16, SAUCER_PARTS, 3.5, 3.6, 2.0, ", boss: true", "[]"),
        ("ether_wyvern", "Serpe Etérea", 140, 11, HARPY_PARTS, 1.3, 0.8, 1.6, "", "[(part: 1, inflate: 0.5)]"),
        ("martian_probe", "Sonda Estelar", 60, 0, EYE_PARTS, 0.6, 0.6, 0.6, "", "[(part: 0, inflate: 0.5)]"),
        ("ether_crystal_mob", "Cristal Etéreo", 600, 0, CRYSTAL_PARTS, 1.8, 0.8, 1.6, "", "[]"),
    ]
    for name, disp, hp, atk, parts, scale, w, h, flag, layers in flyers:
        kind = "Passive" if name == "ether_crystal_mob" else "Hostile"
        out.append(f"""    (
        name: "{name}", display_name: "{disp}", kind: {kind}, health: {hp}, speed: 0.1, attack: {atk},
        drops: [], xp: {min(250, hp // 10)},
        width: {w}, height: {h}, spawn: Never, scale: {scale}, flies: true{flag},
        parts: {parts},
        layers: {layers},
    ),""")
    return out


def skins_l8():
    person = lambda name, col: (g.ROLES["player"], g.townsfolk(name, g.PAINTERS["player"], col), g.townsfolk(name, g.OVERLAYS["player"], col))
    out = {name: person(name, v[4]) for name, v in PERSON_L8.items()}
    ICE = pal("#2a4a6a", "#4a7aa0", "#7aaed4", "#b4daf4", "#eef8ff")
    out["ice_queen"] = (["body", "head", "wing", "wing", "skirt"], zoned({"head": eyes_on(shade(ICE), (40, 80, 160), (3,)), "*": shade(ICE)}), None)
    METAL = pal("#3a3e48", "#4c515c", "#646a78", "#7e8492", "#9aa0ae")
    out["saucer"] = (["disc", "dome"], zoned({"disc": bands(METAL, 3), "dome": shade(pal("#2a6a4a", "#4aa07a", "#7ad4a8", "#b0f0d0", "#e8fff4"))}), None)
    WYV = pal("#2a1436", "#4a2a6a", "#7a4aa0", "#a87ad0", "#e0c8ff")
    out["ether_wyvern"] = (["body", "head", "wing", "wing", "leg", "leg"], zoned({"head": eyes_on(shade(WYV), (255, 140, 255), (2,)), "*": shade(WYV)}), None)
    out["martian_probe"] = (["eye", "stalk", "stalk", "stalk"], e5s.eye_painter(pal("#a8acb4", "#bcc0c8", "#d0d4dc"), pal("#2a2c32", "#383b42", "#484c54"), pal("#2a80c4", "#5ab4f0", "#c0f0ff"), metal=pal("#383b42", "#5a5e68", "#767b86")), e5s.eye_overlay(pal("#2a2c32", "#383b42", "#484c54"), metal=pal("#484c54", "#5a5e68", "#767b86")))
    out["ether_crystal_mob"] = (["crystal"], wisp_painter(pal("#2a0a3a", "#7a2aa0", "#c45ad4", "#ffc0ff", "#ffffff")), None)
    return out


def items_l8():
    stack1 = "max_stack: Some(1)"
    gi.MATERIALS["pumpkin"] = pal("#5a2a04", "#a04a08", "#e07a18", "#f8b84a", "#fff0a0")
    gi.MATERIALS["cutlass"] = pal("#5a606c", "#8a92a0", "#c0c8d4", "#eef2f8", "#ffffff")
    gi.WAND_GEMS["blizzard"] = pal("#2a4a6a", "#4a7aa0", "#7aaed4", "#b4daf4", "#eef8ff")
    out = [(n, d, icon2(CRATE, p, edge), "max_stack: Some(16)") for n, d, p, edge in CRATES_L8]
    out += [(n, d, icon2(FISH_ICON, p, pal("#141418")), "") for n, d, p in FISH_L8]
    out += [
        ("apprentice_bait", "Isca de Aprendiz", icon(BAIT, pal("#3a2a1a", "#6a4a2a", "#9a6a3a", "#c49a5a", "#e8c890")), ""),
        ("journeyman_bait", "Isca de Viajante", icon(BAIT, pal("#3a0a1a", "#8a2a4a", "#c45a7a", "#f09ab0", "#ffe0e8")), ""),
        ("master_bait", "Isca de Mestre", icon(BAIT, pal("#0a2a4a", "#2a6aa0", "#5ab4f0", "#b0e8ff", "#ffffff")), ""),
        ("reinforced_rod", "Vara Reforçada", icon2(ROD, STEEL, pal("#c8c0b0", "#5a3a1a", "#3a2410")), stack1 + ", tool: Some((kind: Hoe, tier: 0, speed: 1.0, damage: 1.0, durability: 400))"),
        ("magma_rod", "Vara de Magma", icon2(ROD, SOLAR, pal("#c8c0b0", "#5a3a1a", "#3a2410")), stack1 + ", tool: Some((kind: Hoe, tier: 0, speed: 1.0, damage: 1.0, durability: 600))"),
        ("golden_rod", "Vara Dourada", icon2(ROD, pal("#6a4a08", "#a07c16", "#d4aa18", "#f4ca28", "#fff0a0"), pal("#c8c0b0", "#5a3a1a", "#3a2410")), stack1 + ", tool: Some((kind: Hoe, tier: 0, speed: 1.0, damage: 1.0, durability: 1200))"),
        ("fishing_potion", "Poção de Pesca", icon2(POTION, pal("#c8d2dc", "#2080c4", "#5ab4f0", "#d0f0ff"), pal("#5a3a1a", "#c4a060")), "max_stack: Some(16)"),
        ("crate_potion", "Poção de Caixas", icon2(POTION, pal("#c8d2dc", "#a07c16", "#d4aa18", "#fff0a0"), pal("#5a3a1a", "#c4a060")), "max_stack: Some(16)"),
        ("sonar_potion", "Poção de Sonar", icon2(POTION, pal("#c8d2dc", "#18887a", "#3ac4a8", "#a8ffe8"), pal("#5a3a1a", "#c4a060")), "max_stack: Some(16)"),
        ("pumpkin_medallion", "Medalhão de Abóbora", icon2(MEDAL, pal("#5a2a04", "#a04a08", "#e07a18", "#f8b84a", "#fff0a0"), pal("#c49a10", "#f4ca28")), ""),
        ("frost_medallion", "Medalhão de Gelo", icon2(MEDAL, pal("#2a4a6a", "#4a7aa0", "#7aaed4", "#b4daf4", "#eef8ff"), pal("#c8d2dc", "#ffffff")), ""),
        ("pirate_map", "Mapa do Tesouro", icon2(MAP, pal("#6a5a3a", "#a89470", "#d8c8a0", "#ece0c0", "#f8f0dc"), pal("#a03030", "#3a2a1a")), ""),
        ("ether_crystal", "Cristal Etéreo", icon(SHARD, pal("#2a0a3a", "#7a2aa0", "#c45ad4", "#ffc0ff", "#ffffff")), ""),
        ("horseman_blade", "Lâmina do Cavaleiro", gi.tool("pumpkin", "sword"), "max_stack: Some(1), tool: Some((kind: Sword, tier: 4, speed: 8.0, damage: 10.0, durability: 2000))"),
        ("cutlass", "Alfanje", gi.tool("cutlass", "sword"), "max_stack: Some(1), tool: Some((kind: Sword, tier: 4, speed: 10.0, damage: 9.0, durability: 1800))"),
        ("blizzard_staff", "Cajado da Nevasca", gi.staff("blizzard", "silver"), stack1),
        ("laser_rifle", "Rifle de Laser", icon2(RAPID, pal("#2a6a4a", "#4aa07a", "#7ad4a8", "#b0f0d0", "#e8fff4"), STEEL), stack1),
        ("flame_sentry_rod", "Cajado da Sentinela de Chamas", icon2(SENTRY, SOLAR, WOOD), stack1),
    ]
    return out


SHAPED_L8 = [
    (["  I", " IS", "R S"], {"I": "iron_ingot", "S": "string", "R": "fishing_rod"}, "reinforced_rod", 1),
    (["  M", " MS", "R S"], {"M": "magmarite_ingot", "S": "string", "R": "fishing_rod"}, "magma_rod", 1),
    ([" E ", "EPE", " S "], {"E": "ectoplasm", "P": "pumpkin", "S": "soul_of_night"}, "pumpkin_medallion", 1),
    ([" E ", "ESE", " R "], {"E": "ectoplasm", "S": "snowball", "R": "radiant_ingot"}, "frost_medallion", 1),
    (["PPP", "PCP", "PGP"], {"P": "paper", "C": "compass", "G": "gold_ingot"}, "pirate_map", 1),
    ([" A ", "ALA", " A "], {"A": "amethyst", "L": "soul_of_light"}, "ether_crystal", 1),
]
SHAPELESS_L8 = [
    (["wheat_seeds", "rotten_flesh"], "apprentice_bait", 3),
    (["spider_eye", "glow_berries"], "journeyman_bait", 3),
    (["glow_ink_sac", "gold_nugget"], "master_bait", 3),
    (["glass_bottle", "cod", "glowstone_dust"], "fishing_potion", 1),
    (["glass_bottle", "gold_nugget", "salmon"], "crate_potion", 1),
    (["glass_bottle", "pufferfish", "ink_sac"], "sonar_potion", 1),
]



# ==============================================================================================
# Lote 9: the crafting stations (Infernal Forge, Arcane Anvil, Titanic Forge, Alchemy Table),
# the utility potions, the magic mirror and conch, the PDA, the piggy bank and safe, the combo
# accessories, the sprayer, javelins, the Shimmer Lake with its permanent upgrades, the oasis
# and the graveyard
# ==============================================================================================

MIRROR = [
    "................",
    ".....11111......",
    "....1aaaaa1.....",
    "...1abbbbaa1....",
    "..1abccbbbaa1...",
    "..1abcbbbbba1...",
    "..1abbbbbbba1...",
    "..1abbbbbbba1...",
    "...1aabbbba1....",
    "....1aaaaa1.....",
    ".....11111......",
    "......323.......",
    "......323.......",
    "......424.......",
    "......444.......",
    "................",
]
CONCH = [
    "................",
    "................",
    "........11......",
    "......112211....",
    ".....12233221...",
    "....1233443321..",
    "...123445543321.",
    "...12345aa54321.",
    "...1234aaaa4321.",
    "....123a44a321..",
    ".....1223332211.",
    "......11222111..",
    "........1111....",
    "................",
    "................",
    "................",
]
PDA = [
    "................",
    "....11111111....",
    "...1222222221...",
    "...12aaaaaa21...",
    "...12abbbba21...",
    "...12abccba21...",
    "...12abbbba21...",
    "...12aaaaaa21...",
    "...1222222221...",
    "...1232323221...",
    "...1223232321...",
    "...1232323221...",
    "...1222222221...",
    "....11111111....",
    "................",
    "................",
]
BOOTS = [
    "................",
    "................",
    "....1111........",
    "....1221........",
    "....1231........",
    "....1231........",
    "....1231........",
    "....1231........",
    "...12331........",
    "..1a2331........",
    "..1a23332111....",
    "..1aa2333333211.",
    "..1bbbbbbbbbb1..",
    "...1111111111...",
    "................",
    "................",
]
BALLOONS = [
    "................",
    "..111.....111...",
    ".1aab1...1ccd1..",
    ".1abb1.111cdd1..",
    ".1bbb11222dd1...",
    "..111123321.....",
    "....1233321.....",
    "....12333321....",
    ".....1222211....",
    "......1111......",
    ".......5.5......",
    "........5.......",
    "........5.......",
    ".......5........",
    "........5.......",
    "................",
]
SPRAYER = [
    "................",
    "................",
    "................",
    "..........aa....",
    ".........abba...",
    "..11111111aa....",
    ".1222222221.....",
    "13333333321.....",
    ".1222222221aaa..",
    "..1111111aabba..",
    ".....12221aa....",
    ".....12221......",
    ".....1221.......",
    "....1221........",
    "....111.........",
    "................",
]
JAVELIN = [
    "...............1",
    "..............12",
    ".............121",
    "............a21.",
    "...........a1...",
    "..........a.....",
    ".........a......",
    "........a.......",
    ".......a........",
    "......b.........",
    ".....b..........",
    "....a...........",
    "...a............",
    "..a.............",
    ".a..............",
    "................",
]
PIGGY = pal("#7a3a4a", "#b45a70", "#e08aa0", "#f8b8c8", "#ffe0e8")
SHIMMER = pal("#3a2a6a", "#6a4aa8", "#a07ad8", "#d4b0f8", "#fff0ff")
GOLDEN = pal("#6a4a08", "#a07c16", "#d4aa18", "#f4ca28", "#fff0a0")
GLASS_P = pal("#5a7a8a", "#a8d0e0", "#e8f8ff")
FLASK = pal("#5a3a1a", "#c4a060")

POTIONS_L9 = [
    ("recall_potion", "Poção de Retorno", pal("#c8d2dc", "#2a6aa0", "#5ab4f0", "#d0f0ff")),
    ("teleport_potion", "Poção de Teletransporte", pal("#c8d2dc", "#7a2aa0", "#c45ad4", "#ffc0ff")),
    ("gravity_potion", "Poção de Gravidade", pal("#c8d2dc", "#4a3a8a", "#8a7ad4", "#e0d8ff")),
    ("wormhole_potion", "Poção do Buraco de Minhoca", pal("#c8d2dc", "#18887a", "#3ac4a8", "#a8ffe8")),
]
SOLUTIONS_L9 = [
    ("pure_solution", "Solução Pura", pal("#c8d2dc", "#2a8a2a", "#6ad46a", "#d0ffd0")),
    ("vile_solution", "Solução Vil", pal("#c8d2dc", "#5a1a6a", "#a04ac4", "#f0c0ff")),
]


def items_l9():
    stack1 = "max_stack: Some(1)"
    out = [(n, d, icon2(POTION, p, FLASK), "max_stack: Some(16)") for n, d, p in POTIONS_L9 + SOLUTIONS_L9]
    out += [
        ("magic_mirror", "Espelho Mágico", icon2(MIRROR, GOLDEN, GLASS_P), stack1),
        ("magic_conch", "Concha Mágica", icon2(CONCH, pal("#6a3a2a", "#b46a4a", "#e0a080", "#f8d0b0", "#fff0e0"), pal("#f8c8d0")), stack1),
        ("pda", "PDA do Explorador", icon2(PDA, pal("#14141a", "#3a3a48", "#6a6a80"), pal("#0e3a2a", "#2a8a6a", "#a8ffe0")), stack1),
        ("money_trough", "Cocho do Cofrinho", icon2(MIRROR, PIGGY, pal("#3a2410", "#7a5a2a", "#c4a060")), stack1),
        ("terraspark_boots", "Botas Terrafagulha", icon2(BOOTS, pal("#3a0e08", "#7a2210", "#c4461a", "#f08a3a"), pal("#5ab4f0", "#d0f0ff")), stack1),
        ("bundle_of_balloons", "Cacho de Balões", icon2(BALLOONS, pal("#3a3a48", "#c8d2dc", "#eef2f8", "#ffffff", "#8a8a9a"), pal("#c4461a", "#f08a3a", "#5ab4f0", "#d0f0ff")), stack1),
        ("sprayer", "Aspersor", icon2(SPRAYER, STEEL, pal("#2a8a2a", "#6ad46a", "#d0ffd0")), stack1),
        ("javelin", "Azagaia", icon2(JAVELIN, STEEL, WOOD), "max_stack: Some(64)"),
        ("bone_javelin", "Azagaia de Osso", icon2(JAVELIN, pal("#8a8270", "#c8c0a8", "#f0ead8"), WOOD), "max_stack: Some(64)"),
        ("sun_javelin", "Azagaia do Sol", icon2(JAVELIN, SOLAR, pal("#a03008", "#f89a30")), "max_stack: Some(64)"),
        ("vital_crystal", "Cristal Vital", icon(SHARD, pal("#5a0a1a", "#a02a3a", "#e05a6a", "#ffb0c0", "#ffffff")), ""),
        ("arcane_shard", "Lasca Arcana", icon(SHARD, pal("#0a1a5a", "#2a4aa0", "#5a8ae0", "#b0d0ff", "#ffffff")), ""),
        ("aegis_fruit", "Fruta da Égide", icon2(APPLE, SHIMMER, pal("#3a2a10", "#7a4ab4")), ""),
    ]
    return out


BLOCKS_L9 = [
    '(name: "infernal_forge", display_name: Some("Forja Infernal"), light: 10, hardness: 5.0, tool: Some(Pickaxe), textures: (top: Some("infernal_forge_top"), side: Some("infernal_forge_side"), bottom: Some("infernal_forge_side")))',
    '(name: "arcane_anvil", display_name: Some("Bigorna Arcana"), light: 4, hardness: 6.0, tool: Some(Pickaxe), tier: 3, textures: (top: Some("arcane_anvil_top"), side: Some("arcane_anvil_side"), bottom: Some("arcane_anvil_side")))',
    '(name: "titanic_forge", display_name: Some("Forja Titânica"), light: 12, hardness: 8.0, tool: Some(Pickaxe), tier: 4, textures: (top: Some("titanic_forge_top"), side: Some("titanic_forge_side"), bottom: Some("titanic_forge_side")))',
    '(name: "alchemy_table", display_name: Some("Mesa de Alquimia"), light: 3, hardness: 2.0, tool: Some(Axe), textures: (top: Some("alchemy_table_top"), side: Some("alchemy_table_side"), bottom: Some("alchemy_table_side")))',
    '(name: "piggy_bank", display_name: Some("Cofrinho"), hardness: 0.8, textures: (top: Some("piggy_bank_top"), side: Some("piggy_bank_side"), front: Some("piggy_bank_front"), bottom: Some("piggy_bank_side")))',
    '(name: "safe", display_name: Some("Cofre"), hardness: 10.0, tool: Some(Pickaxe), textures: (top: Some("safe_top"), side: Some("safe_top"), front: Some("safe_front"), bottom: Some("safe_top")))',
    '(name: "shimmer", display_name: Some("Brilho Cintilante"), shape: Liquid, transparency: Translucent, solid: false, light: 11, hardness: 100.0, in_inventory: false, drops: Some([]))',
    '(name: "shimmer_crystal", display_name: Some("Cristal Cintilante"), light: 13, hardness: 1.5, tool: Some(Pickaxe), transparency: Cutout)',
    '(name: "tombstone", display_name: Some("Lápide"), shape: Wall, hardness: 2.0, tool: Some(Pickaxe), textures: (all: Some("tombstone")))',
]


def blocks_l9():
    def rim(x, y, w=1):
        return min(x, y, 15 - x, 15 - y) < w

    def brick(x, y):
        row = y // 4
        off = 4 if row % 2 else 0
        return 0.12 if y % 4 == 3 or (x + off) % 8 == 7 else 0.45 + 0.2 * (((x + off) // 8 + row) % 2)

    def forge_top(x, y):
        if rim(x, y, 2):
            return brick(x, y)
        return 0.75 + 0.2 * ((x * 3 + y * 5) % 7 > 3)

    def forge_side(x, y):
        mouth = 4 <= x <= 11 and 6 <= y <= 11
        return 0.9 - 0.04 * abs(y - 9) if mouth else brick(x, y)

    def anvil_top(x, y):
        return 0.2 if rim(x, y) else 0.6 + 0.25 * (abs(x - 7.5) + abs(y - 7.5) < 4)

    def anvil_side(x, y):
        body = (y < 5) or (6 <= x <= 9 and y < 11) or (y >= 11 and 2 <= x <= 13)
        return 0.15 if not body else (0.95 if (x + y) % 9 == 0 else 0.55 + 0.02 * (8 - y))

    def ti_top(x, y):
        dx, dy = abs(x - 7.5), abs(y - 7.5)
        return 0.15 if rim(x, y) else (0.95 if dx < 3 and dy < 3 else 0.5 + 0.1 * ((x // 4 + y // 4) % 2))

    def ti_side(x, y):
        plate = x % 8 in (0, 7) or y % 8 in (0, 7)
        glow = 6 <= x <= 9 and 4 <= y <= 11
        return 0.95 if glow else (0.25 if plate else 0.55)

    def alch_top(x, y):
        bottle = any((x - cx) ** 2 + (y - cy) ** 2 < 5 for cx, cy in ((4, 4), (11, 5), (7, 11)))
        return 0.9 if bottle else (0.15 if rim(x, y) else 0.45 + 0.08 * (y % 3 == 0))

    def alch_side(x, y):
        return 0.2 if y < 2 or (y > 3 and x in (1, 2, 13, 14)) else (0.5 if y < 4 else 0.08)

    def piggy_side(x, y):
        return 0.3 if rim(x, y) else 0.6 + 0.15 * (((x - 7.5) ** 2 + (y - 7.5) ** 2) < 20)

    def piggy_front(x, y):
        snout = 5 <= x <= 10 and 8 <= y <= 12
        nostril = y == 10 and x in (6, 9)
        eye = y == 5 and x in (4, 11)
        return 0.05 if nostril or eye else (0.85 if snout else piggy_side(x, y))

    def piggy_top(x, y):
        return 0.05 if 5 <= x <= 10 and y in (7, 8) else piggy_side(x, y)

    def safe_top(x, y):
        return 0.2 if rim(x, y) else 0.5 + 0.06 * ((x + y) % 4 == 0)

    def safe_front(x, y):
        d = ((x - 8) ** 2 + (y - 8) ** 2) ** 0.5
        return 0.95 if d < 1.5 else (0.75 if d < 3.5 else (0.3 if d < 4.3 else safe_top(x, y)))

    def crystal(x, y):
        dx, dy = abs(x - 7.5), abs(y - 7.5)
        spikes = (dx < 2 and y > 1) or (abs(x - 3.5) < 1.6 and y > 5) or (abs(x - 11.5) < 1.6 and y > 4)
        return 0.0 if not spikes else 0.6 + 0.35 * (1 - dy / 8)

    def stone(x, y):
        cross = (7 <= x <= 8 and 2 <= y <= 9) or (5 <= x <= 10 and 4 <= y <= 5)
        return 0.2 if cross else 0.5 + 0.1 * ((x * 7 + y * 3) % 5 == 0)

    hot = pal("#2a0a04", "#6a1a08", "#c43a10", "#f88a20", "#ffe080")
    dark = pal("#1a1418", "#2e262c", "#4a3e44", "#6a5a60", "#8a787e")
    arcane = pal("#14183a", "#2a3a7a", "#4a6ab4", "#8ab0f0", "#e0f0ff")
    titan = pal("#1a1a22", "#3a3a4a", "#6a6a84", "#a8a8c4", "#f0ffe0")
    wood = pal("#3a2410", "#5a3a1a", "#7a5a2a", "#a07c3a", "#c4a060")
    alch = pal("#3a2410", "#5a3a1a", "#7a5a2a", "#a07c3a", "#b8f0e0")
    steel = pal("#2a2e36", "#4a505c", "#6a7280", "#9aa2b0", "#d0d8e0")
    rock = pal("#3a3a3e", "#5a5a60", "#7a7a80", "#9a9aa0", "#babac0")
    out = {
        "infernal_forge_top": block_noise(90, forge_top, hot),
        "infernal_forge_side": block_noise(91, lambda x, y: forge_side(x, y), [*dark[:3], *hot[3:]]),
        "arcane_anvil_top": block_noise(92, anvil_top, arcane),
        "arcane_anvil_side": block_noise(93, anvil_side, arcane),
        "titanic_forge_top": block_noise(94, ti_top, titan),
        "titanic_forge_side": block_noise(95, ti_side, titan),
        "alchemy_table_top": block_noise(96, alch_top, alch),
        "alchemy_table_side": block_noise(97, alch_side, wood),
        "piggy_bank_top": block_noise(98, piggy_top, PIGGY),
        "piggy_bank_side": block_noise(99, piggy_side, PIGGY),
        "piggy_bank_front": block_noise(100, piggy_front, PIGGY),
        "safe_top": block_noise(101, safe_top, steel),
        "safe_front": block_noise(102, safe_front, steel),
        "shimmer": block_noise(103, lambda x, y: 0.55 + 0.3 * ((x + 2 * y) % 6 < 2) - 0.1 * ((x * y) % 5 == 0), SHIMMER),
        "tombstone": block_noise(105, stone, rock),
    }
    cr = block_noise(104, crystal, SHIMMER)
    for y in range(N):
        for x in range(N):
            if crystal(x, y) == 0.0:
                cr[y, x, 3] = 0
    out["shimmer_crystal"] = cr
    return out


SHAPED_L9 = [
    (["OMO", "MFM", "OOO"], {"O": "obsidian", "M": "magmarite_ore", "F": "furnace"}, "infernal_forge", 1),
    (["MMM", " A ", "MMM"], {"M": "mythril_ingot", "A": "anvil"}, "arcane_anvil", 1),
    (["MMM", " A ", "MMM"], {"M": "orichalcum_ingot", "A": "anvil"}, "arcane_anvil", 1),
    (["AEA", "AFA", "AAA"], {"A": "adamantite_ingot", "E": "ectoplasm", "F": "infernal_forge"}, "titanic_forge", 1),
    (["AEA", "AFA", "AAA"], {"A": "titanium_ingot", "E": "ectoplasm", "F": "infernal_forge"}, "titanic_forge", 1),
    (["CCC", "CGC", "CCC"], {"C": "clay_ball", "G": "gold_nugget"}, "piggy_bank", 1),
    (["III", "IGI", "III"], {"I": "iron_ingot", "G": "gold_ingot"}, "safe", 1),
    (["GgG", "gRg", "GgG"], {"G": "gold_ingot", "g": "glass", "R": "recall_potion"}, "magic_mirror", 1),
    (["RRB", " K ", " R "], {"R": "radiant_ingot", "B": "bucket", "K": "redstone"}, "sprayer", 1),
    (["  F", " S ", "S  "], {"F": "flint", "S": "stick"}, "javelin", 8),
    (["  F", " S ", "S  "], {"F": "bone", "S": "stick"}, "bone_javelin", 8),
    (["  F", " S ", "S  "], {"F": "solar_fragment", "S": "stick"}, "sun_javelin", 16),
]
SHAPELESS_L9 = [
    (["brewing_stand", "glass_bottle", "crafting_table"], "alchemy_table", 1),
    (["glass_bottle", "glowstone_dust", "feather"], "recall_potion", 1),
    (["glass_bottle", "ender_pearl", "chorus_fruit"], "teleport_potion", 1),
    (["glass_bottle", "phantom_membrane", "feather"], "gravity_potion", 1),
    (["glass_bottle", "ender_pearl", "prismarine_shard"], "wormhole_potion", 1),
    (["purification_powder", "glass_bottle"], "pure_solution", 4),
    (["vile_powder", "glass_bottle"], "vile_solution", 4),
    (["piggy_bank", "bucket"], "money_trough", 1),
    (["explorer_kit", "creature_radar", "battle_tally", "ore_finder", "magic_mirror"], "pda", 1),
    (["frost_spark_boots", "obsidian_charm", "water_walking_boots"], "terraspark_boots", 1),
    (["cloud_bottle", "blizzard_bottle", "sandstorm_bottle", "shiny_balloon"], "bundle_of_balloons", 1),
]


def structures_l9():
    """The Shimmer Lake: a hidden cave 30 blocks down with a shaft of crystals; the oasis (a
    pool under palms in the desert); the graveyard (tombstones, a fence and a chest)."""
    W, H, UP = 15, 7, 24
    lake = []
    for y in range(H + UP):
        rows = []
        for z in range(W):
            row = ""
            for x in range(W):
                if y >= H:
                    # The shaft: a ladder on a stone spine; two crystals mark it on the surface.
                    top = y >= H + UP - 2
                    ch = "L" if (x, z) == (2, 7) else ("c" if top and (x, z) == (3, 7) else ("S" if (x, z) == (3, 7) else " "))
                    row += ch
                    continue
                d = ((x - 7) ** 2 + (z - 7) ** 2) ** 0.5
                edge = x in (0, W - 1) or z in (0, W - 1) or y in (0, H - 1)
                if edge:
                    ch = "S"
                elif y == 1 and d < 4.6:
                    ch = "~"
                elif y == 1:
                    ch = "a"
                elif y == 2 and 3.6 < d < 4.6:
                    ch = "a"
                elif y in (2, 3) and (x, z) in ((2, 2), (12, 2), (2, 12), (12, 12)):
                    ch = "c"
                elif y == 2 and (x, z) == (12, 7):
                    ch = "K"
                else:
                    ch = "."
                if (x, z) == (2, 7) and y > 0:
                    ch = "L"
                if (x, z) == (3, 7) and y > 1:
                    ch = "S"
                row += ch
            rows.append(row)
        lake.append(rows)
    oasis = []
    for y in range(7):
        rows = []
        for z in range(11):
            row = ""
            for x in range(11):
                d = ((x - 5) ** 2 + (z - 5) ** 2) ** 0.5
                palm = (x, z) in ((1, 1), (9, 9))
                ch = " "
                if y == 0:
                    ch = "w" if d < 3.2 else ("G" if d < 5.2 else " ")
                elif y == 1 and d >= 3.2 and d < 4.5 and (x + z) % 3 == 0:
                    ch = "f"
                if palm and 1 <= y <= 5:
                    ch = "T"
                if y == 6 and any(abs(x - px) + abs(z - pz) <= 2 for px, pz in ((1, 1), (9, 9))):
                    ch = "e"
                if y == 1 and (x, z) == (9, 1):
                    ch = "K"
                row += ch
            rows.append(row)
        oasis.append(rows)
    grave = []
    for y in range(3):
        rows = []
        for z in range(11):
            row = ""
            for x in range(11):
                fence = (x in (0, 10) or z in (0, 10)) and not (z == 10 and x == 5)
                ch = " "
                if y == 0:
                    ch = "P"
                elif y == 1:
                    if fence:
                        ch = "F"
                    elif x % 3 == 2 and z % 3 == 2 and (x, z) != (5, 5):
                        ch = "t"
                    elif (x, z) == (5, 5):
                        ch = "K"
                    elif (x * 3 + z) % 7 == 0:
                        ch = "d"
                row += ch
            rows.append(row)
        grave.append(rows)
    return (lake, H + UP - 2), oasis, grave



import re  # noqa: E402

# ==============================================================================================
# Lote 10: world difficulty (Normal, Especialista, Mestre), treasure bags with an item only
# they hold, relics in Mestre, the bestiary
# ==============================================================================================

BAG = [
    "................",
    "......1111......",
    ".....1a22a1.....",
    "......1bb1......",
    ".....122221.....",
    "....12333321....",
    "...1233443321...",
    "..123344443321..",
    "..123444c44321..",
    "..12344ccc4321..",
    "..123444c44321..",
    "..123344443321..",
    "...1233333321...",
    "....11222211....",
    "......1111......",
    "................",
]
BOOK_ICON = [
    "................",
    "..11111111111...",
    "..1aa22222221...",
    "..1aa23333321...",
    "..1aa23b3b321...",
    "..1aa233b3321...",
    "..1aa23b3b321...",
    "..1aa23333321...",
    "..1aa22222221...",
    "..1aa2cccc221...",
    "..1aa2cccc221...",
    "..1aa22222221...",
    "..1aa22222221...",
    "..11111111111...",
    "...1cccccccc1...",
    "................",
]
STONE_ICON = [
    "................",
    "................",
    "......1111......",
    ".....122221.....",
    "....12333321....",
    "...1233443321...",
    "...123aa44321...",
    "..1234aaa44321..",
    "..123444a44321..",
    "..123344443321..",
    "...1233333321...",
    "....12222221....",
    ".....111111.....",
    "................",
    "................",
    "................",
]

# Boss -> (bag colours, expert item (name, display, icon rows) or None).
BOSSES_L10 = [
    ("king_jelly", pal("#0e2a6a", "#1e4aa0", "#3a7ad4", "#7ab4f0", "#d0ecff"), ("jelly_heart", "Coração de Geleia", GEL)),
    ("great_eye", pal("#5a0a0a", "#8a1a1a", "#c43a3a", "#f08a8a", "#ffe0e0"), ("eye_of_the_watcher", "Olho Vigia", LENS)),
    ("blight_mind", pal("#4a0a14", "#7a1a2a", "#b4344a", "#e47a8a", "#ffd0d8"), ("mind_veil", "Véu da Mente", TISSUE)),
    ("hive_queen", pal("#5a3a04", "#8a5a08", "#c48a18", "#f0c040", "#fff0a0"), ("hive_heart", "Coração da Colmeia", COMB)),
    ("frost_colossus", pal("#1a3a5a", "#2a5a8a", "#4a8ac4", "#9ad0f0", "#eaf8ff"), ("frost_core", "Núcleo Gélido", SHARD)),
    ("deep_worm", pal("#2a1a10", "#4a3018", "#7a5028", "#a87a48", "#e0c0a0"), ("worm_scarf", "Cachecol do Verme", RING)),
    ("bone_king", pal("#3a3428", "#6a6050", "#9a9078", "#c8c0a8", "#f0ead8"), ("bone_glove", "Luva de Osso", HORN)),
    ("world_breaker", pal("#2a0a2a", "#4a1446", "#7a2a70", "#b45aa8", "#f0b0e8"), ("breaker_heart", "Coração do Quebra-Mundos", CORE)),
    ("mech_eye", pal("#2a2e36", "#4a505c", "#7a828e", "#b0b8c4", "#eef2f8"), ("mech_core", "Núcleo Mecânico", CORE)),
    ("mech_worm", pal("#2a2e36", "#4a505c", "#7a828e", "#b0b8c4", "#eef2f8"), None),
    ("mech_skull", pal("#2a2e36", "#4a505c", "#7a828e", "#b0b8c4", "#eef2f8"), None),
    ("vine_queen", pal("#0e3a14", "#1e6a26", "#3aa040", "#7ad46a", "#d0ffc0"), ("vine_seed", "Semente da Trepadeira", SHARD)),
    ("jelly_queen", pal("#5a1a4a", "#8a2a7a", "#c45ab4", "#f0a0e0", "#ffe0f8"), ("gel_crown", "Coroa de Gel", CROWN)),
    ("scale_idol", TEMPLE, ("sun_core", "Núcleo Solar", CORE)),
    ("tide_duke", TIDE, ("tide_scale", "Escama das Marés", SHARD)),
    ("prism_empress", CRYSTAL, ("prism_heart", "Coração Prismático", SHARD)),
    ("lunar_prophet", PROPHET, ("prophet_eye", "Olho do Profeta", LENS)),
    ("celestial_herald", STARDUST, ("herald_sigil", "Sigilo do Arauto", MEDAL)),
    ("pumpkin_king", pal("#5a2a04", "#a04a08", "#e07a18", "#f8b84a", "#fff0a0"), None),
    ("ice_queen", pal("#2a4a6a", "#4a7aa0", "#7aaed4", "#b4daf4", "#eef8ff"), None),
    ("pirate_captain", pal("#3a2410", "#5a3a1a", "#7a5a2a", "#a07c3a", "#c4a060"), None),
    ("dark_mage", pal("#1a0a2a", "#2e1446", "#4a2a70", "#7a5aa8", "#c0a0f0"), None),
    ("saucer", pal("#1a3a2a", "#2a6a4a", "#4aa07a", "#7ad4a8", "#e8fff4"), None),
    ("wither", pal("#0e0e14", "#2a2a36", "#4a4a5a", "#7a7a8e", "#b0b0c4"), None),
    ("ender_dragon", pal("#0e0a14", "#2a1a36", "#4a2a6a", "#8a5ab4", "#e0c0ff"), None),
    ("lich", pal("#0a2a2a", "#145a5a", "#2a8a8a", "#5ac4c4", "#c0ffff"), None),
    ("forest_guardian", pal("#1a2a0e", "#2e4a14", "#4a7a20", "#7ab43a", "#c8f08a"), None),
    ("obsidilith", pal("#0e0a14", "#1e1428", "#3a2a4a", "#5a4a70", "#8a7aa0"), None),
    ("void_blossom", pal("#1a0a2a", "#3a1a5a", "#6a3a9a", "#a87ad4", "#f0d0ff"), None),
    ("void_guardian", pal("#0a0a1a", "#1a1a3a", "#2a2a5a", "#4a4a8a", "#8a8ac4"), None),
]


def boss_names():
    names = {}
    for f in (ROOT / "packs/vanilla").rglob("*mobs*.ron"):
        for m in re.finditer(r'name: "([a-z_0-9]+)",\s*display_name: "([^"]+)"', f.read_text()):
            names.setdefault(m.group(1), m.group(2))
    return names


def items_l10():
    names = boss_names()
    gold = pal("#c49a10", "#f4ca28", "#fff0a0")
    out = []
    for boss, p, expert in BOSSES_L10:
        assert boss in names, boss
        out.append((f"bag_{boss}", f"Sacola do Tesouro: {names[boss]}", icon2(BAG, p, gold), "max_stack: Some(16)"))
        if expert:
            n, d, rows = expert
            out.append((n, d, icon2(rows, p, gold) if any(c in "abc" for r in rows for c in r) else icon(rows, p), "max_stack: Some(1)"))
    out += [
        ("bestiary", "Bestiário", icon2(BOOK_ICON, pal("#3a2410", "#c8b890", "#ece0c0"), pal("#5a3a1a", "#7a4a2a", "#c49a10")), "max_stack: Some(1)"),
        ("daring_stone", "Pedra da Ousadia", icon2(STONE_ICON, pal("#1a1418", "#3a2a2e", "#5a3a3e", "#7a4a4e"), pal("#f04a2a")), "max_stack: Some(1)"),
    ]
    return out


def block_defs_l10():
    names = boss_names()
    return [
        f'(name: "relic_{b}", display_name: Some("Relíquia: {names[b]}"), light: 7, hardness: 3.0, tool: Some(Pickaxe), textures: (top: Some("relic_{b}"), side: Some("relic_side"), bottom: Some("relic_side")))'
        for b, _, _ in BOSSES_L10
    ]


def blocks_l10():
    def side(x, y):
        band = y in (2, 3, 12, 13)
        return 0.95 if band and x % 4 == 1 else (0.75 if band else 0.35 + 0.1 * ((x + y) % 5 == 0))

    out = {"relic_side": block_noise(110, side, pal("#3a2a08", "#6a4a10", "#a07c16", "#d4aa18", "#fff0a0"))}
    for k, (b, p, _) in enumerate(BOSSES_L10):
        def top(x, y):
            dx, dy = abs(x - 7.5), abs(y - 7.5)
            if min(x, y, 15 - x, 15 - y) < 2:
                return -1
            return 0.95 - 0.07 * (dx + dy) if dx + dy < 6 else 0.25
        img = block_noise(111 + k, lambda x, y: max(top(x, y), 0.0), p)
        frame = block_noise(200 + k, side, pal("#3a2a08", "#6a4a10", "#a07c16", "#d4aa18", "#fff0a0"))
        for y in range(N):
            for x in range(N):
                if top(x, y) < 0:
                    img[y, x] = frame[y, x]
        out[f"relic_{b}"] = img
    return out


SHAPED_L10 = [
    (["SDS", "DOD", "SDS"], {"S": "stone", "D": "diamond", "O": "obsidian"}, "daring_stone", 1),
]
SHAPELESS_L10 = [
    (["book", "spider_eye", "bone"], "bestiary", 1),
]



# ==============================================================================================
# Lote 11: vanity clothes (worn over the armour), the paint brush, roller and scraper (they
# recolour wool, concrete, terracotta, glass and the like with the dyes), and the music box
# ==============================================================================================

VANITY_L11 = [
    ("pirate", "de Pirata", "red_wool", pal("#3a0a0a", "#6a1414", "#9a2a22", "#c44a3a", "#e88a6a")),
    ("wizard", "de Mago", "purple_wool", pal("#1a0a3a", "#2e1460", "#4a2a8a", "#7a5ab4", "#b49ae0")),
    ("tuxedo", "de Gala", "black_wool", pal("#0a0a0e", "#16161c", "#24242c", "#3a3a46", "#e8e8f0")),
    ("explorer", "de Explorador", "brown_wool", pal("#3a2a10", "#5a4420", "#806434", "#a8885a", "#d4bc90")),
    ("sailor", "de Marinheiro", "blue_wool", pal("#0a1a4a", "#14307a", "#2a4ea8", "#e8eef8", "#ffffff")),
    ("ranger", "de Patrulheiro", "green_wool", pal("#0e2a10", "#1a4a1c", "#2e6a2e", "#4a8a3a", "#8ab46a")),
    ("royal", "Real", "yellow_wool", pal("#5a3a08", "#8a5e10", "#c49418", "#f0c840", "#fff0a0")),
    ("shadow", "das Sombras", "gray_wool", pal("#0e0a14", "#1a1424", "#2a2238", "#40364e", "#6a5a7a")),
]
VANITY_PIECES = {"helmet": "Chapéu", "chestplate": "Casaco", "leggings": "Calças", "boots": "Botas"}

BRUSH = [
    "................",
    "...........aa...",
    "..........abba..",
    ".........abbba..",
    "........1abba...",
    ".......121aa....",
    "......121.......",
    ".....121........",
    "....121.........",
    "...121..........",
    "..121...........",
    ".121............",
    ".11.............",
    "................",
    "................",
    "................",
]
ROLLER = [
    "................",
    "..aaaaaaaaaa....",
    ".abbbbbbbbbba...",
    ".abbbbbbbbbba...",
    "..aaaaaaaaaa1...",
    "............1...",
    "......111111....",
    "......1.........",
    "......1.........",
    ".....121........",
    ".....121........",
    ".....121........",
    ".....121........",
    ".....111........",
    "................",
    "................",
]
SCRAPER = [
    "................",
    "................",
    "..aaaaaa........",
    ".abbbbbba.......",
    "..aaaaaa1.......",
    ".......121......",
    "........121.....",
    ".........121....",
    "..........121...",
    "...........121..",
    "............11..",
    "................",
    "................",
    "................",
    "................",
    "................",
]
RAINBOW_P = pal("#c43a3a", "#f0b030", "#3aa0f0")


def items_l11():
    out = []
    for key, suffix, _, p in VANITY_L11:
        m = f"vanity_{key}"
        gi.ARMOR_MATERIALS[m] = p
        for k in gi.ARMOR_KINDS:
            out.append((f"{m}_{k}", f"{VANITY_PIECES[k]} {suffix}", gi.armor(m, k), "max_stack: Some(1)"))
    out += [
        ("vanity_mirror", "Espelho de Vaidade", icon2(MIRROR, pal("#4a3a5a", "#7a5a9a", "#a88ad0", "#d4b8f0", "#f8e8ff"), GLASS_P), "max_stack: Some(1)"),
        ("paint_brush", "Pincel", icon2(BRUSH, WOOD, RAINBOW_P), "max_stack: Some(1)"),
        ("paint_roller", "Rolo de Pintura", icon2(ROLLER, WOOD, RAINBOW_P), "max_stack: Some(1)"),
        ("paint_scraper", "Raspador de Tinta", icon2(SCRAPER, WOOD, STEEL), "max_stack: Some(1)"),
    ]
    return out


BLOCKS_L11 = [
    '(name: "music_box", display_name: Some("Caixa de Música"), hardness: 1.5, tool: Some(Axe), textures: (top: Some("music_box_top"), side: Some("music_box_side"), bottom: Some("music_box_side")))',
]


def blocks_l11():
    def side(x, y):
        rim = min(x, y, 15 - x, 15 - y) == 0
        grille = 4 <= x <= 11 and 4 <= y <= 11 and (x + y) % 2 == 0
        return 0.15 if rim else (0.1 if grille else 0.5 + 0.12 * (y % 4 == 0))

    def top(x, y):
        drum = 3 <= y <= 6 and 2 <= x <= 13
        comb = 9 <= y <= 12 and 3 <= x <= 12 and x % 2 == 1
        return 0.95 if comb else (0.75 + 0.15 * ((x + y) % 3 == 0) if drum else side(x, y))

    wood = pal("#2a1608", "#4a2a12", "#6a4020", "#8a5a30", "#e0c060")
    return {"music_box_side": block_noise(130, side, wood), "music_box_top": block_noise(131, top, wood)}


def _vanity_recipes():
    out = []
    for key, _, wool, _ in VANITY_L11:
        m = f"vanity_{key}"
        out += [
            (["WWW", "W W"], {"W": wool}, f"{m}_helmet", 1),
            (["W W", "WSW", "WWW"], {"W": wool, "S": "string"}, f"{m}_chestplate", 1),
            (["WWW", "WSW", "W W"], {"W": wool, "S": "string"}, f"{m}_leggings", 1),
            (["W W", "WSW"], {"W": wool, "S": "string"}, f"{m}_boots", 1),
        ]
    return out


SHAPED_L11 = _vanity_recipes() + [
    (["  W", " S ", "S  "], {"W": "white_wool", "S": "stick"}, "paint_brush", 1),
    (["WWW", " S ", " S "], {"W": "white_wool", "S": "stick"}, "paint_roller", 1),
    (["  I", " S ", "S  "], {"I": "iron_ingot", "S": "stick"}, "paint_scraper", 1),
    (["PPP", "PNP", "PGP"], {"P": "oak_planks", "N": "note_block", "G": "gold_ingot"}, "music_box", 1),
]
SHAPELESS_L11 = [
    (["glass", "gold_ingot", "amethyst"], "vanity_mirror", 1),
]



# ==============================================================================================
# What was left (after Lote 11): the big dungeon (three colours of brick, four floors of halls
# under a tower, spikes, locked gold chests opened with golden keys) and the dye vat that dyes
# the armour
# ==============================================================================================

DUNGEON_COLOURS = [
    ("blue", "Azul", pal("#0e1a3a", "#1a2c5a", "#2a447e", "#3e5e9e", "#5a7ab8"), ["Snowy Plains", "Snowy Taiga", "Taiga", "Old Growth Taiga", "Grove", "Windswept Hills", "Stony Shore"]),
    ("green", "Verde", pal("#0e2a1a", "#1a4428", "#2a5e38", "#3e7a4a", "#5a9a62"), ["Forest", "Birch Forest", "Dark Forest", "Jungle", "Swamp", "Mangrove Swamp"]),
    ("pink", "Rosa", pal("#3a0e2a", "#5a1a44", "#7e2a5e", "#9e3e7a", "#b85a96"), ["Plains", "Savanna", "Meadow", "Flower Forest", "Sunflower Plains", "Badlands"]),
]
KEY_ICON = [
    "................",
    "................",
    "....1111........",
    "...122221.......",
    "..12333321......",
    "..1231132111111.",
    "..123..32222221.",
    "..1231132211221.",
    "..12333321..121.",
    "...122221...11..",
    "....1111........",
    "................",
    "................",
    "................",
    "................",
    "................",
]
SHIELD_ICON = [
    "................",
    "...1111111111...",
    "..122222222221..",
    "..123333333321..",
    "..1233aaaa3321..",
    "..123aabbaa321..",
    "..123abbbba321..",
    "..123aabbaa321..",
    "...1233aa3321...",
    "...1233333321...",
    "....12333321....",
    ".....123321.....",
    "......1221......",
    ".......11.......",
    "................",
    "................",
]


def items_l12():
    gi.MATERIALS["katana"] = pal("#14284a", "#2a4a7a", "#5a8ac4", "#a8d0f0", "#eef8ff")
    gi.MATERIALS["bone"] = pal("#6a6250", "#9a9078", "#c8c0a8", "#e8e2cc", "#fffaee")
    gold = pal("#5a3a08", "#a07c16", "#d4aa18", "#f4ca28", "#fff0a0")
    return [
        ("golden_key", "Chave Dourada", icon(KEY_ICON, gold), "max_stack: Some(16)"),
        ("blue_katana", "Katana Azul", gi.tool("katana", "sword"), "max_stack: Some(1), tool: Some((kind: Sword, tier: 3, speed: 12.0, damage: 7.0, durability: 1400))"),
        ("bastion_shield", "Escudo do Bastião", icon2(SHIELD_ICON, pal("#14284a", "#2a4a7a", "#5a8ac4"), pal("#c49a10", "#f4ca28")), "max_stack: Some(1)"),
        ("bone_flail", "Mangual de Ossos", icon2(FLAIL, gi.MATERIALS["bone"], WOOD) if any(c in "ab" for r in FLAIL for c in r) else icon(FLAIL, gi.MATERIALS["bone"]), "max_stack: Some(1)"),
    ]


def block_defs_l12():
    out = []
    for key, disp, _, _ in DUNGEON_COLOURS:
        out.append(f'(name: "dungeon_brick_{key}", display_name: Some("Tijolo da Masmorra {disp}"), hardness: 60.0, tool: Some(Pickaxe), tier: 4)')
    out += [
        '(name: "dungeon_spikes", display_name: Some("Espinhos"), shape: Cross, transparency: Cutout, solid: false, hardness: 4.0, tool: Some(Pickaxe))',
        '(name: "locked_chest", display_name: Some("Baú Trancado"), hardness: 80.0, tool: Some(Pickaxe), tier: 4, textures: (top: Some("locked_chest_top"), side: Some("locked_chest_side"), front: Some("locked_chest_front"), bottom: Some("locked_chest_top")), drops: Some([]))',
        '(name: "dungeon_lantern", display_name: Some("Lanterna da Masmorra"), light: 13, hardness: 1.0, transparency: Cutout)',
        '(name: "dye_vat", display_name: Some("Tina de Tingir"), hardness: 1.5, tool: Some(Axe), textures: (top: Some("dye_vat_top"), side: Some("dye_vat_side"), bottom: Some("dye_vat_side")))',
    ]
    return out


def blocks_l12():
    def brick(x, y):
        row = y // 4
        off = 4 if row % 2 else 0
        return 0.12 if y % 4 == 3 or (x + off) % 8 == 7 else 0.5 + 0.25 * (((x + off) // 8 * 3 + row) % 3 == 0)

    def spikes(x, y):
        for cx in (2, 7, 12):
            h = 13 - (abs(x - cx) * 5)
            if abs(x - cx) <= 2 and y >= 15 - max(h, 0) and y > 2:
                return 0.4 + 0.5 * (1 - abs(x - cx) / 2.5)
        return -1

    def chest_side(x, y):
        rim = min(x, y, 15 - x, 15 - y) == 0 or y == 5
        band = x in (3, 12)
        return 0.15 if rim else (0.9 if band else 0.45 + 0.1 * (y % 3 == 0))

    def chest_front(x, y):
        lock = 6 <= x <= 9 and 4 <= y <= 8
        hole = x in (7, 8) and y in (6, 7)
        return 0.05 if hole else (0.95 if lock else chest_side(x, y))

    def lantern(x, y):
        frame = x in (3, 12) or y in (2, 13)
        inside = 3 < x < 12 and 2 < y < 13
        return 0.2 if frame and 3 <= x <= 12 and 2 <= y <= 13 else (0.95 - 0.04 * abs(y - 8) if inside else -1)

    def vat_top(x, y):
        d = ((x - 7.5) ** 2 + (y - 7.5) ** 2) ** 0.5
        return 0.2 if d > 7 else (0.95 if d < 2 else (0.6 + 0.25 * ((x + y) % 4 < 2) if d < 6 else 0.3))

    def vat_side(x, y):
        hoop = y in (3, 12)
        return 0.15 if hoop else 0.45 + 0.1 * (x % 4 == 0)

    out = {}
    for k, (key, _, p, _) in enumerate(DUNGEON_COLOURS):
        out[f"dungeon_brick_{key}"] = block_noise(140 + k, brick, p)
    steel = pal("#2a2e36", "#4a505c", "#7a828e", "#b0b8c4", "#eef2f8")
    sp = block_noise(150, lambda x, y: max(spikes(x, y), 0.0), steel)
    lt = block_noise(151, lambda x, y: max(lantern(x, y), 0.0), pal("#1a1a22", "#3a3a4a", "#5a8ac4", "#a8d0f0", "#eef8ff"))
    for y in range(N):
        for x in range(N):
            if spikes(x, y) < 0:
                sp[y, x, 3] = 0
            if lantern(x, y) < 0:
                lt[y, x, 3] = 0
    out["dungeon_spikes"] = sp
    out["dungeon_lantern"] = lt
    gold = pal("#3a2a08", "#6a4a10", "#a07c16", "#d4aa18", "#fff0a0")
    out["locked_chest_side"] = block_noise(152, chest_side, gold)
    out["locked_chest_top"] = block_noise(153, lambda x, y: 0.15 if min(x, y, 15 - x, 15 - y) == 0 else 0.5 + 0.2 * (x in (3, 12)), gold)
    out["locked_chest_front"] = block_noise(154, chest_front, gold)
    rainbow = [(170, 40, 40), (220, 140, 30), (230, 210, 60), (70, 160, 60), (50, 110, 200), (130, 60, 170)]
    vt = block_noise(155, vat_top, pal("#2a1608", "#4a2a12", "#6a4020", "#8a5a30", "#a07a40"))
    for y in range(N):
        for x in range(N):
            d = ((x - 7.5) ** 2 + (y - 7.5) ** 2) ** 0.5
            if d < 6:
                c = rainbow[int((np.arctan2(y - 7.5, x - 7.5) + np.pi) / (2 * np.pi) * 6) % 6]
                vt[y, x, :3] = [v * (0.8 + 0.2 * (d < 3)) for v in c]
    out["dye_vat_top"] = vt
    out["dye_vat_side"] = block_noise(156, vat_side, pal("#2a1608", "#4a2a12", "#6a4020", "#8a5a30", "#a07a40"))
    return out


SHAPED_L12 = [
    (["P P", "PDP", "PPP"], {"P": "oak_planks", "D": "water_bucket"}, "dye_vat", 1),
]
SHAPELESS_L12 = []


def dungeon_layout(seed):
    """Four floors of rooms and halls under a brick tower: a 5x5 grid of 8-block cells per
    floor, joined by a random spanning tree (plus a few loops) with 3-wide doorways."""
    import random
    rnd = random.Random(seed)
    C, S = 5, 8
    W = C * S + 1
    FLOORS, FH = 4, 5
    UNDER = FLOORS * FH + 1
    SHAFT = 6  # air between the top floor's ceiling and the ground
    TOWER = 9
    H = UNDER + SHAFT + TOWER
    grid = [[[" "] * W for _ in range(W)] for _ in range(H)]
    mid = C // 2

    def put(y, z, x, ch):
        if 0 <= y < H and 0 <= z < W and 0 <= x < W:
            grid[y][z][x] = ch

    for f in range(FLOORS):
        y0 = f * FH
        # Solid brick block for this floor, then carve.
        for y in range(y0, y0 + FH + 1):
            for z in range(W):
                for x in range(W):
                    if grid[y][z][x] == " ":
                        grid[y][z][x] = "B"
        cells = [(cx, cz) for cz in range(C) for cx in range(C)]
        seen = {(mid, mid)}
        edges = []
        stack = [(mid, mid)]
        while stack:
            c = stack[-1]
            nb = [(c[0] + dx, c[1] + dz) for dx, dz in ((1, 0), (-1, 0), (0, 1), (0, -1))]
            nb = [n for n in nb if 0 <= n[0] < C and 0 <= n[1] < C and n not in seen]
            if not nb:
                stack.pop()
                continue
            n = rnd.choice(nb)
            seen.add(n)
            edges.append((c, n))
            stack.append(n)
        for _ in range(4):
            a = rnd.choice(cells)
            b = (a[0] + rnd.choice((-1, 1)), a[1]) if rnd.random() < 0.5 else (a[0], a[1] + rnd.choice((-1, 1)))
            if 0 <= b[0] < C and 0 <= b[1] < C:
                edges.append((a, b))
        degree = {c: 0 for c in cells}
        for a, b in edges:
            degree[a] += 1
            degree[b] += 1
        # Rooms: the inside of each cell.
        for cx, cz in cells:
            x0, z0 = cx * S + 1, cz * S + 1
            for y in range(y0 + 1, y0 + FH):
                for z in range(z0, z0 + S - 1):
                    for x in range(x0, x0 + S - 1):
                        grid[y][z][x] = "."
        # Doorways.
        for (ax, az), (bx, bz) in edges:
            if ax != bx:
                x = max(ax, bx) * S
                zc = az * S + S // 2
                for y in range(y0 + 1, y0 + 4):
                    for z in range(zc - 1, zc + 2):
                        grid[y][z][x] = "."
            else:
                z = max(az, bz) * S
                xc = ax * S + S // 2
                for y in range(y0 + 1, y0 + 4):
                    for x in range(xc - 1, xc + 2):
                        grid[y][z][x] = "."
        # Furnishing.
        for cx, cz in cells:
            x0, z0 = cx * S + 1, cz * S + 1
            cxm, czm = x0 + 3, z0 + 3
            if (cx, cz) != (mid, mid):
                put(y0 + FH - 1, czm, cxm, "l")
            if (cx, cz) == (mid, mid):
                continue
            r = rnd.random()
            if degree[(cx, cz)] == 1:
                # Dead ends keep the treasure: mostly locked.
                put(y0 + 1, z0 + 1, x0 + 1, "X" if rnd.random() < 0.7 else "K")
            elif r < 0.25:
                for z in range(z0 + 1, z0 + 6):
                    for x in range(x0 + 1, x0 + 6):
                        if (x + z) % 2 == 0:
                            put(y0 + 1, z, x, "v")
            elif r < 0.45:
                for z in range(z0, z0 + S - 1):
                    put(y0 + 1, z, x0, "b")
                    put(y0 + 2, z, x0, "b")
            elif r < 0.6:
                for k in range(4):
                    put(y0 + FH - 1, z0 + rnd.randrange(S - 1), x0 + rnd.randrange(S - 1), "w")
            elif r < 0.7:
                for x in range(x0, x0 + S - 1):
                    put(y0 + 1, z0 + 3, x, "i")
                put(y0 + 1, z0 + 3, x0 + 3, ".")
    # The shaft in the middle cell: through every floor and up to the tower, a ladder on a
    # brick spine.
    sx, sz = mid * S + 2, mid * S + 4
    for y in range(1, UNDER + SHAFT + 1):
        if y >= UNDER:
            for dz in (-1, 0, 1):
                for dx in (-1, 0, 1, 2):
                    put(y, sz + dz, sx + dx, "B")
        put(y, sz, sx, "L")
        put(y, sz, sx + 1, "B")
        if y < UNDER:
            put(y, sz, sx - 1, "." if grid[y][sz][sx - 1] != "B" or y % FH == 0 else grid[y][sz][sx - 1])
    # Floors pierced where the ladder goes up.
    # The tower over the ground: 11x11 brick, a door, lanterns, battlements.
    tx0, tz0 = mid * S - 1, mid * S - 1
    T = 11
    for k in range(TOWER):
        y = UNDER + SHAFT + k
        for z in range(tz0, tz0 + T):
            for x in range(tx0, tx0 + T):
                edge = x in (tx0, tx0 + T - 1) or z in (tz0, tz0 + T - 1)
                if k == 0:
                    ch = "B"
                elif k == TOWER - 1:
                    ch = "B" if edge and (x + z) % 2 == 0 else " "
                elif edge:
                    ch = "B"
                    if (z == tz0 + T - 1) and abs(x - (tx0 + T // 2)) <= 1 and k <= 3:
                        ch = "."
                    if k == 4 and (x - tx0) % 5 == 0 and (z - tz0) % 5 == 0:
                        ch = "l"
                elif k == TOWER - 2:
                    ch = "B"
                else:
                    ch = "."
                put(y, z, x, ch)
        put(UNDER + SHAFT + k, sz, sx, "L" if k < TOWER - 2 else grid[UNDER + SHAFT + k][sz][sx])
        put(UNDER + SHAFT + k, sz, sx + 1, "B")
    put(UNDER + SHAFT, sz, sx, "L")
    sink = UNDER + SHAFT
    return [["".join(r) for r in layer] for layer in grid], sink


def write_skins(skins):
    parsed, layers = g.parse_mobs(str(OUT / "mobs.ron"))
    assert set(parsed) == set(skins), sorted(set(parsed) ^ set(skins))
    imgs = []
    for name, (roles, paint, overlay) in skins.items():
        assert len(roles) == len(parsed[name]), name
        g.ROLES[name] = roles
        g.PAINTERS[name] = as_int(paint)
        g.OVERLAYS[name] = as_int(overlay or (lambda c: None))
        img = g.render_skin(name, parsed[name], layers[name])
        img.save(os.path.join(g.OUT_DIR, f"{name}.png"))
        imgs.append(img)
    return imgs


def main():
    items = items_l1() + items_l2() + items_l3() + items_l4() + items_l5() + items_l6() + items_l7() + items_l8() + items_l9() + items_l10() + items_l11() + items_l12()
    blocks = BLOCKS_L1 + BLOCKS_L2 + BLOCKS_L3 + BLOCKS_L6 + BLOCKS_L7 + BLOCKS_L9 + block_defs_l10() + BLOCKS_L11 + block_defs_l12()
    mobs = mobs_l1() + mobs_l2() + mobs_l3() + mobs_l5() + mobs_l6() + mobs_l7() + mobs_l8()
    write_data(items, blocks, mobs, SHAPED_L1 + SHAPED_L2 + SHAPED_L3 + SHAPED_L4 + SHAPED_L5 + SHAPED_L7 + SHAPED_L8 + SHAPED_L9 + SHAPED_L10 + SHAPED_L11 + SHAPED_L12, SHAPELESS_L1 + SHAPELESS_L2 + SHAPELESS_L3 + SHAPELESS_L4 + SHAPELESS_L5 + SHAPELESS_L7 + SHAPELESS_L8 + SHAPELESS_L9 + SHAPELESS_L10 + SHAPELESS_L11 + SHAPELESS_L12, SMELTING_L2 + SMELTING_L3)
    structures_l3()
    skins = write_skins({**skins_l1(), **skins_l2(), **skins_l3(), **skins_l5(), **skins_l6(), **skins_l7(), **skins_l8()})
    pics = []
    for name, _, img, _ in items:
        im = gt.to_image(img)
        im.save(os.path.join(gi.ITEM_DIR, name + ".png"))
        pics.append(im)
    all_blocks = {**blocks_l1(), **blocks_l2(), **blocks_l3(), **blocks_l6(), **blocks_l7(), **blocks_l9(), **blocks_l10(), **blocks_l11(), **blocks_l12()}
    for name, img in all_blocks.items():
        im = gt.to_image(img)
        im.save(os.path.join(gi.BLOCK_DIR, name + ".png"))
        pics.append(im)
    names = [i[0] for i in items] + list(all_blocks)
    print(f"{len(items)} items, {len(blocks)} blocks, {len(mobs)} mobs, {len(skins)} skins")
    print("--only " + ",".join(names))
    if len(sys.argv) > 1:
        gt.contact_sheet(pics, scale=6, gap=3, cols=12).save(sys.argv[1])
        sheet = Image.new("RGBA", (len(skins) * 70, 134), (24, 24, 28, 255))
        for k, s in enumerate(skins):
            sheet.paste(s, (k * 70 + 3, 3))
        sheet.resize((sheet.width * 3, sheet.height * 3), Image.NEAREST).save(sys.argv[1].replace(".png", "_skins.png"))


if __name__ == "__main__":
    main()
