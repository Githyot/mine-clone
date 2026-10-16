"""Fishing sprites (16x16, our own art): a rod, the float, and five fish raw and cooked."""
from pathlib import Path
from PIL import Image

OUT = Path(__file__).resolve().parent.parent / "packs" / "vanilla" / "textures" / "item"


def img():
    return Image.new("RGBA", (16, 16), (0, 0, 0, 0))


def put(im, x, y, c):
    if 0 <= x < 16 and 0 <= y < 16:
        im.putpixel((x, y), tuple(c) + (255,))


# Rod: a bent stick from bottom-left to top-right with the line hanging from the tip.
rod = img()
for k in range(13):
    x, y = 1 + k, 14 - k
    put(rod, x, y, (110, 76, 40) if k % 3 else (80, 54, 28))
for y in range(2, 12):
    put(rod, 14, y, (210, 210, 205))
put(rod, 13, 12, (160, 160, 160))
put(rod, 14, 12, (140, 140, 140))
rod.save(OUT / "fishing_rod.png")

# Float: red top, white bottom, a little dark rim.
bob = img()
for y in range(4, 12):
    for x in range(4, 12):
        if (x - 7.5) ** 2 + (y - 7.5) ** 2 <= 13:
            put(bob, x, y, (200, 40, 36) if y < 8 else (232, 232, 228))
for x in range(5, 11):
    put(bob, x, 8, (60, 30, 30))
put(bob, 6, 5, (240, 120, 110))
bob.save(OUT / "fishing_bobber.png")


def fish(body, belly, fin, spots=None, long=False):
    """A fish lying head-right: an oval body, tail fan on the left, eye and fins."""
    im = img()
    x0, x1 = (3, 14) if long else (4, 13)
    for x in range(x0, x1):
        t = (x - x0) / (x1 - x0 - 1)
        half = 1 + int(3.2 * (1 - (2 * t - 1) ** 2) ** 0.5 + 0.3)
        for y in range(8 - half, 8 + half):
            put(im, x, y, belly if y >= 8 else body)
    for k in range(4):
        put(im, x0 - 1 - k // 2, 7 - k, fin)
        put(im, x0 - 1 - k // 2, 8 + k, fin)
    put(im, x0 - 1, 7, fin)
    put(im, x0 - 1, 8, fin)
    for x in range(x0 + 3, x0 + 6):
        put(im, x, 8 - 4 if not long else 8 - 3, fin)
    put(im, x1 - 3, 6, (20, 20, 24))
    put(im, x1 - 1, 8, (60, 40, 40))
    if spots:
        for (x, y) in spots:
            put(im, x, y, fin)
    return im


def cooked(im):
    out = img()
    for y in range(16):
        for x in range(16):
            r, g, b, a = im.getpixel((x, y))
            if a:
                # Browned: warmer and darker, the eye gone pale.
                l = (r + g + b) / 3
                out.putpixel((x, y), (int(min(255, l * 0.9 + 60)), int(l * 0.62 + 30), int(l * 0.3 + 12), 255))
    return out


kinds = {
    "cod": fish((150, 130, 96), (206, 192, 160), (110, 92, 66), [(7, 6), (9, 5), (10, 7)]),
    "salmon": fish((160, 60, 56), (226, 140, 112), (110, 40, 36), long=True),
    "trout": fish((96, 120, 84), (210, 180, 150), (70, 84, 60), [(6, 6), (8, 5), (10, 6), (7, 9)], long=True),
    "catfish": fish((84, 80, 72), (150, 140, 120), (54, 50, 44)),
    "tilapia": fish((120, 128, 140), (190, 196, 204), (84, 90, 104), [(7, 6), (8, 6), (9, 6)]),
}
for name, im in kinds.items():
    im.save(OUT / f"{name}.png")
    cooked(im).save(OUT / f"cooked_{name}.png")
print("fish sprites written")
