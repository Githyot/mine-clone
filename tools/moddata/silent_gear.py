"""Silent Gear: tools built from parts. A mold (copied from a wooden tool) and the head material
make a tool head; head + rod + binding make the tool, and every part shows in its stats:

* the head sets the tier, digging speed, damage and most of the durability,
* the rod (stick, bone or blaze rod) adds durability, damage or speed,
* the binding (string or leather) holds it together, leather a little longer.

Every combination is its own item (item stacks carry no extra data), so the stats are fixed
here from the parts and show in the tooltip. Only the plain stick-and-string tools are shown
in the creative menu (crates/game/src/creative.rs); the rest come from crafting."""

from .lib import It, S, tool

KINDS = [
    # kind, tool kind, Portuguese name, base damage, head materials needed
    ("pickaxe", "Pickaxe", "Picareta", 2.0, 3),
    ("axe", "Axe", "Machado", 6.0, 3),
    ("shovel", "Shovel", "Pá", 2.5, 1),
    ("sword", "Sword", "Espada", 4.0, 2),
]
HEADS = [
    # material, Portuguese, ingredient, tier, speed, extra damage, durability
    ("flint", "Pederneira", "flint", 2, 4.5, 1.0, 140),
    ("copper", "Cobre", "copper_ingot", 2, 5.0, 1.0, 190),
    ("iron", "Ferro", "iron_ingot", 3, 6.0, 2.0, 250),
    ("gold", "Ouro", "gold_ingot", 1, 12.0, 0.0, 40),
    ("diamond", "Diamante", "diamond", 4, 8.0, 3.0, 1561),
]
RODS = [
    # rod item, Portuguese, durability factor, extra speed, extra damage
    ("stick", "cabo de madeira", 1.0, 0.0, 0.0),
    ("bone", "cabo de osso", 1.25, 0.0, 0.5),
    ("blaze_rod", "cabo de brasa", 1.5, 1.5, 0.5),
]
BINDINGS = [
    ("string", "amarra de linha", 1.0),
    ("leather", "amarra de couro", 1.2),
]

for k, tk, kpt, base, n in KINDS:
    mold = f"gear_{k}_mold"
    It(mold, f"Molde de {kpt}")
    S(["#planks", "stick", f"wooden_{k}"], mold)
    for m, mpt, ing, tier, speed, dmg, dur in HEADS:
        head = f"gear_{m}_{k}_head"
        It(head, f"Cabeça de {kpt} de {mpt}")
        S([mold] + [ing] * n, head)
        for r, rpt, rdur, rspeed, rdmg in RODS:
            for b, bpt, bdur in BINDINGS:
                name = f"gear_{k}_{m}_{r}_{b}"
                It(name, f"{kpt} de {mpt} ({rpt}, {bpt})",
                   tool=tool(tk, tier, round(speed + rspeed, 2), round(base + dmg + rdmg, 2), int(dur * rdur * bdur)))
                S([head, r, b], name)
