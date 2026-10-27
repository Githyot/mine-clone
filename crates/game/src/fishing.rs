//! Fishing (the Aquaculture idea): cast a rod, wait for the float to dip, reel in at the bite.
//! Mostly fish, sometimes junk, rarely treasure. The float flies, lands and bobs on water, and a
//! line hangs from the rod tip to it.

use glam::{IVec3, Vec3};
use voxel::item::{ItemStack, Look};
use voxel::physics::Terrain;

use crate::audio::{Material, Sfx};
use crate::item_render::GpuItem;
use crate::renderer::LineVertex;
use crate::Game;

pub struct Bobber {
    pos: Vec3,
    vel: Vec3,
    in_water: bool,
    landed: bool,
    /// Seconds until a fish bites.
    wait: f32,
    /// Seconds left to reel in a biting fish.
    bite: f32,
    age: f32,
}

/// (item, weight): what comes up on the line.
const FISH: &[(&str, u32)] = &[("cod", 40), ("salmon", 22), ("trout", 14), ("catfish", 9), ("tilapia", 10)];
/// Extra fish that only live in some biomes (Aquaculture idea): (biomes, item, weight).
const LOCAL_FISH: &[(&[&str], &str, u32)] = &[
    (&["Jungle", "Swamp", "Mangrove Swamp"], "piranha", 18),
    (&["Jungle", "Swamp", "Mangrove Swamp"], "pirarucu", 8),
    (&["Snowy Plains", "Taiga", "Mountains"], "arctic_char", 22),
    (&["Plains", "Forest", "Birch Forest", "Dark Forest", "Cherry Grove", "Savanna"], "perch", 18),
    (&["Ocean", "Beach"], "tuna", 16),
];
const JUNK: &[(&str, u32)] = &[("stick", 3), ("bone", 2), ("leather", 2), ("string", 2), ("rotten_flesh", 2)];
const TREASURE: &[(&str, u32)] = &[("emerald", 3), ("gold_ingot", 3), ("bow", 2), ("diamond", 1), ("lapis_lazuli", 3)];

impl Game {
    fn wait_for_bite(&mut self) -> f32 {
        // Rain brings the fish up.
        let rain = if self.weather.rain > 0.5 { 0.7 } else { 1.0 };
        (5.0 + self.surv.rng.f32() * 20.0) * rain * self.rod_bite_factor()
    }

    /// Right click with the rod: cast, or reel in.
    pub(crate) fn use_rod(&mut self) -> bool {
        if let Some(b) = self.bobber.take() {
            if b.bite > 0.0 {
                self.catch_fish(b.pos);
            } else if b.landed && !b.in_water {
                // Reeling a float stuck in the ground costs a little wear.
                self.wear_rod(1);
            }
            self.sfx(Sfx::Place(Material::Cloth));
            self.anim.start_swing();
            return true;
        }
        let eye = self.body.eye();
        let fwd = self.forward();
        self.bobber = Some(Bobber { pos: eye + fwd * 0.6, vel: fwd * 13.0 + Vec3::Y * 3.0, in_water: false, landed: false, wait: 0.0, bite: 0.0, age: 0.0 });
        self.sfx(Sfx::Place(Material::Cloth));
        self.anim.start_swing();
        true
    }

    fn wear_rod(&mut self, n: u16) {
        if self.survival && self.held_name().as_deref() == Some("fishing_rod") {
            self.wear_held(self.rod_wear(n));
        }
    }

    pub(crate) fn held_name(&self) -> Option<String> {
        self.held_stack().map(|s| self.items.get(s.item).name.clone())
    }

    fn pick(&mut self, table: &[(&'static str, u32)]) -> Option<&'static str> {
        let total: u32 = table.iter().map(|t| t.1).sum();
        let mut r = self.surv.rng.below(total.max(1));
        for &(name, w) in table {
            if r < w {
                return Some(name);
            }
            r -= w;
        }
        None
    }

    fn catch_fish(&mut self, at: Vec3) {
        let roll = self.surv.rng.f32();
        let biome = self.world.gen.base_biome_at(at.x.floor() as i32, at.z.floor() as i32).name();
        let more = crate::mods::aquaculture::MORE_FISH.iter().filter(|f| f.0.contains(&biome)).map(|f| (f.1, f.2));
        let fish: Vec<(&'static str, u32)> = FISH.iter().copied().chain(LOCAL_FISH.iter().filter(|f| f.0.contains(&biome)).map(|f| (f.1, f.2))).chain(more).map(|(n, w)| (n, self.rod_fish_weight(n, w))).collect();
        let junk: Vec<(&'static str, u32)> = JUNK.iter().chain(crate::mods::aquaculture::MORE_JUNK).copied().collect();
        let (fish_p, junk_p) = self.rod_odds();
        let table: &[(&'static str, u32)] = if roll < fish_p { &fish } else if roll < fish_p + junk_p { &junk } else { TREASURE };
        let Some(name) = self.pick(table) else { return };
        let Some(id) = self.items.id(name) else { return };
        // The catch flies to the player.
        let to = self.body.eye() - at;
        let vel = Vec3::new(to.x * 1.0, to.y * 1.0 + to.length().sqrt() * 1.6, to.z * 1.0);
        self.spawn_drop(at + Vec3::Y * 0.2, ItemStack::new(id, 1), vel);
        self.sfx(Sfx::Splash(0.35));
        self.particles.splash(at, self.reg.get(self.reg.id_or_air("water")).faces[2], 0.3);
        if self.survival {
            let n = 1 + self.surv.rng.below(3);
            self.spawn_orbs(self.body.pos + Vec3::Y, n);
        }
        self.wear_rod(1);
        self.rod_caught(Some(name).filter(|_| roll < fish_p), at);
    }

    pub(crate) fn update_fishing(&mut self, dt: f32) {
        if self.bobber.is_none() {
            return;
        }
        if !self.held_name().as_deref().is_some_and(crate::mods::aquaculture::is_rod) {
            self.bobber = None;
            return;
        }
        let mut b = self.bobber.take().unwrap();
        b.age += dt;
        if !b.in_water {
            if !b.landed {
                b.vel.y -= 20.0 * dt;
                b.vel *= 1.0 - 0.4 * dt;
                let next = b.pos + b.vel * dt;
                let cell = next.floor().as_ivec3();
                let id = self.world.block_or_air(cell);
                if self.world.fluid(id) == 1 {
                    b.in_water = true;
                    b.pos = Vec3::new(next.x, cell.y as f32 + 0.85, next.z);
                    // Float to the top of the water.
                    let mut top = cell;
                    while self.world.fluid(self.world.block_or_air(top + IVec3::Y)) == 1 && top.y < cell.y + 16 {
                        top.y += 1;
                    }
                    b.pos.y = top.y as f32 + 0.85;
                    b.vel = Vec3::ZERO;
                    b.wait = self.wait_for_bite();
                    self.particles.splash(b.pos, self.reg.get(id).faces[2], 0.15);
                } else if Terrain::solid(&self.world, cell) {
                    b.landed = true;
                    b.vel = Vec3::ZERO;
                } else {
                    b.pos = next;
                }
            }
        } else if b.bite > 0.0 {
            b.bite -= dt;
            if b.bite <= 0.0 {
                // Got away.
                b.wait = self.wait_for_bite();
            }
        } else {
            b.wait -= dt;
            if b.wait <= 0.0 {
                b.bite = self.rod_bite_window();
                self.sfx(Sfx::Splash(0.22));
                let water = self.reg.get(self.reg.id_or_air("water")).faces[2];
                self.particles.splash(b.pos, water, 0.2);
            }
        }
        if b.pos.distance(self.body.pos) > 32.0 || b.pos.y < voxel::chunk::MIN_Y as f32 || b.age > 300.0 {
            return;
        }
        self.bobber = Some(b);
    }

    /// Where the float is drawn: bobbing on water, pulled under while a fish bites.
    fn bobber_pos(&self) -> Option<Vec3> {
        let b = self.bobber.as_ref()?;
        let mut p = b.pos;
        if b.in_water {
            p.y += (self.now * 2.6).sin() * 0.04;
            if b.bite > 0.0 {
                p.y -= 0.25;
            }
        }
        Some(p)
    }

    pub(crate) fn bobber_instance(&self, cam_block: IVec3) -> Option<GpuItem> {
        let p = self.bobber_pos()? - cam_block.as_vec3();
        let id = self.items.id("fishing_bobber")?;
        let Look::Sprite(l) = self.items.get(id).look else { return None };
        let layer = (self.item_base + l) as f32;
        let lp = self.bobber_pos()?.floor().as_ivec3();
        let light = self.world.light(lp);
        Some(GpuItem { pos: [p.x, p.y + 0.05, p.z, 0.3], info: [-self.yaw, 1.0, (light >> 4) as f32 / 15.0, (light & 15) as f32 / 15.0], layers: [layer; 4], rot: [0.0, 0.0, 0.0, 0.0], ..bytemuck::Zeroable::zeroed() })
    }

    /// The fishing line from the rod tip to the float, sagging a little.
    pub(crate) fn fishing_line(&self, cam_block: IVec3, third: bool) -> Vec<LineVertex> {
        let Some(end) = self.bobber_pos() else { return Vec::new() };
        let fwd = self.forward();
        let right = fwd.cross(Vec3::Y).normalize_or_zero();
        let tip = if third {
            self.body.pos + Vec3::Y * 1.9 + Vec3::new(fwd.x, 0.0, fwd.z).normalize_or_zero() * 0.9 + right * 0.35
        } else {
            self.body.eye() + fwd * 0.9 + right * 0.45 + Vec3::Y * 0.12
        };
        let c = cam_block.as_vec3();
        let n = 10;
        let sag = (tip.distance(end) * 0.08).min(1.5);
        let point = |i: usize| {
            let t = i as f32 / n as f32;
            tip.lerp(end + Vec3::Y * 0.12, t) - Vec3::Y * sag * 4.0 * t * (1.0 - t) - c
        };
        let col = [0.12, 0.12, 0.12, 0.9];
        (0..n).flat_map(|i| [LineVertex { pos: point(i).to_array(), color: col }, LineVertex { pos: point(i + 1).to_array(), color: col }]).collect()
    }
}
