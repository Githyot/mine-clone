"""Ecologics: coconut palms on the beaches, walnut trees on the plains, maples with orange
leaves in the forests (each with its whole wood set), coconuts and walnuts to eat, and thin ice
on frozen water that cracks under your feet. Where the trees grow is in
crates/voxel/src/worldgen/ecologics.rs; the cracking ice in crates/game/src/mods/ecologics.rs.
Saplings are only defined here (they grow through mods/saplings.rs)."""

from .lib import B, It, R, S, drops, food, q, tag, tex

WOODS = [
    # wood, Portuguese name, leaf drops beyond the sapling
    ("coconut", "Coqueiro", []),
    ("walnut", "Nogueira", [("walnut", 1, 1, 0.08)]),
    ("maple", "Bordo", [("stick", 1, 1, 0.03)]),
]

for w, pt, extra in WOODS:
    B(f"{w}_log", f"Tronco de {pt}", hardness="2.0", tool="Some(Axe)", textures=tex(side=f"{w}_log", top=f"{w}_log_top"))
    B(f"{w}_planks", f"Tábuas de {pt}", hardness="2.0", tool="Some(Axe)")
    B(f"{w}_leaves", f"Folhas de {pt}", transparency="Cutout", hardness="0.2",
      drops=drops((f"{w}_sapling", 1, 1, 0.05), *extra))
    B(f"{w}_sapling", f"Muda de {pt}", shape="Cross", transparency="Cutout", solid="false", hardness="0.0")
    B(f"{w}_slab", f"Laje de {pt}", hardness="2.0", shape="Slab", tool="Some(Axe)", textures=tex(all=f"{w}_planks"))
    B(f"{w}_stairs", f"Escada de {pt}", hardness="2.0", shape="Stairs", tool="Some(Axe)", textures=tex(all=f"{w}_planks"))
    B(f"{w}_fence", f"Cerca de {pt}", hardness="2.0", shape="Fence", tool="Some(Axe)", textures=tex(all=f"{w}_planks"))
    B(f"{w}_fence_gate", f"Portão de {pt}", hardness="2.0", shape="FenceGate", tool="Some(Axe)", textures=tex(all=f"{w}_planks"))
    B(f"{w}_door", f"Porta de {pt}", shape="Door", transparency="Cutout", hardness="3.0", tool="Some(Axe)",
      textures=tex(side=f"{w}_door_lower", top=f"{w}_door_upper"))
    It(f"{w}_door", f"Porta de {pt}", texture=f"Some({q(w + '_door')})")
    B(f"{w}_trapdoor", f"Alçapão de {pt}", shape="Trapdoor", transparency="Cutout", hardness="3.0", tool="Some(Axe)")
    tag("planks", f"{w}_planks")
    tag("logs", f"{w}_log")
    S([f"{w}_log"], f"{w}_planks", 4)
    R(["###"], {"#": f"{w}_planks"}, f"{w}_slab", 6)
    R(["#  ", "## ", "###"], {"#": f"{w}_planks"}, f"{w}_stairs", 4)
    R(["#|#", "#|#"], {"#": f"{w}_planks", "|": "stick"}, f"{w}_fence", 3)
    R(["|#|", "|#|"], {"#": f"{w}_planks", "|": "stick"}, f"{w}_fence_gate")
    R(["##", "##", "##"], {"#": f"{w}_planks"}, f"{w}_door", 3)
    R(["###", "###"], {"#": f"{w}_planks"}, f"{w}_trapdoor", 2)

# The coconut hangs under the palm's fronds (a hanging lantern-shaped nut).
B("coconut", "Coco", shape="Lantern", hardness="0.5", tool="Some(Axe)", textures=tex(all="coconut"),
  drops=drops(("coconut_slice", 2, 3)))
It("coconut_slice", "Pedaço de Coco", food=food(3, 1.8))
It("walnut", "Noz", food=food(2, 1.2))
It("walnut_cookie", "Biscoito de Noz", food=food(4, 2.4))
S(["walnut", "wheat"], "walnut_cookie", 2)

# Thin ice: three looks as it cracks (the game swaps them, then it breaks into water).
B("thin_ice", "Gelo Fino", transparency="Translucent", hardness="0.2", tool="Some(Pickaxe)", drops="Some([])")
for k, pt in ((1, "Gelo Fino Rachado"), (2, "Gelo Fino Muito Rachado")):
    B(f"thin_ice_{k}", pt, transparency="Translucent", hardness="0.1", drops="Some([])", in_inventory="false")
