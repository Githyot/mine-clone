"""Spawn eggs for the creatures of the Aether and the Otherside (tools/dim_mobs_spec.py)."""

import dim_mobs_spec as spec

from .lib import It

for _m in spec.MOBS:
    if _m["name"] in spec.EGGS:
        It(f"{_m['name']}_spawn_egg", f"Ovo de {_m['display']}")
