"""Macaw's Doors, Fences and Walls, and Trapdoors: fence gates for every wood, new door styles
(shoji, barn, glass, jail, modern, metal), picket, metal and barbed-wire fences, and two new
trapdoor styles. The gate shape lives in crates/voxel/src/shapes.rs; the barbed wire's sting in
crates/game/src/mods/macaws.rs."""

from .lib import R, It, B, q, tex

WOODS = [("oak", "Carvalho"), ("birch", "Bétula"), ("spruce", "Pinheiro"), ("jungle", "Selva"),
         ("acacia", "Acácia"), ("dark_oak", "Carvalho Escuro"), ("cherry", "Cerejeira"), ("crimson", "Carmesim")]
for w, pt in WOODS:
    B(f"{w}_fence_gate", f"Portão de {pt}", shape="FenceGate", hardness="2.0", tool="Some(Axe)",
      textures=tex(all=f"{w}_planks"))
    R(["|#|", "|#|"], {"#": f"{w}_planks", "|": "stick"}, f"{w}_fence_gate")

# name, Portuguese name, hardness, tool, ingredients (recipe key X / Y)
DOORS = [
    ("shoji_door", "Porta Shoji", "2.0", "Axe", {"X": "#planks", "Y": "paper_lamp"}),
    ("barn_door", "Porta de Celeiro", "3.0", "Axe", {"X": "#planks", "Y": "stick"}),
    ("glass_door", "Porta de Vidro", "1.5", "Axe", {"X": "#planks", "Y": "glass"}),
    ("jail_door", "Porta de Cela", "5.0", "Pickaxe", {"X": "iron_bars", "Y": "iron_ingot"}),
    ("modern_door", "Porta Moderna", "3.0", "Axe", {"X": "dark_oak_planks", "Y": "iron_ingot"}),
    ("metal_door", "Porta de Metal", "5.0", "Pickaxe", {"X": "iron_ingot", "Y": "iron_ingot"}),
]
for n, pt, hard, tk, key in DOORS:
    B(n, pt, shape="Door", transparency="Cutout", hardness=hard, tool=f"Some({tk})",
      textures=tex(side=f"{n}_lower", top=f"{n}_upper"))
    It(n, pt, texture=f"Some({q(n)})")
    R(["XX", "XY", "XX"], key, n, 3)

for n, pt, hard, tk, texture, key in (
    ("picket_fence", "Cerca de Estacas", "2.0", "Axe", "picket_planks", {"#": "#planks", "|": "bone"}),
    ("metal_fence", "Cerca de Metal", "4.0", "Pickaxe", "metal_fence", {"#": "iron_ingot", "|": "iron_bars"}),
    ("wired_fence", "Cerca de Arame Farpado", "2.0", "Axe", "wired_fence", {"#": "#planks", "|": "iron_ingot"}),
):
    B(n, pt, shape="Fence", hardness=hard, tool=f"Some({tk})", textures=tex(all=texture))
    R(["#|#", "#|#"], key, n, 3)
B("picket_fence_gate", "Portão de Estacas", shape="FenceGate", hardness="2.0", tool="Some(Axe)",
  textures=tex(all="picket_planks"))
R(["|#|", "|#|"], {"#": "picket_fence", "|": "stick"}, "picket_fence_gate")
B("metal_fence_gate", "Portão de Metal", shape="FenceGate", hardness="4.0", tool="Some(Pickaxe)",
  textures=tex(all="metal_fence"))
R(["|#|", "|#|"], {"#": "metal_fence", "|": "iron_ingot"}, "metal_fence_gate")

B("ranch_trapdoor", "Alçapão de Rancho", shape="Trapdoor", transparency="Cutout", hardness="3.0", tool="Some(Axe)")
R(["#|#", "#|#"], {"#": "#planks", "|": "stick"}, "ranch_trapdoor", 2)
B("glass_trapdoor", "Alçapão de Vidro", shape="Trapdoor", transparency="Cutout", hardness="1.5", tool="Some(Axe)")
R(["#G#", "#G#"], {"#": "#planks", "G": "glass"}, "glass_trapdoor", 2)
