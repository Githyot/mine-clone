"""Immersive Melodies: five more instruments, all tuned low (the lute, flute and cello are in
the base game). Melodies and how they sound are in crates/game/src/mods/melodies.rs."""

from .lib import It, R

It("didgeridoo", "Didjeridu", max_stack="Some(1)")
R(["  #", " # ", "#  "], {"#": "#logs"}, "didgeridoo")
It("bagpipe", "Gaita de Foles", max_stack="Some(1)")
R(["|||", " L ", " L "], {"|": "stick", "L": "leather"}, "bagpipe")
It("piano", "Piano de Mão", max_stack="Some(1)")
R(["BWB", "###"], {"B": "bone", "W": "white_wool", "#": "#planks"}, "piano")
It("tiny_drum", "Tamborzinho", max_stack="Some(1)")
R(["L", "#"], {"L": "leather", "#": "#planks"}, "tiny_drum")
It("handpan", "Handpan", max_stack="Some(1)")
R([" I ", "I I", " I "], {"I": "iron_ingot"}, "handpan")
