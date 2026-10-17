"""Snow! Real Magic!: snow-covered slabs, stairs, fences and walls (one snowy twin for each of the
base game's, with a white top) and plants buried under snow that come back when it melts.
They are made by the snowfall (crates/game/src/mods/snow.rs), never placed by hand."""

import os
import re

from .lib import B, drops, q, tex

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
BASE = os.path.join(ROOT, "packs", "vanilla", "blocks.ron")

for line in open(BASE, encoding="utf-8"):
    m = re.search(r'\(name: "([^"]+)".*?shape: (Slab|Stairs|Fence|Wall)\b', line)
    if not m:
        continue
    name, shape = m.group(1), m.group(2)
    disp = re.search(r'display_name: Some\("([^"]+)"\)', line)
    t = re.search(r'textures: \(((?:\w+: Some\("[^"]+"\)(?:, )?)+)\)', line)
    faces = dict(re.findall(r'(\w+): Some\("([^"]+)"\)', t.group(1))) if t else {}
    side = faces.get("side") or faces.get("all") or name
    bottom = faces.get("bottom") or faces.get("all") or side
    hard = re.search(r"hardness: ([\d.]+)", line)
    tool_m = re.search(r"tool: Some\((\w+)\)", line)
    tier = re.search(r"tier: (\d+)", line)
    kw = dict(shape=shape, hardness=hard.group(1) if hard else "1.0", in_inventory="false",
              textures=tex(top="snow", side=side, bottom=bottom), drops=drops(name))
    if tool_m:
        kw["tool"] = f"Some({tool_m.group(1)})"
    if tier:
        kw["tier"] = tier.group(1)
    B(f"snowy_{name}", (disp.group(1) if disp else name) + " com Neve", **kw)

# Plants under the snow: a snow layer that remembers the plant beneath.
for plant, pt in (("short_grass", "Grama"), ("poppy", "Papoula"), ("dandelion", "Dente-de-leão"),
                  ("dead_bush", "Arbusto Seco"), ("twig", "Gravetos"), ("pebble", "Pedrinhas")):
    B(f"snowy_{plant}", f"{pt} sob a Neve", shape="Layer", hardness="0.1", tool="Some(Shovel)", in_inventory="false",
      textures=tex(all="snow"), drops="Some([" + f"(item: {q('snow_layer')})" + "])")
