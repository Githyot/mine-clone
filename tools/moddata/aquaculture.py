"""Aquaculture 2: the modular fishing rod and its parts (hooks, lines and baits that change how
fast fish bite and what comes up), more fish by biome, and fillets. How the parts work is in
crates/game/src/mods/aquaculture.rs."""

from .lib import F, R, S, It, food

It("modular_fishing_rod", "Vara de Pesca Modular", max_stack="Some(1)")
R(["  /", " /S", "I S"], {"/": "stick", "S": "string", "I": "iron_ingot"}, "modular_fishing_rod")

# Hooks: what comes up.
It("iron_hook", "Anzol de Ferro")
R([" I", "I "], {"I": "iron_ingot"}, "iron_hook")
It("gold_hook", "Anzol de Ouro")
R([" G", "G "], {"G": "gold_ingot"}, "gold_hook")
It("heavy_hook", "Anzol Pesado")
R([" I", "II"], {"I": "iron_ingot"}, "heavy_hook")
It("double_hook", "Anzol Duplo")
S(["iron_hook", "iron_hook"], "double_hook")

# Lines: how long a bite stays on and how much the rod wears.
It("fishing_line", "Linha de Pesca")
S(["string", "string", "string"], "fishing_line")
It("strong_line", "Linha Reforçada")
S(["fishing_line", "leather"], "strong_line")

# Baits: used up one per catch, they bring the fish sooner.
It("worm", "Minhoca")
It("fish_bait", "Isca de Peixe")
S(["cod", "bone"], "fish_bait", 4)
S(["dirt", "rotten_flesh"], "worm", 2)

FISH = [
    ("bass", "Robalo"), ("carp", "Carpa"), ("bluegill", "Peixe-Sol"), ("halibut", "Linguado"),
    ("pollock", "Escamudo"), ("grouper", "Garoupa"), ("gar", "Peixe-Agulha"),
]
for n, pt in FISH:
    It(n, f"{pt} Cru", food=food(2, 0.6))
    It(f"cooked_{n}", f"{pt} Assado", food=food(6, 7.0))
    F(n, f"cooked_{n}", 0.35)
    S([n], "fish_fillet", 2)
It("fish_fillet", "Filé de Peixe", food=food(1, 0.4))
It("cooked_fish_fillet", "Filé de Peixe Grelhado", food=food(4, 4.0))
F("fish_fillet", "cooked_fish_fillet", 0.2)
# Junk from the water.
It("driftwood", "Madeira Flutuante", fuel="8.0")
S(["driftwood"], "stick", 3)
It("tin_can", "Lata Velha")
