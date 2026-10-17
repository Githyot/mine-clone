"""Apotheosis (adventure module ideas): gems and sockets, the salvaging and gem cutting tables,
salvage materials and world tier sigils. (Affixes and reforging are the game's own, in
affixes.rs.) The logic is in crates/game/src/mods/apotheosis.rs and crates/voxel/src/gems.rs."""

from .lib import B, It, R, S, tex

# The two tables. Their slots live in a small container at the block (like a hopper's).
B("salvaging_table", "Mesa de Desmonte", hardness="2.5", tool="Some(Pickaxe)",
  textures=tex(top="salvaging_table_top", side="salvaging_table_side", bottom="smooth_stone"))
B("gem_cutting_table", "Mesa de Lapidação", hardness="2.5", tool="Some(Axe)",
  textures=tex(top="gem_cutting_table_top", side="gem_cutting_table_side", bottom="oak_planks"))

# Gems: five kinds in three cuts. Three of a cut and a pinch of gem dust make the next cut.
GEMS = [("vital", "Vital"), ("bastion", "Bastião"), ("fury", "da Fúria"), ("swift", "Ligeira"), ("shadow", "Sombria")]
CUTS = [("rough", "Bruta"), ("cut", "Lapidada"), ("flawless", "Perfeita")]
for g, gn in GEMS:
    for c, cn in CUTS:
        It(f"gem_{g}_{c}", f"Gema {gn} {cn}")
It("gem_dust", "Pó de Gema")

# Salvage materials, one per loot rarity; five of one make one of the next.
MATERIALS = [("common", "Sucata Comum"), ("uncommon", "Fragmento Incomum"), ("rare", "Cristal Raro"),
             ("epic", "Essência Épica"), ("mythic", "Centelha Mítica")]
for m, mn in MATERIALS:
    It(f"salvage_{m}", mn)
for (lo, _), (hi, _) in zip(MATERIALS, MATERIALS[1:]):
    S([f"salvage_{lo}"] * 5, f"salvage_{hi}")

# Sigils: world tiers, an extra socket, taking gems back out.
It("sigil_frontier", "Sigilo da Fronteira", max_stack="Some(16)")
It("sigil_ascent", "Sigilo da Ascensão", max_stack="Some(16)")
It("sigil_summit", "Sigilo do Cume", max_stack="Some(16)")
It("sigil_pinnacle", "Sigilo do Pináculo", max_stack="Some(16)")
It("sigil_socketing", "Sigilo do Engaste", max_stack="Some(16)")
It("sigil_withdrawal", "Sigilo da Extração", max_stack="Some(16)")

R(["III", " T ", "SSS"], {"I": "iron_ingot", "T": "crafting_table", "S": "smooth_stone"}, "salvaging_table")
R([" F ", "ITI", "PPP"], {"F": "flint", "I": "iron_ingot", "T": "crafting_table", "P": "#planks"}, "gem_cutting_table")
S(["amethyst"], "gem_dust", 2)
for sigil, mat, mid, core in (("sigil_frontier", "salvage_uncommon", "gold_ingot", "emerald"),
                              ("sigil_ascent", "salvage_rare", "diamond", "ender_pearl"),
                              ("sigil_summit", "salvage_epic", "diamond", "blaze_rod"),
                              ("sigil_pinnacle", "salvage_mythic", "diamond_block", "gem_dust")):
    R(["MDM", "DCD", "MDM"], {"M": mat, "D": mid, "C": core}, sigil)
R([" G ", "GDG", " G "], {"G": "gem_dust", "D": "diamond"}, "sigil_socketing")
R([" G ", "GEG", " G "], {"G": "gem_dust", "E": "ender_pearl"}, "sigil_withdrawal")
