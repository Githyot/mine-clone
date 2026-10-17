"""Immersive Aircraft's idea: five aircraft built from crafted parts (hull, sail, engine,
propeller), a cannon for the warship and four upgrades. Flight is in
crates/game/src/mods/aircraft.rs."""

from .lib import It, R, S

It("aircraft_hull", "Casco de Aeronave")
It("aircraft_sail", "Vela de Aeronave")
It("aircraft_engine", "Motor de Aeronave")
It("aircraft_propeller", "Hélice")
It("rotary_cannon", "Canhão Rotativo")
R(["P P", "PIP", "PPP"], {"P": "#planks", "I": "iron_ingot"}, "aircraft_hull")
R(["WSW", "WSW", "WSW"], {"W": "white_wool", "S": "string"}, "aircraft_sail")
R(["ICI", "IFI", "IRI"], {"I": "iron_ingot", "C": "copper_ingot", "F": "furnace", "R": "redstone"}, "aircraft_engine")
R([" I ", "PIP", " I "], {"I": "iron_ingot", "P": "#planks"}, "aircraft_propeller")
R(["III", "GDG"], {"I": "iron_ingot", "G": "gunpowder", "D": "dispenser"}, "rotary_cannon")

for name, display in (("biplane", "Biplano"), ("airship", "Dirigível"), ("cargo_airship", "Dirigível de Carga"),
                      ("gyrodyne", "Girodino"), ("quadrocopter", "Quadricóptero"), ("warship", "Navio de Guerra Aéreo")):
    It(name, display, max_stack="Some(1)")
R([" P ", "SES", " H "], {"P": "aircraft_propeller", "S": "aircraft_sail", "E": "aircraft_engine", "H": "aircraft_hull"}, "biplane")
R(["SSS", "PEP", " H "], {"P": "aircraft_propeller", "S": "aircraft_sail", "E": "aircraft_engine", "H": "aircraft_hull"}, "airship")
S(["airship", "chest"], "cargo_airship")
R(["PPP", " S ", "SHS"], {"P": "aircraft_propeller", "S": "aircraft_sail", "H": "aircraft_hull"}, "gyrodyne")
R(["P P", "|E|", "P P"], {"P": "aircraft_propeller", "|": "stick", "E": "aircraft_engine"}, "quadrocopter")
R(["SSS", "CEC", "HHH"], {"S": "aircraft_sail", "C": "rotary_cannon", "E": "aircraft_engine", "H": "aircraft_hull"}, "warship")

It("enhanced_propeller", "Hélice Aprimorada", max_stack="Some(1)")
It("eco_engine", "Motor Econômico", max_stack="Some(1)")
It("nether_engine", "Motor do Nether", max_stack="Some(1)")
It("steel_hull", "Casco de Aço", max_stack="Some(1)")
R(["D", "P", "D"], {"D": "diamond", "P": "aircraft_propeller"}, "enhanced_propeller")
R(["G G", " E ", "GRG"], {"G": "gold_ingot", "E": "aircraft_engine", "R": "redstone"}, "eco_engine")
R(["B B", " E ", "BOB"], {"B": "blaze_rod", "E": "aircraft_engine", "O": "obsidian"}, "nether_engine")
R(["III", "IHI", "III"], {"I": "iron_ingot", "H": "aircraft_hull"}, "steel_hull")
