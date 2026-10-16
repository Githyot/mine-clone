"""Bucket sprites (16x16, our own art): a grey iron pail with a handle, empty, with water
and with lava."""
from pathlib import Path
from PIL import Image

OUT = Path(__file__).resolve().parent.parent / "packs" / "vanilla" / "textures" / "item"
RIM = (196, 196, 200)
SIDE = (150, 150, 156)
DARK = (92, 92, 98)
INSIDE = (60, 60, 66)


def bucket(fill=None):
    im = Image.new("RGBA", (16, 16), (0, 0, 0, 0))
    px = lambda x, y, c: im.putpixel((x, y), c + (255,))
    # Handle arcing over the top.
    for x, y in [(4, 3), (5, 2), (6, 1), (7, 1), (8, 1), (9, 1), (10, 2), (11, 3)]:
        px(x, y, DARK)
    # Rim: an ellipse seen from slightly above.
    for x in range(3, 13):
        px(x, 5, RIM)
        px(x, 7, SIDE)
    px(2, 6, RIM)
    px(13, 6, RIM)
    for x in range(3, 13):
        px(x, 6, fill[x % 2] if fill else INSIDE)
    # Body narrowing toward the bottom.
    for y in range(8, 15):
        inset = (y - 8) // 3
        for x in range(3 + inset, 13 - inset):
            c = SIDE
            if x == 3 + inset:
                c = DARK
            elif x == 12 - inset:
                c = DARK
            elif x == 5 + inset:
                c = RIM
            px(x, y, c)
    for x in range(5, 11):
        px(x, 14, DARK)
    return im


bucket().save(OUT / "bucket.png")
bucket([(52, 96, 210), (64, 112, 226)]).save(OUT / "water_bucket.png")
bucket([(222, 96, 18), (250, 150, 40)]).save(OUT / "lava_bucket.png")
