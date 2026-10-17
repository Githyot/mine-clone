"""Better Combat: two-handed weapons (greatswords, spears, halberds) in four materials. How they
swing (combos, arcs, reach) is in crates/game/src/mods/combat.rs; here are only the items."""

from .lib import R, It, tool

# material, Portuguese name ending, ingredient, tier, speed, sword damage, durability
MATS = [
    ("stone", "de Pedra", "cobblestone", 2, 4.0, 5.0, 131),
    ("iron", "de Ferro", "iron_ingot", 3, 6.0, 6.0, 250),
    ("golden", "de Ouro", "gold_ingot", 1, 12.0, 4.0, 32),
    ("diamond", "de Diamante", "diamond", 4, 8.0, 7.0, 1561),
]
# kind, Portuguese name, extra damage over the sword, durability factor, recipe
KINDS = [
    ("greatsword", "Montante", 3.0, 1.3, [" X ", " X ", "X/X"]),
    ("spear", "Lança", 1.0, 1.1, ["  X", " / ", "/  "]),
    ("halberd", "Alabarda", 3.5, 1.4, [" XX", " /X", "/  "]),
]

for m, mpt, ing, tier, speed, dmg, dur in MATS:
    for k, kpt, extra, durk, pat in KINDS:
        n = f"{m}_{k}"
        It(n, f"{kpt} {mpt}", tool=tool("Sword", tier, speed, dmg + extra, int(dur * durk)))
        R(pat, {"X": ing, "/": "stick"}, n)
