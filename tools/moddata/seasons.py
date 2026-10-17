"""Serene Seasons: season sensors and the calendar."""

from .lib import B, It, R, drops, tex

SEASONS = [("spring", "Primavera"), ("summer", "Verão"), ("autumn", "Outono"), ("winter", "Inverno")]
for s, pt in SEASONS:
    for on in (False, True):
        n = f"season_sensor_{s}" + ("_on" if on else "")
        extra = {} if (s == "spring" and not on) else {"in_inventory": "false"}
        B(n, f"Sensor de Estação ({pt})", hardness="0.6", tool="Some(Axe)", height="Some(6)", shape="Slab",
          textures=tex(top=f"season_sensor_{s}" + ("_on" if on else ""), side="season_sensor_side", bottom="oak_planks"),
          drops=drops("season_sensor_spring"), **extra)
It("calendar", "Calendário")
R(["PPP", "PRP", "PPP"], {"P": "#planks", "R": "redstone"}, "calendar")
R(["GGG", "QCQ", "WWW"], {"G": "glass", "Q": "quartz", "C": "calendar", "W": "oak_slab"}, "season_sensor_spring")


