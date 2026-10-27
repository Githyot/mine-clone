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
        drops: [(item: "gel", min: 20, max: 40), (item: "jelly_hook", chance: 0.5), (item: "jelly_staff", chance: 0.5), (item: "royal_gel"), (item: "lesser_healing_potion", min: 3, max: 6)], xp: 90,
        width: 3.2, height: 2.6, spawn: Never, hops: true, scale: 4.5, boss: true,
        parts: {KING_PARTS},
        layers: [(part: 0, inflate: 0.6), (part: 1, inflate: 0.4)],
    ),
    // The Great Eye: hovers, stares, dashes; at half health it opens a mouth of teeth.
    (
        name: "great_eye", display_name: "Grande Olho", kind: Hostile, health: 560, speed: 0.1, attack: 7,
        drops: [(item: "lens", min: 4, max: 8), (item: "watcher_shield"), (item: "emerald", min: 3, max: 6), (item: "life_crystal", chance: 0.3)], xp: 110,
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
        drops: [(item: "beeswax", min: 10, max: 17), (item: "hive_staff", chance: 0.4), (item: "hive_blade", chance: 0.4), (item: "hive_pack"), (item: "honeycomb", min: 10, max: 20)], xp: 140,
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

def write_data(items, blocks, mobs, shaped, shapeless):
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
    rl += ["    ],", ")", ""]
    if bad:
        raise SystemExit(f"unknown items in recipes: {bad}")
    (OUT / "recipes.ron").write_text("\n".join(rl))


def as_int(f):
    def h(c):
        r = f(c)
        return None if r is None else tuple(int(round(float(v))) for v in r[:3])
    return h


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
    items = items_l1()
    blocks = BLOCKS_L1
    mobs = mobs_l1()
    write_data(items, blocks, mobs, SHAPED_L1, SHAPELESS_L1)
    skins = write_skins(skins_l1())
    pics = []
    for name, _, img, _ in items:
        im = gt.to_image(img)
        im.save(os.path.join(gi.ITEM_DIR, name + ".png"))
        pics.append(im)
    for name, img in blocks_l1().items():
        im = gt.to_image(img)
        im.save(os.path.join(gi.BLOCK_DIR, name + ".png"))
        pics.append(im)
    names = [i[0] for i in items] + list(blocks_l1())
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
