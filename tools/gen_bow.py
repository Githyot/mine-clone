"""Bow and arrow sprites (16x16, our own art): a curved wooden bow with a pale string, and
an arrow with a stone head and grey fletching, both drawn on the diagonal like tools."""
from pathlib import Path
from PIL import Image

OUT = Path(__file__).resolve().parent.parent / "packs" / "vanilla" / "textures" / "item"
WOOD = (120, 82, 44)
WOOD_DARK = (78, 52, 26)
STRING = (205, 205, 196)

bow = Image.new("RGBA", (16, 16), (0, 0, 0, 0))
# Limb: a curve from the top-right corner round to the bottom-left corner, bowing out to the top-left.
limb = [(14, 1), (13, 1), (12, 1), (11, 1), (10, 2), (9, 2), (8, 3), (7, 3), (6, 4), (5, 4), (4, 5), (4, 6), (3, 7), (3, 8), (2, 9), (2, 10), (1, 11), (1, 12), (1, 13), (1, 14)]
for i, (x, y) in enumerate(limb):
    bow.putpixel((x, y), (WOOD_DARK if i % 3 == 0 else WOOD) + (255,))
    # Thicker in the middle where the grip is.
    if 6 <= i <= 13:
        bow.putpixel((x + 1, y), WOOD + (255,))
for k in range(1, 14):
    # The string, straight from tip to tip.
    x, y = 14 - k, 1 + k
    if bow.getpixel((x, y))[3] == 0:
        bow.putpixel((x, y), STRING + (255,))
bow.save(OUT / "bow.png")

arrow = Image.new("RGBA", (16, 16), (0, 0, 0, 0))
for k in range(3, 12):
    arrow.putpixel((k, 15 - k), (110, 78, 46, 255))
# Head (top-right).
for x, y in [(12, 3), (13, 2), (12, 2), (13, 3), (11, 2), (13, 4), (14, 1)]:
    arrow.putpixel((x, y), (96, 96, 100, 255))
arrow.putpixel((13, 2), (140, 140, 146, 255))
# Fletching (bottom-left).
for x, y in [(1, 12), (2, 12), (3, 13), (3, 14), (2, 13), (1, 14), (4, 13), (3, 12)]:
    arrow.putpixel((x, y), (186, 186, 186, 255))
arrow.save(OUT / "arrow.png")
