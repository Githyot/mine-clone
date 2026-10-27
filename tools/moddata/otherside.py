"""Deeper and Darker's Otherside: gloomslate and its cut forms, sculk stone and grime, echo
wood, glowing plants, resonarium, the portal frame and its key, and resonarium gear."""

from .lib import B, It, R, S, armor, drops, food, tag, tex, tool

B("gloomslate", "Ardósia Sombria", hardness="3.0", tool="Some(Pickaxe)", tier="1", drops=drops("cobbled_gloomslate"))
B("cobbled_gloomslate", "Ardósia Sombria Pedregosa", hardness="3.0", tool="Some(Pickaxe)", tier="1")
B("polished_gloomslate", "Ardósia Sombria Polida", hardness="3.0", tool="Some(Pickaxe)", tier="1")
B("gloomslate_bricks", "Tijolos de Ardósia Sombria", hardness="3.0", tool="Some(Pickaxe)", tier="1")
B("gloomslate_brick_slab", "Laje de Tijolos de Ardósia Sombria", hardness="3.0", shape="Slab", tool="Some(Pickaxe)",
  tier="1", textures=tex(all="gloomslate_bricks"))
B("gloomslate_brick_stairs", "Escada de Tijolos de Ardósia Sombria", hardness="3.0", shape="Stairs",
  tool="Some(Pickaxe)", tier="1", textures=tex(all="gloomslate_bricks"))
B("sculk_stone", "Pedra de Sculk", hardness="3.0", tool="Some(Pickaxe)", tier="1")
B("sculk_grime", "Fuligem de Sculk", hardness="0.6", tool="Some(Shovel)",
  textures=tex(top="sculk_grime", side="sculk_grime_side", bottom="gloomslate"), drops=drops("cobbled_gloomslate"))
B("echo_soil", "Solo de Eco", hardness="0.6", tool="Some(Shovel)")
B("blooming_moss", "Musgo Florido", hardness="0.6", tool="Some(Shovel)",
  textures=tex(top="blooming_moss", side="blooming_moss_side", bottom="gloomslate"), drops=drops("cobbled_gloomslate"))
B("resonarium_ore", "Minério de Ressonário", hardness="4.0", tool="Some(Pickaxe)", tier="3", drops=drops("resonarium"),
  xp="(2, 5)")
B("crystallized_amber", "Âmbar Cristalizado", hardness="1.0", tool="Some(Pickaxe)", transparency="Translucent", light="7")
B("echo_log", "Tronco de Eco", hardness="2.0", tool="Some(Axe)", textures=tex(side="echo_log", top="echo_log_top"))
B("echo_leaves", "Folhas de Eco", hardness="0.2", transparency="Cutout", light="3",
  drops=drops(("echo_sapling", 1, 1, 0.05)))
B("echo_sapling", "Muda de Eco", shape="Cross", transparency="Cutout", solid="false", hardness="0.0")
B("echo_planks", "Tábuas de Eco", hardness="2.0", tool="Some(Axe)")
B("echo_slab", "Laje de Eco", hardness="2.0", shape="Slab", tool="Some(Axe)", textures=tex(all="echo_planks"))
B("echo_stairs", "Escada de Eco", hardness="2.0", shape="Stairs", tool="Some(Axe)", textures=tex(all="echo_planks"))
B("echo_fence", "Cerca de Eco", hardness="2.0", shape="Fence", tool="Some(Axe)", textures=tex(all="echo_planks"))
tag("planks", "echo_planks")
tag("logs", "echo_log")
B("sculk_tendrils", "Tentáculos de Sculk", shape="Cross", transparency="Cutout", solid="false", hardness="0.0",
  light="2", drops="Some([])")
B("gloom_bloom", "Flor Sombria", shape="Cross", transparency="Cutout", solid="false", hardness="0.0", light="9")
B("echo_bulb", "Bulbo de Eco", shape="Cross", transparency="Cutout", solid="false", hardness="0.0", light="12",
  drops=drops(("glow_berries", 1, 2)))
B("echo_lantern", "Lanterna de Eco", hardness="1.0", tool="Some(Pickaxe)", light="15")
B("sculk_portal_frame", "Moldura do Portal Abissal", hardness="25.0", tool="Some(Pickaxe)", tier="3",
  textures=tex(top="sculk_portal_frame_top", side="sculk_portal_frame", bottom="gloomslate"))
B("otherside_portal", "Portal do Outro Lado", shape="Portal", solid="false", light="9", hardness="0.0", drops="Some([])",
  in_inventory="false", transparency="Translucent")

It("echo_crystal", "Cristal de Eco")
It("resonarium", "Ressonário")
It("sculk_bone", "Osso de Sculk")
It("heart_of_the_deep_shard", "Fragmento do Coração das Profundezas")
It("heart_of_the_deep", "Coração das Profundezas", max_stack="Some(1)")
It("gloom_berry_pie", "Torta Sombria", food=food(8, 6.0))

S(["echo_log"], "echo_planks", 4)
R(["###"], {"#": "echo_planks"}, "echo_slab", 6)
R(["#  ", "## ", "###"], {"#": "echo_planks"}, "echo_stairs", 4)
R(["#|#", "#|#"], {"#": "echo_planks", "|": "stick"}, "echo_fence", 3)
R(["##", "##"], {"#": "cobbled_gloomslate"}, "polished_gloomslate", 4)
R(["##", "##"], {"#": "polished_gloomslate"}, "gloomslate_bricks", 4)
R(["###"], {"#": "gloomslate_bricks"}, "gloomslate_brick_slab", 6)
R(["#  ", "## ", "###"], {"#": "gloomslate_bricks"}, "gloomslate_brick_stairs", 4)
R(["GEG", "ECE", "GEG"], {"G": "glass", "E": "echo_crystal", "C": "crystallized_amber"}, "echo_lantern", 2)
R(["OEO", "EPE", "OEO"], {"O": "obsidian", "E": "ender_eye", "P": "ender_pearl"}, "sculk_portal_frame", 4)
R(["SAS", "ADA", "SAS"], {"S": "heart_of_the_deep_shard", "A": "amethyst", "D": "diamond"}, "heart_of_the_deep")
R(["AEA", "EDE", "AEA"], {"A": "amethyst", "E": "ender_eye", "D": "diamond"}, "heart_of_the_deep")
S(["glow_berries", "glow_berries", "wheat", "wheat"], "gloom_berry_pie")

for kind, tk, kpt, pat, extra in (("pickaxe", "Pickaxe", "Picareta", ["###", " | ", " | "], 0.0),
                                  ("axe", "Axe", "Machado", ["##", "#|", " |"], 3.0),
                                  ("shovel", "Shovel", "Pá", ["#", "|", "|"], -0.5),
                                  ("sword", "Sword", "Espada", ["#", "#", "|"], 3.0)):
    n = f"resonarium_{kind}"
    It(n, f"{kpt} de Ressonário", tool=tool(tk, 4, 9.0, 5.0 + extra, 1800))
    R(pat, {"#": "resonarium", "|": "sculk_bone"}, n)
for piece, slot, ppt, pat, pts, dur in (("helmet", 0, "Capacete", ["XXX", "X X"], 3, 330),
                                        ("chestplate", 1, "Peitoral", ["X X", "XXX", "XXX"], 8, 480),
                                        ("leggings", 2, "Calças", ["XXX", "X X", "X X"], 6, 450),
                                        ("boots", 3, "Botas", ["X X", "X X"], 3, 390)):
    n = f"resonarium_{piece}"
    It(n, f"{ppt} de Ressonário", armor=armor(slot, pts, dur))
    R(pat, {"X": "resonarium"}, n)
