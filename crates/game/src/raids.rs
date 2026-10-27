//! Raids (Illager Invasion idea): some nights, once a town has a few people living in it,
//! raiders come at it in three waves from the dark around it. Each wave comes when the last
//! one is beaten. Marauders shoot, brutes hit hard and invokers throw spells; they go for the
//! townsfolk when you aren't near, and the town guard fights back. Beat all three waves for
//! a pile of emeralds. At dawn whatever is left of the raid gives up.

use glam::Vec3;
use voxel::item::ItemStack;
use voxel::worldgen::Dim;

use crate::audio::Sfx;
use crate::Game;

const RAIDERS: [&str; 4] = ["marauder", "brute", "invoker", "ravager"];
/// Who comes in each wave: (mob, how many).
const WAVES: [&[(&str, u32)]; 3] = [&[("marauder", 3)], &[("marauder", 3), ("brute", 1)], &[("marauder", 3), ("brute", 2), ("invoker", 1), ("ravager", 1)]];

#[derive(Default)]
pub struct Raid {
    /// The town under attack, while a raid is on.
    center: Option<Vec3>,
    wave: usize,
    was_night: bool,
    check_t: f32,
}

impl Game {
    fn town_center(&self) -> Option<Vec3> {
        let n = self.town.npcs.len();
        if n < 3 {
            return None;
        }
        Some(self.town.npcs.iter().map(|h| Vec3::from(h.home)).sum::<Vec3>() / n as f32)
    }

    fn raiders_left(&self, c: Vec3) -> usize {
        self.mobs.list.iter().filter(|m| m.alive() && RAIDERS.contains(&self.mobs.reg.defs[m.def].name.as_str()) && m.body.pos.distance(c) < 120.0).count()
    }

    fn spawn_wave(&mut self, c: Vec3) {
        let a0 = self.surv.rng.f32() * std::f32::consts::TAU;
        let mut k = 0;
        for &(name, n) in WAVES[self.raid.wave] {
            for _ in 0..n {
                // From one side, spread a little.
                let a = a0 + (k as f32 - 2.0) * 0.12;
                k += 1;
                let (x, z) = ((c.x + a.cos() * 30.0) as i32, (c.z + a.sin() * 30.0) as i32);
                let y = self.world.gen.height_at(x, z) + 1;
                self.mobs.spawn(name, Vec3::new(x as f32 + 0.5, y as f32, z as f32 + 0.5));
            }
        }
        self.sfx(Sfx::Thunder(0.35));
    }

    fn start_raid(&mut self, c: Vec3) {
        self.raid.center = Some(c);
        self.raid.wave = 0;
        self.toast("Saqueadores atacam a vila! (onda 1 de 3)", 5.0);
        self.spawn_wave(c);
    }

    pub(crate) fn update_raids(&mut self, dt: f32) {
        if dt == 0.0 || self.dim != Dim::Overworld {
            return;
        }
        self.raid.check_t -= dt;
        if self.raid.check_t > 0.0 {
            return;
        }
        self.raid.check_t = 2.0;
        let night = self.is_night();
        let dusk = night && !self.raid.was_night;
        self.raid.was_night = night;
        let Some(c) = self.raid.center else {
            // A raid may start as night falls on a town you're near.
            // Test hook: a raid on the spot, right away.
            if std::env::var("MC_RAID").is_ok() && self.raid.wave == 0 {
                self.time_of_day = 18000.0;
                self.raid.wave = 9;
                let c = self.body.pos;
                self.start_raid(c);
                for m in &mut self.mobs.list {
                    m.body.pos = c + (m.body.pos - c) * 0.4;
                }
                return;
            }
            if let Some(c) = self.town_center() {
                let near = c.distance(self.body.pos) < 80.0;
                if near && !self.mobs.peaceful && (dusk && self.surv.rng.f32() < 0.3) {
                    self.start_raid(c);
                }
            }
            return;
        };
        if !night {
            self.raid.center = None;
            self.toast("Ao amanhecer, os saqueadores desistem.", 4.0);
            return;
        }
        if self.raiders_left(c) > 0 {
            return;
        }
        self.raid.wave += 1;
        if self.raid.wave < WAVES.len() {
            self.toast(format!("Outra onda de saqueadores! (onda {} de 3)", self.raid.wave + 1), 4.0);
            self.spawn_wave(c);
            return;
        }
        self.raid.center = None;
        self.toast("A vila resistiu à invasão! Os moradores agradecem.", 5.0);
        if let Some(em) = self.items.id("emerald") {
            let n = 4 + (self.surv.rng.f32() * 5.0) as u8;
            self.pop_drop(self.body.pos + Vec3::Y, ItemStack::new(em, n));
        }
        self.sfx(Sfx::Xp);
    }
}
