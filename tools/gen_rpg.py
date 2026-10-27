#!/usr/bin/env python3
"""The Terraria side's stats content (crates/game/src/rpg.rs): its items and recipes in
packs/vanilla/rpg/, and their original 16x16 icons (accessories, healing and mana potions,
summon staffs, whips, minion pictures, slot consumables). Everything is drawn here from
scratch in the same style as gen_etapa5_textures.py; tools/tex64/build.py grows the icons
to 64x64 afterwards.

The buff potions themselves are PotionDefs in effects.rs: tools/gen_fase6.py writes their
items, and this script draws their bottles.

Usage:
    python3 tools/gen_rpg.py [sheet.png]
    python3 tools/tex64/build.py
"""

import os
import re
import sys
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gen_etapa5_textures as e5  # noqa: E402
import gen_fase6_art as f6  # noqa: E402
import gen_items as gi  # noqa: E402
import gen_textures as gt  # noqa: E402
from gen_items import outline, pmap, pal  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
PACK = ROOT / "packs" / "vanilla"
OUT = PACK / "rpg"

from_map = e5.from_map


def icon(rows, p):
    return outline(from_map(rows, pmap(p)))


# --------------------------------------------------------------------------
# Shapes (16x16 maps; '1' darkest .. '5' lightest)
# --------------------------------------------------------------------------

EMBLEM = e5.ACC["warrior_emblem"][0]
EMBLEM_ARROW = [
    "................",
    "....11111111....",
    "...1222222221...",
    "...1233333321...",
    "...1233335521...",
    "...1233355321...",
    "...1253553321...",
    "...1235533321...",
    "...1255333321...",
    "....12333321....",
    "....12333321....",
    ".....123321.....",
    "......1221......",
    ".......11.......",
]
EMBLEM_BELL = [
    "................",
    "....11111111....",
    "...1222222221...",
    "...1233553321...",
    "...1235555321...",
    "...1235555321...",
    "...1355555531...",
    "...1233553321...",
    "...1233333321...",
    "....12333321....",
    "....12333321....",
    ".....123321.....",
    "......1221......",
    ".......11.......",
]
EMBLEM_STAR = [
    "................",
    "....11111111....",
    "...1222222221...",
    "...1233553321...",
    "...1233553321...",
    "...1555555551...",
    "...1235555321...",
    "...1235335321...",
    "...1253333521...",
    "....12333321....",
    "....12333321....",
    ".....123321.....",
    "......1221......",
    ".......11.......",
]
GLOVE = [
    "................",
    "....1.1.1.......",
    "...121212121....",
    "...12121212.1...",
    "...121212121....",
    "...1222222221...",
    "...12222222221..",
    "...122222222321.",
    "...12222222231..",
    "...1222222221...",
    "....12222221....",
    "....14444441....",
    "....15555551....",
    ".....111111.....",
]
STONE = [
    "................",
    "................",
    ".....1111.......",
    "....1233211.....",
    "...123455321....",
    "..1234555433211.",
    "..1234544333321.",
    "..1233443333221.",
    "..12333333322211",
    "...1233333222221",
    "...12223322221..",
    "....1122222111..",
    "......11111.....",
]
GLAND = [
    "................",
    "......11........",
    ".....1221.......",
    "......121.......",
    ".....12221......",
    "....1233321.....",
    "...123455321....",
    "..12345553321...",
    "..12345533321...",
    "..12334333321...",
    "..12333333221...",
    "...122332221....",
    "....1122211.....",
    "......111.......",
]
FANG_NECKLACE = [
    "................",
    "..1..........1..",
    "...1........1...",
    "....1......1....",
    ".....1....1.....",
    "......1..1......",
    "...4.4.44.4.4...",
    "...5.5.55.5.5...",
    "...5.5.55.5.5...",
    "...5.5.5..5.5...",
    ".......5........",
]
QUIVER = [
    "..........3.3...",
    ".........3.3.3..",
    "..........5.5...",
    "..........5.5...",
    ".........11111..",
    "........122221..",
    ".......1223321..",
    "......1223321...",
    ".....1223321....",
    "....1223321.....",
    "...1244321......",
    "..1223321.......",
    "..122321........",
    "..11111.........",
]
LENS = [
    "................",
    ".....111111.....",
    "....12222221....",
    "...1235555321...",
    "...1254444521...",
    "...1254334521...",
    "...1254334521...",
    "...1254444521...",
    "...1235555321...",
    "....12222221....",
    ".....111111.....",
    ".......11.......",
    "......1221......",
    ".....122221.....",
]
FLOWER = [
    "................",
    ".....11..11.....",
    "....1441144l....",
    "....14544541....",
    "..11.144441.11..",
    ".14411455411441.",
    ".14554555545541.",
    "..1144555541411.",
    "....14444441....",
    "....14411441....",
    ".....11221.1....",
    "........2.......",
    ".......32.......",
    "........3.......",
]
CUFFS = [
    "................",
    "................",
    "..1111....1111..",
    ".122221..122221.",
    "12333321123333211",
    "12355321123553211",
    "12333321123333211",
    "12333321123333211",
    "12355321123553211",
    "12333321123333211",
    ".122221..122221.",
    "..1111....1111..",
]
BELL = [
    "................",
    ".......11.......",
    "......1331......",
    ".......11.......",
    "......1221......",
    ".....123321.....",
    "....12345321....",
    "....12345321....",
    "...1234553321...",
    "...1234553321...",
    "..123344433321..",
    "..122222222221..",
    "...1111441111...",
    "......1441......",
    ".......11.......",
]
BUCKLER = [
    "................",
    ".....111111.....",
    "....12222221....",
    "...1233333321...",
    "..123333333321..",
    "..123335533321..",
    "..123354453321..",
    "..123354453321..",
    "..123335533321..",
    "..123333333321..",
    "...1233333321...",
    "....12222221....",
    ".....111111.....",
]
VEIL = [
    "................",
    "...1111111111...",
    "..122222222221..",
    "..123353333321..",
    "..123333335321..",
    ".1233533333321..",
    ".12333333533321.",
    ".12335333333321.",
    ".12333335333321.",
    "..1233333335321.",
    "..1253333333321.",
    "..12333353332211",
    "...122222222211.",
    "....1111111111..",
]
TURTLE = [
    "................",
    "................",
    ".....111111.....",
    "....12434421....",
    "...1243443421...",
    "..124434434421..",
    "..123333333321..",
    "..124434434421..",
    "..124434434421..",
    "...1233333321...",
    "....12434421....",
    ".....111111.....",
]
SCARF = [
    "................",
    "...11111111.....",
    "..1233333321....",
    ".12322222232....",
    ".1231....1321...",
    ".1231....1321...",
    "..12311113221...",
    "...12333332221..",
    "......1231221...",
    "......1231121...",
    "......14411441..",
    "......15511551..",
    "......1111.11...",
]
MIRROR = [
    "................",
    ".....111111.....",
    "....12222221....",
    "...1255544321...",
    "...1254443321...",
    "...1244433321...",
    "...1244333321...",
    "...1243333321...",
    "....12333321....",
    ".....122221.....",
    "......1221......",
    "......1221......",
    ".....122221.....",
    ".....111111.....",
]
CHARM = e5.ACC["obsidian_charm"][0]
BANDAGE = [
    "................",
    "................",
    ".....111111.....",
    "....12333321....",
    "...1233443321...",
    "...1234554321...",
    "...1234554321...",
    "...1233443321...",
    "...12333333211..",
    "....122222212221",
    ".....11111.12221",
    "............1111",
]
POLISH = [
    "................",
    "......1111......",
    ".....155551.....",
    "......1221......",
    "....11111111....",
    "...1233333321...",
    "...1244444421...",
    "...1255555521...",
    "...1244444421...",
    "...1233333321...",
    "...1233333321...",
    "...1222222221...",
    "....11111111....",
]
POUCH = [
    "................",
    "......1..1......",
    ".....1.11.1.....",
    "......1221......",
    ".....122221.....",
    "....12333321....",
    "...1233443321...",
    "..123344443321..",
    "..123334433321..",
    "..123333333321..",
    "..122333333221..",
    "...1222222221...",
    "....11111111....",
]
GLASSES = [
    "................",
    "................",
    "................",
    "................",
    ".1............1.",
    ".11111....11111.",
    "1233321111233321",
    "1245421..1245421",
    "1244421..1244421",
    ".12221....12221.",
    "..111......111..",
]
CLOCK = [
    "................",
    "......1111......",
    ".......11.......",
    ".....111111.....",
    "....12222221....",
    "...1255555521...",
    "..125555155521..",
    "..125555155521..",
    "..125555111521..",
    "..125555555521..",
    "...1255555521...",
    "....12222221....",
    ".....111111.....",
]
NAZAR = [
    "................",
    ".....111111.....",
    "....12222221....",
    "...1225555221...",
    "..122544445221..",
    "..125443344521..",
    "..125431134521..",
    "..125431134521..",
    "..125443344521..",
    "..122544445221..",
    "...1225555221...",
    "....12222221....",
    ".....111111.....",
]
HORN = [
    "................",
    "................",
    "...........111..",
    "..........12221.",
    ".........123321.",
    "........1234421.",
    "..111..12344421.",
    ".12221123444521.",
    ".1233333345521..",
    ".122222222221...",
    "..11111111111...",
]
MAP = [
    "................",
    "..1111111111....",
    ".12555555555211.",
    ".15553555355511.",
    ".15535355535511.",
    ".15555353555511.",
    ".15555535555511.",
    ".15335555545511.",
    ".15553555544511.",
    ".15555355555511.",
    ".12555555555211.",
    "..1111111111....",
]
KIT = [
    "................",
    "......1111......",
    ".....1....1.....",
    "..111111111111..",
    ".12222222222221.",
    ".12222244222221.",
    ".12222244222221.",
    ".12224444442221.",
    ".12224444442221.",
    ".12222244222221.",
    ".12222244222221.",
    ".12222222222221.",
    "..111111111111..",
]
ANKH = [
    "................",
    "......1111......",
    ".....155551.....",
    "....15511551....",
    "....151..151....",
    "....151..151....",
    ".....151151.....",
    "..1111155111111.",
    ".15555555555551.",
    "..1111155111111.",
    "......1551......",
    "......1551......",
    "......1551......",
    "......1551......",
    ".......11.......",
]
ANKH_SHIELD = [
    "................",
    "...1111111111...",
    "..122222222221..",
    "..123335533321..",
    "..123353353321..",
    "..123335533321..",
    "..125555555521..",
    "..123335533321..",
    "..123335533321..",
    "...1233553321...",
    "...1233333321...",
    "....12333321....",
    ".....123321.....",
    "......1221......",
    ".......11.......",
]
FROG = [
    "................",
    "................",
    "....11....11....",
    "...1451..1451...",
    "...1411111141...",
    "..12333333321...",
    "..123333333321..",
    "..122222222221..",
    "...1222222221...",
    "..11222222211...",
    ".12212222122211.",
    ".11.11111111.11.",
]
FINS = [
    "................",
    "................",
    "..1..1.....1..1.",
    "..11.11....11.11",
    "..121121...12112",
    "..1222221..12222",
    "..12332221.12332",
    "..12333221.12333",
    "...1233221..1233",
    "....12221....122",
    ".....111......11",
]
MASK = [
    "................",
    "................",
    "....11111111....",
    "...1222222221...",
    "..122222222221..",
    "..124444444421..",
    "..145555555541..",
    "..145533335541..",
    "..124444444421..",
    "...1222222221...",
    "....111..111....",
    ".......11.......",
    "......1331......",
    ".......11.......",
]
MAGNET = [
    "................",
    "................",
    "...111....111...",
    "..15551..15551..",
    "..15551..15551..",
    "..12221..12221..",
    "..12221..12221..",
    "..122211112221..",
    "...1222222221...",
    "...1223333221...",
    "....12333321....",
    ".....111111.....",
]
COIN = [
    "................",
    "................",
    ".....111111.....",
    "....12333321....",
    "...1234554321...",
    "..123451154321..",
    "..123451154321..",
    "..123454454321..",
    "..123455554321..",
    "...1234444321...",
    "....12333321....",
    ".....111111.....",
]
SKYSTONE = [
    "................",
    ".......11.......",
    "......1551......",
    ".....154451.....",
    "....15444431....",
    "...1544333331...",
    "..154433333321..",
    "..143333332221..",
    "...1333322221...",
    "....12222221....",
    ".....122221.....",
    "......1221......",
    ".......11.......",
]
RADAR = [
    "................",
    "..........5.....",
    ".........5.5....",
    "..111111..5.....",
    ".1233332111.....",
    ".12344433321....",
    "..1234443321....",
    "...12333321.....",
    "....122221......",
    "......11........",
    ".....1221.......",
    "....122221......",
    "....111111......",
]
TALLY = [
    "................",
    "..111111111111..",
    ".12222222222221.",
    ".12555555555521.",
    ".12515151515521.",
    ".12515151515521.",
    ".12515151515521.",
    ".12555511555521.",
    ".12555555555521.",
    ".12222222222221.",
    "..111111111111..",
]
ORE_FINDER = [
    "...........5....",
    "..........5.....",
    ".........1......",
    "...111111111....",
    "..12222222221...",
    "..12555555521...",
    "..12544445521...",
    "..12545545521...",
    "..12555555521...",
    "..12222222221...",
    "..12233233221...",
    "..12222222221...",
    "...111111111....",
]
VANE = [
    "................",
    "...1.......11...",
    "..141.....1441..",
    ".1444111111441..",
    "..141.....1441..",
    "...1...1...11...",
    ".......1........",
    ".....11111......",
    "......121.......",
    ".......2........",
    ".......2........",
    ".......2........",
    "......222.......",
    ".....22222......",
]
BAG = [
    "................",
    ".....11111......",
    "....1....1......",
    "...1111111111...",
    "..122223322221..",
    "..123333333321..",
    "..123344443321..",
    "..123345543321..",
    "..123344443321..",
    "..123333333321..",
    "..122222222221..",
    "...1111111111...",
]
HEART = [
    "................",
    "................",
    "...111...111....",
    "..12221.12221...",
    ".1234452344321..",
    ".1234555434321..",
    ".1233454433321..",
    ".1233333333321..",
    "..12333333321...",
    "...123333321....",
    "....1233321.....",
    ".....12321......",
    "......121.......",
    ".......1........",
]
SOCKET = [
    "................",
    ".....111111.....",
    "....12222221....",
    "...1223553221...",
    "..122355553221..",
    "..123555555321..",
    "..125554455521..",
    "..123555555321..",
    "..122355553221..",
    "...1223553221...",
    "....12222221....",
    ".....111111.....",
]
FRUIT = [
    "................",
    ".........66.....",
    "........66......",
    ".......1........",
    ".....111111.....",
    "....12333321....",
    "...1234553321...",
    "...1234533321...",
    "...1233333321...",
    "...1233333221...",
    "....12332221....",
    ".....111111.....",
]
FLASK_S = [
    "................",
    "................",
    "................",
    "......1111......",
    "......1661......",
    "......1661......",
    ".....155331.....",
    "....15533331....",
    "....14333331....",
    "....14322221....",
    ".....122221.....",
    "......1111......",
]
FLASK_M = [
    "................",
    "......1111......",
    "......1661......",
    "......1661......",
    ".....155331.....",
    "....15533331....",
    "...1543333331...",
    "...1433333331...",
    "...1433332221...",
    "...1433222221...",
    "....12222221....",
    ".....111111.....",
]
FLASK_L = [
    "......1111......",
    "......1661......",
    "......1661......",
    ".....155331.....",
    "....15533331....",
    "...154333333l...",
    "..15433333333l..",
    "..14333333333l..",
    "..14333333322l..",
    "..14333332222l..",
    "..14332222222l..",
    "...122222222l...",
    "....11111111....",
]
STEAM_BOOTS = [
    "..5..5..........",
    ".5..5...........",
    "..5..5..........",
] + e5.ACC["running_boots"][0][3:]
WISP = [
    "................",
    ".......44.......",
    "......4554......",
    ".....455554.....",
    "....45555554....",
    "...4555555554...",
    "...4551551554...",
    "...4551551554...",
    "...4555555554...",
    "....45533554....",
    ".....455554.....",
    "......4..4......",
    ".....4....4.....",
]
WASP = [
    "................",
    "....55....55....",
    "...5445..5445...",
    "...54445544445..",
    "....544554445...",
    "......1111......",
    ".....133331.....",
    "....13111131....",
    "....13333331....",
    "....11111111....",
    ".....133331.....",
    "......1111......",
    ".......11.......",
    ".......1........",
]
IMP = [
    "................",
    "..1..........1..",
    "..21........12..",
    "...21111111121..",
    "...1233333321...",
    "..123553355321..",
    "..123333333321..",
    "..123311113321..",
    "...1233333321...",
    "..1.12333321.1..",
    ".12..122221..21.",
    ".1...12..21...1.",
    ".....1....1.....",
]
DRAGON = [
    "................",
    "...1.......1....",
    "...21.....12....",
    "...1211111121...",
    "..122333333221..",
    ".12335333353321.",
    ".12333333333321.",
    ".123333333333321",
    "..12344444443221",
    "...123333333331.",
    "...12333111111..",
    "....1221........",
    ".....11.........",
]


def recolor(rows, mapping):
    return [r.translate(str.maketrans(mapping)) for r in rows]


# --------------------------------------------------------------------------
# Palettes
# --------------------------------------------------------------------------

P = {
    "ranger": pal("#1e3a10", "#3c6a1e", "#5a9a30", "#8ac850", "#e0f0a0"),
    "summoner": pal("#123a3a", "#1e6a6a", "#30a0a0", "#60d0c8", "#e0fff8"),
    "avenger": pal("#3a0a10", "#7a1a24", "#b02a38", "#e06a50", "#ffe0a0"),
    "champion": pal("#4a3008", "#8a6010", "#c49418", "#f0d040", "#fffbd8"),
    "leather": pal("#3a200c", "#6a3e1c", "#9c6432", "#c48a50", "#e8c8a0"),
    "gauntlet": pal("#2a2e38", "#4e5462", "#7a8292", "#b8bec8", "#eef2f8"),
    "ember": pal("#3a0804", "#8a1a08", "#d04a10", "#ff9a30", "#fff0a0"),
    "rime": pal("#0a2a4a", "#1e5a8a", "#4a9ad0", "#9ad8ff", "#ffffff"),
    "venom": pal("#1e0a2a", "#4a1a5a", "#7a2a8a", "#b060c0", "#e8c0f0"),
    "bone": pal("#3a3020", "#6a6048", "#9a9070", "#d0c8a8", "#f8f4e8"),
    "quiver": pal("#2a1608", "#5a3418", "#8a5a2a", "#b8844a", "#e8c88a"),
    "lens": pal("#1a1a22", "#3a3a48", "#6a7a9a", "#a8c8e8", "#ffffff"),
    "mana_flower": pal("#0c1a5a", "#2a4ab0", "#4a7aff", "#9ac0ff", "#e8f0ff", "#1e7a2a"),
    "arcane": pal("#1a0c3a", "#3a1c7a", "#6a3ab8", "#a070f0", "#f0dcff"),
    "sage": pal("#0a2a3a", "#1e4a6a", "#3a8ab0", "#7ad0e8", "#e8ffff"),
    "totem": pal("#2a1608", "#6a3a18", "#a85a2a", "#40a060", "#e0f0a0"),
    "spirit_bell": pal("#3a3008", "#7a6418", "#c4a030", "#f0d870", "#fffbd8"),
    "iron": pal("#3a3a3a", "#6a6a6a", "#9a9a9a", "#c8c8c8", "#ffffff"),
    "holy": pal("#5a4a10", "#a08a20", "#e0c040", "#fff0a0", "#ffffff"),
    "veil": pal("#0a0c2a", "#1e2a5a", "#2a3a8a", "#4a5ab0", "#fce47a"),
    "thorn": pal("#142a0a", "#2a4a14", "#4a7a24", "#7aa840", "#c8e080"),
    "turtle": pal("#0e2a10", "#1e4a1e", "#3a7a2a", "#6aa840", "#c8d870"),
    "scarf": pal("#3a0a10", "#7a1a24", "#b02a38", "#e8e0d0", "#ffffff"),
    "mirror": pal("#2a2a38", "#6a6a80", "#a8b8d0", "#d8e8ff", "#ffffff"),
    "panic": pal("#3a0a0a", "#8a1a1a", "#d03030", "#ff8080", "#ffe0e0"),
    "honey": pal("#4a2a04", "#8a5a0a", "#d09018", "#f8c840", "#fff0a0"),
    "antidote": pal("#0a3a1a", "#1e6a2a", "#30a040", "#80e080", "#e0ffe0"),
    "bandage": pal("#6a6050", "#a89a80", "#d8d0c0", "#f0ece0", "#c03030"),
    "polish": pal("#2a2e38", "#5a6070", "#8a94a8", "#c8d0e0", "#ffffff"),
    "vitamin": pal("#4a2a08", "#8a5a18", "#c08030", "#f0b040", "#ffe8a0"),
    "dark_lenses": pal("#0a0a0a", "#2a2a2a", "#4a4a4a", "#1a1a2a", "#5a5a7a"),
    "fast_clock": pal("#2a1a08", "#6a4a18", "#c49418", "#fce47a", "#fffbd8"),
    "nazar": pal("#0a1a5a", "#1e3ab0", "#ffffff", "#5aa8ff", "#c8e8ff"),
    "brass": pal("#4a3008", "#8a6010", "#c49418", "#f0d040", "#fffbd8"),
    "map": pal("#4a3018", "#8a6a3a", "#c03030", "#6a4a2a", "#f0e4c0"),
    "kit": pal("#3a0a0a", "#c8c8c8", "#e8e8e8", "#c03030", "#ffffff"),
    "ankh": pal("#4a3008", "#8a6010", "#c49418", "#f0d040", "#fffbd8"),
    "ankh_shield": pal("#1a1a2a", "#3a3a5a", "#5a5a8a", "#8a8ab0", "#f0d040"),
    "frog": pal("#0e2a10", "#2a6a1e", "#4aa030", "#8ad860", "#101010"),
    "fins": pal("#0a1e3a", "#1e4a7a", "#3a7ab0", "#7ab0e0", "#e0f0ff"),
    "mask": pal("#1a1a1a", "#3a3a3a", "#5a5a5a", "#6ac0e8", "#c8f0ff"),
    "magma": pal("#1a0804", "#4a1a0a", "#8a2a10", "#d04a10", "#ff9a30"),
    "steam": pal("#2a2e38", "#5a3418", "#8a5a2a", "#b8bec8", "#e0e0e0"),
    "shadow": pal("#08040e", "#1a1028", "#2a1c40", "#4a3a6a", "#8a7aba"),
    "magnet": pal("#3a0a0a", "#a01a1a", "#d03030", "#ff7070", "#e8e8e8"),
    "coin": pal("#4a3008", "#8a6010", "#c49418", "#f0d040", "#fffbd8"),
    "sky": pal("#0a1a3a", "#1e3a7a", "#4a7ad0", "#9ac8ff", "#ffffff"),
    "watch": pal("#4a3008", "#8a6010", "#c49418", "#f0e8c8", "#fffbf0"),
    "radar": pal("#1a1a22", "#3a3a48", "#7a8292", "#b8bec8", "#60ff80"),
    "tally": pal("#2a1608", "#6a3e1c", "#9c6432", "#202020", "#e8e0c8"),
    "finder": pal("#1a1a22", "#3a3a48", "#7a8292", "#e0b020", "#60e0ff"),
    "vane": pal("#2a1a08", "#4a3018", "#8a6010", "#c49418", "#f0d040"),
    "bag": pal("#2a1608", "#5a3418", "#8a5a2a", "#c03030", "#f0d040"),
    "heart": pal("#3a0408", "#8a0a18", "#c41a2a", "#ff5a6a", "#ffd0d8"),
    "socket": pal("#0a1a3a", "#1e3a7a", "#4a7ad0", "#fce47a", "#ffffff"),
    "fruit": pal("#3a1a04", "#8a3a08", "#d06a10", "#f0a040", "#fff0c0", "#3a8a2a"),
    "heal": pal("#3a0408", "#8a0a18", "#c41a2a", "#ff5a6a", "#ffd0d8", "#c8b090"),
    "mana": pal("#04083a", "#0a1e8a", "#1a3ac4", "#5a8aff", "#d0e0ff", "#c8b090"),
    "restore": pal("#2a043a", "#5a0a8a", "#8a1ac4", "#c05aff", "#f0d0ff", "#c8b090"),
    "wisp": pal("#0a1a3a", "#1e3a7a", "#4a7ad0", "#9ad8ff", "#ffffff"),
    "wasp": pal("#1a1408", "#3a2a10", "#f0c020", "#c8e0ff", "#ffffff"),
    "imp": pal("#2a0804", "#6a1408", "#c43a10", "#ff9a30", "#fff0a0"),
    "dragon": pal("#062a2a", "#0a5a5a", "#18a0a0", "#60e0d8", "#fffbd8"),
}


# --------------------------------------------------------------------------
# Items: (name, display name, picture, extra fields)
# --------------------------------------------------------------------------

def acc(name, display, rows, p):
    return (name, display, lambda: icon(rows, P[p]), "max_stack: Some(1)")


ITEMS = [
    acc("ranger_emblem", "Emblema do Arqueiro", EMBLEM_ARROW, "ranger"),
    acc("summoner_emblem", "Emblema do Invocador", EMBLEM_BELL, "summoner"),
    acc("avenger_emblem", "Emblema do Vingador", EMBLEM, "avenger"),
    acc("champion_emblem", "Emblema do Campeão", EMBLEM_STAR, "champion"),
    acc("claw_glove", "Luva de Garras", GLOVE, "leather"),
    acc("giant_gauntlet", "Manopla do Gigante", GLOVE, "gauntlet"),
    acc("ember_stone", "Pedra de Brasa", STONE, "ember"),
    acc("rime_stone", "Pedra de Geada", STONE, "rime"),
    acc("venom_gland", "Glândula de Peçonha", GLAND, "venom"),
    acc("fang_necklace", "Colar de Presas", FANG_NECKLACE, "bone"),
    acc("hunter_quiver", "Aljava do Caçador", QUIVER, "quiver"),
    acc("marksman_lens", "Lente do Atirador", LENS, "lens"),
    ("mana_blossom", "Flor de Mana", lambda: outline(from_map(recolor(FLOWER, {"l": "1"}), {**pmap(P["mana_flower"][:5]), "2": P["mana_flower"][5], "3": P["mana_flower"][5] * 0.7})), "max_stack: Some(1)"),
    acc("arcane_cuffs", "Braceletes Arcanos", CUFFS, "arcane"),
    acc("sage_pendant", "Pingente do Sábio", CHARM, "sage"),
    acc("totem_necklace", "Colar Totêmico", e5.ACC["star_amulet"][0], "totem"),
    acc("spirit_bell", "Sino dos Espíritos", BELL, "spirit_bell"),
    acc("iron_buckler", "Broquel de Ferro", BUCKLER, "iron"),
    acc("holy_pendant", "Pingente Sagrado", CHARM, "holy"),
    acc("star_veil", "Véu Estrelado", VEIL, "veil"),
    acc("thorned_band", "Bracelete de Espinhos", e5.ACC["regen_band"][0], "thorn"),
    acc("turtle_shell", "Casco Antigo", TURTLE, "turtle"),
    acc("warding_scarf", "Cachecol Protetor", SCARF, "scarf"),
    acc("mirror_charm", "Amuleto do Espelho", MIRROR, "mirror"),
    acc("panic_charm", "Amuleto do Pânico", HEART, "panic"),
    acc("honey_locket", "Medalhão de Mel", CHARM, "honey"),
    acc("antidote_charm", "Amuleto Antídoto", CHARM, "antidote"),
    acc("bandage_wrap", "Atadura", BANDAGE, "bandage"),
    acc("steel_polish", "Polidor de Aço", POLISH, "polish"),
    acc("vitamin_pouch", "Bolsa de Vitaminas", POUCH, "vitamin"),
    acc("dark_lenses", "Lentes Escuras", GLASSES, "dark_lenses"),
    acc("pocket_clock", "Relógio Ligeiro", CLOCK, "fast_clock"),
    acc("warding_eye", "Olho Protetor", NAZAR, "nazar"),
    acc("brass_horn", "Corneta de Latão", HORN, "brass"),
    acc("old_map", "Mapa Velho", MAP, "map"),
    acc("medicine_kit", "Kit de Remédios", KIT, "kit"),
    acc("ankh_charm", "Amuleto Ankh", ANKH, "ankh"),
    acc("ankh_shield", "Escudo Ankh", ANKH_SHIELD, "ankh_shield"),
    acc("frog_charm", "Amuleto do Sapo", FROG, "frog"),
    acc("swim_fins", "Nadadeiras", FINS, "fins"),
    acc("diving_mask", "Máscara de Mergulho", MASK, "mask"),
    acc("magma_boots", "Botas de Magma", e5.ACC["running_boots"][0], "magma"),
    acc("steam_boots", "Botas a Vapor", STEAM_BOOTS, "steam"),
    acc("shadow_gear", "Equipamento das Sombras", e5.ACC["wind_cape"][0], "shadow"),
    acc("magnet_ring", "Ímã da Sorte", MAGNET, "magnet"),
    acc("lucky_coin", "Moeda da Sorte", COIN, "coin"),
    acc("sky_stone", "Pedra Celeste", SKYSTONE, "sky"),
    acc("pocket_watch", "Relógio de Bolso", CLOCK, "watch"),
    acc("creature_radar", "Radar de Criaturas", RADAR, "radar"),
    acc("battle_tally", "Contador de Batalha", TALLY, "tally"),
    acc("ore_finder", "Detector de Minério", ORE_FINDER, "finder"),
    acc("weather_vane", "Cata-Vento", VANE, "vane"),
    acc("explorer_kit", "Kit do Explorador", BAG, "bag"),
    # Consumables.
    ("ancestral_heart", "Coração Ancestral", lambda: icon(HEART, P["heart"]), "max_stack: Some(1)"),
    ("celestial_socket", "Encaixe Celestial", lambda: icon(SOCKET, P["socket"]), "max_stack: Some(1)"),
    ("life_fruit", "Fruta da Vida", lambda: outline(from_map(FRUIT, {**pmap(P["fruit"][:5]), "6": P["fruit"][5]})), "max_stack: Some(16)"),
]


def flask(rows, p):
    cols = {**pmap(p[:5]), "6": p[5], "l": p[0]}
    return outline(from_map(rows, cols))


for name, display, rows, p in [
    ("lesser_healing_potion", "Poção de Vida Menor", FLASK_S, "heal"),
    ("healing_potion", "Poção de Vida", FLASK_M, "heal"),
    ("greater_healing_potion", "Poção de Vida Maior", FLASK_L, "heal"),
    ("super_healing_potion", "Superpoção de Vida", FLASK_L, "heal"),
    ("restoration_potion", "Poção de Restauração", FLASK_M, "restore"),
    ("lesser_mana_potion", "Poção de Mana Menor", FLASK_S, "mana"),
    ("mana_potion", "Poção de Mana", FLASK_M, "mana"),
    ("greater_mana_potion", "Poção de Mana Maior", FLASK_L, "mana"),
]:
    pp = P[p]
    if name == "super_healing_potion":
        pp = pal("#3a0430", "#8a0a50", "#e01a5a", "#ff7aa0", "#fff0f4", "#f0d040")
    ITEMS.append((name, display, (lambda rows=rows, pp=pp: flask(rows, pp)), "max_stack: Some(30)"))

STAFF_GEMS = {
    "wisp": pal("#0a1e4a", "#1e4ab0", "#5a9aff", "#a8d8ff", "#ffffff"),
    "wasp": pal("#4a3004", "#a07008", "#f0c020", "#fce47a", "#fffbd8"),
    "imp": e5.METALS["palladium"],
    "blade": e5.METALS["titanium"],
    "dragon": e5.METALS["lumite"],
}
for staff, display, gem, metal in [
    ("wisp_staff", "Cajado do Fogo-Fátuo", "wisp", "golden"),
    ("wasp_staff", "Cajado das Vespas", "wasp", "iron"),
    ("imp_staff", "Cajado do Diabrete", "imp", "copper"),
    ("blade_staff", "Cajado das Lâminas", "blade", "titanium"),
    ("star_dragon_staff", "Cajado do Dragão Estelar", "dragon", "lumite"),
]:
    gi.WAND_GEMS["rpg_" + gem] = STAFF_GEMS[gem]
    gi.MATERIALS.setdefault("titanium", e5.METALS["titanium"])
    gi.MATERIALS.setdefault("lumite", e5.METALS["lumite"])
    ITEMS.append((staff, display, (lambda gem=gem, metal=metal: gi.staff("rpg_" + gem, metal)), "max_stack: Some(1)"))


def whip(handle, lash, tip):
    rows = [
        "................",
        "..........1111..",
        ".........1....1.",
        "........1......1",
        ".......1.......1",
        "......1.......1.",
        ".....1.......1..",
        "....22.......1..",
        "...223........1.",
        "..223..........4",
        ".223............",
        ".33.............",
    ]
    cols = {"1": gt.hexrgb(lash), "2": gt.hexrgb(handle), "3": gt.hexrgb("#2a1a0a"), "4": gt.hexrgb(tip)}
    return outline(from_map(rows, cols))


ITEMS.append(("leather_whip", "Chicote de Couro", lambda: whip("#7a5530", "#9c6432", "#c48a50"), "tool: Some((kind: Sword, tier: 1, speed: 6.0, damage: 4.0, durability: 250))"))
ITEMS.append(("thorn_whip", "Chicote de Espinhos", lambda: whip("#3a5a1a", "#5a8a2a", "#c8e080"), "tool: Some((kind: Sword, tier: 2, speed: 6.0, damage: 6.0, durability: 420))"))

for name, display, rows, p in [
    ("minion_wisp", "Fogo-fátuo", WISP, "wisp"),
    ("minion_wasp", "Vespa", WASP, "wasp"),
    ("minion_imp", "Diabrete", IMP, "imp"),
    ("minion_dragon", "Dragão Estelar", DRAGON, "dragon"),
]:
    ITEMS.append((name, display, (lambda rows=rows, p=p: icon(rows, P[p])), "max_stack: Some(1)"))
ITEMS.append(("minion_blade", "Lâmina Voadora", lambda: gi.tool("titanium", "sword"), "max_stack: Some(1)"))


# --------------------------------------------------------------------------
# Recipes
# --------------------------------------------------------------------------

SHAPED = [
    (["F F", "FEF", "F F"], {"F": "feather", "E": "warrior_emblem"}, "ranger_emblem"),
    (["B B", "BEB", "B B"], {"B": "bone", "E": "warrior_emblem"}, "summoner_emblem"),
    (["WRS", " A ", "   "], {"W": "warrior_emblem", "R": "ranger_emblem", "S": "summoner_emblem", "A": "arcane_emblem"}, "avenger_emblem"),
    (["SAS", "SBS", "SSS"], {"S": "star_fragment", "A": "avenger_emblem", "B": "breaker_core"}, "champion_emblem"),
    (["S S", "LLL", "LLL"], {"S": "shadow_scale", "L": "leather"}, "claw_glove"),
    (["IGI", "ICI", "III"], {"I": "titanium_ingot", "G": "soul_of_might", "C": "claw_glove"}, "giant_gauntlet"),
    ([" M ", "MBM", " M "], {"M": "magma_block", "B": "blaze_powder"}, "ember_stone"),
    ([" I ", "ISI", " I "], {"I": "blue_ice", "S": "sapphire"}, "rime_stone"),
    (["SSS", "SFS", " S "], {"S": "spider_eye", "F": "fermented_spider_eye"}, "venom_gland"),
    (["S S", " S ", "BBB"], {"S": "string", "B": "bone"}, "fang_necklace"),
    (["L A", "L A", "LLL"], {"L": "leather", "A": "arrow"}, "hunter_quiver"),
    (["GGG", "GSG", " I "], {"G": "glass_pane", "S": "spyglass", "I": "iron_ingot"}, "marksman_lens"),
    ([" M ", "MRM", " P "], {"M": "mana_crystal", "R": "mana_ring", "P": "lesser_mana_potion"}, "mana_blossom"),
    (["I I", "MAM", "I I"], {"I": "iron_ingot", "M": "mana_crystal", "A": "amethyst"}, "arcane_cuffs"),
    ([" S ", "SAS", " G "], {"S": "string", "A": "amethyst", "G": "gold_ingot"}, "sage_pendant"),
    ([" S ", "S S", " T "], {"S": "string", "T": "spectral_bone"}, "totem_necklace"),
    ([" I ", "GGG", "G G"], {"I": "iron_ingot", "G": "gold_ingot"}, "spirit_bell"),
    (["III", "IOI", "III"], {"I": "iron_ingot", "O": "oak_planks"}, "iron_buckler"),
    ([" S ", "S S", " C "], {"S": "string", "C": "life_crystal"}, "holy_pendant"),
    (["SSS", "SPS", "SSS"], {"S": "star_fragment", "P": "holy_pendant"}, "star_veil"),
    ([" C ", "CBC", " C "], {"C": "cactus", "B": "regen_band"}, "thorned_band"),
    (["SSS", "SVS", "SSS"], {"S": "turtle_scute", "V": "living_vine"}, "turtle_shell"),
    (["WWW", "WSW", "W W"], {"W": "red_wool", "S": "soul_of_fright"}, "warding_scarf"),
    ([" G ", "GPG", " S "], {"G": "gold_ingot", "P": "glass_pane", "S": "soul_of_sight"}, "mirror_charm"),
    ([" S ", "S S", " H "], {"S": "string", "H": "redstone_block"}, "panic_charm"),
    ([" S ", "S S", " H "], {"S": "string", "H": "honeycomb"}, "honey_locket"),
    ([" S ", "S S", " E "], {"S": "string", "E": "fermented_spider_eye"}, "antidote_charm"),
    (["WWW", "WSW", "WWW"], {"W": "white_wool", "S": "string"}, "bandage_wrap"),
    (["SIS", "SAS", "SSS"], {"I": "iron_nugget", "S": "slime_ball", "A": "iron_ingot"}, "steel_polish"),
    (["L L", "LAL", "LLL"], {"L": "leather", "A": "golden_apple"}, "vitamin_pouch"),
    (["I I", "GIG"], {"I": "iron_ingot", "G": "black_stained_glass"}, "dark_lenses"),
    ([" G ", "GCG", " S "], {"G": "gold_ingot", "C": "clock", "S": "sugar"}, "pocket_clock"),
    ([" L ", "LEL", " L "], {"L": "lapis_lazuli", "E": "ender_eye"}, "warding_eye"),
    (["  C", " C ", "CCC"], {"C": "copper_ingot"}, "brass_horn"),
    (["PPP", "PCP", "PPP"], {"P": "paper", "C": "compass"}, "old_map"),
    (["A B", "   "], {"A": "antidote_charm", "B": "bandage_wrap"}, "medicine_kit"),
    (["MSP", "VDE", "HKC"], {"M": "medicine_kit", "S": "steel_polish", "P": "vitamin_pouch", "V": "dark_lenses", "D": "pocket_clock", "E": "warding_eye", "H": "brass_horn", "K": "old_map", "C": "soul_of_might"}, "ankh_charm"),
    (["A G", " O "], {"A": "ankh_charm", "G": "guardian_shield", "O": "obsidian_charm"}, "ankh_shield"),
    ([" S ", "S S", " F "], {"S": "string", "F": "slime_ball"}, "frog_charm"),
    (["K K", "K K"], {"K": "kelp", }, "swim_fins"),
    (["III", "G G", " T "], {"I": "iron_ingot", "G": "glass", "T": "turtle_helmet"}, "diving_mask"),
    (["M M", "B B"], {"M": "magma_cream", "B": "running_boots"}, "magma_boots"),
    (["I I", "BFB", "P P"], {"I": "iron_ingot", "B": "running_boots", "F": "furnace", "P": "piston"}, "steam_boots"),
    (["C S", " R "], {"C": "wind_cape", "S": "climbing_claws", "R": "mirror_charm"}, "shadow_gear"),
    (["R I", "I I", "III"], {"R": "redstone", "I": "iron_ingot"}, "magnet_ring"),
    (["GGG", "GEG", "GGG"], {"G": "gold_nugget", "E": "emerald"}, "lucky_coin"),
    (["FFF", "FDF", "FFF"], {"F": "star_fragment", "D": "diamond_block"}, "sky_stone"),
    ([" G ", "GRG", " G "], {"G": "gold_ingot", "R": "clock"}, "pocket_watch"),
    (["  R", "III", " I "], {"R": "redstone_torch", "I": "iron_ingot"}, "creature_radar"),
    (["PPP", "PRP", "PPP"], {"P": "oak_planks", "R": "redstone"}, "battle_tally"),
    (["  R", "III", "IGI"], {"R": "redstone_torch", "I": "iron_ingot", "G": "gold_ingot"}, "ore_finder"),
    (["CCC", " S ", " S "], {"C": "copper_ingot", "S": "stick"}, "weather_vane"),
    (["DWV", "LLL"], {"D": "depth_meter", "W": "pocket_watch", "V": "weather_vane", "L": "leather"}, "explorer_kit"),
    # Slot consumables and life fruit.
    (["SBS", "BCB", "SBS"], {"S": "soul_of_might", "B": "breaker_core", "C": "life_crystal"}, "ancestral_heart"),
    (["LSL", "SDS", "LSL"], {"L": "lumite_ingot", "S": "star_fragment", "D": "diamond_block"}, "celestial_socket"),
    # Summon staffs.
    (["  W", " S ", "S  "], {"W": "soul_lantern", "S": "stick"}, "wisp_staff"),
    (["  H", " S ", "S  "], {"H": "honeycomb", "S": "stick"}, "wasp_staff"),
    (["  B", " R ", "R  "], {"B": "blaze_powder", "R": "blaze_rod"}, "imp_staff"),
    (["  T", " S ", "S  "], {"T": "titanium_sword", "S": "soul_of_might"}, "blade_staff"),
    (["  L", " F ", "F  "], {"L": "lumite_ingot", "F": "star_fragment"}, "star_dragon_staff"),
    # Whips.
    (["LLS", "L S", "  S"], {"L": "leather", "S": "string"}, "leather_whip"),
    (["VVS", "V S", "  S"], {"V": "vine", "S": "leather_whip"}, "thorn_whip"),
]

SHAPELESS = [
    (["glass_bottle", "red_mushroom", "sweet_berries"], "lesser_healing_potion", 2),
    (["lesser_healing_potion", "lesser_healing_potion", "glistering_melon_slice"], "healing_potion", 1),
    (["healing_potion", "healing_potion", "soul_of_sight", "glow_berries"], "greater_healing_potion", 2),
    (["greater_healing_potion", "greater_healing_potion", "greater_healing_potion", "greater_healing_potion", "star_fragment"], "super_healing_potion", 4),
    (["healing_potion", "mana_potion", "glowstone_dust"], "restoration_potion", 2),
    (["glass_bottle", "lapis_lazuli", "glowstone_dust"], "lesser_mana_potion", 2),
    (["lesser_mana_potion", "lesser_mana_potion", "amethyst"], "mana_potion", 1),
    (["mana_potion", "mana_potion", "soul_of_sight", "lapis_lazuli"], "greater_mana_potion", 2),
    (["verdite_ingot", "glow_berries", "golden_apple"], "life_fruit", 1),
]


# --------------------------------------------------------------------------
# Writing
# --------------------------------------------------------------------------

def known_items():
    names = set()
    for f in [PACK / "items.ron", PACK / "etapa5" / "items.ron", PACK / "fase6" / "items.ron", PACK / "mods" / "items.ron"]:
        names |= set(re.findall(r'name: "([a-z0-9_]+)"', f.read_text()))
    for f in [PACK / "blocks.ron", PACK / "etapa5" / "blocks.ron", PACK / "fase6" / "blocks.ron", PACK / "mods" / "blocks.ron"]:
        # Blocks with an item of their own.
        names |= set(re.findall(r'name: "([a-z0-9_]+)"', f.read_text()))
    return names


def write_data():
    OUT.mkdir(parents=True, exist_ok=True)
    known = known_items() | {i[0] for i in ITEMS}
    lines = ["// The Terraria side's stats content (rpg.rs). Written by tools/gen_rpg.py.", "(", "    items: ["]
    for name, display, _, extra in ITEMS:
        lines.append(f'        (name: "{name}", display_name: Some("{display}"), {extra}),')
    lines += ["    ],", ")", ""]
    (OUT / "items.ron").write_text("\n".join(lines))
    bad = []
    rl = ["// Recipes of the Terraria side's stats content (rpg.rs). Written by tools/gen_rpg.py.", "(", "    shaped: ["]
    for pattern, key, result in SHAPED:
        used = set("".join(pattern)) - {" "}
        assert used == set(key), (result, used, set(key))
        for v in list(key.values()) + [result]:
            if v not in known:
                bad.append((result, v))
        pat = ", ".join(f'"{r}"' for r in pattern)
        k = ", ".join(f"'{c}': \"{v}\"" for c, v in key.items())
        rl.append(f"        (pattern: [{pat}], key: {{{k}}}, result: \"{result}\"),")
    rl += ["    ],", "    shapeless: ["]
    for ings, result, count in SHAPELESS:
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


BUFF_POTIONS = ["ironskin", "mana_regeneration", "magic_power", "archery", "wrath", "rage", "endurance", "lifeforce", "thorns", "titan", "summoning", "spelunker", "hunter", "shine", "builder", "warmth", "inferno"]


def potion_icons():
    colors, potions = f6.effect_colors()
    out = {}
    for key, effs in potions:
        if key not in BUFF_POTIONS:
            continue
        c = f6.potion_color(effs, colors)
        out[f"potion_{key}"] = outline(f6.bottle(f6.BOTTLE, c), 0.4)
        out[f"splash_potion_{key}"] = outline(f6.bottle(f6.SPLASH, c, pal("#6a6a6a")[0]), 0.4)
        out[f"lingering_potion_{key}"] = outline(f6.bottle(f6.LINGER, c, pal("#c8c8c8")[0]), 0.4)
        out[f"tipped_arrow_{key}"] = f6.tipped_arrow(c)
    assert len(out) == 4 * len(BUFF_POTIONS), "a buff potion is missing from effects.rs"
    return out


def main():
    write_data()
    imgs = []
    pics = {name: draw() for name, _, draw, _ in ITEMS}
    pics.update(potion_icons())
    for name, img in pics.items():
        im = gt.to_image(img)
        im.save(os.path.join(gi.ITEM_DIR, name + ".png"))
        imgs.append(im)
    print(f"{len(ITEMS)} items, {len(SHAPED) + len(SHAPELESS)} recipes, {len(imgs)} icons")
    if len(sys.argv) > 1:
        gt.contact_sheet(imgs, scale=6, gap=3, cols=16).save(sys.argv[1])


if __name__ == "__main__":
    main()
