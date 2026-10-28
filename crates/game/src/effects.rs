//! Status effects (Phase 6): what potions, beacons, conduits, food and some monsters do to the
//! player for a while — faster or slower, stronger, harder to hurt, breathing under water,
//! poisoned, blinded... — the potions themselves (drunk, thrown or left as a cloud), and the
//! icons down the right side of the screen.

use glam::Vec3;
use voxel::item::ItemStack;

use crate::audio::{Material, Sfx};
use crate::Game;

pub const SPEED: u8 = 1;
pub const SLOWNESS: u8 = 2;
pub const HASTE: u8 = 3;
pub const MINING_FATIGUE: u8 = 4;
pub const STRENGTH: u8 = 5;
pub const INSTANT_HEALTH: u8 = 6;
pub const INSTANT_DAMAGE: u8 = 7;
pub const JUMP_BOOST: u8 = 8;
pub const NAUSEA: u8 = 9;
pub const REGENERATION: u8 = 10;
pub const RESISTANCE: u8 = 11;
pub const FIRE_RESISTANCE: u8 = 12;
pub const WATER_BREATHING: u8 = 13;
pub const INVISIBILITY: u8 = 14;
pub const BLINDNESS: u8 = 15;
pub const NIGHT_VISION: u8 = 16;
pub const HUNGER: u8 = 17;
pub const WEAKNESS: u8 = 18;
pub const POISON: u8 = 19;
pub const WITHER: u8 = 20;
pub const HEALTH_BOOST: u8 = 21;
pub const ABSORPTION: u8 = 22;
pub const SATURATION: u8 = 23;
pub const GLOWING: u8 = 24;
pub const LEVITATION: u8 = 25;
pub const LUCK: u8 = 26;
pub const UNLUCK: u8 = 27;
pub const SLOW_FALLING: u8 = 28;
pub const CONDUIT_POWER: u8 = 29;
pub const DOLPHINS_GRACE: u8 = 30;
pub const BAD_OMEN: u8 = 31;
pub const HERO: u8 = 32;
pub const DARKNESS: u8 = 33;
pub const TRIAL_OMEN: u8 = 34;
pub const RAID_OMEN: u8 = 35;
pub const WIND_CHARGED: u8 = 36;
pub const WEAVING: u8 = 37;
pub const OOZING: u8 = 38;
pub const INFESTED: u8 = 39;
pub const BREATH_OF_THE_NAUTILUS: u8 = 40;
// Terraria-style buffs and debuffs (rpg.rs gives most of them their meaning).
pub const BLEEDING: u8 = 41;
pub const FROSTBURN: u8 = 42;
pub const CURSED_FLAMES: u8 = 43;
pub const ICHOR: u8 = 44;
pub const BROKEN_ARMOR: u8 = 45;
pub const CONFUSED: u8 = 46;
pub const SILENCED: u8 = 47;
pub const CHILLED: u8 = 48;
pub const FROZEN: u8 = 49;
pub const VENOM: u8 = 50;
pub const POTION_SICKNESS: u8 = 51;
pub const MANA_SICKNESS: u8 = 52;
pub const IRONSKIN: u8 = 53;
pub const MANA_REGEN: u8 = 54;
pub const MAGIC_POWER: u8 = 55;
pub const ARCHERY: u8 = 56;
pub const WRATH: u8 = 57;
pub const RAGE: u8 = 58;
pub const ENDURANCE: u8 = 59;
pub const LIFEFORCE: u8 = 60;
pub const THORNS: u8 = 61;
pub const TITAN: u8 = 62;
pub const SUMMONING: u8 = 63;
pub const SPELUNKER: u8 = 64;
pub const HUNTER: u8 = 65;
pub const SHINE: u8 = 66;
pub const WELL_FED: u8 = 67;
pub const COZY_FIRE: u8 = 68;
pub const BUILDER: u8 = 69;
pub const WARMTH: u8 = 70;
pub const INFERNO: u8 = 71;
// Comfort at home (tr_comfort.rs).
pub const HEART_LAMP: u8 = 72;
pub const STAR_BOTTLE: u8 = 73;
pub const CLAIRVOYANCE: u8 = 74;
pub const SHARPENED: u8 = 75;
pub const AMMO_BOX: u8 = 76;
pub const STRATEGIST: u8 = 77;
pub const BEWITCHED: u8 = 78;
pub const BATTLE: u8 = 79;
pub const CALM: u8 = 80;
pub const HUNTING: u8 = 81;

/// Name, colour (for the icon, particles and the potion), harmful.
pub fn info(id: u8) -> (&'static str, [u8; 3], bool) {
    match id {
        SPEED => ("Velocidade", [51, 235, 255], false),
        SLOWNESS => ("Lentidão", [139, 175, 224], true),
        HASTE => ("Pressa", [217, 192, 67], false),
        MINING_FATIGUE => ("Fadiga de Mineração", [74, 66, 23], true),
        STRENGTH => ("Força", [255, 199, 0], false),
        INSTANT_HEALTH => ("Vida Instantânea", [248, 36, 35], false),
        INSTANT_DAMAGE => ("Dano Instantâneo", [169, 101, 106], true),
        JUMP_BOOST => ("Super Pulo", [253, 255, 132], false),
        NAUSEA => ("Náusea", [85, 29, 74], true),
        REGENERATION => ("Regeneração", [205, 92, 171], false),
        RESISTANCE => ("Resistência", [145, 70, 240], false),
        FIRE_RESISTANCE => ("Resistência ao Fogo", [255, 153, 0], false),
        WATER_BREATHING => ("Respiração Aquática", [152, 218, 192], false),
        INVISIBILITY => ("Invisibilidade", [246, 246, 246], false),
        BLINDNESS => ("Cegueira", [31, 31, 35], true),
        NIGHT_VISION => ("Visão Noturna", [194, 255, 102], false),
        HUNGER => ("Fome", [88, 118, 83], true),
        WEAKNESS => ("Fraqueza", [72, 77, 72], true),
        POISON => ("Veneno", [135, 163, 99], true),
        WITHER => ("Murchar", [115, 97, 86], true),
        HEALTH_BOOST => ("Vida Extra", [248, 125, 35], false),
        ABSORPTION => ("Absorção", [37, 82, 165], false),
        SATURATION => ("Saturação", [248, 36, 35], false),
        GLOWING => ("Brilho", [148, 160, 97], false),
        LEVITATION => ("Levitação", [206, 255, 255], true),
        LUCK => ("Sorte", [89, 193, 6], false),
        UNLUCK => ("Azar", [192, 164, 77], true),
        SLOW_FALLING => ("Queda Lenta", [243, 207, 185], false),
        CONDUIT_POWER => ("Poder do Conduíte", [29, 194, 209], false),
        DOLPHINS_GRACE => ("Graça do Golfinho", [136, 163, 190], false),
        BAD_OMEN => ("Mau Presságio", [11, 97, 56], true),
        HERO => ("Herói da Vila", [68, 255, 68], false),
        DARKNESS => ("Escuridão", [41, 39, 33], true),
        TRIAL_OMEN => ("Presságio do Desafio", [22, 166, 166], true),
        RAID_OMEN => ("Presságio de Invasão", [222, 64, 88], true),
        WIND_CHARGED => ("Carregado de Vento", [189, 201, 255], true),
        WEAVING => ("Tecelagem", [120, 105, 90], true),
        OOZING => ("Gosma", [153, 255, 163], true),
        INFESTED => ("Infestação", [140, 155, 140], true),
        BREATH_OF_THE_NAUTILUS => ("Fôlego do Náutilo", [0, 140, 160], false),
        BLEEDING => ("Sangramento", [170, 20, 30], true),
        FROSTBURN => ("Queimadura de Gelo", [120, 200, 255], true),
        CURSED_FLAMES => ("Chamas Malditas", [140, 230, 40], true),
        ICHOR => ("Icor", [235, 200, 40], true),
        BROKEN_ARMOR => ("Armadura Quebrada", [150, 150, 160], true),
        CONFUSED => ("Confusão", [200, 120, 220], true),
        SILENCED => ("Silêncio", [90, 90, 120], true),
        CHILLED => ("Frio", [170, 220, 255], true),
        FROZEN => ("Congelado", [210, 240, 255], true),
        VENOM => ("Peçonha", [120, 40, 160], true),
        POTION_SICKNESS => ("Enjoo de Poção", [170, 140, 90], true),
        MANA_SICKNESS => ("Enjoo de Mana", [90, 90, 200], true),
        IRONSKIN => ("Pele de Ferro", [200, 200, 210], false),
        MANA_REGEN => ("Regeneração de Mana", [80, 120, 255], false),
        MAGIC_POWER => ("Poder Mágico", [190, 80, 255], false),
        ARCHERY => ("Arquearia", [200, 150, 90], false),
        WRATH => ("Ira", [230, 60, 40], false),
        RAGE => ("Fúria", [250, 120, 40], false),
        ENDURANCE => ("Resiliência", [100, 140, 170], false),
        LIFEFORCE => ("Força Vital", [255, 110, 140], false),
        THORNS => ("Espinhos", [90, 150, 60], false),
        TITAN => ("Titã", [150, 110, 80], false),
        SUMMONING => ("Invocação", [120, 230, 200], false),
        SPELUNKER => ("Espeleólogo", [240, 200, 80], false),
        HUNTER => ("Caçador", [230, 120, 90], false),
        SHINE => ("Brilho Interior", [255, 240, 170], false),
        WELL_FED => ("Bem Alimentado", [230, 170, 90], false),
        COZY_FIRE => ("Fogueira Aconchegante", [255, 150, 60], false),
        BUILDER => ("Construtor", [180, 140, 100], false),
        WARMTH => ("Aquecido", [255, 170, 110], false),
        INFERNO => ("Inferno", [255, 90, 30], false),
        HEART_LAMP => ("Lanterna de Coração", [232, 70, 90], false),
        STAR_BOTTLE => ("Estrela no Pote", [248, 224, 112], false),
        CLAIRVOYANCE => ("Clarividência", [120, 120, 240], false),
        SHARPENED => ("Afiado", [176, 176, 190], false),
        AMMO_BOX => ("Caixa de Munição", [150, 170, 90], false),
        STRATEGIST => ("Estrategista", [196, 160, 100], false),
        BEWITCHED => ("Enfeitiçado", [170, 110, 220], false),
        BATTLE => ("Batalha", [200, 50, 40], false),
        CALM => ("Calmaria", [90, 150, 220], false),
        HUNTING => ("Estandarte de Caça", [220, 150, 70], false),
        _ => ("?", [200, 200, 200], false),
    }
}

pub const ALL: u8 = 81;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Active {
    pub id: u8,
    /// 0 = level I.
    pub amp: u8,
    /// Seconds left.
    pub time: f32,
    /// From a beacon or conduit: no particles, refreshed while near.
    #[serde(default)]
    pub ambient: bool,
}

/// One potion: its item name stem, the effects it gives (id, amplifier, seconds), colour.
pub struct PotionDef {
    pub key: &'static str,
    pub name: &'static str,
    pub effects: &'static [(u8, u8, f32)],
}

macro_rules! p {
    ($k:expr, $n:expr, [$(($e:expr, $a:expr, $t:expr)),*]) => {
        PotionDef { key: $k, name: $n, effects: &[$(($e, $a, $t)),*] }
    };
}

/// Every potion, by the stem of its items (`potion_<key>`, `splash_potion_<key>`,
/// `lingering_potion_<key>`, `tipped_arrow_<key>`). `tools/gen_fase6.py` makes the items.
pub static POTIONS: &[PotionDef] = &[
    p!("water", "Garrafa de Água", []),
    p!("mundane", "Poção Mundana", []),
    p!("thick", "Poção Grossa", []),
    p!("awkward", "Poção Estranha", []),
    p!("night_vision", "Poção de Visão Noturna", [(NIGHT_VISION, 0, 180.0)]),
    p!("long_night_vision", "Poção de Visão Noturna (longa)", [(NIGHT_VISION, 0, 480.0)]),
    p!("invisibility", "Poção de Invisibilidade", [(INVISIBILITY, 0, 180.0)]),
    p!("long_invisibility", "Poção de Invisibilidade (longa)", [(INVISIBILITY, 0, 480.0)]),
    p!("leaping", "Poção de Salto", [(JUMP_BOOST, 0, 180.0)]),
    p!("long_leaping", "Poção de Salto (longa)", [(JUMP_BOOST, 0, 480.0)]),
    p!("strong_leaping", "Poção de Salto II", [(JUMP_BOOST, 1, 90.0)]),
    p!("fire_resistance", "Poção de Resistência ao Fogo", [(FIRE_RESISTANCE, 0, 180.0)]),
    p!("long_fire_resistance", "Poção de Resistência ao Fogo (longa)", [(FIRE_RESISTANCE, 0, 480.0)]),
    p!("swiftness", "Poção de Velocidade", [(SPEED, 0, 180.0)]),
    p!("long_swiftness", "Poção de Velocidade (longa)", [(SPEED, 0, 480.0)]),
    p!("strong_swiftness", "Poção de Velocidade II", [(SPEED, 1, 90.0)]),
    p!("slowness", "Poção de Lentidão", [(SLOWNESS, 0, 90.0)]),
    p!("long_slowness", "Poção de Lentidão (longa)", [(SLOWNESS, 0, 240.0)]),
    p!("strong_slowness", "Poção de Lentidão IV", [(SLOWNESS, 3, 20.0)]),
    p!("turtle_master", "Poção do Mestre Tartaruga", [(SLOWNESS, 3, 20.0), (RESISTANCE, 2, 20.0)]),
    p!("long_turtle_master", "Poção do Mestre Tartaruga (longa)", [(SLOWNESS, 3, 40.0), (RESISTANCE, 2, 40.0)]),
    p!("strong_turtle_master", "Poção do Mestre Tartaruga II", [(SLOWNESS, 5, 20.0), (RESISTANCE, 3, 20.0)]),
    p!("water_breathing", "Poção de Respiração Aquática", [(WATER_BREATHING, 0, 180.0)]),
    p!("long_water_breathing", "Poção de Respiração Aquática (longa)", [(WATER_BREATHING, 0, 480.0)]),
    p!("healing", "Poção de Cura", [(INSTANT_HEALTH, 0, 0.0)]),
    p!("strong_healing", "Poção de Cura II", [(INSTANT_HEALTH, 1, 0.0)]),
    p!("harming", "Poção de Dano", [(INSTANT_DAMAGE, 0, 0.0)]),
    p!("strong_harming", "Poção de Dano II", [(INSTANT_DAMAGE, 1, 0.0)]),
    p!("poison", "Poção de Veneno", [(POISON, 0, 45.0)]),
    p!("long_poison", "Poção de Veneno (longa)", [(POISON, 0, 90.0)]),
    p!("strong_poison", "Poção de Veneno II", [(POISON, 1, 21.6)]),
    p!("regeneration", "Poção de Regeneração", [(REGENERATION, 0, 45.0)]),
    p!("long_regeneration", "Poção de Regeneração (longa)", [(REGENERATION, 0, 90.0)]),
    p!("strong_regeneration", "Poção de Regeneração II", [(REGENERATION, 1, 22.5)]),
    p!("strength", "Poção de Força", [(STRENGTH, 0, 180.0)]),
    p!("long_strength", "Poção de Força (longa)", [(STRENGTH, 0, 480.0)]),
    p!("strong_strength", "Poção de Força II", [(STRENGTH, 1, 90.0)]),
    p!("weakness", "Poção de Fraqueza", [(WEAKNESS, 0, 90.0)]),
    p!("long_weakness", "Poção de Fraqueza (longa)", [(WEAKNESS, 0, 240.0)]),
    p!("luck", "Poção da Sorte", [(LUCK, 0, 300.0)]),
    p!("slow_falling", "Poção de Queda Lenta", [(SLOW_FALLING, 0, 90.0)]),
    p!("long_slow_falling", "Poção de Queda Lenta (longa)", [(SLOW_FALLING, 0, 240.0)]),
    p!("wind_charged", "Poção Carregada de Vento", [(WIND_CHARGED, 0, 180.0)]),
    p!("weaving", "Poção de Tecelagem", [(WEAVING, 0, 180.0)]),
    p!("oozing", "Poção de Gosma", [(OOZING, 0, 180.0)]),
    p!("infested", "Poção de Infestação", [(INFESTED, 0, 180.0)]),
    // The Terraria side's buff potions (rpg.rs); brewed from an awkward potion (stations.rs).
    p!("ironskin", "Poção de Pele de Ferro", [(IRONSKIN, 0, 480.0)]),
    p!("mana_regeneration", "Poção de Regeneração de Mana", [(MANA_REGEN, 0, 420.0)]),
    p!("magic_power", "Poção de Poder Mágico", [(MAGIC_POWER, 0, 240.0)]),
    p!("archery", "Poção de Arquearia", [(ARCHERY, 0, 240.0)]),
    p!("wrath", "Poção da Ira", [(WRATH, 0, 240.0)]),
    p!("rage", "Poção da Fúria", [(RAGE, 0, 240.0)]),
    p!("endurance", "Poção de Resiliência", [(ENDURANCE, 0, 240.0)]),
    p!("lifeforce", "Poção de Força Vital", [(LIFEFORCE, 0, 480.0)]),
    p!("thorns", "Poção de Espinhos", [(THORNS, 0, 120.0)]),
    p!("titan", "Poção do Titã", [(TITAN, 0, 240.0)]),
    p!("summoning", "Poção de Invocação", [(SUMMONING, 0, 360.0)]),
    p!("spelunker", "Poção do Espeleólogo", [(SPELUNKER, 0, 300.0)]),
    p!("hunter", "Poção do Caçador", [(HUNTER, 0, 300.0)]),
    p!("shine", "Poção do Brilho", [(SHINE, 0, 300.0)]),
    p!("builder", "Poção do Construtor", [(BUILDER, 0, 900.0)]),
    p!("warmth", "Poção do Calor", [(WARMTH, 0, 420.0)]),
    p!("inferno", "Poção do Inferno", [(INFERNO, 0, 240.0)]),
];

pub fn potion(key: &str) -> Option<&'static PotionDef> {
    POTIONS.iter().find(|p| p.key == key)
}

/// How a potion item is used, and which potion it holds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Form {
    Drink,
    Splash,
    Lingering,
    Arrow,
}

pub fn potion_of(item_name: &str) -> Option<(Form, &'static PotionDef)> {
    let (form, key) = if let Some(k) = item_name.strip_prefix("splash_potion_") {
        (Form::Splash, k)
    } else if let Some(k) = item_name.strip_prefix("lingering_potion_") {
        (Form::Lingering, k)
    } else if let Some(k) = item_name.strip_prefix("tipped_arrow_") {
        (Form::Arrow, k)
    } else if let Some(k) = item_name.strip_prefix("potion_") {
        (Form::Drink, k)
    } else {
        return None;
    };
    potion(key).map(|p| (form, p))
}

/// Seconds as "m:ss".
pub fn clock(t: f32) -> String {
    if !t.is_finite() || t > 35999.0 {
        return "**:**".into();
    }
    let s = t.max(0.0) as u32;
    format!("{}:{:02}", s / 60, s % 60)
}

/// Undead mobs: healing hurts them and harming heals them.
pub fn undead(name: &str) -> bool {
    matches!(name, "zombie" | "skeleton" | "lich" | "ember_wraith" | "nether_zombie" | "ash_skeleton" | "mutant_zombie" | "mutant_skeleton" | "husk" | "stray" | "drowned" | "wither_skeleton" | "bogged")
}

/// A lingering potion's cloud on the ground.
pub struct Cloud {
    pub pos: Vec3,
    pub radius: f32,
    pub time: f32,
    pub effects: &'static [(u8, u8, f32)],
    pub color: [u8; 3],
    pub tick: f32,
}

/// A thrown potion in flight.
pub struct Thrown {
    pub pos: Vec3,
    pub vel: Vec3,
    pub potion: &'static PotionDef,
    pub lingering: bool,
}

pub fn potion_color(p: &PotionDef) -> [u8; 3] {
    if p.effects.is_empty() {
        return [56, 92, 196];
    }
    let mut c = [0u32; 3];
    for &(e, _, _) in p.effects {
        let k = info(e).1;
        for i in 0..3 {
            c[i] += k[i] as u32;
        }
    }
    let n = p.effects.len() as u32;
    [(c[0] / n) as u8, (c[1] / n) as u8, (c[2] / n) as u8]
}

impl Game {
    pub(crate) fn has_effect(&self, id: u8) -> bool {
        self.prog.effects.iter().any(|e| e.id == id)
    }

    /// Amplifier + 1 (0 when the effect is not on).
    pub(crate) fn effect_level(&self, id: u8) -> u32 {
        self.prog.effects.iter().filter(|e| e.id == id).map(|e| e.amp as u32 + 1).max().unwrap_or(0)
    }

    /// Put an effect on the player. A stronger one replaces a weaker; the same strength keeps the
    /// longer time. Instant effects act at once.
    pub(crate) fn add_effect(&mut self, id: u8, amp: u8, secs: f32, ambient: bool) {
        if !self.survival && matches!(id, INSTANT_DAMAGE | POISON | WITHER | HUNGER) {
            return;
        }
        // Accessories and buffs that keep a debuff away (rpg.rs).
        if self.immune_to(id) {
            return;
        }
        match id {
            INSTANT_HEALTH => {
                self.health = (self.health + (4u32 << amp.min(5)) as f32).min(self.max_health);
                return;
            }
            INSTANT_DAMAGE => {
                self.damage((6u32 << amp.min(5)) as f32, None);
                return;
            }
            SATURATION if secs <= 1.0 => {
                self.surv.hunger = (self.surv.hunger + (amp as f32 + 1.0)).min(20.0);
                self.surv.saturation = (self.surv.saturation + 2.0 * (amp as f32 + 1.0)).min(self.surv.hunger);
                return;
            }
            _ => {}
        }
        if let Some(e) = self.prog.effects.iter_mut().find(|e| e.id == id) {
            if amp > e.amp || (amp == e.amp && secs > e.time) {
                let old = e.amp;
                e.amp = amp;
                e.time = secs;
                e.ambient = ambient;
                if (id == HEALTH_BOOST || id == LIFEFORCE) && amp != old {
                    self.max_health += 4.0 * (amp as f32 - old as f32);
                }
                if id == ABSORPTION {
                    self.prog.absorb = self.prog.absorb.max(4.0 * (amp as f32 + 1.0));
                }
            }
            return;
        }
        self.prog.effects.push(Active { id, amp, time: secs, ambient });
        match id {
            HEALTH_BOOST | LIFEFORCE => self.max_health += 4.0 * (amp as f32 + 1.0),
            ABSORPTION => self.prog.absorb = self.prog.absorb.max(4.0 * (amp as f32 + 1.0)),
            _ => {}
        }
    }

    fn effect_ended(&mut self, e: &Active) {
        match e.id {
            HEALTH_BOOST | LIFEFORCE => {
                self.max_health = (self.max_health - 4.0 * (e.amp as f32 + 1.0)).max(2.0);
                self.health = self.health.min(self.max_health);
            }
            ABSORPTION => self.prog.absorb = 0.0,
            _ => {}
        }
    }

    /// Milk (and the totem, when there is one): every effect goes away.
    pub(crate) fn clear_effects(&mut self) {
        let list = std::mem::take(&mut self.prog.effects);
        for e in &list {
            self.effect_ended(e);
        }
    }

    /// Each frame: count effects down and do what they do over time.
    pub(crate) fn update_effects(&mut self, dt: f32) {
        if dt <= 0.0 || self.screen == crate::Screen::Dead {
            return;
        }
        // Conduit power is water breathing and night vision under water, and digs faster.
        let mut ended = Vec::new();
        for e in self.prog.effects.iter_mut() {
            e.time -= dt;
            if e.time <= 0.0 {
                ended.push(*e);
            }
        }
        self.prog.effects.retain(|e| e.time > 0.0);
        for e in &ended {
            self.effect_ended(e);
        }
        self.prog.fx_tick += dt;
        let ticks = (self.prog.fx_tick / 0.05) as u32;
        self.prog.fx_tick -= ticks as f32 * 0.05;
        for _ in 0..ticks {
            self.prog.fx_ticks = self.prog.fx_ticks.wrapping_add(1);
            let t = self.prog.fx_ticks;
            let list = self.prog.effects.clone();
            for e in list {
                let a = e.amp as u32;
                match e.id {
                    REGENERATION => {
                        let every = (50u32 >> a.min(5)).max(1);
                        if t % every == 0 && self.health < self.max_health {
                            self.health = (self.health + 1.0).min(self.max_health);
                        }
                    }
                    POISON => {
                        let every = (25u32 >> a.min(5)).max(1);
                        if t % every == 0 && self.health > 1.0 {
                            self.prog.next_kind = crate::progression::DamageKind::Magic;
                            self.damage(1.0, None);
                        }
                    }
                    WITHER => {
                        let every = (40u32 >> a.min(5)).max(1);
                        if t % every == 0 {
                            self.prog.next_kind = crate::progression::DamageKind::Magic;
                            self.damage(1.0, None);
                        }
                    }
                    // Terraria-style damage over time: one point every so many ticks.
                    FROSTBURN | CURSED_FLAMES | VENOM | BLEEDING => {
                        let every = match e.id {
                            FROSTBURN => 15,
                            CURSED_FLAMES => 10,
                            VENOM => 8,
                            _ => 40,
                        };
                        if t % every == 0 && (e.id != BLEEDING || self.health > 1.0) {
                            self.prog.next_kind = crate::progression::DamageKind::Magic;
                            self.damage(1.0, None);
                        }
                    }
                    HUNGER => self.surv.exhaustion += 0.005 * (a + 1) as f32,
                    SATURATION => {
                        self.surv.hunger = (self.surv.hunger + (a + 1) as f32).min(20.0);
                        self.surv.saturation = (self.surv.saturation + 2.0 * (a + 1) as f32).min(self.surv.hunger);
                    }
                    _ => {}
                }
            }
        }
        // Swirls rise from the player while an effect is on (not from beacons).
        if !self.prog.effects.is_empty() && self.camera_mode != 0 {
            self.prog.swirl_t -= dt;
            if self.prog.swirl_t <= 0.0 {
                self.prog.swirl_t = 0.3;
                if let Some(e) = self.prog.effects.iter().find(|e| !e.ambient && e.id != INVISIBILITY) {
                    let c = info(e.id).1;
                    let glow = self.glow_layer();
                    let r = self.prog.rng.f32() * std::f32::consts::TAU;
                    let p = self.body.pos + Vec3::new(r.cos() * 0.4, 0.5 + self.prog.rng.f32() * 1.2, r.sin() * 0.4);
                    self.particles.glow(p, Vec3::Y * 0.6, glow, [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0], 0.08, 1.0);
                }
            }
        }
        self.update_thrown_potions(dt);
        self.update_potion_clouds(dt);
    }

    /// Speed effects, sneaking enchantments and soul speed: how much faster than walking.
    pub(crate) fn move_multiplier(&self) -> f32 {
        let mut m = 1.0 + 0.2 * self.effect_level(SPEED) as f32;
        m *= (1.0 - 0.15 * self.effect_level(SLOWNESS) as f32).max(0.0);
        if self.has_effect(CHILLED) {
            m *= 0.75;
        }
        if self.has_effect(FROZEN) {
            m = 0.0;
        }
        m
    }

    /// Damage dealt by the player's melee hits, changed by strength and weakness.
    pub(crate) fn melee_effects(&self, dmg: f32) -> f32 {
        (dmg + 3.0 * self.effect_level(STRENGTH) as f32 - 4.0 * self.effect_level(WEAKNESS) as f32).max(0.0)
    }

    /// Digging-speed factor from haste, conduit power and mining fatigue.
    pub(crate) fn dig_effects(&self) -> f32 {
        let haste = self.effect_level(HASTE).max(self.effect_level(CONDUIT_POWER));
        let mut k = 1.0 + 0.2 * haste as f32;
        let f = self.effect_level(MINING_FATIGUE);
        if f > 0 {
            k *= [0.3, 0.09, 0.0027, 0.00081][(f as usize - 1).min(3)];
        }
        k
    }

    // -----------------------------------------------------------------------------------------
    // Potions.

    /// Right click with a potion: drink it (held, like food) or throw it.
    pub(crate) fn use_potion(&mut self, name: &str) -> bool {
        let Some((form, p)) = potion_of(name) else { return false };
        match form {
            Form::Drink => {
                // Drinking takes a moment: handled while the button is held (update_drinking).
                true
            }
            Form::Splash | Form::Lingering => {
                let f = self.forward();
                let from = self.body.eye() + f * 0.4;
                self.prog.thrown.push(Thrown { pos: from, vel: f * 12.0 + Vec3::Y * 2.0 + self.body.vel * 0.5, potion: p, lingering: form == Form::Lingering });
                if self.survival {
                    let sel = self.selected;
                    self.surv.inv.take(sel, 1);
                }
                self.anim.start_swing();
                self.sfx(Sfx::Place(Material::Glass));
                true
            }
            Form::Arrow => false,
        }
    }

    /// Hold the use button with a drink (potion, milk, honey) in hand.
    pub(crate) fn update_drinking(&mut self, dt: f32, use_held: bool) {
        let held = self.held_stack();
        let name = held.map(|s| self.items.get(s.item).name.clone()).unwrap_or_default();
        let drink = potion_of(&name).is_some_and(|(f, _)| f == Form::Drink) || name == "milk_bucket" || name == "honey_bottle" || name == "ominous_bottle";
        if !(use_held && drink && self.screen == crate::Screen::Playing && dt > 0.0) {
            self.prog.drink = 0.0;
            return;
        }
        self.prog.drink += dt;
        self.prog.drink_sound -= dt;
        if self.prog.drink_sound <= 0.0 {
            self.prog.drink_sound = 0.25;
            self.sfx(Sfx::Bubbles);
        }
        if self.prog.drink < 1.6 {
            return;
        }
        self.prog.drink = 0.0;
        let sel = self.selected;
        let back = match name.as_str() {
            "milk_bucket" => {
                self.clear_effects();
                "bucket"
            }
            "honey_bottle" => {
                self.prog.effects.retain(|e| e.id != POISON);
                self.surv.hunger = (self.surv.hunger + 6.0).min(20.0);
                self.surv.saturation = (self.surv.saturation + 1.2).min(self.surv.hunger);
                "glass_bottle"
            }
            "ominous_bottle" => {
                let amp = held.map(|s| s.damage.min(4) as u8).unwrap_or(0);
                self.add_effect(BAD_OMEN, amp, 6000.0, false);
                ""
            }
            _ => {
                if let Some((_, p)) = potion_of(&name) {
                    let longer = self.skill_potion_time();
                    for &(e, a, t) in p.effects {
                        self.add_effect(e, a, t * longer, false);
                    }
                }
                "glass_bottle"
            }
        };
        self.prog.stats.add("potions_drunk", 1);
        self.prog.event(crate::progression::Event::Drank(name.clone()));
        self.nature_consumed(&name);
        if !self.survival {
            return;
        }
        self.surv.inv.take(sel, 1);
        if let Some(b) = self.items.id(back) {
            let st = ItemStack::new(b, 1);
            if self.surv.inv.slots[sel].is_none() {
                self.surv.inv.slots[sel] = Some(st);
            } else if let Some(left) = self.surv.inv.add(&self.items, st) {
                self.throw_stack(left);
            }
        }
    }

    fn update_thrown_potions(&mut self, dt: f32) {
        let mut burst = Vec::new();
        for t in self.prog.thrown.iter_mut() {
            t.vel.y -= 20.0 * dt;
            let next = t.pos + t.vel * dt;
            let hit_block = voxel::physics::Terrain::solid(&self.world, next.floor().as_ivec3());
            let hit_mob = self.mobs.list.iter().any(|m| m.alive() && (m.body.pos + Vec3::Y * m.body.height * 0.5 - next).length() < m.body.half_width + 0.5);
            if hit_block || hit_mob || t.pos.y < voxel::chunk::MIN_Y as f32 - 64.0 {
                burst.push((t.pos, t.potion, t.lingering));
                t.pos.y = -1000.0;
            } else {
                t.pos = next;
            }
        }
        self.prog.thrown.retain(|t| t.pos.y > -999.0);
        for (at, p, lingering) in burst {
            self.potion_burst(at, p, lingering);
        }
    }

    /// A splash potion breaks: everyone near gets its effects, less the further they stand.
    pub(crate) fn potion_burst(&mut self, at: Vec3, p: &'static PotionDef, lingering: bool) {
        self.sfx(Sfx::Break(Material::Glass));
        let c = potion_color(p);
        let col = [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0];
        let glow = self.glow_layer();
        for k in 0..24 {
            let a = k as f32 * 0.7;
            let v = Vec3::new(a.cos() * 2.0, 1.5 + (k % 5) as f32 * 0.4, a.sin() * 2.0);
            self.particles.glow(at, v, glow, col, 0.1, 1.0);
        }
        if lingering {
            self.prog.clouds.push(Cloud { pos: at, radius: 3.0, time: 30.0, effects: p.effects, color: c, tick: 0.0 });
            return;
        }
        // Water puts out fire and hurts nothing.
        let d = (self.body.pos + Vec3::Y - at).length();
        if d < 4.0 {
            let k = 1.0 - d / 4.0;
            for &(e, a, t) in p.effects {
                if t == 0.0 {
                    if k > 0.3 {
                        self.add_effect(e, a, 0.0, false);
                    }
                } else {
                    self.add_effect(e, a, t * 0.75 * k.max(0.25), false);
                }
            }
        }
        for i in 0..self.mobs.list.len() {
            let m = &self.mobs.list[i];
            if !m.alive() {
                continue;
            }
            let d = (m.body.pos + Vec3::Y * 0.5 - at).length();
            if d < 4.0 {
                let name = self.mobs.reg.defs[m.def].name.clone();
                self.potion_on_mob(i, &name, p.effects, 1.0 - d / 4.0, at);
            }
        }
    }

    /// What a potion does to a mob: instant ones at once, the rest roughly (poison and wither
    /// bite now, slowness chills, fire resistance puts the fire out).
    pub(crate) fn potion_on_mob(&mut self, i: usize, name: &str, effects: &[(u8, u8, f32)], k: f32, from: Vec3) {
        let undead = undead(name);
        let m = &mut self.mobs.list[i];
        for &(e, a, t) in effects {
            let pow = (a as u32 + 1) as f32;
            match (e, undead) {
                (INSTANT_HEALTH, false) | (INSTANT_DAMAGE, true) => m.health = (m.health + 4.0 * pow * k).min(m.max_health),
                (INSTANT_DAMAGE, false) | (INSTANT_HEALTH, true) => {
                    m.hurt(6.0 * pow * k, from);
                }
                (POISON, false) => {
                    let dmg = (t / 1.25 * pow * 0.5 * k).min(m.health - 1.0).max(0.0);
                    m.health -= dmg;
                    m.hurt_age = 0.0;
                }
                (WITHER, _) => {
                    m.hurt(t / 2.0 * pow * 0.3 * k, from);
                }
                (SLOWNESS, _) => m.chill = m.chill.max(t * 0.5 * k),
                (FIRE_RESISTANCE, _) => m.on_fire = 0.0,
                (REGENERATION, false) => m.health = (m.health + t * 0.4 * pow * k).min(m.max_health),
                _ => {}
            }
        }
    }

    fn update_potion_clouds(&mut self, dt: f32) {
        let glow = self.glow_layer();
        let mut hits: Vec<(&'static [(u8, u8, f32)], Vec3, f32)> = Vec::new();
        for c in self.prog.clouds.iter_mut() {
            c.time -= dt;
            c.radius = (c.radius - dt * 0.05).max(0.5);
            c.tick -= dt;
            if c.tick <= 0.0 {
                c.tick = 0.25;
                hits.push((c.effects, c.pos, c.radius));
                let col = [c.color[0] as f32 / 255.0, c.color[1] as f32 / 255.0, c.color[2] as f32 / 255.0];
                for k in 0..6 {
                    let a = (self.prog.fx_ticks as f32 * 0.37 + k as f32 * 1.05) % std::f32::consts::TAU;
                    let r = c.radius * (k as f32 / 6.0);
                    self.particles.glow(c.pos + Vec3::new(a.cos() * r, 0.1, a.sin() * r), Vec3::Y * 0.3, glow, col, 0.08, 1.2);
                }
            }
        }
        self.prog.clouds.retain(|c| c.time > 0.0);
        for (effects, at, r) in hits {
            if (self.body.pos - at).length() < r + 0.3 {
                for &(e, a, t) in effects {
                    // A quarter of the potion's time each touch, like vanilla's lingering clouds.
                    self.add_effect(e, a, if t == 0.0 { 0.0 } else { (t * 0.25).max(5.0) }, false);
                }
            }
            for i in 0..self.mobs.list.len() {
                let m = &self.mobs.list[i];
                if m.alive() && (m.body.pos - at).length() < r {
                    let name = self.mobs.reg.defs[m.def].name.clone();
                    self.potion_on_mob(i, &name, effects, 0.25, at);
                }
            }
        }
    }

    /// The icons of active effects, top right, with their time.
    pub(crate) fn draw_effects_hud(&mut self, w: f32) {
        if self.prog.effects.is_empty() || self.hide_hud {
            return;
        }
        let s = self.ui.scale;
        let list = self.prog.effects.clone();
        let mut x = w - 26.0 * s;
        let mut y = 2.0 * s;
        for (k, e) in list.iter().enumerate() {
            let (name, c, bad) = info(e.id);
            let blink = e.time < 10.0 && ((self.now * 4.0) as i32) % 2 == 0;
            let bg = if e.ambient { [40, 120, 140, 200] } else if bad { [70, 30, 30, 200] } else { [30, 30, 30, 200] };
            self.ui.rect(x, y, 24.0 * s, 24.0 * s, bg);
            self.ui.frame(x, y, 24.0 * s, 24.0 * s, s, [200, 200, 200, 255]);
            if !blink {
                draw_effect_icon(&mut self.ui, e.id, c, x + 4.0 * s, y + 3.0 * s, s);
            }
            let t = clock(e.time);
            let tw = self.ui.text_width(&t, s * 0.75);
            self.ui.text_scaled(x + 12.0 * s - tw / 2.0, y + 17.0 * s, &t, [255, 255, 255, 255], s * 0.75);
            if e.amp > 0 {
                let r = crate::enchanting::roman(e.amp + 1);
                self.ui.text_scaled(x + 15.0 * s, y + 2.0 * s, r, [255, 255, 255, 255], s * 0.75);
            }
            let _ = name;
            x -= 26.0 * s;
            if k % 8 == 7 {
                x = w - 26.0 * s;
                y += 26.0 * s;
            }
        }
    }

    /// Dark screens: blindness closes in, darkness pulses (the warden's), nausea sways green.
    pub(crate) fn draw_effect_overlays(&mut self, w: f32, h: f32) {
        let blind = self.has_effect(BLINDNESS);
        let dark = self.has_effect(DARKNESS);
        if blind || dark {
            let pulse = if dark { 0.55 + 0.35 * (self.now * 1.6).sin().abs() } else { 0.0 };
            let a = if blind { 0.86 } else { pulse };
            // A vignette: dark at the edges, a little light left in the middle.
            let rings = 10;
            for r in 0..rings {
                let k = r as f32 / rings as f32;
                let inset_x = w * 0.5 * k;
                let inset_y = h * 0.5 * k;
                let alpha = (a * (0.35 + 0.65 * (1.0 - k)) * 255.0 / rings as f32 * 2.2).min(255.0) as u8;
                crate::survival::rect_around(&mut self.ui, (inset_x, inset_y, w - 2.0 * inset_x, h - 2.0 * inset_y), Some((inset_x + w * 0.05, inset_y + h * 0.05, (w - 2.0 * inset_x - w * 0.1).max(0.0), (h - 2.0 * inset_y - h * 0.1).max(0.0))), [0, 0, 0, alpha]);
            }
            self.ui.rect(0.0, 0.0, w, h, [0, 0, 0, (a * 120.0) as u8]);
        }
        if self.has_effect(NAUSEA) {
            let k = (self.now * 1.3).sin() * 0.5 + 0.5;
            self.ui.rect(0.0, 0.0, w, h, [60, 120, 40, (30.0 + 40.0 * k) as u8]);
        }
        if self.has_effect(NIGHT_VISION) && !blind {
            // A cool, light wash: the dark parts of the picture are lifted in the shader
            // (terrain_effects); this tints the screen like the vanilla effect.
            self.ui.rect(0.0, 0.0, w, h, [120, 140, 255, 14]);
        }
        if self.has_effect(CONDUIT_POWER) && self.body.head_in_water {
            self.ui.rect(0.0, 0.0, w, h, [40, 200, 220, 18]);
        }
    }
}

/// A small picture for each effect, drawn with rectangles in its colour (our own icons).
pub fn draw_effect_icon(ui: &mut crate::ui::UiBatch, id: u8, c: [u8; 3], x: f32, y: f32, s: f32) {
    let col = [c[0], c[1], c[2], 255];
    let dark = [c[0] / 2, c[1] / 2, c[2] / 2, 255];
    let px = |ui: &mut crate::ui::UiBatch, a: f32, b: f32, w: f32, h: f32, k: [u8; 4]| ui.rect(x + a * s, y + b * s, w * s, h * s, k);
    match id {
        SPEED | DOLPHINS_GRACE => {
            // A boot with speed lines.
            px(ui, 6.0, 3.0, 5.0, 8.0, col);
            px(ui, 6.0, 9.0, 9.0, 3.0, col);
            px(ui, 0.0, 4.0, 4.0, 1.0, dark);
            px(ui, 1.0, 7.0, 4.0, 1.0, dark);
        }
        SLOWNESS => {
            px(ui, 4.0, 6.0, 8.0, 5.0, col);
            px(ui, 2.0, 8.0, 3.0, 3.0, dark);
            px(ui, 11.0, 4.0, 3.0, 3.0, dark);
        }
        HASTE | MINING_FATIGUE | CONDUIT_POWER | BUILDER => {
            // A pickaxe.
            px(ui, 2.0, 2.0, 12.0, 3.0, col);
            px(ui, 7.0, 4.0, 2.0, 10.0, dark);
        }
        STRENGTH | WEAKNESS | WRATH | RAGE | TITAN | SHARPENED | BATTLE | HUNTING => {
            // A sword.
            px(ui, 7.0, 0.0, 2.0, 10.0, col);
            px(ui, 4.0, 10.0, 8.0, 2.0, dark);
            px(ui, 7.0, 12.0, 2.0, 3.0, dark);
        }
        INSTANT_HEALTH | REGENERATION | HEALTH_BOOST | ABSORPTION | INSTANT_DAMAGE | LIFEFORCE | POTION_SICKNESS | WELL_FED | HEART_LAMP => {
            // A heart.
            px(ui, 2.0, 3.0, 5.0, 5.0, col);
            px(ui, 9.0, 3.0, 5.0, 5.0, col);
            px(ui, 3.0, 7.0, 10.0, 3.0, col);
            px(ui, 5.0, 10.0, 6.0, 2.0, col);
            px(ui, 7.0, 12.0, 2.0, 2.0, col);
        }
        JUMP_BOOST | LEVITATION | SLOW_FALLING | WIND_CHARGED => {
            // An arrow up.
            px(ui, 7.0, 2.0, 2.0, 12.0, col);
            px(ui, 4.0, 5.0, 8.0, 2.0, col);
            px(ui, 5.0, 3.0, 6.0, 2.0, col);
        }
        RESISTANCE | FIRE_RESISTANCE | IRONSKIN | ENDURANCE | BROKEN_ARMOR | ICHOR | CALM => {
            // A shield.
            px(ui, 3.0, 2.0, 10.0, 8.0, col);
            px(ui, 5.0, 10.0, 6.0, 2.0, col);
            px(ui, 7.0, 12.0, 2.0, 2.0, col);
            px(ui, 7.0, 3.0, 2.0, 8.0, dark);
        }
        WATER_BREATHING | BREATH_OF_THE_NAUTILUS => {
            for (a, b) in [(3.0, 8.0), (8.0, 4.0), (11.0, 10.0)] {
                px(ui, a, b, 4.0, 4.0, col);
            }
        }
        NIGHT_VISION | BLINDNESS | DARKNESS | INVISIBILITY | GLOWING | HUNTER | SPELUNKER | SHINE | CONFUSED => {
            // An eye.
            px(ui, 1.0, 6.0, 14.0, 4.0, col);
            px(ui, 3.0, 4.0, 10.0, 8.0, col);
            px(ui, 6.0, 6.0, 4.0, 4.0, dark);
        }
        POISON | WITHER | HUNGER | NAUSEA | OOZING | INFESTED | WEAVING | BLEEDING | VENOM => {
            // A drop.
            px(ui, 7.0, 2.0, 2.0, 2.0, col);
            px(ui, 5.0, 4.0, 6.0, 4.0, col);
            px(ui, 4.0, 8.0, 8.0, 5.0, col);
            px(ui, 6.0, 13.0, 4.0, 1.0, dark);
        }
        FROSTBURN | CURSED_FLAMES | INFERNO | WARMTH | COZY_FIRE => {
            // A flame.
            px(ui, 6.0, 1.0, 3.0, 3.0, col);
            px(ui, 4.0, 4.0, 7.0, 5.0, col);
            px(ui, 3.0, 8.0, 10.0, 5.0, col);
            px(ui, 6.0, 9.0, 4.0, 4.0, dark);
        }
        MANA_REGEN | MAGIC_POWER | MANA_SICKNESS | SILENCED | SUMMONING | STAR_BOTTLE | CLAIRVOYANCE | STRATEGIST | BEWITCHED => {
            // A star.
            px(ui, 7.0, 1.0, 2.0, 14.0, col);
            px(ui, 1.0, 6.0, 14.0, 3.0, col);
            px(ui, 4.0, 4.0, 8.0, 8.0, col);
            px(ui, 6.0, 6.0, 4.0, 4.0, dark);
        }
        CHILLED | FROZEN => {
            // A snowflake.
            px(ui, 7.0, 1.0, 2.0, 14.0, col);
            px(ui, 1.0, 7.0, 14.0, 2.0, col);
            px(ui, 3.0, 3.0, 2.0, 2.0, col);
            px(ui, 11.0, 3.0, 2.0, 2.0, col);
            px(ui, 3.0, 11.0, 2.0, 2.0, col);
            px(ui, 11.0, 11.0, 2.0, 2.0, col);
        }
        ARCHERY | THORNS | AMMO_BOX => {
            // An arrow.
            for k in 0..10 {
                px(ui, 2.0 + k as f32, 12.0 - k as f32, 2.0, 2.0, col);
            }
            px(ui, 10.0, 2.0, 4.0, 2.0, dark);
            px(ui, 12.0, 2.0, 2.0, 4.0, dark);
        }
        _ => {
            // A banner (omens, luck, hero).
            px(ui, 4.0, 1.0, 1.0, 14.0, dark);
            px(ui, 5.0, 2.0, 8.0, 7.0, col);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn potion_forms_parse() {
        assert_eq!(potion_of("potion_healing").unwrap().0, Form::Drink);
        assert_eq!(potion_of("splash_potion_strong_poison").unwrap().0, Form::Splash);
        assert_eq!(potion_of("lingering_potion_water").unwrap().1.key, "water");
        assert_eq!(potion_of("tipped_arrow_slowness").unwrap().0, Form::Arrow);
        assert!(potion_of("potion_of_nothing").is_none());
    }

    #[test]
    fn every_effect_has_a_name() {
        for id in 1..=ALL {
            assert_ne!(info(id).0, "?", "effect {id}");
        }
    }
}
