//! L20 of the Terraria side's third round (plan/terraria-completo.md, "Reauditoria 3"): tools
//! and weapons that were still missing.
//!
//! - Drills and chainsaws (fast pickaxes and axes of the hardmode metals), the pickaxe axe and
//!   picksaw (one tool for stone and wood) and the hamaxes (wood and earth): a multi-tool
//!   becomes the right tool for each block you dig (`held_tool_for`).
//! - Builder's things: the toolbelt, toolbox, extendo grip and architect gizmo reach further
//!   (accessories, rpg.rs). The Rod of Discord teleports you to where you look, but twice in a
//!   row it hurts. Bottomless buckets never run dry; the super absorbent sponges drink a pool.
//! - Guns (tr_weapons.rs flies them): the minishark and megashark, the sniper rifle (sneak to
//!   look down its scope), the flamethrower that burns gel, the blowpipe that shoots seeds,
//!   the star cannon, the coin gun (the costlier the coin, the harder it hits), the elf gun,
//!   and the rocket and grenade launchers.
//! - Terraria's arrows, each with its own bite (`arrow_traits`, used by weapons.rs).
//! - The Medusa Head: everything hostile that sees you is hurt and slowed.

use glam::{IVec3, Vec3};
use voxel::block::{BlockId, ToolKind};
use voxel::item::ToolDef;
use voxel::physics::Terrain;

use crate::audio::Sfx;
use crate::Game;

/// Tools that are several at once: (item, the kinds of block they dig as their own).
const MULTI: &[(&str, &[ToolKind])] = &[
    ("pickaxe_axe", &[ToolKind::Pickaxe, ToolKind::Axe]),
    ("picksaw", &[ToolKind::Pickaxe, ToolKind::Axe]),
    ("molten_hamaxe", &[ToolKind::Axe, ToolKind::Shovel]),
    ("lumite_hamaxe", &[ToolKind::Axe, ToolKind::Shovel]),
];

/// Arrows of the Terraria side: (item, damage times a plain arrow's, mobs it passes through,
/// sets on fire).
const ARROWS: &[(&str, f32, u8, bool)] = &[
    ("flaming_arrow", 1.15, 0, true),
    ("frostburn_arrow", 1.3, 0, false),
    ("unholy_arrow", 1.4, 2, false),
    ("jester_arrow", 1.2, 5, false),
    ("hellfire_arrow", 1.6, 0, true),
    ("holy_arrow", 1.5, 1, false),
];

pub fn is_arrow(name: &str) -> bool {
    ARROWS.iter().any(|a| a.0 == name)
}

/// (damage factor, pierce, flame) of one of these arrows.
pub fn arrow_traits(name: &str) -> Option<(f32, u8, bool)> {
    ARROWS.iter().find(|a| a.0 == name).map(|a| (a.1, a.2, a.3))
}

/// What a gun of tr_weapons.rs shoots, when it isn't musket balls.
pub fn gun_ammo(gun: &str) -> Option<(&'static [&'static str], &'static str)> {
    Some(match gun {
        "blowpipe" => (&["wheat_seeds", "pumpkin_seeds", "melon_seeds", "beetroot_seeds"], "sementes"),
        "star_cannon" => (&["fallen_star"], "estrelas caídas"),
        "coin_gun" => (&["copper_coin", "silver_coin", "gold_coin", "platinum_coin"], "moedas"),
        "flamethrower" => (&["gel"], "gel"),
        "rocket_launcher" => (&["rocket"], "foguetes"),
        "grenade_launcher" => (&["grenade"], "granadas"),
        _ => return None,
    })
}

/// A coin gun's shot hurts more for a dearer coin (which of `gun_ammo` was used).
pub fn coin_factor(kind: usize) -> f32 {
    [1.0, 2.2, 5.0, 11.0][kind.min(3)]
}

impl Game {
    /// The held tool as it works on `block`: a multi-tool takes the kind the block wants.
    pub(crate) fn held_tool_for(&self, block: BlockId) -> Option<ToolDef> {
        // Riding the drill mount (tr3_world.rs): it digs like the best pickaxe, faster.
        if self.t5.comp.mount.is_some_and(|(k, _)| crate::companions::MOUNTS[k].item == "drill_containment_unit") {
            let mut t = self.items.id("netherite_pickaxe").and_then(|id| self.items.get(id).tool.clone())?;
            t.kind = self.reg.get(block).tool.unwrap_or(t.kind);
            t.speed *= 1.6;
            return Some(t);
        }
        let mut t = self.held_tool()?;
        let name = self.held_key();
        if let Some((_, kinds)) = MULTI.iter().find(|m| m.0 == name) {
            if let Some(want) = self.reg.get(block).tool {
                if kinds.contains(&want) {
                    t.kind = want;
                }
            }
        }
        Some(t)
    }

    /// The scope: sneaking with the sniper rifle, or with any gun and a rifle scope worn.
    pub(crate) fn scope_up(&self) -> bool {
        if self.crouch < 0.5 {
            return false;
        }
        let n = self.held_key();
        n == "sniper_rifle" || (crate::tr_weapons::is_gun(&n) && self.wears_accessory("rifle_scope"))
    }

    fn wears_accessory(&self, name: &str) -> bool {
        self.t5.rpg.loadouts[self.t5.rpg.loadout].acc.iter().flatten().any(|s| self.items.get(s.item).name == name)
    }

    pub(crate) fn tools_use(&mut self, name: &str) -> Option<bool> {
        match name {
            "rod_of_discord" => Some(self.discord()),
            "bottomless_water_bucket" | "bottomless_lava_bucket" => Some(self.pour(if name == "bottomless_water_bucket" { 1 } else { 2 })),
            "absorbent_sponge" | "lava_sponge" => Some(self.soak(if name == "absorbent_sponge" { 1 } else { 2 })),
            "medusa_head" => Some(self.medusa_head()),
            _ => None,
        }
    }

    /// Teleport to where you look; a second jump while the chaos lasts hurts a seventh of your life.
    fn discord(&mut self) -> bool {
        if self.t5.tr.r3.tools.discord_cd > 0.0 {
            return true;
        }
        let eye = self.body.eye();
        let f = self.forward();
        let world = &self.world;
        let hit = voxel::raycast::raycast(eye, f, 60.0, |p| Terrain::solid(world, p));
        let Some(hit) = hit else {
            self.toast("Longe demais", 1.2);
            return true;
        };
        // Stand on top of the block hit, or just before it on a wall.
        let mut to = hit.block + hit.normal;
        if hit.normal.y == 0 {
            for _ in 0..3 {
                if !self.world.solid(to - IVec3::Y) {
                    to -= IVec3::Y;
                }
            }
        }
        if self.world.solid(to) || self.world.solid(to + IVec3::Y) {
            self.toast("Não há espaço lá", 1.2);
            return true;
        }
        let from = self.body.pos;
        let glow = self.glow_layer();
        for i in 0..24 {
            let a = i as f32 / 24.0 * std::f32::consts::TAU;
            self.particles.glow(from + Vec3::new(a.cos() * 0.5, 1.0, a.sin() * 0.5), Vec3::new(a.cos(), 1.0, a.sin()), glow, [1.6, 0.6, 2.2], 0.09, 0.6);
        }
        self.body.pos = to.as_vec3() + Vec3::new(0.5, 0.0, 0.5);
        self.body.vel = Vec3::ZERO;
        self.body.fall_distance = 0.0;
        for i in 0..24 {
            let a = i as f32 / 24.0 * std::f32::consts::TAU;
            self.particles.glow(self.body.pos + Vec3::new(a.cos() * 0.5, 1.0, a.sin() * 0.5), Vec3::new(-a.cos(), 0.5, -a.sin()), glow, [1.6, 0.6, 2.2], 0.09, 0.6);
        }
        self.sfx(Sfx::Cast(3));
        let t = &mut self.t5.tr.r3.tools;
        t.discord_cd = 0.4;
        if t.chaos > 0.0 {
            let hurt = self.max_health / 7.0;
            self.damage(hurt, None);
            self.toast("Caos! Teleportar de novo tão cedo machuca", 2.0);
        }
        self.t5.tr.r3.tools.chaos = 6.0;
        self.anim.start_swing();
        true
    }

    /// A bottomless bucket pours a source and stays full.
    fn pour(&mut self, fluid: u8) -> bool {
        let Some(hit) = self.target else { return false };
        let pos = if self.reg.get(self.world.block_or_air(hit.block)).shape == voxel::block::Shape::Cross { hit.block } else { hit.block + hit.normal };
        let cur = self.world.block_or_air(pos);
        if !(cur == voxel::AIR || self.reg.get(cur).shape == voxel::block::Shape::Cross || (self.world.fluid(cur) != 0 && self.reg.get(cur).variant != 0)) {
            return false;
        }
        if fluid == 1 && self.dim == voxel::worldgen::Dim::Nether {
            self.sfx(Sfx::Fizz);
            return true;
        }
        self.world.set_block(pos, self.fluids.id(fluid, 0));
        self.sfx(if fluid == 1 { Sfx::Splash(0.4) } else { Sfx::Fizz });
        self.anim.start_swing();
        true
    }

    /// The sponge drinks every block of its liquid within four blocks of what you point at.
    fn soak(&mut self, fluid: u8) -> bool {
        let center = self.target.map(|h| h.block + h.normal).unwrap_or_else(|| self.body.pos.floor().as_ivec3());
        let mut n = 0;
        for x in -4..=4 {
            for y in -4..=4 {
                for z in -4..=4 {
                    if x * x + y * y + z * z > 18 {
                        continue;
                    }
                    let p = center + IVec3::new(x, y, z);
                    let id = self.world.block_or_air(p);
                    if self.world.fluid(id) == fluid {
                        self.world.set_block(p, voxel::AIR);
                        n += 1;
                    }
                }
            }
        }
        if n > 0 {
            self.sfx(if fluid == 1 { Sfx::Splash(0.3) } else { Sfx::Fizz });
            self.anim.start_swing();
        }
        true
    }

    /// The Medusa Head: every hostile that can see you, within 14 blocks, takes a stony blow.
    fn medusa_head(&mut self) -> bool {
        if self.t5.tr.r3.tools.medusa_cd > 0.0 {
            return true;
        }
        self.t5.tr.r3.tools.medusa_cd = 1.2;
        let me = self.body.eye();
        let mut hit = Vec::new();
        for (k, m) in self.mobs.list.iter().enumerate() {
            if !m.alive() || self.mobs.reg.defs[m.def].kind != voxel::mob::Kind::Hostile {
                continue;
            }
            let c = m.body.center();
            if c.distance(me) < 14.0 && self.line_clear(me, c) {
                hit.push((k, c));
            }
        }
        let glow = self.glow_layer();
        let dmg = 24.0 * self.t5.rpg.stats.dmg[crate::rpg::MAGIC];
        for &(k, c) in &hit {
            let m = &mut self.mobs.list[k];
            m.hurt(dmg, me);
            m.aggro = true;
            m.chill = m.chill.max(3.0);
            for i in 0..8 {
                let v = (me - c).normalize_or_zero() * (1.0 + i as f32 * 0.6);
                self.particles.glow(c, v, glow, [2.0, 2.0, 0.8], 0.06, 0.4);
            }
        }
        for i in 0..20 {
            let a = i as f32 / 20.0 * std::f32::consts::TAU;
            self.particles.glow(me + self.forward() * 0.6, Vec3::new(a.cos(), a.sin(), 0.0) * 3.0, glow, [2.0, 2.0, 0.8], 0.06, 0.4);
        }
        self.sfx(Sfx::Cast(2));
        self.anim.start_swing();
        true
    }

    /// Nothing solid between two points (stepping a quarter block).
    fn line_clear(&self, a: Vec3, b: Vec3) -> bool {
        let d = b - a;
        let n = (d.length() * 4.0) as i32;
        (1..n).all(|i| !self.world.solid((a + d * (i as f32 / n as f32)).floor().as_ivec3()))
    }

    pub(crate) fn update_tools(&mut self, dt: f32) {
        let t = &mut self.t5.tr.r3.tools;
        t.discord_cd -= dt;
        t.chaos -= dt;
        t.medusa_cd -= dt;
        // Drills and chainsaws buzz while they bite.
        t.buzz_t -= dt;
        let n = self.held_key();
        if (n.ends_with("_drill") || n.ends_with("_chainsaw")) && self.mouse_held[0] && self.target.is_some() {
            if self.t5.tr.r3.tools.buzz_t <= 0.0 {
                self.t5.tr.r3.tools.buzz_t = 0.12;
                if let Some(h) = self.target {
                    let at = h.block.as_vec3() + Vec3::splat(0.5) + h.normal.as_vec3() * 0.55;
                    let glow = self.glow_layer();
                    let r = &mut self.surv.rng;
                    let v = Vec3::new(r.f32() - 0.5, r.f32(), r.f32() - 0.5) * 3.0;
                    self.particles.glow(at, v, glow, [1.8, 1.4, 0.8], 0.03, 0.25);
                    self.sfx_at(Sfx::Hit(crate::audio::Material::Metal), at, 0.25, 12.0);
                }
            }
        }
    }
}

#[derive(Default)]
pub struct Tools {
    discord_cd: f32,
    /// Seconds of chaos left after a teleport.
    chaos: f32,
    medusa_cd: f32,
    buzz_t: f32,
}
