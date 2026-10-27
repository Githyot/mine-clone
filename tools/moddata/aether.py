"""The Aether: the sky dimension's ground, woods, ores, clouds, dungeon stones, utility blocks
(altar, freezer, incubator), the portal, and the tools and armour of its four tiers."""

from .lib import B, F, It, R, S, armor, drops, food, tag, tex, tool

# Ground.
B("aether_grass_block", "Grama do Aether", hardness="0.6", tool="Some(Shovel)",
  textures=tex(top="aether_grass_top", side="aether_grass_side", bottom="aether_dirt"), drops=drops("aether_dirt"))
B("aether_dirt", "Terra do Aether", hardness="0.5", tool="Some(Shovel)")
B("holystone", "Pedra Sagrada", hardness="1.5", tool="Some(Pickaxe)", tier="1")
B("mossy_holystone", "Pedra Sagrada Musgosa", hardness="1.5", tool="Some(Pickaxe)", tier="1")
B("holystone_bricks", "Tijolos de Pedra Sagrada", hardness="2.0", tool="Some(Pickaxe)", tier="1")
B("holystone_brick_slab", "Laje de Tijolos de Pedra Sagrada", hardness="2.0", shape="Slab", tool="Some(Pickaxe)", tier="1",
  textures=tex(all="holystone_bricks"))
B("holystone_brick_stairs", "Escada de Tijolos de Pedra Sagrada", hardness="2.0", shape="Stairs", tool="Some(Pickaxe)",
  tier="1", textures=tex(all="holystone_bricks"))
B("quicksoil", "Areia Movediça Celeste", hardness="0.5", tool="Some(Shovel)")
B("quicksoil_glass", "Vidro de Areia Celeste", hardness="0.3", transparency="Translucent", light="6", drops="Some([])")
B("icestone", "Pedra de Gelo", hardness="3.0", tool="Some(Pickaxe)", tier="1", light="4")
B("ambrosium_ore", "Minério de Ambrósio", hardness="3.0", tool="Some(Pickaxe)", tier="1",
  drops=drops("ambrosium_shard"), xp="(0, 2)")
B("zanite_ore", "Minério de Zanita", hardness="3.0", tool="Some(Pickaxe)", tier="2", drops=drops("zanite_gemstone"),
  xp="(1, 3)")
B("gravitite_ore", "Minério de Gravitita", hardness="5.0", tool="Some(Pickaxe)", tier="3")
B("enchanted_gravitite", "Gravitita Encantada", hardness="5.0", tool="Some(Pickaxe)", tier="3", light="3")
B("zanite_block", "Bloco de Zanita", hardness="5.0", tool="Some(Pickaxe)", tier="2")

# Clouds: soft to land on (no fall damage), the blue one throws you up, the golden one lets you
# sink slowly through it.
B("cold_aercloud", "Aeronuvem Fria", hardness="0.2", transparency="Translucent")
B("blue_aercloud", "Aeronuvem Azul", hardness="0.2", transparency="Translucent")
B("golden_aercloud", "Aeronuvem Dourada", hardness="0.2", transparency="Translucent", solid="false", light="5")

# Woods.
for wood, pt in (("skyroot", "Raiz-Celeste"), ("golden_oak", "Carvalho Dourado")):
    B(f"{wood}_log", f"Tronco de {pt}", hardness="2.0", tool="Some(Axe)",
      textures=tex(side=f"{wood}_log", top=f"{wood}_log_top"),
      drops=drops(f"{wood}_log", *([("golden_amber", 1, 2, 0.5)] if wood == "golden_oak" else [])))
    B(f"{wood}_leaves", f"Folhas de {pt}", hardness="0.2", transparency="Cutout",
      light="6" if wood == "golden_oak" else "0",
      drops=drops((f"{wood}_sapling", 1, 1, 0.05), *([("skyroot_stick", 1, 1, 0.04)] if wood == "skyroot" else [])))
    B(f"{wood}_sapling", f"Muda de {pt}", shape="Cross", transparency="Cutout", solid="false", hardness="0.0")
    S([f"{wood}_log"], "skyroot_planks", 4)
B("skyroot_planks", "Tábuas de Raiz-Celeste", hardness="2.0", tool="Some(Axe)")
B("skyroot_slab", "Laje de Raiz-Celeste", hardness="2.0", shape="Slab", tool="Some(Axe)", textures=tex(all="skyroot_planks"))
B("skyroot_stairs", "Escada de Raiz-Celeste", hardness="2.0", shape="Stairs", tool="Some(Axe)",
  textures=tex(all="skyroot_planks"))
B("skyroot_fence", "Cerca de Raiz-Celeste", hardness="2.0", shape="Fence", tool="Some(Axe)",
  textures=tex(all="skyroot_planks"))
tag("planks", "skyroot_planks")
tag("logs", "skyroot_log", "golden_oak_log")

# Plants.
B("aether_tall_grass", "Grama Alta do Aether", shape="Cross", transparency="Cutout", solid="false", hardness="0.0",
  drops="Some([])")
B("purple_flower", "Flor Roxa", shape="Cross", transparency="Cutout", solid="false", hardness="0.0")
B("white_flower", "Flor Branca", shape="Cross", transparency="Cutout", solid="false", hardness="0.0")
B("blueberry_bush", "Arbusto de Mirtilo", shape="Cross", transparency="Cutout", solid="false", hardness="0.0",
  drops=drops(("blue_berry", 1, 3)))

# Utility blocks (mods/aether.rs gives them their use).
B("altar", "Altar", hardness="2.0", tool="Some(Pickaxe)",
  textures=tex(top="altar_top", side="altar_side", bottom="holystone"))
B("freezer", "Congelador", hardness="2.0", tool="Some(Pickaxe)",
  textures=tex(top="freezer_top", side="freezer_side", bottom="holystone"))
B("incubator", "Incubadora", hardness="2.0", tool="Some(Pickaxe)",
  textures=tex(top="incubator_top", side="incubator_side", bottom="holystone"))

# The portal: a frame of glowstone lit with water.
B("aether_portal", "Portal do Aether", shape="Portal", solid="false", light="11", hardness="0.0", drops="Some([])",
  in_inventory="false", transparency="Translucent")

# Dungeon stones: bronze (carved), silver (angelic), gold (hellfire).
for n, pt, light in (("carved_stone", "Pedra Entalhada", "0"), ("sentry_stone", "Pedra Sentinela", "8"),
                     ("angelic_stone", "Pedra Angelical", "0"), ("light_angelic_stone", "Pedra Angelical Luminosa", "13"),
                     ("hellfire_stone", "Pedra do Fogo Infernal", "0"),
                     ("light_hellfire_stone", "Pedra do Fogo Infernal Luminosa", "13")):
    B(n, pt, hardness="3.0", tool="Some(Pickaxe)", tier="1", light=light)

# Items.
It("ambrosium_shard", "Fragmento de Ambrósio", food=food(1, 1.0), fuel="20.0")
It("zanite_gemstone", "Gema de Zanita")
It("skyroot_stick", "Graveto de Raiz-Celeste")
It("golden_amber", "Âmbar Dourado")
It("swet_ball", "Bola de Swet")
It("aechor_petal", "Pétala de Aechor")
It("blue_berry", "Mirtilo", food=food(2, 1.2))
It("enchanted_berry", "Mirtilo Encantado", food=food(6, 7.0))
It("healing_stone", "Pedra Curativa", food=food(4, 8.0))
It("moa_feather", "Pena de Moa")
It("moa_egg", "Ovo de Moa")
It("victory_medal", "Medalha da Vitória")
It("phoenix_feather", "Pena de Fênix")
It("bronze_key", "Chave de Bronze")
It("silver_key", "Chave de Prata")
It("gold_key", "Chave de Ouro")
It("cloud_parachute", "Paraquedas de Nuvem", max_stack="Some(1)")
It("golden_parachute", "Paraquedas Dourado", max_stack="Some(1)")
It("valkyrie_lance", "Lança da Valquíria", tool=tool("Sword", 4, 6.0, 10.0, 1600))

S(["skyroot_planks", "skyroot_planks"], "skyroot_stick", 4)
R(["##", "##"], {"#": "holystone"}, "holystone_bricks", 4)
R(["###"], {"#": "holystone_bricks"}, "holystone_brick_slab", 6)
R(["#  ", "## ", "###"], {"#": "holystone_bricks"}, "holystone_brick_stairs", 4)
R(["###"], {"#": "skyroot_planks"}, "skyroot_slab", 6)
R(["#  ", "## ", "###"], {"#": "skyroot_planks"}, "skyroot_stairs", 4)
R(["#|#", "#|#"], {"#": "skyroot_planks", "|": "skyroot_stick"}, "skyroot_fence", 3)
R(["###", "###", "###"], {"#": "zanite_gemstone"}, "zanite_block")
S(["zanite_block"], "zanite_gemstone", 9)
R(["HHH", "HZH", "HHH"], {"H": "holystone", "Z": "zanite_gemstone"}, "altar")
R(["HHH", "HIH", "PPP"], {"H": "holystone", "I": "icestone", "P": "skyroot_planks"}, "freezer")
R(["HHH", "HAH", "HHH"], {"H": "holystone", "A": "ambrosium_shard"}, "incubator")
R(["CC", "CC"], {"C": "cold_aercloud"}, "cloud_parachute")
R(["CC", "CC"], {"C": "golden_aercloud"}, "golden_parachute")
F("quicksoil", "quicksoil_glass", 0.1)
S(["holystone", "ambrosium_shard"], "healing_stone")

# Tools of the four tiers, on skyroot sticks.
TIERS = (("skyroot", "skyroot_planks", "Raiz-Celeste", 1, 2.0, 2.0, 59),
         ("holystone", "holystone", "Pedra Sagrada", 2, 4.0, 3.0, 131),
         ("zanite", "zanite_gemstone", "Zanita", 3, 6.5, 4.0, 250),
         ("gravitite", "enchanted_gravitite", "Gravitita", 4, 8.5, 5.0, 1561))
KINDS = (("pickaxe", "Pickaxe", "Picareta", ["###", " | ", " | "], 0.0),
         ("axe", "Axe", "Machado", ["##", "#|", " |"], 2.0),
         ("shovel", "Shovel", "Pá", ["#", "|", "|"], -0.5),
         ("sword", "Sword", "Espada", ["#", "#", "|"], 2.0))
for mat, ing, pt, tier, speed, dmg, dur in TIERS:
    for kind, tk, kpt, pat, extra in KINDS:
        n = f"{mat}_{kind}"
        It(n, f"{kpt} de {pt}", tool=tool(tk, tier, speed, dmg + extra, dur),
           **({"fuel": "10.0"} if mat == "skyroot" else {}))
        R(pat, {"#": ing, "|": "skyroot_stick"}, n)

# Armour: zanite and gravitite are crafted, valkyrie from the medals its warriors drop,
# phoenix from the Sun Spirit's feathers.
ARMOR = (("zanite", "zanite_gemstone", "Zanita", 1.0, 1.0),
         ("gravitite", "enchanted_gravitite", "Gravitita", 1.3, 1.6),
         ("valkyrie", "victory_medal", "Valquíria", 1.4, 2.0),
         ("phoenix", "phoenix_feather", "Fênix", 1.5, 2.2))
PIECES = (("helmet", 0, "Capacete", ["XXX", "X X"], 2, 165),
          ("chestplate", 1, "Peitoral", ["X X", "XXX", "XXX"], 6, 240),
          ("leggings", 2, "Calças", ["XXX", "X X", "X X"], 5, 225),
          ("boots", 3, "Botas", ["X X", "X X"], 2, 195))
for mat, ing, pt, pk, dk in ARMOR:
    for piece, slot, ppt, pat, pts, dur in PIECES:
        n = f"{mat}_{piece}"
        It(n, f"{ppt} de {pt}", armor=armor(slot, round(pts * pk), int(dur * dk)))
        R(pat, {"X": ing}, n)
