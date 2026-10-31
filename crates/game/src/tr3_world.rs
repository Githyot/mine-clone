//! Lote 22: the world and the character (Terraria).
//!
//! - The moon has eight phases, one a night. A full moon brings more of the night's monsters
//!   and better fishing, a new moon fewer bites. The sky draws the phase (sky.wgsl).
//! - Windy days: one day in four the wind blows hard. Hold a kite outside and it climbs on its
//!   string into the wind, higher the stronger it blows, its tail flapping, all in 3D.
//! - Luck: a garden gnome nearby, a galaxy pearl, the Torch God's favour and the luck effect
//!   make sifting, coins and catches better.
//! - The Torch God: light too many torches too fast and they come alive, whirling round you in
//!   3D and spitting fire for half a minute. Live through it and they settle back as torches of
//!   the land they are in, and you keep the favour: torches you place turn into biome torches.
//! - Permanent upgrades: ambrosia (digs faster), gummy worm (fishing), galaxy pearl (luck),
//!   artisan loaf (meals last longer), minecart kit (faster minecarts).
//! - The altar of fate sets what death costs: Soft (you keep everything, half your coins drop),
//!   Medium (everything stays in your body, as before) or Hard (everything is lost, wallet and
//!   upgrades too).
//! - Favourite items (Alt + click in the bag): never thrown, sold, sorted or put away.
//! - The void bag catches what doesn't fit in the bag; right click to open it.
//! - Emotes (I, then a number): a balloon over your head, which town folk nearby answer, and
//!   which other players see.
//! - Fountains bring the water of their land (and its fish) around them; monoliths, switched
//!   on, turn the sky to their celestial pillar's colours.
//! - Special seeds when making a world: "for the worthy", "not the bees", "the constant" and
//!   "celebration".

use std::collections::HashMap;

use glam::{IVec3, Vec2, Vec3};
use serde::{Deserialize, Serialize};
use voxel::worldgen::Biome;

use crate::audio::Sfx;
use crate::Game;

/// Upgrades: (item, bit, what it says).
const PERMA: &[(&str, u8, &str)] = &[
    ("ambrosia", 1, "Ambrosia: você cava 5% mais rápido para sempre"),
    ("gummy_worm", 2, "Minhoca de Goma: +3 de poder de pesca para sempre"),
    ("galaxy_pearl", 4, "Pérola Galáctica: um pouco mais de sorte para sempre"),
    ("artisan_loaf", 8, "Pão Artesanal: as refeições duram 25% mais para sempre"),
    ("minecart_kit", 16, "Kit do Carrinho: seus carrinhos andam 30% mais rápido"),
];
const MOON: [&str; 8] = ["Lua cheia", "Lua minguante gibosa", "Quarto minguante", "Lua minguante", "Lua nova", "Lua crescente", "Quarto crescente", "Lua crescente gibosa"];
/// Fountains: (block, the water of which land).
const FOUNTAINS: &[(&str, &str)] = &[("desert_fountain", "desert"), ("snow_fountain", "snow"), ("jungle_fountain", "swamp"), ("ocean_fountain", "ocean"), ("evil_fountain", "evil")];
/// Monoliths: (block, sky top, sky horizon, motes).
const MONOLITHS: &[(&str, [f32; 3], [f32; 3], [f32; 3])] = &[
    ("solar_monolith", [0.5, 0.14, 0.04], [0.95, 0.45, 0.15], [2.0, 0.8, 0.2]),
    ("vortex_monolith", [0.02, 0.16, 0.14], [0.1, 0.45, 0.38], [0.4, 1.8, 1.4]),
    ("nebula_monolith", [0.22, 0.03, 0.28], [0.6, 0.2, 0.65], [1.7, 0.5, 1.9]),
    ("stardust_monolith", [0.03, 0.06, 0.25], [0.2, 0.35, 0.75], [0.7, 1.1, 2.2]),
];
/// Emotes: 8x8 pictures, one character per pixel ('.' clear, else a colour from the key).
const EMOTES: &[(&str, [&str; 8])] = &[
    ("Feliz", ["..yyyy..", ".yyyyyy.", "yykyykyy", "yyyyyyyy", "ykyyyyky", "yykkkkyy", ".yyyyyy.", "..yyyy.."]),
    ("Triste", ["..yyyy..", ".yyyyyy.", "yykyykyy", "yybyyyyy", "yybyyyyy", "yyykkyyy", ".ykyyky.", "..yyyy.."]),
    ("Bravo", ["..rrrr..", ".rkrrkr.", "rrkrrkrr", "rrrrrrrr", "rrrkkrrr", "rrkrrkrr", ".rrrrrr.", "..rrrr.."]),
    ("Coração", [".rr..rr.", "rrrrrrrr", "rwrrrrrr", "rrrrrrrr", ".rrrrrr.", "..rrrr..", "...rr...", "........"]),
    ("Pergunta", ["..wwww..", ".ww..ww.", ".....ww.", "....ww..", "...ww...", "...ww...", "........", "...ww..."]),
    ("Exclamação", ["...yy...", "...yy...", "...yy...", "...yy...", "...yy...", "........", "...yy...", "...yy..."]),
    ("Nota", ["...bbbb.", "...b..b.", "...b..b.", "...b..b.", ".bbb.bbb", "bbbbbbbb", "bbb..bbb", "........"]),
    ("Zzz", ["wwww....", "...w....", "..w.....", ".w......", "wwww.ww.", "......w.", ".....w..", "....www."]),
    ("Sol", ["y..y..y.", ".y.yy.y.", "..yyyy..", "yyyyyyyy", "..yyyy..", ".y.yy.y.", "y..y..y.", "........"]),
    ("Caveira", [".wwwwww.", "wwwwwwww", "wkkwwkkw", "wkkwwkkw", "wwwkkwww", ".wwwwww.", ".wkwkwk.", "........"]),
];

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct SavedWorld3 {
    #[serde(default)]
    pub day: u32,
    /// 0 none, 1 has the Torch God's favour (auto torches off), 2 on.
    #[serde(default)]
    pub favor: u8,
    #[serde(default)]
    pub perma: u8,
    /// None is Medium.
    #[serde(default)]
    pub fate: Option<u8>,
    #[serde(default)]
    pub special: String,
    /// Favourite bag slots and the item in each.
    #[serde(default)]
    pub favs: Vec<(u8, String)>,
    /// Monoliths switched on.
    #[serde(default)]
    pub monoliths: Vec<[i32; 3]>,
}

struct God {
    t: f32,
    /// The torches that came alive: where they were and what block.
    flames: Vec<(IVec3, voxel::BlockId)>,
    shot_t: f32,
}

#[derive(Default)]
pub struct World3 {
    pub(crate) s: SavedWorld3,
    favs: Vec<(usize, voxel::item::ItemId)>,
    last_tod: f32,
    was_night: bool,
    told_day: u32,
    kite: Option<u32>,
    string_t: f32,
    torch_log: Vec<f32>,
    god: Option<God>,
    pub(crate) emote_menu: bool,
    /// Balloons: over a mob (net id) or a player by name ("" = you), the emote, until when.
    mob_bubbles: Vec<(u32, u8, f32)>,
    player_bubbles: HashMap<String, (u8, f32)>,
    reply_t: Option<(f32, u8)>,
    fx_t: f32,
    spawn_t: f32,
    dark_t: f32,
    loaded: bool,
}

impl World3 {
    pub fn load(&mut self, s: SavedWorld3) {
        self.s = s;
    }

    pub fn saved(&self, items: &voxel::item::ItemRegistry) -> SavedWorld3 {
        let mut s = self.s.clone();
        s.favs = self.favs.iter().map(|&(k, id)| (k as u8, items.get(id).name.clone())).collect();
        s
    }
}

/// A seed typed when making a world that turns on one of Terraria's secret worlds.
pub(crate) fn special_seed(text: &str) -> Option<&'static str> {
    let t: String = text.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect();
    match t.as_str() {
        "fortheworthy" | "ftw" | "paraosdignos" => Some("worthy"),
        "notthebees" | "ntb" | "asabelhasnao" | "abelhasnao" => Some("bees"),
        "theconstant" | "constant" | "dontstarve" | "naomorradefome" => Some("constant"),
        "celebrationmk10" | "05162020" | "5162020" | "festa" => Some("party"),
        _ => None,
    }
}

fn hash(a: u64, b: u64) -> f32 {
    let mut h = a ^ b.wrapping_mul(0x9E3779B97F4A7C15);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;
    (h % 10_000) as f32 / 10_000.0
}

impl Game {
    // -----------------------------------------------------------------------------------------
    // The moon and the wind.

    /// 0 full .. 4 new .. 7.
    pub(crate) fn moon_phase(&self) -> u8 {
        (self.t5.tr.r3.world.s.day % 8) as u8
    }

    pub(crate) fn windy(&self) -> bool {
        let d = self.t5.tr.r3.world.s.day;
        hash(self.seed, d as u64 + 7) < 0.25 && self.dim == voxel::worldgen::Dim::Overworld
    }

    /// The wind here, stronger on a windy day.
    pub(crate) fn wind_here(&self) -> Vec3 {
        let p = self.body.pos;
        let w = crate::wind::at(self.now, Vec2::new(p.x, p.z), self.weather.rain, self.weather.state.thunder);
        if self.windy() {
            w * 3.5
        } else {
            w
        }
    }

    /// The sky's moon phase for the shader (anim.w): phase + 1.
    pub(crate) fn moon_uniform(&self) -> f32 {
        self.moon_phase() as f32 + 1.0
    }

    // -----------------------------------------------------------------------------------------
    // Luck and upgrades.

    pub(crate) fn has_perma(&self, item: &str) -> bool {
        PERMA.iter().any(|p| p.0 == item && self.t5.tr.r3.world.s.perma & p.1 != 0)
    }

    /// -1 .. 1: how lucky the player is now.
    pub(crate) fn luck(&self) -> f32 {
        use crate::effects::*;
        let mut l = 0.1 * self.effect_level(LUCK) as f32 - 0.1 * self.effect_level(UNLUCK) as f32;
        if self.has_perma("galaxy_pearl") {
            l += 0.1;
        }
        if self.t5.tr.r3.world.s.favor > 0 {
            l += 0.1;
        }
        let me = self.body.pos;
        if self.t5.tr.r3.traps.placed.iter().any(|q| q.1 == 50 && IVec3::from_array(q.0).as_vec3().distance(me) < 20.0) {
            l += 0.2;
        }
        l.clamp(-1.0, 1.0)
    }

    pub(crate) fn tr3_dig_factor(&self) -> f32 {
        if self.has_perma("ambrosia") {
            1.05
        } else {
            1.0
        }
    }

    pub(crate) fn tr3_cart_factor(&self) -> f32 {
        if self.has_perma("minecart_kit") {
            1.3
        } else {
            1.0
        }
    }

    pub(crate) fn tr3_meal_factor(&self) -> f32 {
        if self.has_perma("artisan_loaf") {
            1.25
        } else {
            1.0
        }
    }

    /// Fishing power from the moon and the gummy worm.
    pub(crate) fn tr3_fish_bonus(&self) -> f32 {
        let moon = [10.0, 5.0, 0.0, -5.0, -10.0, -5.0, 0.0, 5.0][self.moon_phase() as usize];
        let moon = if self.is_night() { moon } else { 0.0 };
        moon + if self.has_perma("gummy_worm") { 3.0 } else { 0.0 }
    }

    /// Coins dropped by monsters, for luck and the worthy world.
    pub(crate) fn tr3_coin_factor(&self) -> f32 {
        (1.0 + self.luck() * 0.5) * if self.t5.tr.r3.world.s.special == "worthy" { 1.5 } else { 1.0 }
    }

    /// Damage taken, in the worthy world.
    pub(crate) fn tr3_incoming(&self, amount: f32) -> f32 {
        if self.t5.tr.r3.world.s.special == "worthy" {
            amount * 1.35
        } else {
            amount
        }
    }

    // -----------------------------------------------------------------------------------------
    // Right click with this lote's items and on its blocks.

    pub(crate) fn world3_use(&mut self, name: &str) -> Option<bool> {
        if let Some(&(_, bit, text)) = PERMA.iter().find(|p| p.0 == name) {
            let w = &mut self.t5.tr.r3.world.s;
            if w.perma & bit != 0 {
                self.toast("Você já usou um destes", 2.0);
                return Some(true);
            }
            w.perma |= bit;
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
            self.toast(text, 4.0);
            self.sfx(Sfx::Xp);
            return Some(true);
        }
        match name {
            "void_bag" => Some(self.open_bank(3)),
            "torch_gods_favor" => {
                let w = &mut self.t5.tr.r3.world.s;
                w.favor = if w.favor == 2 { 1 } else { 2 };
                let on = w.favor == 2;
                self.toast(if on { "Tochas de bioma: ligadas. As tochas que você puser viram a tocha da terra onde estão." } else { "Tochas de bioma: desligadas" }, 3.0);
                self.sfx(Sfx::Cast(0));
                Some(true)
            }
            n if n.starts_with("kite_") => Some(true),
            _ => None,
        }
    }

    pub(crate) fn world3_use_block(&mut self, name: &str, at: IVec3) -> bool {
        if name == "fate_altar" {
            let w = &mut self.t5.tr.r3.world.s;
            let next = match w.fate.unwrap_or(1) {
                0 => 1,
                1 => 2,
                _ => 0,
            };
            w.fate = Some(next);
            let text = match next {
                0 => "Destino Suave: ao morrer você guarda tudo e só metade das moedas cai",
                1 => "Destino Médio: ao morrer tudo fica no seu corpo, para buscar",
                _ => "Destino Extremo: ao morrer você perde tudo, carteira e melhorias também",
            };
            self.toast(text, 4.0);
            self.sfx(Sfx::Cast(3));
            return true;
        }
        if MONOLITHS.iter().any(|m| m.0 == name) {
            self.toggle_monolith(at);
            return true;
        }
        false
    }

    pub(crate) fn toggle_monolith(&mut self, at: IVec3) {
        let list = &mut self.t5.tr.r3.world.s.monoliths;
        if let Some(i) = list.iter().position(|&q| q == at.to_array()) {
            list.swap_remove(i);
        } else {
            list.push(at.to_array());
        }
        self.sfx(Sfx::Cast(1));
    }

    /// The sky of a switched-on monolith nearby.
    pub(crate) fn tr3_sky(&self, top: [f32; 3], horizon: [f32; 3], daylight: f32) -> ([f32; 3], [f32; 3]) {
        let Some(k) = self.monolith_here() else { return (top, horizon) };
        let (_, t, h, _) = MONOLITHS[k];
        let a = 0.75;
        let dim = 0.35 + 0.65 * daylight;
        let mix = |x: [f32; 3], y: [f32; 3]| [x[0] * (1.0 - a) + y[0] * a * dim, x[1] * (1.0 - a) + y[1] * a * dim, x[2] * (1.0 - a) + y[2] * a * dim];
        (mix(top, t), mix(horizon, h))
    }

    fn monolith_here(&self) -> Option<usize> {
        let me = self.body.pos;
        let w = &self.t5.tr.r3.world.s;
        let p = w.monoliths.iter().map(|&q| IVec3::from_array(q)).filter(|q| q.as_vec3().distance(me) < 48.0).min_by_key(|q| q.as_vec3().distance(me) as i32)?;
        let name = &self.reg.get(self.world.block_or_air(p)).name;
        MONOLITHS.iter().position(|m| &m.0 == name)
    }

    // -----------------------------------------------------------------------------------------
    // Each frame.

    pub(crate) fn update_world3(&mut self, dt: f32) {
        if !self.t5.tr.r3.world.loaded {
            self.t5.tr.r3.world.loaded = true;
            self.t5.tr.r3.world.last_tod = self.time_of_day;
            let favs = std::mem::take(&mut self.t5.tr.r3.world.s.favs);
            self.t5.tr.r3.world.favs = favs.iter().filter_map(|(k, n)| Some((*k as usize, self.items.id(n)?))).collect();
            let special = self.t5.tr.r3.world.s.special.clone();
            let text = match special.as_str() {
                "worthy" => "Mundo \"Para os Dignos\": os monstros batem mais forte e deixam mais moedas",
                "bees" => "Mundo \"Não as Abelhas\": abelhas por toda parte",
                "constant" => "Mundo \"A Constante\": a fome aperta e algo espreita no escuro. Leve luz.",
                "party" => "Mundo \"Celebração\": a festa não acaba nunca",
                _ => "",
            };
            if !text.is_empty() {
                self.toast(text, 6.0);
            }
        }
        self.count_days(dt);
        self.update_kite(dt);
        self.update_god(dt);
        self.world3_fx(dt);
        self.special_world(dt);
        self.emote_replies();
        let w = &mut self.t5.tr.r3.world;
        let now = self.now;
        w.mob_bubbles.retain(|b| b.2 > now);
        w.player_bubbles.retain(|_, b| b.1 > now);
        // Favourites whose item went away are forgotten.
        let slots = &self.surv.inv.slots;
        w.favs.retain(|&(k, id)| slots.get(k).copied().flatten().is_some_and(|s| s.item == id));
    }

    /// A new day when the clock wraps round; the moon and the wind are told at dusk and dawn.
    fn count_days(&mut self, dt: f32) {
        let tod = self.time_of_day;
        let w = &mut self.t5.tr.r3.world;
        if tod + 1000.0 < w.last_tod {
            w.s.day += 1;
        }
        w.last_tod = tod;
        let night = self.is_night();
        let was = std::mem::replace(&mut self.t5.tr.r3.world.was_night, night);
        if self.dim != voxel::worldgen::Dim::Overworld || self.args.screenshot.is_some() {
            return;
        }
        if night && !was {
            let ph = self.moon_phase();
            let extra = match ph {
                0 => ": mais monstros e peixes mordendo mais",
                4 => ": os peixes quase não mordem",
                _ => "",
            };
            self.toast(format!("{} esta noite{extra}", MOON[ph as usize]), 4.0);
        }
        let day = self.t5.tr.r3.world.s.day;
        if !night && was && self.windy() && self.t5.tr.r3.world.told_day != day {
            self.t5.tr.r3.world.told_day = day;
            self.toast("Dia de vento! Bom para soltar pipa.", 4.0);
        }
        // A full moon brings a few more of the night's walkers.
        let full = self.moon_phase() == 0;
        let w = &mut self.t5.tr.r3.world;
        w.spawn_t -= dt;
        if night && full && w.spawn_t <= 0.0 && self.body.pos.y > 40.0 && self.mobs.monster_room() && !self.mobs.calm {
            w.spawn_t = 25.0;
            if self.count_alive(&["zombie", "skeleton"]) < 8 {
                let name = if self.surv.rng.f32() < 0.6 { "zombie" } else { "skeleton" };
                self.spawn_around(name, false);
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // Kites.

    fn drop_kite(&mut self) {
        if let Some(id) = self.t5.tr.r3.world.kite.take() {
            if let Some(k) = self.mobs.list.iter().position(|m| m.net_id == id) {
                self.mobs.list.remove(k);
            }
        }
    }

    fn update_kite(&mut self, dt: f32) {
        let held = self.held_key();
        let eye = self.body.eye();
        let open = self.world.surface_y(eye.x.floor() as i32, eye.z.floor() as i32).is_none_or(|y| (y as f32) < eye.y) && self.dim == voxel::worldgen::Dim::Overworld;
        if !held.starts_with("kite_") || !open {
            self.drop_kite();
            return;
        }
        // The kite of the held kind.
        let id = self.t5.tr.r3.world.kite;
        let k = id.and_then(|id| self.mobs.list.iter().position(|m| m.net_id == id && m.alive()));
        let k = match k {
            Some(k) if self.mobs.reg.defs[self.mobs.list[k].def].name == held => k,
            _ => {
                self.drop_kite();
                let at = eye + Vec3::Y * 2.0 - self.forward();
                if !self.mobs.spawn(&held, at) {
                    return;
                }
                let m = self.mobs.list.last_mut().unwrap();
                m.ghost = true;
                m.still = true;
                self.t5.tr.r3.world.kite = Some(m.net_id);
                self.mobs.list.len() - 1
            }
        };
        let wind = self.wind_here();
        let strength = ((wind.length() - 0.3) / 1.4).clamp(0.0, 1.0);
        let dir = Vec3::new(wind.x, 0.0, wind.z).normalize_or(Vec3::X);
        let side = Vec3::new(-dir.z, 0.0, dir.x);
        let hand = eye + self.forward() * 0.5 - Vec3::Y * 0.35;
        let t = self.now;
        // Higher and further out the harder it blows; a calm day leaves it low and drooping.
        let up = 2.5 + 13.0 * strength;
        let out = 3.5 + 9.0 * strength;
        let sway = side * ((t * 1.3).sin() * (0.5 + 1.2 * strength)) + Vec3::Y * ((t * 2.1).cos() * 0.5);
        let target = hand + dir * out + Vec3::Y * up + sway;
        let m = &mut self.mobs.list[k];
        let ease = 1.0 - (-2.5 * dt).exp();
        m.body.pos += (target - m.body.pos) * ease;
        m.body.vel = Vec3::ZERO;
        // Facing into the wind, leaning back, the tail flapping.
        m.yaw = (-dir.x).atan2(-dir.z);
        m.head_pitch = -0.6 + (t * 3.0).sin() * 0.1 * strength;
        m.walk += dt * (2.0 + 6.0 * strength);
        m.walk_speed = 0.3 + strength;
        let kite = m.body.pos + Vec3::Y * 0.4;
        // The string: a dotted line that sags a little in the middle.
        let w = &mut self.t5.tr.r3.world;
        w.string_t -= dt;
        if w.string_t <= 0.0 {
            w.string_t = 0.05;
            let glow = self.glow_layer();
            let n = 14;
            for i in 1..n {
                let f = i as f32 / n as f32;
                let sag = (f * std::f32::consts::PI).sin() * (1.6 - strength);
                let p = hand + (kite - hand) * f - Vec3::Y * sag;
                self.particles.glow(p, Vec3::ZERO, glow, [0.35, 0.33, 0.3], 0.018, 0.07);
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // Torches and the Torch God.

    fn biome_torch(&self, p: IVec3) -> &'static str {
        use crate::tr_world::Land;
        if p.y < -600 || self.dim == voxel::worldgen::Dim::Nether {
            return "demon_torch";
        }
        match self.t5.tr.lands.here {
            Some(Land::Prism) => return "hallowed_torch",
            Some(_) => return if p.x.rem_euclid(2) == 0 { "cursed_torch" } else { "ichor_torch" },
            None => {}
        }
        if self.in_dungeon() {
            return "bone_torch";
        }
        match self.world.gen.base_biome_at(p.x, p.z) {
            Biome::Desert | Biome::Badlands | Biome::WoodedBadlands | Biome::ErodedBadlands | Biome::Caatinga => "desert_torch",
            Biome::Snowy | Biome::SnowyTaiga | Biome::SnowyPeaks | Biome::SnowySlopes | Biome::IceSpikes | Biome::FrozenOcean | Biome::SnowyBeach | Biome::Grove => "ice_torch",
            Biome::Jungle | Biome::BambooJungle | Biome::SparseJungle | Biome::MataAtlantica | Biome::Pantanal => "jungle_torch",
            Biome::Ocean | Biome::Beach | Biome::WarmOcean | Biome::LukewarmOcean | Biome::DeepOcean | Biome::ColdOcean => "coral_torch",
            _ => "torch",
        }
    }

    fn in_dungeon(&self) -> bool {
        let p = self.body.pos.floor().as_ivec3();
        let n = &self.reg.get(self.world.block_or_air(p - IVec3::Y)).name;
        n.contains("dungeon") || n.ends_with("_brick_dungeon")
    }

    /// A torch went down: biome torches with the favour, and the Torch God's temper.
    pub(crate) fn world3_placed(&mut self, p: IVec3) {
        let id = self.world.block_or_air(p);
        let base = self.reg.get(id).base;
        if self.reg.get(base).name != "torch" {
            return;
        }
        if self.t5.tr.r3.world.s.favor == 2 {
            let b = self.biome_torch(p);
            if let Some(bid) = self.reg.id(b).filter(|_| b != "torch") {
                let v = self.reg.get(id).variant;
                let nid = self.reg.variant(bid, v);
                self.world.set_block(p, nid);
            }
            return;
        }
        if self.t5.tr.r3.world.s.favor > 0 || self.t5.tr.r3.world.god.is_some() {
            return;
        }
        let now = self.now;
        let log = &mut self.t5.tr.r3.world.torch_log;
        log.push(now);
        log.retain(|&t| now - t < 120.0);
        if log.len() < 40 {
            return;
        }
        // Lots of torches close together: they come alive.
        let me = self.body.pos.floor().as_ivec3();
        let mut found = Vec::new();
        for x in -12..=12 {
            for y in -8..=8 {
                for z in -12..=12 {
                    let q = me + IVec3::new(x, y, z);
                    let b = self.world.block_or_air(q);
                    if b != voxel::AIR && self.reg.get(self.reg.get(b).base).name == "torch" {
                        found.push((q, b));
                    }
                }
            }
        }
        if found.len() < 30 {
            return;
        }
        found.sort_by_key(|(q, _)| (*q - me).length_squared());
        found.truncate(60);
        for &(q, _) in &found {
            self.world.set_block(q, voxel::AIR);
        }
        self.t5.tr.r3.world.torch_log.clear();
        self.t5.tr.r3.world.god = Some(God { t: 0.0, flames: found, shot_t: 2.0 });
        self.toast("As tochas ganharam vida! Aguente o Deus das Tochas por meio minuto.", 5.0);
        self.sfx(Sfx::Thunder(0.3));
    }

    fn update_god(&mut self, dt: f32) {
        let Some(god) = self.t5.tr.r3.world.god.as_mut() else { return };
        god.t += dt;
        god.shot_t -= dt;
        let (t, n) = (god.t, god.flames.len());
        let dead = self.screen == crate::Screen::Dead;
        if t >= 30.0 || dead {
            let god = self.t5.tr.r3.world.god.take().unwrap();
            for (q, b) in god.flames {
                if self.world.block_or_air(q) != voxel::AIR {
                    continue;
                }
                let nb = if dead {
                    b
                } else {
                    let name = self.biome_torch(q);
                    self.reg.id(name).map(|bid| self.reg.variant(bid, self.reg.get(b).variant)).unwrap_or(b)
                };
                self.world.set_block(q, nb);
            }
            if !dead {
                self.t5.tr.r3.world.s.favor = 2;
                self.give_or_drop("torch_gods_favor", 1);
                self.toast("O Deus das Tochas está satisfeito: você ganhou o Favor (tochas de bioma ligadas)", 5.0);
                self.sfx(Sfx::Xp);
            }
            return;
        }
        // The flames whirl round you at different heights and speeds.
        let me = self.body.pos + Vec3::Y;
        let glow = self.glow_layer();
        let now = self.now;
        let mut spots = Vec::with_capacity(n);
        for i in 0..n {
            let ph = i as f32 * 2.399;
            let dir = if i % 2 == 0 { 1.0 } else { -1.0 };
            let a = ph + now * (0.9 + (i % 5) as f32 * 0.15) * dir;
            let r = 4.5 + (i % 4) as f32 * 1.3 + (now * 0.7 + ph).sin() * 0.8;
            let h = (i % 3) as f32 * 1.4 - 0.5 + (now * 1.7 + ph).sin() * 1.2;
            let p = me + Vec3::new(a.cos() * r, h, a.sin() * r);
            spots.push(p);
            self.particles.glow(p, Vec3::Y * 0.4, glow, [2.4, 1.1, 0.3], 0.1, 0.12);
        }
        let god = self.t5.tr.r3.world.god.as_mut().unwrap();
        if god.shot_t <= 0.0 && n > 0 {
            god.shot_t = if t > 20.0 { 0.35 } else { 0.7 };
            let k = (hash(now.to_bits() as u64, n as u64) * n as f32) as usize % n;
            let from = spots[k];
            let vel = (self.body.eye() - from).normalize_or_zero() * 11.0;
            self.enemy_bolt(from, vel, 0, 5.0);
        }
    }

    // -----------------------------------------------------------------------------------------
    // Fountains, monoliths and the special worlds.

    /// The water of a fountain nearby, for fishing.
    pub(crate) fn tr3_fountain_kind(&self, at: Vec3) -> Option<&'static str> {
        self.t5.tr.r3.traps.placed.iter().filter(|q| (30..35).contains(&q.1) && IVec3::from_array(q.0).as_vec3().distance(at) < 24.0).map(|q| FOUNTAINS[(q.1 - 30) as usize].1).next()
    }

    fn world3_fx(&mut self, dt: f32) {
        let w = &mut self.t5.tr.r3.world;
        w.fx_t -= dt;
        if w.fx_t > 0.0 {
            return;
        }
        w.fx_t = 0.05;
        let me = self.body.pos;
        let glow = self.glow_layer();
        // Fountains spray.
        let near: Vec<(IVec3, u8)> = self.t5.tr.r3.traps.placed.iter().filter(|q| (30..35).contains(&q.1)).map(|q| (IVec3::from_array(q.0), q.1)).filter(|q| q.0.as_vec3().distance(me) < 32.0).collect();
        for (p, k) in near {
            let tint = match FOUNTAINS[(k - 30) as usize].1 {
                "desert" => [0.5, 0.8, 0.85],
                "snow" => [0.7, 0.85, 1.0],
                "swamp" => [0.3, 0.75, 0.5],
                "evil" => [0.6, 0.4, 0.8],
                _ => [0.3, 0.55, 0.95],
            };
            let top = p.as_vec3() + Vec3::new(0.5, 0.95, 0.5);
            for i in 0..3 {
                let a = self.now * 3.0 + i as f32 * 2.1;
                self.particles.glow(top, Vec3::new(a.cos() * 0.9, 2.6, a.sin() * 0.9), glow, tint, 0.035, 0.55);
            }
        }
        // A monolith's sky: motes drifting high up.
        if let Some(k) = self.monolith_here() {
            let tint = MONOLITHS[k].3;
            for _ in 0..2 {
                let off = Vec3::new(self.surv.rng.f32() - 0.5, 0.0, self.surv.rng.f32() - 0.5) * 60.0;
                self.particles.glow(me + off + Vec3::Y * (14.0 + self.surv.rng.f32() * 10.0), Vec3::new(0.3, -0.6, 0.2), glow, tint, 0.12, 2.5);
            }
        }
    }

    fn special_world(&mut self, dt: f32) {
        let special = self.t5.tr.r3.world.s.special.clone();
        if special.is_empty() || !self.survival {
            return;
        }
        match special.as_str() {
            "bees" => {
                let w = &mut self.t5.tr.r3.world;
                w.spawn_t -= dt;
                if w.spawn_t <= 0.0 {
                    w.spawn_t = 9.0;
                    if !self.is_night() && self.count_alive(&["bee"]) < 8 && self.body.pos.y > 40.0 {
                        self.spawn_around("bee", true);
                    }
                }
            }
            "constant" => {
                // Hunger bites harder, and the dark is not safe.
                self.surv.exhaustion += dt * 0.04;
                let eye = self.body.eye().floor().as_ivec3();
                let l = self.world.light(eye);
                let sky = if self.is_night() { 0 } else { l >> 4 };
                let lit = (l & 15).max(sky) >= 4 || self.hand_light_level() > 0.2;
                let w = &mut self.t5.tr.r3.world;
                if lit || self.screen == crate::Screen::Dead {
                    w.dark_t = 0.0;
                    return;
                }
                w.dark_t += dt;
                if w.dark_t > 5.0 {
                    w.dark_t = 3.0;
                    self.toast("Algo te ataca no escuro!", 2.0);
                    self.damage(3.0, None);
                }
            }
            "party" => {
                if self.t5.tr.town.party <= 0.0 {
                    self.start_party(600.0);
                }
            }
            _ => {}
        }
    }

    // -----------------------------------------------------------------------------------------
    // Death, favourites and the void bag.

    /// 0 soft, 1 medium, 2 hard.
    pub(crate) fn fate(&self) -> u8 {
        self.t5.tr.r3.world.s.fate.unwrap_or(1)
    }

    /// Soft death: everything stays, half the wallet falls where you died.
    pub(crate) fn soft_death(&mut self) {
        let half = self.surv.money / 2;
        if half > 0 {
            self.surv.money -= half;
            let at = self.body.pos + Vec3::Y * 0.5;
            self.drop_coins(at, half);
            self.toast("Destino Suave: você guardou seus itens; metade das moedas ficou no chão", 6.0);
        }
    }

    /// Hard death: the wallet and the upgrades are gone with the items.
    pub(crate) fn hard_death(&mut self) {
        self.surv.money = 0;
        self.t5.tr.r3.world.s.perma = 0;
        self.t5.tr.util.perma = 0;
        self.toast("Destino Extremo: você perdeu tudo. Recomece com coragem.", 8.0);
    }

    pub(crate) fn is_favorite(&self, k: usize) -> bool {
        self.t5.tr.r3.world.favs.iter().any(|f| f.0 == k) && self.surv.inv.slots.get(k).copied().flatten().is_some_and(|s| self.t5.tr.r3.world.favs.iter().any(|f| f.0 == k && f.1 == s.item))
    }

    /// Alt + click on a bag slot: favourite on or off.
    pub(crate) fn toggle_favorite(&mut self, k: usize) {
        let Some(st) = self.surv.inv.slots.get(k).copied().flatten() else { return };
        let favs = &mut self.t5.tr.r3.world.favs;
        if let Some(i) = favs.iter().position(|f| f.0 == k) {
            favs.swap_remove(i);
            self.toast("Item desmarcado", 1.0);
        } else {
            favs.push((k, st.item));
            self.toast("Favorito: não é jogado fora, vendido, organizado nem guardado", 2.0);
        }
        self.sfx(Sfx::Click);
    }

    /// What doesn't fit in the bag goes into the void bag, if you carry one.
    pub(crate) fn void_stash(&mut self, st: voxel::item::ItemStack) -> Option<voxel::item::ItemStack> {
        let Some(id) = self.items.id("void_bag") else { return Some(st) };
        if !self.surv.inv.slots.iter().flatten().any(|s| s.item == id) {
            return Some(st);
        }
        let p = IVec3::new(crate::tr_utility::BANK_X, 0, 3);
        let c = self.surv.containers.entry(p).or_insert_with(|| crate::survival::Container::Chest(voxel::item::Inventory::new(36)));
        match c {
            crate::survival::Container::Chest(inv) => inv.add(&self.items, st),
            _ => Some(st),
        }
    }

    // -----------------------------------------------------------------------------------------
    // Emotes.

    /// I: the emote row opens; a number picks one.
    pub(crate) fn emote_key(&mut self) {
        let w = &mut self.t5.tr.r3.world;
        w.emote_menu = !w.emote_menu;
    }

    /// A number key while the emote row is open. True when it was taken.
    pub(crate) fn emote_pick(&mut self, i: usize) -> bool {
        if !self.t5.tr.r3.world.emote_menu || i >= EMOTES.len() {
            return false;
        }
        self.t5.tr.r3.world.emote_menu = false;
        self.emote(i as u8);
        true
    }

    fn emote(&mut self, e: u8) {
        let until = self.now + 4.0;
        self.t5.tr.r3.world.player_bubbles.insert(String::new(), (e, until));
        self.send_emote(e);
        // Town folk nearby answer in a moment.
        self.t5.tr.r3.world.reply_t = Some((self.now + 0.8, e));
    }

    fn emote_replies(&mut self) {
        let Some((t, e)) = self.t5.tr.r3.world.reply_t else { return };
        if self.now < t {
            return;
        }
        self.t5.tr.r3.world.reply_t = None;
        let me = self.body.pos;
        let near: Vec<u32> = self.mobs.list.iter().filter(|m| m.alive() && m.home.is_some() && m.body.pos.distance(me) < 12.0).map(|m| m.net_id).take(3).collect();
        for (i, id) in near.into_iter().enumerate() {
            // Happy for happy, a heart for a heart, a question for most others.
            let r = match e {
                0 | 3 | 8 => e,
                1 => 3,
                2 => 1,
                7 => 7,
                _ => 4,
            };
            let r = if i == 1 && r == 4 { 6 } else { r };
            self.t5.tr.r3.world.mob_bubbles.push((id, r, self.now + 3.0 + i as f32 * 0.3));
        }
    }

    /// Another player emoted (multiplayer.rs passes chat lines here first). True when it was one.
    pub(crate) fn net_emote(&mut self, from: &str, text: &str) -> bool {
        let Some(e) = text.strip_prefix("\u{1}E").and_then(|t| t.parse::<u8>().ok()) else { return false };
        if (e as usize) < EMOTES.len() {
            let until = self.now + 4.0;
            self.t5.tr.r3.world.player_bubbles.insert(from.to_string(), (e, until));
        }
        true
    }

    fn draw_emote(&mut self, e: u8, x: f32, y: f32, px: f32, alpha: u8) {
        let pic = EMOTES[e as usize % EMOTES.len()].1;
        let size = 8.0 * px;
        // The balloon, with its little tail.
        self.ui.rect(x - size / 2.0 - 2.0 * px, y - size - 4.0 * px, size + 4.0 * px, size + 4.0 * px, [30, 30, 34, alpha]);
        self.ui.rect(x - size / 2.0 - px, y - size - 3.0 * px, size + 2.0 * px, size + 2.0 * px, [245, 245, 240, alpha]);
        self.ui.rect(x - px, y, 2.0 * px, px, [245, 245, 240, alpha]);
        for (r, row) in pic.iter().enumerate() {
            for (c, ch) in row.chars().enumerate() {
                let col = match ch {
                    'y' => [250, 200, 40],
                    'k' => [40, 30, 20],
                    'r' => [220, 50, 50],
                    'w' => [90, 90, 100],
                    'b' => [50, 90, 200],
                    _ => continue,
                };
                let col = if ch == 'w' && pic.iter().any(|r| r.contains('k')) { [235, 235, 230] } else { col };
                self.ui.rect(x - size / 2.0 + c as f32 * px, y - size - 2.0 * px + r as f32 * px, px, px, [col[0], col[1], col[2], alpha]);
            }
        }
    }

    /// Balloons over heads, the emote row and the Torch God's bar.
    pub(crate) fn draw_tr3_hud(&mut self, w: f32, h: f32) {
        let s = self.ui.scale;
        let px = (s * 1.5).round().max(1.0);
        if let Some((vp, cam_block)) = self.ui_view_proj {
            let now = self.now;
            let mut spots: Vec<(Vec3, u8, f32)> = Vec::new();
            for &(id, e, until) in &self.t5.tr.r3.world.mob_bubbles {
                if let Some(m) = self.mobs.list.iter().find(|m| m.net_id == id && m.alive()) {
                    let def = &self.mobs.reg.defs[m.def];
                    spots.push((m.body.pos + Vec3::Y * (def.height * m.size.max(1.0) + 0.6), e, until));
                }
            }
            for (name, &(e, until)) in &self.t5.tr.r3.world.player_bubbles {
                if name.is_empty() {
                    if self.camera_mode != 0 {
                        spots.push((self.body.pos + Vec3::Y * 2.4, e, until));
                    }
                } else if let Some(r) = self.net.remotes.values().find(|r| &r.name == name) {
                    spots.push((r.shown + Vec3::Y * 2.4, e, until));
                }
            }
            for (p, e, until) in spots {
                if !self.in_sight(&[p]) {
                    continue;
                }
                let c = vp * (p - cam_block.as_vec3()).extend(1.0);
                if c.w < 0.1 || (c.x / c.w).abs() > 1.05 || (c.y / c.w).abs() > 1.05 {
                    continue;
                }
                let (x, y) = ((c.x / c.w * 0.5 + 0.5) * w, (0.5 - c.y / c.w * 0.5) * h);
                let alpha = ((until - now) * 2.0).clamp(0.0, 1.0);
                self.draw_emote(e, x, y, px, (alpha * 255.0) as u8);
            }
        }
        // First person: your own balloon at the top of the screen.
        if self.camera_mode == 0 {
            if let Some(&(e, until)) = self.t5.tr.r3.world.player_bubbles.get("") {
                let alpha = ((until - self.now) * 2.0).clamp(0.0, 1.0);
                self.draw_emote(e, w / 2.0, 70.0 * s, px, (alpha * 255.0) as u8);
            }
        }
        if self.t5.tr.r3.world.emote_menu {
            let n = EMOTES.len();
            let cell = 14.0 * px;
            let x0 = w / 2.0 - n as f32 * cell / 2.0;
            let y0 = h * 0.3;
            self.ui.rect(x0 - 4.0 * px, y0 - cell - 14.0 * px, n as f32 * cell + 8.0 * px, cell + 22.0 * px, [0, 0, 0, 140]);
            for i in 0..n {
                let x = x0 + i as f32 * cell + cell / 2.0;
                self.draw_emote(i as u8, x, y0, px, 255);
                let label = format!("{}", (i + 1) % 10);
                self.ui.text(x - 3.0 * px, y0 + 2.0 * px, &label, [255, 255, 255, 255]);
            }
            self.ui.text_centered(w / 2.0, y0 - cell - 12.0 * px, "Emotes: aperte um número", [255, 240, 200, 255]);
        }
        if let Some(god) = &self.t5.tr.r3.world.god {
            let left = (30.0 - god.t).max(0.0);
            let bw = 200.0 * s;
            self.ui.rect(w / 2.0 - bw / 2.0, 30.0 * s, bw, 6.0 * s, [40, 20, 10, 200]);
            self.ui.rect(w / 2.0 - bw / 2.0, 30.0 * s, bw * left / 30.0, 6.0 * s, [255, 150, 40, 230]);
            self.ui.text_centered(w / 2.0, 18.0 * s, "Deus das Tochas", [255, 200, 120, 255]);
        }
    }

    /// Test scenes: MC_TR=world3 (the new blocks, a kite in the wind, emotes) and livingtree.
    pub(crate) fn test_world3(&mut self, scene: &str) {
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = flat.cross(Vec3::Y);
        let spot = |g: &Game, d: f32, s: f32| {
            let p = g.body.pos + flat * d + side * s;
            IVec3::new(p.x.floor() as i32, g.body.pos.y.floor() as i32, p.z.floor() as i32)
        };
        match scene {
            "world3" => {
                let names = [
                    "desert_torch", "ice_torch", "jungle_torch", "coral_torch", "bone_torch", "demon_torch", "cursed_torch", "ichor_torch", "hallowed_torch",
                ];
                for (i, n) in names.iter().enumerate() {
                    let p = spot(self, 4.0, -4.0 + i as f32);
                    if let Some(id) = self.reg.id(n) {
                        self.world.set_block(p - IVec3::Y, self.reg.id_or_air("stone"));
                        self.world.set_block(p, id);
                    }
                }
                let row2 = ["desert_fountain", "snow_fountain", "solar_monolith", "nebula_monolith", "garden_gnome", "fate_altar", "neon_moss", "lava_moss", "xenon_moss", "ruby_gem_leaves", "diamond_gem_leaves", "living_wood"];
                for (i, n) in row2.iter().enumerate() {
                    let p = spot(self, 7.0, -6.0 + i as f32);
                    if let Some(id) = self.reg.id(n) {
                        self.world.set_block(p - IVec3::Y, self.reg.id_or_air("stone"));
                        self.world.set_block(p, id);
                        self.tr3_placed(p, n);
                    }
                }
                self.t5.tr.r3.world.player_bubbles.insert(String::new(), (0, self.now + 100.0));
                self.t5.tr.r3.world.emote_menu = true;
                if let Some(id) = self.items.id("kite_wyvern") {
                    self.surv.inv.slots[self.selected] = Some(voxel::item::ItemStack::new(id, 1));
                }
                // A windy day, and the kite already up.
                while !self.windy() {
                    self.t5.tr.r3.world.s.day += 1;
                }
                for _ in 0..200 {
                    self.update_kite(0.05);
                }
            }
            _ => {
                // Where the nearest living tree is (run again with --pos there to see it).
                let p = self.body.pos.floor().as_ivec3();
                match self.world.gen.find_structure("arvore_viva", p.x, p.z, 12) {
                    Some((o, size)) => println!("arvore_viva at {} {} {} size {} {} {}", o.x, o.y, o.z, size.x, size.y, size.z),
                    None => println!("arvore_viva: none near"),
                }
            }
        }
    }
}
