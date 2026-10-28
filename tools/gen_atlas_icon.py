"""The atlas item icon (atlas.rs): a thick leather book with a little map on its cover.

Drawn straight as 32x32 pixel art in the style of tools/tex64/pixel.py (chunky pixels, a small
palette, a dark one-pixel outline) and grown to 64x64 nearest-neighbour.

    python3 tools/gen_atlas_icon.py
"""

from pathlib import Path

from PIL import Image

OUT = Path(__file__).resolve().parent.parent / "packs/vanilla/textures/item/atlas.png"

PAL = {
    ".": None,
    "o": (46, 26, 16),     # outline
    "L": (122, 66, 36),    # leather
    "l": (150, 86, 48),    # leather, lit
    "d": (88, 46, 26),     # leather, shade / spine
    "g": (214, 170, 64),   # brass corners
    "G": (246, 214, 120),  # brass glint
    "p": (234, 220, 180),  # page edges
    "q": (196, 176, 134),  # page lines
    "m": (220, 200, 150),  # map paper
    "w": (126, 158, 172),  # map water
    "f": (120, 150, 86),   # map land
    "r": (170, 44, 32),    # red ink
}

ART = [
    "................................",
    "................................",
    "................................",
    "....oooooooooooooooooooooo......",
    "...oGgdllllllllllllllllggoo.....",
    "...ogddllllllllllllllllllgoo....",
    "...oddLlllllllllllllllllllopo...",
    "...oddLLooooooooooooooooLlopo...",
    "...oddLLommmmmmmmmmmmmmoLLoqpo..",
    "...oddLLommwwwmmmmmfffmoLLopqo..",
    "...oddLLomwwwwwmmmffffmoLLoqpo..",
    "...oddLLomwwwwffmfffffmoLLopqo..",
    "...oddLLommwwffffffmrmmoLLoqpo..",
    "...oddLLommmwfffffmrrrmoLLopqo..",
    "...oddLLommmmfffffmmrmmoLLoqpo..",
    "...oddLLomfffffwwmmmmmmoLLopqo..",
    "...oddLLomffffwwwwmmrmmoLLoqpo..",
    "...oddLLommfffwwwwmmmmmoLLopqo..",
    "...oddLLommmmmmwwmmmmmmoLLoqpo..",
    "...oddLLommmmmmmmmmmmmmoLLopqo..",
    "...oddLLooooooooooooooooLLoqpo..",
    "...oddLLLLLLLLLLLLLLLLLLLLopqo..",
    "...oddLLLLLLLLLLLLLLLLLLLLoqpo..",
    "...oddLLLLLLLLLLLLLLLLLLLLopqo..",
    "...oddddddddddddddddddddddoqpo..",
    "...ogdddddddddddddddddddggopo...",
    "...oGgoooooooooooooooooooGoo....",
    "....oppppppppppppppppppppqo.....",
    ".....oqqqqqqqqqqqqqqqqqqqoo.....",
    "......ooooooooooooooooooooo.....",
    "................................",
    "................................",
]


def main():
    assert len(ART) == 32 and all(len(r) == 32 for r in ART), [len(r) for r in ART]
    im = Image.new("RGBA", (32, 32), (0, 0, 0, 0))
    for y, row in enumerate(ART):
        for x, ch in enumerate(row):
            c = PAL[ch]
            if c is not None:
                im.putpixel((x, y), c + (255,))
    im.resize((64, 64), Image.NEAREST).save(OUT)
    print("wrote", OUT)


if __name__ == "__main__":
    main()
