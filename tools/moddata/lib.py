"""Helpers shared by the mod data modules (see tools/gen_mod_data.py)."""

blocks = []
items = []
shaped = []
shapeless = []
smelting = []
tags = {}


def q(s):
    return '"' + s.replace('"', '\\"') + '"'


def opt(v):
    return f"Some({v})"


def B(name, display, **kw):
    """One block. Keyword arguments are written as they are (RON values as strings)."""
    parts = [f"name: {q(name)}", f"display_name: Some({q(display)})"]
    for k, v in kw.items():
        parts.append(f"{k}: {v}")
    blocks.append("(" + ", ".join(parts) + ")")


def tex(**faces):
    return "(" + ", ".join(f"{k}: Some({q(v)})" for k, v in faces.items()) + ")"


def drops(*entries):
    out = []
    for e in entries:
        if isinstance(e, str):
            out.append(f"(item: {q(e)})")
        else:
            name, mn, mx, *ch = e
            s = f"(item: {q(name)}, min: {mn}, max: {mx}"
            if ch:
                s += f", chance: {ch[0]}"
            out.append(s + ")")
    return "Some([" + ", ".join(out) + "])"


def It(name, display, **kw):
    parts = [f"name: {q(name)}", f"display_name: Some({q(display)})"]
    for k, v in kw.items():
        parts.append(f"{k}: {v}")
    items.append("(" + ", ".join(parts) + ")")


def tool(kind, tier, speed, damage, durability):
    return f"Some((kind: {kind}, tier: {tier}, speed: {speed}, damage: {damage}, durability: {durability}))"


def armor(slot, points, durability):
    return f"Some((slot: {slot}, points: {points}, durability: {durability}))"


def food(hunger, saturation):
    return f"Some((hunger: {hunger}, saturation: {saturation}))"


def R(pattern, key, result, count=1):
    keys = ", ".join(f"'{k}': {q(v)}" for k, v in key.items())
    pat = ", ".join(q(r) for r in pattern)
    c = f", count: {count}" if count != 1 else ""
    shaped.append(f"(pattern: [{pat}], key: {{{keys}}}, result: {q(result)}{c})")


def S(ingredients, result, count=1):
    ing = ", ".join(q(i) for i in ingredients)
    c = f", count: {count}" if count != 1 else ""
    shapeless.append(f"(ingredients: [{ing}], result: {q(result)}{c})")


def F(inp, result, xp=0.1):
    smelting.append(f"(input: {q(inp)}, result: {q(result)}, xp: {xp})")


def tag(name, *members):
    tags.setdefault(name, []).extend(members)


