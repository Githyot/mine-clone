#!/usr/bin/env python3
"""Build every block and item texture of the base game at 64x64.

    python3 tools/tex64/build.py [--preview DIR] [--only NAME,NAME] [--jobs N]

How it works (safe to re-run any time; output is deterministic):

1. Any 16x16 PNG found in packs/vanilla/textures/{block,item}/ is a *design*: it is copied into
   tools/tex64/src16/ (the archive of all 16x16 designs). That is how the old generators
   (tools/gen_*.py) and anything other people add keep working: they write 16x16 files as
   before, and this script picks them up.
2. For every design: a hand-made 64x64 generator (blocks.py / items.py) is used when there is
   one, otherwise upscale.py turns the 16x16 design into 64x64 in the same style.
3. Hand-made textures of natural blocks are nudged to the old design's average colour, so biome
   tints, distant terrain colours and the map keep working the same.
4. The 64x64 result is written back to packs/vanilla/textures/{block,item}/.

Mob skins, armour and the interface are built by entities.py and gui.py (also called from here).
"""

import argparse
import os
import shutil
import sys
from multiprocessing import Pool

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
ROOT = os.path.dirname(os.path.dirname(HERE))
PACK = os.path.join(ROOT, "packs", "vanilla", "textures")
SRC = os.path.join(HERE, "src16")
KINDS = ("block", "item")

# Hand-made textures whose average colour should stay that of the 16x16 design.
MATCH_MEAN = {
    "grass_block_top", "dirt", "sand", "red_sand", "stone", "gravel", "clay", "snow", "water",
    "sandstone_top", "sandstone_side", "sandstone_bottom", "cobblestone", "netherrack", "end_stone",
    "soul_sand", "soul_soil", "moss_block", "rocky_dirt", "farmland",
}


def load(path):
    return np.asarray(Image.open(path).convert("RGBA"), dtype=np.float64)


def save(img, path):
    from style import to_u8
    Image.fromarray(to_u8(img), "RGBA").save(path, optimize=True)


def sync_designs():
    """Copy new or changed 16x16 designs out of the pack into src16/."""
    changed = 0
    for kind in KINDS:
        os.makedirs(os.path.join(SRC, kind), exist_ok=True)
        d = os.path.join(PACK, kind)
        for f in sorted(os.listdir(d)):
            if not f.endswith(".png") or f == "preview.png":
                continue
            p = os.path.join(d, f)
            with Image.open(p) as im:
                small = im.size == (16, 16)
            if small:
                dst = os.path.join(SRC, kind, f)
                if not os.path.exists(dst) or open(dst, "rb").read() != open(p, "rb").read():
                    shutil.copyfile(p, dst)
                    changed += 1
    return changed


def match_mean(img, src):
    from style import from_oklab, to_oklab
    a = img[..., 3] > 127
    b = src[..., 3] > 127
    if a.sum() < 16 or b.sum() < 4:
        return img
    lab = to_oklab(img[..., :3])
    want = to_oklab(src[..., :3])[b].mean(0)
    have = lab[a].mean(0)
    lab = lab + (want - have)
    out = img.copy()
    out[..., :3] = np.where(a[..., None], from_oklab(lab), img[..., :3])
    return out


def make(job):
    kind, name = job
    import blocks
    import items
    from upscale import upscale
    srcp = os.path.join(SRC, kind, name + ".png")
    src = load(srcp) if os.path.exists(srcp) else None
    reg = blocks.REGISTRY if kind == "block" else items.REGISTRY
    how = "hand"
    if name in reg:
        img = reg[name]()
        if src is not None and name in MATCH_MEAN:
            img = match_mean(img, src)
    elif src is not None:
        img = upscale(src, name, kind)
        how = "up"
    else:
        return kind, name, "missing"
    assert img.shape == (64, 64, 4), (name, img.shape)
    save(img, os.path.join(PACK, kind, name + ".png"))
    return kind, name, how


def contact_sheet(paths, out, cols=16, scale=2):
    T = 64 * scale
    rows = (len(paths) + cols - 1) // cols
    sheet = Image.new("RGBA", (cols * (T + 2) + 2, rows * (T + 2) + 2), (96, 96, 96, 255))
    for i, p in enumerate(paths):
        r, c = divmod(i, cols)
        im = Image.open(p).convert("RGBA").resize((T, T), Image.NEAREST)
        sheet.alpha_composite(im, (2 + c * (T + 2), 2 + r * (T + 2)))
    sheet.save(out)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--preview", help="folder for contact sheets of the result")
    ap.add_argument("--only", help="comma-separated texture names")
    ap.add_argument("--jobs", type=int, default=os.cpu_count() or 2)
    ap.add_argument("--skip-entities", action="store_true")
    args = ap.parse_args()

    n = sync_designs()
    print(f"designs: {n} new or changed 16x16 files copied to {os.path.relpath(SRC, ROOT)}")
    import blocks
    import items
    jobs = []
    for kind in KINDS:
        names = {f[:-4] for f in os.listdir(os.path.join(SRC, kind)) if f.endswith(".png")}
        names.discard("preview")
        jobs += [(kind, nm) for nm in sorted(names)]
    if args.only:
        want = set(args.only.split(","))
        jobs = [j for j in jobs if j[1] in want]
    with Pool(args.jobs) as pool:
        res = pool.map(make, jobs)
    for kind in KINDS:
        hand = sum(1 for k, _, h in res if k == kind and h == "hand")
        up = sum(1 for k, _, h in res if k == kind and h == "up")
        print(f"{kind}: {hand} hand-made, {up} upscaled from 16x16")
    for k, nm, h in res:
        if h == "missing":
            print(f"  missing design: {k}/{nm}")
    if not args.only and not args.skip_entities:
        import entities
        import gui
        entities.main()
        gui.main()
    if args.preview:
        os.makedirs(args.preview, exist_ok=True)
        for kind in KINDS:
            paths = [os.path.join(PACK, kind, nm + ".png") for k, nm, h in res if k == kind and h != "missing"]
            if paths:
                contact_sheet(paths, os.path.join(args.preview, f"{kind}s.png"))
        print(f"preview sheets in {args.preview}")


if __name__ == "__main__":
    main()
