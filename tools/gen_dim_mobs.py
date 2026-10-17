#!/usr/bin/env python3
"""Writes the creatures of the mod pack's dimensions (tools/dim_mobs_spec.py): their entries in
packs/vanilla/mods/dims_mobs.ron and their 64x128 skins in packs/vanilla/textures/entity/.
Every pixel is painted here from short palettes and noise; nothing is read from any mod.

    python3 tools/gen_dim_mobs.py [preview.png]
"""

import os
import sys
import zlib

from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import dim_mobs_spec as spec  # noqa: E402
from gen_mob_skins import face_rects, fbm, hash01, hexrgb, local_pos, mix, mul  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT_RON = os.path.join(ROOT, "packs", "vanilla", "mods", "dims_mobs.ron")
OUT_DIR = os.path.join(ROOT, "packs", "vanilla", "textures", "entity")


def q(s):
    return '"' + s + '"'


def shade(pal, t):
    t = max(0.0, min(0.999, t))
    k = t * (len(pal) - 1)
    i = int(k)
    return mix(pal[i], pal[min(i + 1, len(pal) - 1)], k - i)


def paint(look, face, i, j, fw, fh, g, seed):
    """Colour of one pixel of a face; g is the model-space position."""
    pal = [hexrgb(c) for c in look["pal"]]
    n = fbm(seed, g[0], g[1], g[2], 4.0)
    jit = hash01(seed, g[0] * 7, g[1] * 13, g[2] * 3) - 0.5
    pat = look["pattern"]
    t = 0.25 + 0.55 * n + 0.25 * jit
    if pat == "feathers":
        t += 0.25 * ((j % 3 == 0) - 0.4) + 0.1 * ((i + j // 3) % 2)
    elif pat == "wool":
        t = 0.3 + 0.6 * fbm(seed + 3, g[0], g[1], g[2], 1.6) + 0.2 * jit
    elif pat == "patches":
        t = 0.7 + 0.3 * n if fbm(seed + 9, g[0], g[1], g[2], 6.0) > 0.5 else 0.15 + 0.3 * n
    elif pat == "cloud":
        t = 0.45 + 0.6 * fbm(seed + 5, g[0], g[1], g[2], 3.0) + 0.1 * jit
    elif pat == "jelly":
        t = 0.35 + 0.4 * n + (0.35 if (i + j) % 7 == 0 else 0.0)
    elif pat == "carved":
        t = 0.3 + 0.3 * n + 0.15 * jit - (0.35 if i % 8 in (0, 7) or j % 8 in (0, 7) else 0.0)
    elif pat == "flame":
        t = 0.2 + 0.8 * fbm(seed + 11, g[0], g[1] * 0.6, g[2], 2.5) + 0.2 * jit
    elif pat == "sculk":
        vein = abs(fbm(seed + 13, g[0], g[1], g[2], 3.0) - 0.5) < 0.05
        t = 0.95 if vein else 0.15 + 0.45 * n
    elif pat == "rings":
        t = (0.8 if int(g[2]) % 4 == 0 else 0.2 + 0.3 * n)
    elif pat == "ribs":
        t = 0.8 if (face == "front" and j % 3 == 1 and 1 < i < fw - 2) else 0.15 + 0.4 * n
    elif pat == "plate":
        t = 0.5 + 0.3 * n + (0.3 if j == 0 or i == 0 else 0.0) - (0.3 if j == fh - 1 else 0.0)
    elif pat == "petals":
        t = 0.3 + 0.6 * (abs((i - fw / 2) * (j - fh / 2)) % 5) / 5
    elif pat == "planks":
        t = 0.3 + 0.35 * n + 0.15 * jit - (0.3 if int(g[1] + 64) % 4 == 0 else 0.0)
    elif pat == "cloth":
        t = 0.45 + 0.25 * n + 0.1 * jit - (0.2 if int(g[2] + 64) % 6 == 0 or int(g[0] + 64) % 6 == 0 else 0.0)
    elif pat == "rosettes":
        # A jaguar's coat: dark rings with a darker-gold middle on the light ground.
        cx, cy = (i + 2 * (j // 4)) % 5 - 2, j % 4 - 1.5
        d = cx * cx + cy * cy
        t = 0.0 if 1.5 < d < 3.5 and hash01(seed, i // 5, j // 4, 1) > 0.3 else (0.45 if d <= 1.5 else 0.6 + 0.3 * n)
    elif pat == "leaves":
        t = 0.2 + 0.7 * fbm(seed + 17, g[0], g[1], g[2], 1.5)
    if face == "top":
        t += 0.1
    elif face == "bottom":
        t -= 0.15
    rgb = shade(pal, t)
    if look.get("belly") and face == "bottom":
        rgb = shade([hexrgb(c) for c in look["belly"]], 0.5 + 0.4 * jit)
    if look.get("eyes") and face == "front":
        col, kind = look["eyes"]
        ey = max(1, fh // 2 - 1)
        ex = max(1, fw // 4)
        if j == ey and (i in (ex, fw - 1 - ex)):
            rgb = col
        elif j == ey and kind is None and (i in (ex + 1, fw - 2 - ex)) and fw >= 8:
            rgb = spec.WHITE
        elif kind == "glow" and j == ey - 1 and i in (ex, fw - 1 - ex):
            rgb = mul(col, 0.7)
    if look.get("mouth") and face == "front":
        my = min(fh - 2, fh // 2 + 2)
        if j == my and 1 < i < fw - 2:
            rgb = look["mouth"] if (i % 2 == 0) else mul(rgb, 0.4)
    return rgb


def render(name, model):
    img = Image.new("RGBA", (64, 128), (0, 0, 0, 0))
    px = img.load()
    seed = zlib.crc32(name.encode())
    done = set()
    for p in model.parts:
        if p["uv"] in done:
            continue
        done.add(p["uv"])
        for face, (x0, y0, fw, fh) in face_rects(p["size"], p["uv"]).items():
            for j in range(fh):
                for i in range(fw):
                    lx, ly, lz = local_pos(face, i, j, p["size"])
                    g = (p["origin"][0] + lx, p["origin"][1] + ly, p["origin"][2] + lz)
                    rgb = paint(p["look"], face, i, j, fw, fh, g, seed)
                    px[x0 + i, y0 + j] = (*rgb, 255)
    # Relief layers: sparse tufts over the part, in the lower half.
    for idx, _ in model.layers:
        p = model.parts[idx]
        for face, (x0, y0, fw, fh) in face_rects(p["size"], p["uv"]).items():
            for j in range(fh):
                for i in range(fw):
                    lx, ly, lz = local_pos(face, i, j, p["size"])
                    g = (p["origin"][0] + lx, p["origin"][1] + ly, p["origin"][2] + lz)
                    if fbm(seed + 77, g[0], g[1], g[2], 2.0) > 0.52 and not (face == "front" and p["look"].get("eyes")):
                        rgb = paint(p["look"], face, i, j, fw, fh, g, seed + 1)
                        px[x0 + i, y0 + j + 64] = (*mul(rgb, 1.08), 255)
    return img


def anim(a):
    return a if a else None


def ron(m):
    model = m["model"]
    kw = m["kw"]
    parts = []
    for p in model.parts:
        s = f"(size: {p['size']}, origin: {p['origin']}, uv: {p['uv']}"
        if p["anim"]:
            s += f", anim: {p['anim']}, pivot: {p['pivot']}"
        parts.append(s + ")")
    fields = [f"name: {q(m['name'])}", f"display_name: {q(m['display'])}", f"kind: {m['kind']}",
              f"health: {m['health']}", f"speed: {m['speed']}", f"width: {m['size'][0]}", f"height: {m['size'][1]}",
              f"spawn: {m['spawn']}"]
    for k, v in kw.items():
        if k == "drops":
            v = "[" + ", ".join(f"(item: {q(n)}, min: {a}, max: {b})" for n, a, b in v) + "]"
        elif isinstance(v, bool):
            v = "true" if v else "false"
        elif isinstance(v, list):
            v = "[" + ", ".join(q(x) for x in v) + "]"
        fields.append(f"{k}: {v}")
    out = "    (\n        " + ", ".join(fields) + ",\n        parts: [\n"
    out += "".join(f"            {p},\n" for p in parts) + "        ],\n"
    if model.layers:
        out += "        layers: [" + ", ".join(f"(part: {a}, inflate: {b})" for a, b in model.layers) + "],\n"
    return out + "    ),\n"


def main():
    os.makedirs(OUT_DIR, exist_ok=True)
    text = "// Made by tools/gen_dim_mobs.py from tools/dim_mobs_spec.py: edit those and run it again.\n(mobs: [\n"
    imgs = []
    for m in spec.MOBS:
        text += ron(m)
        img = render(m["name"], m["model"])
        img.save(os.path.join(OUT_DIR, m["name"] + ".png"))
        imgs.append(img)
    text += "])\n"
    with open(OUT_RON, "w") as f:
        f.write(text)
    if len(sys.argv) > 1:
        cols = 8
        sheet = Image.new("RGBA", (cols * 66 * 3, ((len(imgs) + cols - 1) // cols) * 130 * 3), (30, 30, 34, 255))
        for k, img in enumerate(imgs):
            sheet.paste(img.resize((192, 384), Image.NEAREST), ((k % cols) * 198, (k // cols) * 390))
        sheet.save(sys.argv[1])
    print(f"wrote {len(spec.MOBS)} creatures")


if __name__ == "__main__":
    main()
