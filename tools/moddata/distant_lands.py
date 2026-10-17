"""Distant Lands (Brazilian biomes): the ipê, araucária, buriti and jabuticaba woods with their
whole block families and hollow logs, the fruits (jabuticaba, pinhão, buriti, açaí), the plants
of the five biomes (bromélias, mandacaru, capim-dourado, jurema, aguapé), their soils and the
carpenter table (mods/distant_lands.rs gives it its use)."""

from .lib import B, F, It, R, S, drops, food, tag, tex

WOODS = (("ipe", "Ipê"), ("araucaria", "Araucária"), ("buriti", "Buriti"), ("jabuticaba", "Jabuticabeira"))
# Ipê blossoms come in three colours, each with its own leaves and sapling (one log for all).
IPES = (("ipe_pink", "Ipê-Rosa"), ("ipe_yellow", "Ipê-Amarelo"), ("ipe_purple", "Ipê-Roxo"))
# Extra drops of each wood's leaves.
LEAF_EXTRA = {"araucaria": [("pinhao", 1, 1, 0.06)], "buriti": [], "jabuticaba": [], "ipe": []}

for w, pt in WOODS:
    B(f"{w}_log", f"Tronco de {pt}", hardness="2.0", tool="Some(Axe)", textures=tex(side=f"{w}_log", top=f"{w}_log_top"))
    B(f"hollow_{w}_log", f"Tronco Oco de {pt}", hardness="2.0", tool="Some(Axe)", shape="Hollow",
      transparency="Cutout", textures=tex(all=f"{w}_log"))
    B(f"{w}_planks", f"Tábuas de {pt}", hardness="2.0", tool="Some(Axe)")
    B(f"{w}_slab", f"Laje de {pt}", hardness="2.0", shape="Slab", tool="Some(Axe)", textures=tex(all=f"{w}_planks"))
    B(f"{w}_stairs", f"Escada de {pt}", hardness="2.0", shape="Stairs", tool="Some(Axe)", textures=tex(all=f"{w}_planks"))
    B(f"{w}_fence", f"Cerca de {pt}", hardness="2.0", shape="Fence", tool="Some(Axe)", textures=tex(all=f"{w}_planks"))
    leaves = IPES if w == "ipe" else ((w, pt),)
    for lw, lpt in leaves:
        B(f"{lw}_leaves", f"Folhas de {lpt}", hardness="0.2", transparency="Cutout",
          drops=drops((f"{lw}_sapling", 1, 1, 0.05), ("stick", 1, 1, 0.03), *LEAF_EXTRA[w]))
        B(f"{lw}_sapling", f"Muda de {lpt}", shape="Cross", transparency="Cutout", solid="false", hardness="0.0")
        It(f"{lw}_sapling", f"Muda de {lpt}", fuel="5.0")
    It(f"{w}_log", f"Tronco de {pt}", fuel="15.0")
    It(f"{w}_planks", f"Tábuas de {pt}", fuel="15.0")
    It(f"{w}_slab", f"Laje de {pt}", fuel="7.5")
    S([f"{w}_log"], f"{w}_planks", 4)
    R(["###"], {"#": f"{w}_planks"}, f"{w}_slab", 6)
    R(["#  ", "## ", "###"], {"#": f"{w}_planks"}, f"{w}_stairs", 4)
    R(["#|#", "#|#"], {"#": f"{w}_planks", "|": "stick"}, f"{w}_fence", 3)
    R(["L L", "L L"], {"L": f"{w}_log"}, f"hollow_{w}_log", 4)
    tag("planks", f"{w}_planks")
    tag("logs", f"{w}_log")

# Fruit hanging on the trees: under the buriti and açaí crowns, on the jabuticaba's trunk.
for n, pt, item, lo, hi in (("jabuticaba_cluster", "Cacho de Jabuticabas", "jabuticaba", 2, 4),
                            ("buriti_cluster", "Cacho de Buriti", "buriti_fruit", 1, 3),
                            ("acai_cluster", "Cacho de Açaí", "acai", 2, 4)):
    B(n, pt, shape="Cross", transparency="Cutout", solid="false", hardness="0.0", drops=drops((item, lo, hi)),
      in_inventory="false")

It("jabuticaba", "Jabuticaba", food=food(2, 1.2))
It("buriti_fruit", "Fruto de Buriti", food=food(3, 2.0))
It("acai", "Açaí", food=food(1, 0.6))
It("pinhao", "Pinhão", food=food(1, 0.5))
It("cooked_pinhao", "Pinhão Cozido", food=food(5, 6.0))
It("acai_pulp", "Polpa de Açaí", food=food(6, 7.2), max_stack="Some(16)")
It("jabuticaba_jam", "Geleia de Jabuticaba", food=food(6, 7.2), max_stack="Some(16)")
F("pinhao", "cooked_pinhao", 0.35)
S(["acai", "acai", "acai", "acai", "buriti_fruit"], "acai_pulp")
S(["jabuticaba", "jabuticaba", "jabuticaba", "jabuticaba", "apple"], "jabuticaba_jam")

# Plants.
for n, pt, light in (("bromelia_red", "Bromélia Vermelha", "0"), ("bromelia_orange", "Bromélia Laranja", "0"),
                     ("capim_dourado", "Capim-Dourado", "0"), ("jurema", "Jurema", "0"),
                     ("samambaia", "Samambaia", "0")):
    B(n, pt, shape="Cross", transparency="Cutout", solid="false", hardness="0.0",
      **({"drops": drops(("stick", 0, 1))} if n == "jurema" else {}))
B("mandacaru", "Mandacaru", shape="Inset", transparency="Cutout", hardness="0.4",
  textures=tex(side="mandacaru_side", top="mandacaru_top", bottom="mandacaru_top"))
B("mandacaru_flower", "Flor de Mandacaru", shape="Cross", transparency="Cutout", solid="false", hardness="0.0")
B("aguape", "Aguapé", shape="Layer", transparency="Cutout", hardness="0.0",
  textures=tex(top="aguape", bottom="aguape", side="aguape_side"))

# Soils: the cerrado's red earth, the caatinga's cracked ground, the pantanal's mud.
B("red_soil", "Terra Vermelha", hardness="0.5", tool="Some(Shovel)")
B("cracked_dirt", "Terra Rachada", hardness="0.6", tool="Some(Shovel)")
B("mud", "Lama", hardness="0.5", tool="Some(Shovel)")

# Golden grass weaves into hay.
S(["capim_dourado", "capim_dourado", "capim_dourado", "capim_dourado", "capim_dourado", "capim_dourado",
   "capim_dourado", "capim_dourado", "capim_dourado"], "hay_block")

# The carpenter table: hold logs or planks and right click to cut them into planks, slabs,
# stairs, fences or hollow logs (empty hand picks which).
B("carpenter_table", "Mesa de Carpinteiro", hardness="2.5", tool="Some(Axe)",
  textures=tex(top="carpenter_table_top", side="carpenter_table_side", bottom="ipe_planks"))
R(["II", "PP", "PP"], {"I": "iron_ingot", "P": "#planks"}, "carpenter_table")
