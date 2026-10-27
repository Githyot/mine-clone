"""Spawn egg sprites: a speckled egg in two colours per mob (16x16, our own art)."""
from pathlib import Path
from PIL import Image

OUT = Path(__file__).resolve().parent.parent / "packs" / "vanilla" / "textures" / "item"
MOBS = {
    "pig": ((240, 160, 156), (219, 99, 94)),
    "cow": ((68, 54, 38), (161, 161, 161)),
    "sheep": ((231, 231, 231), (255, 181, 181)),
    "chicken": ((161, 161, 161), (255, 0, 0)),
    "zombie": ((0, 175, 175), (121, 156, 101)),
    "skeleton": ((193, 193, 193), (73, 73, 73)),
    "stalker": ((20, 120, 40), (0, 0, 0)),
    "spider": ((52, 45, 39), (163, 0, 0)),
    "deer": ((143, 100, 59), (236, 224, 194)),
    "fox": ((208, 100, 36), (40, 30, 24)),
    "rabbit": ((140, 118, 94), (230, 222, 208)),
    "bear": ((74, 46, 28), (160, 126, 88)),
    "duck": ((46, 122, 72), (224, 184, 48)),
}
# Egg outline, rows top to bottom: (first x, last x).
ROWS = [(6, 9), (5, 10), (4, 11), (4, 11), (3, 12), (3, 12), (3, 12), (2, 13), (2, 13), (2, 13), (2, 13), (3, 12), (3, 12), (4, 11)]
SPOTS = [(6, 4), (9, 6), (5, 8), (10, 10), (7, 11), (4, 11), (11, 7), (8, 8)]
for name, (base, spot) in MOBS.items():
    img = Image.new("RGBA", (16, 16), (0, 0, 0, 0))
    for i, (a, b) in enumerate(ROWS):
        y = i + 1
        for x in range(a, b + 1):
            edge = x in (a, b) or i in (0, len(ROWS) - 1)
            shade = 0.6 if edge else (1.1 if x < 7 and y < 8 else 1.0)
            c = tuple(min(255, int(v * shade)) for v in base)
            img.putpixel((x, y), c + (255,))
    for x, y in SPOTS:
        img.putpixel((x, y), spot + (255,))
        if x + 1 < 16:
            img.putpixel((x + 1, y), spot + (255,))
    img.save(OUT / f"{name}_spawn_egg.png")
print("ok")
