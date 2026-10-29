"""Furniture of the mod woods (a Terraria idea): a chair, table, bench and shelf for each wood,
built in that wood's own style (see crates/voxel/src/furniture.rs). Vanilla woods have theirs in
packs/vanilla/blocks.ron."""

from .lib import B, R, tex

# wood, Portuguese name, furniture style, accent texture (cushions, bark, thatch...)
WOODS = [
    ("maple", "Bordo", "Shaker", "orange_wool"),
    ("walnut", "Nogueira", "Modern", "cyan_wool"),
    ("araucaria", "Araucária", "Colonial", "araucaria_log"),
    ("buriti", "Buriti", "Wicker", "thatch"),
    ("coconut", "Coqueiro", "Beach", "thatch"),
    ("ipe", "Ipê", "Deck", None),
    ("jabuticaba", "Jabuticabeira", "Rococo", "purple_wool"),
    ("echo", "Eco", "Ancient", "sculk"),
    ("skyroot", "Raiz-Celeste", "Celestial", "white_wool"),
]

PIECES = [("chair", "Cadeira", "Chair"), ("table", "Mesa", "Table"), ("bench", "Banco", "Bench"), ("shelf", "Prateleira", "Shelf")]

for w, pt, style, accent in WOODS:
    faces = dict(all=f"{w}_planks")
    if accent:
        faces["accent"] = accent
    for k, name, shape in PIECES:
        B(f"{w}_{k}", f"{name} de {pt}", shape=shape, style=style, hardness="2.0", tool="Some(Axe)", textures=tex(**faces))
    R(["|  ", "###", "| |"], {"#": f"{w}_planks", "|": "stick"}, f"{w}_chair", 2)
    R(["###", "| |", "| |"], {"#": f"{w}_planks", "|": "stick"}, f"{w}_table")
    R(["###", "| |"], {"#": f"{w}_planks", "|": "stick"}, f"{w}_bench", 2)
    R(["###", " | "], {"#": f"{w}_slab", "|": "stick"}, f"{w}_shelf", 3)
