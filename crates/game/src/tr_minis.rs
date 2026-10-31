//! Lote 13 of the Terraria side, second round (plan/terraria-completo.md): mini-bosses,
//! trophies, two more town folk and the holidays.
//!
//! - In hard mode, at night, werewolves come out (more on a full moon, one night in eight) and
//!   leap at you. In the caves a lost girl waits: she is a nymph. On a blood moon a clown
//!   throws bombs; in the snow an ice golem shoots frost; in a goblin invasion a summoner
//!   calls spirits.
//! - A Key of Night or of Light (nine souls) used on a chest wakes a mimic of the land.
//! - A boss sometimes leaves its trophy, a pennant to hang.
//! - The Mycologist (hard mode, carrying a mushroom) and the Winter Elder (in December, or
//!   after the Ice Queen) move into town.
//! - From October 10 to November 1 monsters sometimes drop a treat bag; from December 15 to
//!   31, a present.

use glam::{IVec3, Vec3};
use voxel::item::ItemStack;
use voxel::worldgen::{Biome, Dim};

use crate::audio::{Material, Sfx};
use crate::survival::Container;
use crate::Game;

const MINIS: &[&str] = &["werewolf", "lost_girl", "nymph", "clown", "ice_golem", "goblin_summoner"];

const GOODIES: &[(&str, u8, u8)] = &[("fluffy_carrot", 1, 1), ("pumpkin_pie", 1, 3), ("cookie", 2, 6), ("jack_o_lantern", 1, 2), ("carved_pumpkin", 1, 1), ("emerald", 1, 4), ("bone", 2, 5), ("vanity_shadow_helmet", 1, 1), ("vanity_wizard_helmet", 1, 1)];
const PRESENTS: &[(&str, u8, u8)] = &[("cake", 1, 1), ("cookie", 4, 8), ("diamond", 1, 2), ("firework_rocket", 4, 10), ("emerald", 2, 6), ("snowball", 8, 16), ("spruce_sapling", 1, 3), ("vanity_royal_helmet", 1, 1)];

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Holiday {
    None,
    Halloween,
    Christmas,
}

/// Today's month and day (UTC), or MC_DATE=MM-DD for tests.
fn today() -> (u32, u32) {
    if let Ok(s) = std::env::var("MC_DATE") {
        if let Some((m, d)) = s.split_once('-') {
            if let (Ok(m), Ok(d)) = (m.parse(), d.parse()) {
                return (m, d);
            }
        }
    }
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    month_day(secs / 86400)
}

/// Days since 1970-01-01 to (month, day) (Howard Hinnant's civil_from_days).
fn month_day(days: u64) -> (u32, u32) {
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (m as u32, d as u32)
}

pub fn holiday() -> Holiday {
    match today() {
        (10, d) if d >= 10 => Holiday::Halloween,
        (11, 1) => Holiday::Halloween,
        (12, d) if d >= 15 => Holiday::Christmas,
        _ => Holiday::None,
    }
}

#[derive(Default)]
pub struct Minis {
    spawn_t: f32,
    /// Nights since the world was loaded (a full moon every eighth).
    nights: u32,
    was_night: bool,
    told_holiday: bool,
    /// A tortured soul was freed with purification powder: the tax collector may move in.
    pub tax_freed: bool,
    /// Copper the tax collector has gathered and not handed over yet.
    pub tax_owed: u64,
    tax_t: f32,
}

/// Taxes: a few copper per townsperson every minute, kept until the collector is talked to.
const TAX_EVERY: f32 = 60.0;
const TAX_PER_HEAD: u64 = 10;

impl Game {
    fn full_moon(&self) -> bool {
        self.moon_phase() == 0
    }

    pub(crate) fn update_tr_minis(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let night = self.is_night();
        if night && !self.t5.tr.minis.was_night {
            self.t5.tr.minis.nights += 1;
            if self.full_moon() && self.t5.hard.on && self.dim == Dim::Overworld {
                self.toast("A lua cheia nasce...", 3.0);
            }
        }
        self.t5.tr.minis.was_night = night;
        if !self.t5.tr.minis.told_holiday {
            self.t5.tr.minis.told_holiday = true;
            match holiday() {
                Holiday::Halloween => self.toast("É tempo de Halloween: os monstros às vezes deixam Sacos de Gostosuras!", 5.0),
                Holiday::Christmas => self.toast("É Natal: os monstros às vezes deixam Presentes!", 5.0),
                Holiday::None => {}
            }
        }
        self.steer_minis(dt);
        self.collect_taxes(dt);
        self.t5.tr.minis.spawn_t -= dt;
        if self.t5.tr.minis.spawn_t > 0.0 || !self.survival || self.mobs.peaceful || self.dim != Dim::Overworld {
            return;
        }
        self.t5.tr.minis.spawn_t = 6.0;
        let me = self.body.pos;
        let hard = self.t5.hard.on;
        let r = self.surv.rng.f32();
        let surface = me.y > 40.0;
        let (who, cave, chance) = if hard && self.t5.hard.invasion().is_some() && self.count_alive(&["goblin_summoner"]) == 0 {
            ("goblin_summoner", false, 0.4)
        } else if hard && self.t5.hard.blood_moon() && surface && self.count_alive(&["clown"]) == 0 {
            ("clown", false, 0.35)
        } else if hard && night && surface && self.count_alive(&["werewolf"]) < 2 && !self.is_snowy(me) {
            ("werewolf", false, if self.full_moon() { 0.8 } else { 0.15 })
        } else if hard && surface && self.is_snowy(me) && (night || self.weather.state.raining) && self.count_alive(&["ice_golem"]) == 0 {
            ("ice_golem", false, 0.15)
        } else if !surface && self.ubiome() == voxel::worldgen::deep::UBiome::Marble && self.count_alive(&["hoplite"]) < 3 {
            ("hoplite", true, 0.6)
        } else if hard && !self.t5.tr.minis.tax_freed && self.ubiome() == voxel::worldgen::deep::UBiome::Underworld && self.count_alive(&["tortured_soul"]) == 0 {
            ("tortured_soul", true, 0.3)
        } else if !surface && me.y < 20.0 && self.count_alive(&["lost_girl", "nymph"]) == 0 {
            ("lost_girl", true, 0.06)
        } else {
            return;
        };
        if r >= chance {
            return;
        }
        if let Some(at) = self.spot_near(me, 14.0, 26.0, cave) {
            if self.mobs.spawn(who, at + Vec3::Y * 0.1) && !matches!(who, "lost_girl" | "tortured_soul") {
                if let Some(m) = self.mobs.list.last_mut() {
                    m.aggro = true;
                }
            }
        }
    }

    fn collect_taxes(&mut self, dt: f32) {
        let m = &mut self.t5.tr.minis;
        m.tax_t += dt;
        if m.tax_t < TAX_EVERY {
            return;
        }
        m.tax_t = 0.0;
        if !self.town.npcs.iter().any(|n| n.name == "npc_tax_collector") {
            return;
        }
        let heads = self.town.npcs.len() as u64;
        let m = &mut self.t5.tr.minis;
        m.tax_owed = (m.tax_owed + heads * TAX_PER_HEAD).min(crate::money::g(10) as u64);
    }

    /// Talking to the tax collector: he hands over what the town paid.
    pub(crate) fn tax_talk(&mut self, dn: &str) {
        let owed = std::mem::take(&mut self.t5.tr.minis.tax_owed);
        if owed == 0 {
            self.toast(format!("{dn}: Ainda não juntei nada. Mais moradores, mais impostos!"), 4.0);
            return;
        }
        self.earn(owed);
        let text = crate::money::text(owed);
        self.toast(format!("{dn}: Aqui estão os impostos da vila: {text}. Bah!"), 4.0);
        self.sfx(Sfx::Pickup);
    }

    /// Purification powder near a tortured soul sets it free.
    fn free_soul(&mut self) -> Option<bool> {
        let me = self.body.pos;
        let k = self.mobs.list.iter().position(|m| m.alive() && self.mobs.reg.defs[m.def].name == "tortured_soul" && m.body.pos.distance(me) < 6.0)?;
        let pos = self.mobs.list[k].body.pos;
        self.mobs.list[k].health = 0.0;
        self.mobs.list[k].dead_age = 10.0;
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        let glow = self.glow_layer();
        for i in 0..30 {
            let a = i as f32 / 30.0 * std::f32::consts::TAU;
            self.particles.glow(pos + Vec3::Y, Vec3::new(a.cos() * 1.5, 3.0, a.sin() * 1.5), glow, [0.8, 1.6, 1.8], 0.08, 1.2);
        }
        self.t5.tr.minis.tax_freed = true;
        self.toast("A alma atormentada está livre! Talvez ela se mude para a sua vila...", 5.0);
        self.sfx(Sfx::Xp);
        Some(true)
    }

    fn is_snowy(&self, p: Vec3) -> bool {
        matches!(self.world.gen.base_biome_at(p.x.floor() as i32, p.z.floor() as i32), Biome::Snowy | Biome::Taiga | Biome::SnowyPeaks | Biome::FrozenOcean)
    }

    /// What each mini-boss does besides walking at you.
    fn steer_minis(&mut self, dt: f32) {
        let me = self.body.eye();
        let now = self.now;
        // (who, where, which mob)
        let mut acts: Vec<(&'static str, Vec3, usize)> = Vec::new();
        for (k, m) in self.mobs.list.iter_mut().enumerate() {
            if !m.alive() {
                continue;
            }
            let name = self.mobs.reg.defs[m.def].name.as_str();
            let Some(&who) = MINIS.iter().find(|n| **n == name) else { continue };
            let pos = m.body.pos;
            let d = pos.distance(me);
            // A cheap clock per mob: true about once every `every` seconds.
            let every = |every: f32| (now / every + k as f32 * 0.37).fract() < dt / every;
            match who {
                "werewolf" if m.body.on_ground && (3.0..9.0).contains(&d) && every(2.5) => {
                    let to = (me - pos).normalize_or_zero();
                    m.body.vel = Vec3::new(to.x * 10.0, 6.5, to.z * 10.0);
                    acts.push(("howl", pos, k));
                }
                "lost_girl" if d < 3.5 => acts.push(("nymph", pos, k)),
                "clown" if d < 20.0 && every(3.0) => acts.push(("bomb", pos, k)),
                "ice_golem" if d < 24.0 && every(2.0) => acts.push(("frost", pos, k)),
                "goblin_summoner" if d < 28.0 && every(6.0) => acts.push(("summon", pos, k)),
                _ => {}
            }
        }
        for (what, pos, k) in acts {
            match what {
                "howl" => self.sfx(Sfx::MobHit),
                "nymph" => {
                    self.mobs.list[k].health = 0.0;
                    self.mobs.list[k].dead_age = 10.0;
                    if self.mobs.spawn("nymph", pos) {
                        if let Some(m) = self.mobs.list.last_mut() {
                            m.aggro = true;
                        }
                    }
                    self.toast("A garota perdida era uma Ninfa!", 3.0);
                    self.sfx(Sfx::Cast(2));
                }
                "bomb" | "frost" => {
                    let from = pos + Vec3::Y * 1.6;
                    let dir = (me - from).normalize_or_zero();
                    let (school, dmg, speed) = if what == "bomb" { (0, 16.0, 14.0) } else { (1, 14.0, 20.0) };
                    self.enemy_bolt(from + dir, dir * speed + Vec3::Y * 0.5, school, dmg);
                }
                "summon" => {
                    let near = self.mobs.list.iter().filter(|m| m.alive() && self.mobs.reg.defs[m.def].name == "dungeon_spirit" && m.body.pos.distance(pos) < 20.0).count();
                    if near < 4 {
                        for j in [-1.0, 1.0] {
                            if self.mobs.spawn("dungeon_spirit", pos + Vec3::new(j * 1.5, 2.0, 0.0)) {
                                if let Some(m) = self.mobs.list.last_mut() {
                                    m.aggro = true;
                                }
                            }
                        }
                        let glow = self.glow_layer();
                        for i in 0..20 {
                            let a = i as f32 / 20.0 * std::f32::consts::TAU;
                            self.particles.glow(pos + Vec3::new(a.cos() * 1.2, 0.3, a.sin() * 1.2), Vec3::Y * 1.5, glow, [1.0, 0.4, 1.4], 0.09, 0.6);
                        }
                        self.sfx(Sfx::Cast(3));
                    }
                }
                _ => {}
            }
        }
    }

    /// A key of night or light used on a chest wakes a mimic.
    fn use_mimic_key(&mut self, light: bool) -> bool {
        let Some((p, _)) = self.looked_block() else { return true };
        if self.reg.get(self.world.block_or_air(p)).name != "chest" {
            self.toast("Use a chave olhando para um baú vazio", 2.5);
            return true;
        }
        if !self.t5.hard.on && self.survival {
            self.toast("A chave não reage... ainda não é o modo difícil", 2.5);
            return true;
        }
        if let Some(Container::Chest(inv)) = self.surv.containers.get(&p) {
            if inv.slots.iter().any(|s| s.is_some()) {
                self.toast("O baú precisa estar vazio", 2.5);
                return true;
            }
        }
        self.surv.containers.remove(&p);
        self.world.set_block(p, voxel::AIR);
        let who = if light {
            "mimic_prism"
        } else if crate::tr_world::world_evil(self.seed) == crate::tr_world::Land::Ruin {
            "mimic_ruin"
        } else {
            "mimic_blight"
        };
        if self.mobs.spawn(who, p.as_vec3() + Vec3::new(0.5, 0.1, 0.5)) {
            if let Some(m) = self.mobs.list.last_mut() {
                m.aggro = true;
            }
        }
        self.toast("O baú acorda!", 2.5);
        self.sfx(Sfx::Thunder(0.3));
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        true
    }

    fn open_gift(&mut self, pool: &[(&str, u8, u8)]) {
        let at = self.body.eye() + self.forward() * 0.8;
        for _ in 0..2 + self.surv.rng.below(2) {
            let (item, lo, hi) = pool[self.surv.rng.below(pool.len() as u32) as usize];
            let Some(id) = self.items.id(item) else { continue };
            let n = lo + self.surv.rng.below((hi - lo + 1) as u32) as u8;
            let v = Vec3::new(self.surv.rng.f32() - 0.5, 2.5, self.surv.rng.f32() - 0.5);
            self.spawn_drop(at, ItemStack::new(id, u16::from(n)), v);
        }
        self.sfx(Sfx::Place(Material::Cloth));
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
    }

    pub(crate) fn tr_minis_use(&mut self, name: &str) -> Option<bool> {
        match name {
            "purification_powder" => self.free_soul(),
            "key_of_night" => Some(self.use_mimic_key(false)),
            "key_of_light" => Some(self.use_mimic_key(true)),
            "goodie_bag" => {
                self.open_gift(GOODIES);
                Some(true)
            }
            "present" => {
                self.open_gift(PRESENTS);
                Some(true)
            }
            _ => None,
        }
    }

    /// Holiday gifts from monsters, and the broken hero sword from the eclipse moth.
    pub(crate) fn minis_deaths(&mut self, deaths: &[(usize, Vec3)]) {
        if let Some(id) = self.items.id("broken_hero_sword") {
            for &(def, pos) in deaths {
                if self.mobs.reg.defs[def].name == "eclipse_moth" && self.surv.rng.f32() < 0.25 {
                    self.spawn_drop(pos + Vec3::Y * 0.5, ItemStack::new(id, 1), Vec3::Y * 2.0);
                }
            }
        }
        let gift = match holiday() {
            Holiday::Halloween => "goodie_bag",
            Holiday::Christmas => "present",
            Holiday::None => return,
        };
        let Some(id) = self.items.id(gift) else { return };
        for &(def, pos) in deaths {
            if self.mobs.reg.defs[def].kind == voxel::mob::Kind::Hostile && self.surv.rng.f32() < 1.0 / 25.0 {
                self.spawn_drop(pos + Vec3::Y * 0.5, ItemStack::new(id, 1), Vec3::Y * 2.0);
            }
        }
    }

    /// A boss fell: one time in ten, its trophy (and some bosses' own prizes).
    pub(crate) fn minis_boss_defeated(&mut self, name: &str) {
        let prize = match name {
            "celestial_herald" => Some(("celestial_wrath", 0.35)),
            "saucer" => Some(("star_key", 0.3)),
            _ => None,
        };
        if let Some((item, chance)) = prize {
            if let (Some(id), true) = (self.items.id(item), self.surv.rng.f32() < chance) {
                let at = self.body.pos + Vec3::Y * 1.5;
                self.spawn_drop(at, ItemStack::new(id, 1), Vec3::Y * 3.0);
            }
        }
        if self.surv.rng.f32() >= 0.1 {
            return;
        }
        if let Some(id) = self.items.id(&format!("trophy_{name}")) {
            let at = self.body.pos + Vec3::Y * 1.5;
            self.spawn_drop(at, ItemStack::new(id, 1), Vec3::Y * 3.0);
        }
    }

    /// Test scene: MC_TR=minis, the mini-bosses in a row (and a mimic), trophies on posts.
    pub(crate) fn test_tr_minis(&mut self) {
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = Vec3::new(-flat.z, 0.0, flat.x);
        let ground = |g: &Game, p: Vec3| g.world.surface_y(p.x.floor() as i32, p.z.floor() as i32).map(|y| y as f32 + 1.0).unwrap_or(p.y);
        let names = ["werewolf", "lost_girl", "nymph", "clown", "ice_golem", "goblin_summoner", "mimic_ruin", "mimic_prism", "npc_mycologist", "npc_winter_elder", "tortured_soul", "npc_tax_collector"];
        for (k, n) in names.iter().enumerate() {
            let p = self.body.pos + flat * (7.0 + (k % 2) as f32 * 2.0) + side * ((k as f32 - 4.5) * 1.6);
            let p = Vec3::new(p.x, ground(self, p), p.z);
            self.mobs.spawn(n, p);
            let last = self.mobs.list.len() - 1;
            let m = &mut self.mobs.list[last];
            let look = self.body.eye() - m.body.center();
            m.yaw = look.x.atan2(look.z);
        }
        for (k, b) in ["king_jelly", "great_eye", "hive_queen", "prism_empress"].iter().enumerate() {
            let p = (self.body.pos + flat * 4.0 + side * ((k as f32 - 1.5) * 1.5)).floor().as_ivec3();
            let p = IVec3::new(p.x, self.body.pos.y.floor() as i32 + 1, p.z);
            let id = self.reg.id_or_air(&format!("trophy_{b}"));
            self.world.set_block(p, id);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn calendar() {
        assert_eq!(super::month_day(0), (1, 1));
        // 2026-10-27
        assert_eq!(super::month_day(20_753), (10, 27));
        assert_eq!(super::month_day(20_812), (12, 25));
    }
}
