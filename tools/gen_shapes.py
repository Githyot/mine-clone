"""Door and trapdoor textures in the 16x16 style, coloured from each wood's planks."""
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parent.parent / "packs" / "vanilla" / "textures"
WOODS = ["oak", "birch", "spruce", "jungle", "acacia", "dark_oak", "cherry", "mangrove", "crimson", "warped"]
iron = (70, 70, 74)


def make(wood):
    planks = Image.open(ROOT / "block" / f"{wood}_planks.png").convert("RGBA")
    cols = sorted({planks.getpixel((x, y))[:3] for x in range(16) for y in range(16)}, key=sum)
    dark, mid, light = cols[0], cols[len(cols) // 2], cols[-1]
    edge = tuple(max(0, c - 30) for c in dark)

    def board(img, y0, y1, window=False):
        for y in range(y0, y1):
            for x in range(16):
                frame = x in (0, 15) or y in (y0, y1 - 1)
                seam = x in (5, 10)
                c = edge if frame else (dark if seam else (light if (x + y * 3) % 7 == 0 else mid))
                img.putpixel((x, y), c + (255,))
        if window:
            for y in range(y0 + 2, y0 + 7):
                for x in range(2, 14):
                    if x in (7, 8) or y == y0 + 4:
                        img.putpixel((x, y), edge + (255,))
                    else:
                        img.putpixel((x, y), (0, 0, 0, 0))

    lower = Image.new("RGBA", (16, 16))
    board(lower, 0, 16)
    for y in (5, 11):  # hinges
        for x in (1, 2):
            lower.putpixel((x, y), iron + (255,))
    lower.putpixel((13, 2), iron + (255,))  # handle
    lower.putpixel((13, 3), iron + (255,))
    lower.save(ROOT / "block" / f"{wood}_door_lower.png")

    upper = Image.new("RGBA", (16, 16))
    board(upper, 0, 16, window=True)
    for x in (1, 2):
        upper.putpixel((x, 10), iron + (255,))
    upper.save(ROOT / "block" / f"{wood}_door_upper.png")

    trap = Image.new("RGBA", (16, 16))
    board(trap, 0, 16)
    for y in range(2, 14):
        for x in range(2, 14):
            if (x in (4, 5, 10, 11)) and y not in (7, 8):
                trap.putpixel((x, y), (0, 0, 0, 0))
    trap.save(ROOT / "block" / f"{wood}_trapdoor.png")

    # Inventory sprite: both halves squeezed into a narrow door.
    item = Image.new("RGBA", (16, 16))
    both = Image.new("RGBA", (16, 32))
    both.paste(upper, (0, 0))
    both.paste(lower, (0, 16))
    small = both.resize((8, 16), Image.NEAREST)
    item.paste(small, (4, 0))
    item.save(ROOT / "item" / f"{wood}_door.png")


for w in WOODS:
    make(w)
print("ok", len(WOODS), "woods")
