//! The rest of the Nether and the End (roadmap phase 5): what their creatures throw (ghast
//! fireballs you can bat back, blaze volleys, wither skulls, shulker bullets that lift you
//! off the ground, the dragon's acid breath), magma cubes splitting, piglins that hate anyone
//! without gold and trade for it, the Wither built from soul sand and skulls, the ender dragon
//! circling its pillars, end gateways out to the far islands, the respawn anchor, and elytra
//! gliding with firework rockets. Our own take, from the public descriptions of the game.

use glam::{IVec3, Vec3};
use voxel::item::ItemStack;
use voxel::physics::Terrain;
use voxel::worldgen::{Dim, END_EXIT_Y};
use voxel::AIR;

use crate::audio::{Material, Sfx};
use crate::Game;

pub const DRAGON: &str = "ender_dragon";
pub const WITHER: &str = "wither";

/// How far out the end gateways stand from the middle of the main island.
const GATEWAY_RING: f32 = 96.0;
/// How far a gateway throws you.
const GATEWAY_JUMP: f32 = 1000.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShotKind {
    /// A ghast's big fireball: explodes where it lands; a well-timed swing sends it back.
    Ghast,
    /// A blaze's small fireball: burns.
    Blaze,
    /// The Wither's skull: a small blast and the withering sickness.
    Skull,
    /// A shulker's bullet: follows you and lifts you up into the air.
    Shulker,
    /// The dragon's fireball: leaves a cloud of acid breath where it lands.
    Dragon,
}

/// Mobs that throw one of our shots instead of an arrow.
pub fn shot_kind(mob: &str) -> Option<ShotKind> {
    match mob {
        "ghast" => Some(ShotKind::Ghast),
        "blaze" => Some(ShotKind::Blaze),
        WITHER => Some(ShotKind::Skull),
        "shulker" => Some(ShotKind::Shulker),
        _ => None,
    }
}

/// Bosses that fight their own way (the generic boss specials leave them alone).
pub fn own_boss(name: &str) -> bool {
    name == DRAGON || name == WITHER
}

pub struct Shot {
    pub pos: Vec3,
    pub vel: Vec3,
    pub kind: ShotKind,
    pub life: f32,
    /// Batted back by the player: now it hurts monsters instead.
    pub friendly: bool,
}

/// A lingering cloud of dragon's breath.
pub struct Cloud {
    pub pos: Vec3,
    pub life: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum Phase {
    /// Circling the pillars, healing from the crystals, spitting fireballs now and then.
    #[default]
    Circle,
    /// Diving at the player.
    Swoop,
    /// Sitting over the exit fountain breathing acid.
    Perch,
}

#[derive(Default)]
struct DragonBrain {
    phase: Phase,
    /// Seconds in this phase.
    t: f32,
    angle: f32,
    shot_t: f32,
    rounds: u32,
    held: Option<(Vec3, Vec3, f32)>,
}

#[derive(Default)]
pub struct NetherEnd {
    pub shots: Vec<Shot>,
    pub clouds: Vec<Cloud>,
    /// Seconds left floating up (shulker bullets).
    pub levitate: f32,
    /// Seconds left of the withering sickness, and of burning.
    pub withering: f32,
    pub burning: f32,
    tick: f32,
    /// Gliding on elytra, and seconds of rocket boost left.
    pub gliding: bool,
    pub boost: f32,
    jump_was: bool,
    dragon: DragonBrain,
    gateway_cd: f32,
    piglin_t: f32,
    wither_t: f32,
    wither_wake: f32,
    check_gateway: bool,
    pub gateway_landing: bool,
    /// Riding a strider over the lava: which one (by its per-mob phase), and where it was.
    strider: Option<f32>,
    strider_held: Option<(Vec3, f32)>,
    /// The way back to build beside a far island once its chunks are in.
    pending_gateway: Option<IVec3>,
    /// A charged respawn anchor in the Nether: where, and charges left.
    pub anchor: Option<(IVec3, u8)>,
}

impl NetherEnd {
    fn anchor_path(dir: &std::path::Path) -> std::path::PathBuf {
        dir.join("anchor.ron")
    }

    /// The respawn anchor saved with the Nether's folder.
    pub fn load_anchor(&mut self, nether_dir: &std::path::Path) {
        self.anchor = std::fs::read_to_string(Self::anchor_path(nether_dir)).ok().and_then(|t| ron::from_str::<(i32, i32, i32, u8)>(&t).ok()).map(|(x, y, z, c)| (IVec3::new(x, y, z), c));
    }

    pub fn save_anchor(&self, nether_dir: &std::path::Path) {
        let path = Self::anchor_path(nether_dir);
        match self.anchor {
            Some((p, c)) => {
                let _ = std::fs::create_dir_all(nether_dir);
                let _ = std::fs::write(path, ron::to_string(&(p.x, p.y, p.z, c)).unwrap_or_default());
            }
            None => {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

impl Game {
    fn ne_mob_at(&self, p: Vec3, friendly_only: bool) -> Option<usize> {
        self.mobs.list.iter().position(|m| {
            if !m.alive() {
                return false;
            }
            let d = &self.mobs.reg.defs[m.def];
            if friendly_only && d.kind == voxel::mob::Kind::Passive {
                return false;
            }
            let b = m.body.pos;
            let r = d.width * 0.5 + 0.2;
            (p.x - b.x).abs() < r && (p.z - b.z).abs() < r && p.y > b.y - 0.1 && p.y < b.y + d.height + 0.1
        })
    }

    fn mob_name(&self, i: usize) -> &str {
        &self.mobs.reg.defs[self.mobs.list[i].def].name
    }

    /// Fire one of our shots from a mob at `at` (the player's middle).
    fn throw_shot(&mut self, kind: ShotKind, from: Vec3, at: Vec3) {
        let dir = (at - from).normalize_or_zero();
        let speed = match kind {
            ShotKind::Ghast => 13.0,
            ShotKind::Blaze => 16.0,
            ShotKind::Skull => 15.0,
            ShotKind::Shulker => 7.0,
            ShotKind::Dragon => 18.0,
        };
        let n = if kind == ShotKind::Blaze { 3 } else { 1 };
        for k in 0..n {
            // Blazes spit three in a loose spread.
            let spread = if n > 1 { Vec3::new((k as f32 - 1.0) * 0.08, (k % 2) as f32 * 0.05, -(k as f32 - 1.0) * 0.06) } else { Vec3::ZERO };
            let v = (dir + spread).normalize_or_zero() * speed;
            self.ne.shots.push(Shot { pos: from + dir * 0.8 + spread, vel: v, kind, life: 8.0, friendly: false });
        }
        let s = match kind {
            ShotKind::Ghast | ShotKind::Blaze => Sfx::Cast(0),
            ShotKind::Skull | ShotKind::Dragon => Sfx::Cast(3),
            ShotKind::Shulker => Sfx::Click,
        };
        if from.distance(self.body.pos) < 48.0 {
            self.sfx(s);
        }
    }

    /// Everything this module does each tick, after the mobs have moved.
    pub(crate) fn update_nether_end(&mut self, dt: f32, special: &[(ShotKind, Vec3, Vec3)], deaths: &[(usize, Vec3)]) {
        for &(kind, from, at) in special {
            self.throw_shot(kind, from, at);
        }
        // Magma cubes break into little ones.
        for &(def, pos) in deaths {
            if self.mobs.reg.defs[def].name == "magma_cube" {
                let n = 2 + (self.surv.rng.f32() * 3.0) as i32;
                for k in 0..n {
                    let a = k as f32 * 2.3;
                    self.mobs.spawn("magma_cube_small", pos + Vec3::new(a.cos() * 0.6, 0.3, a.sin() * 0.6));
                }
            }
        }
        if dt <= 0.0 {
            self.draw_shots();
            return;
        }
        self.update_shots(dt);
        self.update_clouds(dt);
        self.player_effects(dt);
        self.update_piglins(dt);
        if let (Some(i), Some((p, yaw))) = (self.strider_index(), self.ne.strider_held.take()) {
            let m = &mut self.mobs.list[i];
            m.body.pos = p;
            m.body.vel = Vec3::ZERO;
            m.yaw = yaw;
        }
        self.update_dragon(dt);
        self.update_wither(dt);
        self.gateway_step(dt);
        self.keep_gateway();
        self.draw_shots();
    }

    fn draw_shots(&mut self) {
        let glow = self.glow_layer();
        for s in &self.ne.shots {
            let (tint, size) = match s.kind {
                ShotKind::Ghast => ([1.4, 0.6, 0.15], 0.55),
                ShotKind::Blaze => ([1.3, 0.75, 0.2], 0.25),
                ShotKind::Skull => ([0.25, 0.22, 0.3], 0.4),
                ShotKind::Shulker => ([1.2, 1.15, 0.9], 0.22),
                ShotKind::Dragon => ([0.9, 0.3, 1.2], 0.45),
            };
            self.particles.glow(s.pos, Vec3::ZERO, glow, tint, size, 0.06);
        }
    }

    fn update_shots(&mut self, dt: f32) {
        let mut shots = std::mem::take(&mut self.ne.shots);
        let me = self.body.pos;
        let eye = self.body.eye() - Vec3::Y * 0.4;
        let glow = self.glow_layer();
        let mut blasts: Vec<(Vec3, ShotKind, bool)> = Vec::new();
        let mut hits: Vec<ShotKind> = Vec::new();
        let mut mob_hits: Vec<(usize, ShotKind, Vec3)> = Vec::new();
        shots.retain_mut(|s| {
            s.life -= dt;
            if s.life <= 0.0 {
                return false;
            }
            if s.kind == ShotKind::Shulker && !s.friendly {
                // Homing, slowly.
                let want = (eye - s.pos).normalize_or_zero() * 7.0;
                s.vel = s.vel.lerp(want, (dt * 1.5).min(1.0));
            }
            // A trail of sparks or smoke behind.
            if self.surv.rng.f32() < dt * 30.0 {
                let tint = match s.kind {
                    ShotKind::Ghast | ShotKind::Blaze => [1.1, 0.5, 0.1],
                    ShotKind::Skull => [0.2, 0.18, 0.22],
                    ShotKind::Shulker => [1.0, 1.0, 0.85],
                    ShotKind::Dragon => [0.8, 0.25, 1.0],
                };
                self.particles.glow(s.pos, -s.vel * 0.05, glow, tint, 0.12, 0.5);
            }
            let step = s.vel * dt;
            let n = (step.length() / 0.25).ceil().max(1.0) as usize;
            for _ in 0..n {
                s.pos += step / n as f32;
                if s.friendly {
                    if let Some(i) = self.ne_mob_at(s.pos, true) {
                        mob_hits.push((i, s.kind, s.pos));
                        return false;
                    }
                } else {
                    let d = s.pos - me;
                    if d.x.abs() < 0.5 && d.z.abs() < 0.5 && d.y > -0.1 && d.y < 1.9 {
                        if matches!(s.kind, ShotKind::Ghast | ShotKind::Skull | ShotKind::Dragon) {
                            blasts.push((s.pos, s.kind, false));
                        }
                        hits.push(s.kind);
                        return false;
                    }
                }
                if self.world.solid(s.pos.floor().as_ivec3()) {
                    blasts.push((s.pos - step / n as f32, s.kind, s.friendly));
                    return false;
                }
            }
            true
        });
        self.ne.shots = shots;
        for kind in hits {
            match kind {
                ShotKind::Ghast => self.damage(6.0, None),
                ShotKind::Blaze => {
                    self.damage(5.0, None);
                    self.ne.burning = self.ne.burning.max(4.0);
                }
                ShotKind::Skull => {
                    self.damage(5.0, None);
                    self.ne.withering = self.ne.withering.max(8.0);
                }
                ShotKind::Shulker => {
                    self.damage(4.0, None);
                    self.ne.levitate = 5.0;
                    self.toast("Você está levitando!", 2.0);
                }
                ShotKind::Dragon => self.damage(6.0, None),
            }
        }
        for (i, kind, at) in mob_hits {
            let dmg = match kind {
                // A ghast hit by its own fireball dies at once, like the classic achievement.
                ShotKind::Ghast if self.mob_name(i) == "ghast" => 1000.0,
                ShotKind::Ghast => 12.0,
                _ => 6.0,
            };
            let me = self.body.pos;
            let m = &mut self.mobs.list[i];
            if m.hurt(dmg, me) {
                let (d, p) = (m.def, m.body.pos);
                let name = self.mobs.reg.defs[d].name.clone();
                if name == "ghast" {
                    self.toast("Devolvido ao remetente!", 3.0);
                }
                if self.survival {
                    self.surv_mob_deaths(&[(d, p)]);
                }
            }
            if kind == ShotKind::Ghast {
                self.explode(at, 1.2);
            }
        }
        for (at, kind, _) in blasts {
            match kind {
                ShotKind::Ghast => self.explode(at, 1.4),
                ShotKind::Skull => {
                    self.explode(at, 0.9);
                    if at.distance(self.body.pos + Vec3::Y) < 2.5 {
                        self.ne.withering = self.ne.withering.max(5.0);
                    }
                }
                ShotKind::Dragon => {
                    self.ne.clouds.push(Cloud { pos: at, life: 9.0 });
                    self.sfx(Sfx::Fizz);
                }
                ShotKind::Blaze => {
                    // Small fireballs light the ground they hit.
                    let p = at.floor().as_ivec3();
                    let (layer, tint) = self.block_look(self.reg.id_or_air("magma_block"));
                    self.particles.block_bits(p, layer, tint, 4, 0.5);
                }
                ShotKind::Shulker => {}
            }
        }
    }

    fn update_clouds(&mut self, dt: f32) {
        let glow = self.glow_layer();
        let me = self.body.pos;
        let mut hurt = false;
        for c in &mut self.ne.clouds {
            c.life -= dt;
            let r = 3.0;
            if self.surv.rng.f32() < dt * 25.0 {
                let a = self.surv.rng.f32() * std::f32::consts::TAU;
                let d = self.surv.rng.f32().sqrt() * r;
                self.particles.glow(c.pos + Vec3::new(a.cos() * d, 0.2, a.sin() * d), Vec3::Y * 0.3, glow, [0.8, 0.3, 1.0], 0.18, 1.2);
            }
            if (me - c.pos).length() < r && (me.y - c.pos.y).abs() < 2.0 {
                hurt = true;
            }
        }
        self.ne.clouds.retain(|c| c.life > 0.0);
        self.ne.tick += dt;
        if hurt && self.ne.tick >= 0.75 {
            self.ne.tick = 0.0;
            self.damage(2.0, None);
        }
    }

    /// Levitation, withering and burning.
    fn player_effects(&mut self, dt: f32) {
        if self.ne.levitate > 0.0 {
            self.ne.levitate -= dt;
            self.body.vel.y = 1.6;
            self.body.flying = false;
        }
        let glow = self.glow_layer();
        if self.ne.withering > 0.0 {
            let before = self.ne.withering;
            self.ne.withering -= dt;
            if before.floor() != self.ne.withering.floor() && self.ne.withering > 0.0 {
                self.damage(1.0, None);
            }
            if self.surv.rng.f32() < dt * 8.0 {
                self.particles.glow(self.body.pos + Vec3::Y * 1.2, Vec3::Y * 0.4, glow, [0.15, 0.12, 0.18], 0.15, 0.8);
            }
        }
        if self.ne.burning > 0.0 {
            if self.body.in_water {
                self.ne.burning = 0.0;
            }
            let before = self.ne.burning;
            self.ne.burning -= dt;
            if before.floor() != self.ne.burning.floor() && self.ne.burning > 0.0 {
                self.damage(1.0, None);
            }
            if self.surv.rng.f32() < dt * 20.0 {
                let a = self.surv.rng.f32() * std::f32::consts::TAU;
                self.particles.glow(self.body.pos + Vec3::new(a.cos() * 0.3, 0.3 + self.surv.rng.f32() * 1.4, a.sin() * 0.3), Vec3::Y * 1.2, glow, [1.3, 0.6, 0.15], 0.14, 0.4);
            }
        }
    }

    /// Wearing a piece of gold armour keeps piglins friendly.
    fn wears_gold(&self) -> bool {
        self.surv.armor.iter().flatten().any(|s| self.items.get(s.item).name.starts_with("golden_"))
    }

    fn update_piglins(&mut self, dt: f32) {
        self.ne.piglin_t -= dt;
        if self.ne.piglin_t > 0.0 || !self.survival {
            return;
        }
        self.ne.piglin_t = 0.5;
        let gold = self.wears_gold();
        let me = self.body.pos;
        for m in self.mobs.list.iter_mut() {
            if !m.alive() || self.mobs.reg.defs[m.def].name != "piglin" {
                continue;
            }
            if !gold && m.body.pos.distance(me) < 10.0 {
                m.aggro = true;
            }
        }
    }

    /// Right-click a piglin holding gold: it takes the ingot, looks it over and throws you
    /// something back (the classic barter, our own list).
    pub(crate) fn barter_with_piglin(&mut self) -> bool {
        let Some(st) = self.held_stack() else { return false };
        if self.items.get(st.item).name != "gold_ingot" {
            return false;
        }
        let Some((i, t)) = self.mobs.pick(self.body.eye(), self.forward(), 4.0) else { return false };
        if t > self.target.map(|h| h.distance).unwrap_or(f32::MAX) || self.mob_name(i) != "piglin" || self.mobs.list[i].aggro {
            return false;
        }
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        const BARTER: &[(&str, u8, u8, u32)] = &[
            ("ender_pearl", 2, 4, 10),
            ("string", 3, 9, 20),
            ("quartz", 5, 12, 20),
            ("obsidian", 1, 1, 40),
            ("crying_obsidian", 1, 3, 40),
            ("fire_charge", 1, 1, 40),
            ("leather", 2, 4, 40),
            ("soul_sand", 2, 8, 40),
            ("nether_brick", 2, 8, 40),
            ("gravel", 8, 16, 40),
            ("blackstone", 8, 16, 40),
            ("magma_cream", 2, 6, 20),
            ("iron_nugget", 10, 36, 10),
            ("gold_nugget", 6, 18, 20),
        ];
        let total: u32 = BARTER.iter().map(|b| b.3).sum();
        let mut roll = (self.surv.rng.f32() * total as f32) as u32;
        let mut pick = BARTER[0];
        for b in BARTER {
            if roll < b.3 {
                pick = *b;
                break;
            }
            roll -= b.3;
        }
        let at = self.mobs.list[i].body.pos + Vec3::Y;
        let n = pick.1 + (self.surv.rng.f32() * (pick.2 - pick.1 + 1) as f32) as u8;
        let item = self.items.id(pick.0).or_else(|| self.items.id("gold_nugget"));
        if let Some(item) = item {
            self.pop_drop(at, ItemStack::new(item, n.max(1)));
        }
        // It turns to look at you while it thinks.
        let m = &mut self.mobs.list[i];
        let to = self.body.pos - m.body.pos;
        m.yaw = to.x.atan2(to.z);
        self.sfx(Sfx::Pickup);
        self.anim.start_swing();
        true
    }

    /// A swing near an incoming fireball sends it back the way it came.
    pub(crate) fn deflect_shot(&mut self) -> bool {
        let eye = self.body.eye();
        let f = self.forward();
        for s in self.ne.shots.iter_mut() {
            if s.friendly || !matches!(s.kind, ShotKind::Ghast | ShotKind::Blaze | ShotKind::Shulker | ShotKind::Dragon) {
                continue;
            }
            let to = s.pos - eye;
            if to.length() < 3.5 && to.normalize_or_zero().dot(f) > 0.5 {
                let speed = s.vel.length().max(12.0);
                s.vel = f * speed;
                s.friendly = true;
                s.life = 6.0;
                self.sfx(Sfx::MobHit);
                self.anim.start_swing();
                return true;
            }
        }
        false
    }

    /// Right-click with an item this module knows: rockets while gliding, fire charges.
    pub(crate) fn nether_end_use(&mut self, item: &str) -> bool {
        match item {
            "firework_rocket" => {
                if !self.ne.gliding {
                    return false;
                }
                self.ne.boost = 1.6;
                if self.survival {
                    let sel = self.selected;
                    self.surv.inv.take(sel, 1);
                }
                self.sfx(Sfx::Explode);
                true
            }
            "fire_charge" => {
                let Some(hit) = self.target else { return false };
                if self.light_portal(hit) || self.redstone.kind(self.world.block_or_air(hit.block)) == crate::redstone::Kind::Tnt {
                    if self.redstone.kind(self.world.block_or_air(hit.block)) == crate::redstone::Kind::Tnt {
                        self.prime_tnt(hit.block, 4.0);
                    }
                    if self.survival {
                        let sel = self.selected;
                        self.surv.inv.take(sel, 1);
                    }
                    self.anim.start_swing();
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    /// Right-click on a block this module knows: the respawn anchor takes glowstone and
    /// becomes your spawn in the Nether (and blows up anywhere else).
    pub(crate) fn nether_end_block_use(&mut self, block: IVec3, name: &str) -> bool {
        if name != "respawn_anchor" {
            return false;
        }
        let holding_glow = self.held_stack().is_some_and(|s| self.items.get(s.item).name == "glowstone");
        if self.dim != Dim::Nether {
            if holding_glow {
                self.world.set_block(block, AIR);
                self.explode(block.as_vec3() + Vec3::splat(0.5), 4.0);
                return true;
            }
            return false;
        }
        let charges = match self.ne.anchor {
            Some((p, c)) if p == block => c,
            _ => 0,
        };
        if holding_glow && charges < 4 {
            if self.survival {
                let sel = self.selected;
                self.surv.inv.take(sel, 1);
            }
            self.ne.anchor = Some((block, charges + 1));
            self.toast(format!("Âncora carregada: {} de 4", charges + 1), 3.0);
            self.sfx(Sfx::Portal);
        } else if charges > 0 {
            self.ne.anchor = Some((block, charges));
            self.toast("Ponto de renascimento no Nether marcado", 3.0);
        } else {
            self.toast("A âncora precisa de pedra luminosa", 3.0);
        }
        self.anim.start_swing();
        self.save_anchor();
        true
    }

    /// A dimension (or a world) was loaded: shots and clouds stay behind, the anchor is read.
    pub(crate) fn nether_end_loaded(&mut self) {
        let dir = crate::level_path_hint(&self.base, &self.world_name).join(Dim::Nether.folder());
        let (levitate, withering, burning) = (self.ne.levitate, self.ne.withering, self.ne.burning);
        self.ne = NetherEnd { levitate, withering, burning, ..Default::default() };
        self.ne.load_anchor(&dir);
    }

    pub(crate) fn save_anchor(&self) {
        let dir = crate::level_path_hint(&self.base, &self.world_name).join(Dim::Nether.folder());
        self.ne.save_anchor(&dir);
    }

    /// Died in the Nether with a charged anchor: get up next to it instead of going home.
    pub(crate) fn respawn_at_anchor(&mut self) -> bool {
        let Some((p, c)) = self.ne.anchor else { return false };
        if self.dim != Dim::Nether || c == 0 || self.reg.get(self.world.block_or_air(p)).name != "respawn_anchor" {
            return false;
        }
        self.ne.anchor = if c > 1 { Some((p, c - 1)) } else { None };
        self.save_anchor();
        self.body.pos = p.as_vec3() + Vec3::new(0.5, 1.0, 0.5);
        self.unstick();
        self.body.vel = Vec3::ZERO;
        self.health = self.max_health;
        self.air = 10.0;
        self.hurt_age = 100.0;
        self.surv_respawned();
        self.fall_peak = self.body.pos.y;
        self.ne = NetherEnd { anchor: self.ne.anchor, ..Default::default() };
        self.sfx(Sfx::Respawn);
        self.set_screen(crate::Screen::Playing);
        true
    }

    /// After a block goes down: a wither skull topping a T of soul sand wakes the Wither.
    pub(crate) fn after_place(&mut self, pos: IVec3) {
        if self.reg.get(self.world.block_or_air(pos)).name != "wither_skeleton_skull" {
            return;
        }
        let soul = |g: &Game, p: IVec3| matches!(g.reg.get(g.world.block_or_air(p)).name.as_str(), "soul_sand" | "soul_soil");
        let skull = |g: &Game, p: IVec3| g.reg.get(g.world.block_or_air(p)).name == "wither_skeleton_skull";
        for side in [IVec3::X, IVec3::Z] {
            for k in -1..=1 {
                // `mid` is the middle skull; the arms of the T run along `side`.
                let mid = pos - side * k;
                let top = mid - IVec3::Y;
                let ok = (-1..=1).all(|a| skull(self, mid + side * a) && soul(self, top + side * a)) && soul(self, top - IVec3::Y);
                if !ok {
                    continue;
                }
                for a in -1..=1 {
                    self.world.set_block(mid + side * a, AIR);
                    self.world.set_block(top + side * a, AIR);
                }
                self.world.set_block(top - IVec3::Y, AIR);
                let at = (top - IVec3::Y).as_vec3() + Vec3::new(0.5, 0.0, 0.5);
                if self.mobs.spawn(WITHER, at) {
                    // It rises slowly out of the sand, untouchable, then bursts.
                    self.ne.wither_wake = 10.0;
                }
                self.ne.wither_t = 0.0;
                self.weather.flash = 0.8;
                self.sfx(Sfx::Thunder(0.8));
                self.toast("O Wither despertou!", 4.0);
                return;
            }
        }
    }

    fn update_wither(&mut self, dt: f32) {
        let Some(i) = self.mobs.list.iter().position(|m| m.alive() && self.mobs.reg.defs[m.def].name == WITHER) else { return };
        let me = self.body.pos;
        let m = &mut self.mobs.list[i];
        m.aggro = true;
        let (pos, health, max) = (m.body.pos, m.health, m.max_health);
        // Waking up: it grows in power for ten seconds, untouchable, then blasts the ground.
        if self.ne.wither_wake > 0.0 {
            self.ne.wither_wake -= dt;
            m.health = (max * (1.0 - self.ne.wither_wake / 10.0)).clamp(1.0, max);
            m.body.vel = Vec3::Y * 0.3;
            m.hurt_age = 100.0;
            if self.ne.wither_wake <= 0.0 {
                self.explode(pos + Vec3::Y * 1.5, 5.0);
                self.sfx(Sfx::Thunder(1.0));
            }
            return;
        }
        // At half health it calls wither skeletons once.
        if health < max * 0.5 && !self.boss.called_help {
            self.boss.called_help = true;
            for k in 0..3 {
                let a = k as f32 * 2.1;
                let at = pos + Vec3::new(a.cos() * 3.0, 0.0, a.sin() * 3.0);
                let y = self.ground_y(at.x.floor() as i32, at.z.floor() as i32, pos.y as i32 + 4).unwrap_or(pos.y as i32) + 1;
                self.mobs.spawn("wither_skeleton", Vec3::new(at.x, y as f32, at.z));
            }
            self.toast("O Wither chama seus esqueletos!", 3.0);
        }
        // Its side heads spit extra skulls when angry.
        self.ne.wither_t += dt;
        if self.ne.wither_t > 2.6 && health < max * 0.5 && self.survival {
            self.ne.wither_t = 0.0;
            let from = pos + Vec3::Y * 3.0;
            let at = me + Vec3::Y;
            self.throw_shot(ShotKind::Skull, from + Vec3::X * 1.2, at);
            self.throw_shot(ShotKind::Skull, from - Vec3::X * 1.2, at);
        }
    }

    // ------------------------------------------------------------------ the dragon

    /// The End's chunks are in and its dragon still lives: wake it over the island.
    pub(crate) fn wake_dragon(&mut self) {
        if self.mobs.list.iter().any(|m| m.alive() && self.mobs.reg.defs[m.def].name == DRAGON) {
            return;
        }
        if self.mobs.spawn(DRAGON, Vec3::new(0.0, 90.0, 50.0)) {
            self.ne.dragon = DragonBrain::default();
            self.boss.called_help = false;
            self.toast("O Dragão do End sobrevoa a ilha", 4.0);
        }
    }

    fn dragon_index(&self) -> Option<usize> {
        self.mobs.list.iter().position(|m| m.alive() && self.mobs.reg.defs[m.def].name == DRAGON)
    }

    /// The dragon flies its own path (and a ridden strider goes where its rider steers):
    /// remember where they were so the mobs' walking leaves them be.
    pub(crate) fn before_mobs(&mut self) {
        self.ne.strider_held = self.strider_index().map(|i| (self.mobs.list[i].body.pos, self.mobs.list[i].yaw));
        self.ne.dragon.held = self.dragon_index().map(|i| (self.mobs.list[i].body.pos, self.mobs.list[i].body.vel, self.mobs.list[i].yaw));
    }

    fn update_dragon(&mut self, dt: f32) {
        if self.dim != Dim::End {
            return;
        }
        let Some(i) = self.dragon_index() else { return };
        if let Some((p, v, yaw)) = self.ne.dragon.held.take() {
            let m = &mut self.mobs.list[i];
            m.body.pos = p;
            m.body.vel = v;
            m.yaw = yaw;
        }
        let crystals = self.live_crystals();
        let me = self.body.pos;
        let creative = !self.survival;
        let glow = self.glow_layer();
        let b = &mut self.ne.dragon;
        b.t += dt;
        b.shot_t -= dt;
        let pos = self.mobs.list[i].body.pos;
        let (target, speed): (Vec3, f32) = match b.phase {
            Phase::Circle => {
                b.angle += dt * 0.28;
                let r = 52.0 + (b.angle * 1.7).sin() * 10.0;
                let t = Vec3::new(b.angle.cos() * r, 86.0 + (b.angle * 2.0).sin() * 8.0, b.angle.sin() * r);
                if b.t > 14.0 + (b.rounds % 3) as f32 * 3.0 {
                    b.t = 0.0;
                    b.rounds += 1;
                    b.phase = if b.rounds % 3 == 0 { Phase::Perch } else if !creative && me.distance(pos) < 150.0 { Phase::Swoop } else { Phase::Circle };
                }
                (t, 15.0)
            }
            Phase::Swoop => {
                let t = me + Vec3::Y * 1.5;
                if b.t > 7.0 || pos.distance(t) < 3.0 {
                    b.t = 0.0;
                    b.phase = Phase::Circle;
                    b.angle = pos.z.atan2(pos.x);
                }
                (t, 21.0)
            }
            Phase::Perch => {
                let t = Vec3::new(0.0, END_EXIT_Y as f32 + 5.0, 0.0);
                if b.t > 14.0 {
                    b.t = 0.0;
                    b.phase = Phase::Circle;
                    b.angle = pos.z.atan2(pos.x);
                }
                (t, 12.0)
            }
        };
        let phase = b.phase;
        let perched = phase == Phase::Perch && pos.distance(target) < 2.0;
        // Spit a fireball at the player now and then while circling; breathe while perched.
        let mut shoot = None;
        if b.shot_t <= 0.0 && !creative && me.distance(pos) < 90.0 {
            if phase == Phase::Circle {
                b.shot_t = 6.0;
                shoot = Some(ShotKind::Dragon);
            } else if perched {
                b.shot_t = 4.0;
                shoot = Some(ShotKind::Dragon);
            }
        }
        let m = &mut self.mobs.list[i];
        m.aggro = true;
        let to = target - pos;
        let dist = to.length();
        let want = if perched { Vec3::ZERO } else { to / dist.max(0.01) * speed.min(dist * 3.0) };
        let k = (dt * 2.0).min(1.0);
        m.body.vel += (want - m.body.vel) * k;
        m.body.pos += m.body.vel * dt;
        let v = if perched { me - pos } else { m.body.vel };
        if v.length_squared() > 0.01 {
            let target_yaw = v.x.atan2(v.z);
            let mut d = target_yaw - m.yaw;
            while d > std::f32::consts::PI {
                d -= std::f32::consts::TAU;
            }
            while d < -std::f32::consts::PI {
                d += std::f32::consts::TAU;
            }
            m.yaw += d.clamp(-dt * 2.5, dt * 2.5);
        }
        m.head_pitch = if perched { 0.4 } else { (-m.body.vel.y / speed).clamp(-0.6, 0.6) };
        // The wings beat: walk drives the Wing animation.
        m.walk += dt * if perched { 3.0 } else { 7.0 };
        m.walk_speed = 4.0;
        // Crystals heal it, a beam of sparks shows which one.
        if !crystals.is_empty() && m.health < m.max_health {
            m.health = (m.health + dt * 1.0 * crystals.len().min(4) as f32).min(m.max_health);
            let c = crystals[(self.surv.rng.f32() * crystals.len() as f32) as usize % crystals.len()];
            let from = c.as_vec3() + Vec3::splat(0.5);
            if self.surv.rng.f32() < dt * 8.0 {
                let head = m.body.pos + Vec3::Y * 2.0;
                self.particles.glow(from, (head - from) * 0.9, glow, [0.95, 0.6, 1.0], 0.2, 1.1);
            }
        }
        // Flying into the player hurts and throws them.
        let body = m.body.pos + Vec3::Y * 1.5;
        let hit = !creative && body.distance(me + Vec3::Y) < 4.0 && phase == Phase::Swoop;
        let head = m.body.pos + Vec3::new(m.yaw.sin(), 0.6, m.yaw.cos()) * 5.0;
        if let Some(kind) = shoot {
            self.throw_shot(kind, head, me + Vec3::Y * 0.5);
        }
        if hit {
            let away = Vec3::new(me.x - body.x, 0.0, me.z - body.z).normalize_or_zero();
            self.body.vel += away * 10.0 + Vec3::Y * 8.0;
            self.damage(8.0, Some(body));
            self.ne.dragon.t = 0.0;
            self.ne.dragon.phase = Phase::Circle;
            self.ne.dragon.angle = body.z.atan2(body.x);
        }
        if self.surv.rng.f32() < dt * 2.0 && me.distance(body) < 80.0 {
            // The deep beat of its wings.
            self.sfx(Sfx::Step(Material::Cloth));
        }
    }

    /// A boss of this module died.
    pub(crate) fn nether_end_boss_defeated(&mut self, name: &str) {
        if name != DRAGON {
            return;
        }
        self.open_end_exit();
        // The egg sits on top of the fountain's pillar.
        let egg = self.reg.id_or_air("dragon_egg");
        self.world.set_block(IVec3::new(0, END_EXIT_Y + 4, 0), egg);
        self.build_gateway(self.gateway_spot());
        self.toast("Uma passagem se abriu no céu, longe da ilha", 5.0);
        let glow = self.glow_layer();
        for k in 0..80 {
            let a = k as f32 * 0.7;
            let v = Vec3::new(a.cos(), 1.0 + (k % 5) as f32 * 0.5, a.sin()) * 3.0;
            self.particles.glow(Vec3::new(0.0, END_EXIT_Y as f32 + 12.0, 0.0), v, glow, [1.0, 0.85, 1.2], 0.35, 3.0);
        }
    }

    /// Where the first end gateway stands: on the ring, the same for every world of a seed.
    fn gateway_spot(&self) -> IVec3 {
        let a = (self.seed % 360) as f32 / 360.0 * std::f32::consts::TAU;
        IVec3::new((a.cos() * GATEWAY_RING) as i32, 75, (a.sin() * GATEWAY_RING) as i32)
    }

    /// A gateway block in a little bedrock cage.
    fn build_gateway(&mut self, at: IVec3) {
        let gate = self.reg.id_or_air("end_gateway");
        let bedrock = self.reg.id_or_air("bedrock");
        for d in [IVec3::Y * 2, -IVec3::Y * 2] {
            self.world.set_block(at + d, bedrock);
        }
        for d in [IVec3::Y, -IVec3::Y] {
            for s in [IVec3::X, -IVec3::X, IVec3::Z, -IVec3::Z] {
                self.world.set_block(at + d + s, bedrock);
            }
            self.world.set_block(at + d, AIR);
        }
        self.world.set_block(at, gate);
    }

    /// A cleared End keeps its first gateway (worlds that beat the old guardian get one too):
    /// built as soon as its chunk is in.
    pub(crate) fn restore_gateways(&mut self) {
        self.ne.check_gateway = true;
    }

    fn keep_gateway(&mut self) {
        if !self.ne.check_gateway || self.dim != Dim::End {
            return;
        }
        let g = self.gateway_spot();
        let Some(b) = self.world.block(g) else { return };
        self.ne.check_gateway = false;
        if self.reg.get(b).name != "end_gateway" {
            self.build_gateway(g);
        }
    }

    /// Stepping into a gateway: out to the far islands, or back to the main one.
    fn gateway_step(&mut self, dt: f32) {
        self.ne.gateway_cd -= dt;
        if self.dim != Dim::End || self.ne.gateway_cd > 0.0 {
            return;
        }
        let p = self.body.pos;
        let inside = [p, p + Vec3::Y].iter().any(|q| self.reg.get(self.world.block_or_air(q.floor().as_ivec3())).name == "end_gateway");
        if !inside {
            return;
        }
        self.ne.gateway_cd = 3.0;
        let flat = Vec3::new(p.x, 0.0, p.z);
        let dir = flat.normalize_or_zero();
        let outward = flat.length() < GATEWAY_RING * 2.0;
        let dest = if outward {
            // The nearest outer island past the jump, along the same line.
            let gen = self.world.gen.clone();
            let mut land = None;
            for step in 0..60 {
                for side in [0.0f32, 12.0, -12.0, 24.0, -24.0] {
                    let c = dir * (GATEWAY_JUMP + step as f32 * 8.0) + Vec3::new(-dir.z, 0.0, dir.x) * side;
                    if let Some(top) = gen.end_top(c.x as i32, c.z as i32) {
                        land = Some(Vec3::new(c.x.floor() + 0.5, top as f32 + 1.0, c.z.floor() + 0.5));
                        break;
                    }
                }
                if land.is_some() {
                    break;
                }
            }
            land.unwrap_or(dir * GATEWAY_JUMP + Vec3::Y * 70.0)
        } else {
            let g = self.gateway_spot().as_vec3();
            g + Vec3::new(g.x, 0.0, g.z).normalize_or_zero() * -4.0 + Vec3::new(0.5, -1.0, 0.5)
        };
        self.sfx(Sfx::Portal);
        self.toast("A passagem leva você longe", 2.0);
        if outward {
            // A return gateway on the far island, beside where you land.
            let back = (dest + Vec3::Y * 4.0 + dir * 3.0).floor().as_ivec3();
            self.ne.pending_gateway = Some(back);
        }
        self.travel_within(dest);
    }

    /// A long jump inside the same dimension: wait for the chunks there, then land.
    fn travel_within(&mut self, at: Vec3) {
        self.body.pos = at;
        self.body.vel = Vec3::ZERO;
        self.fall_peak = at.y;
        self.arrival = Some(at);
        self.ne.gateway_landing = true;
        self.loaded_frames = 0;
        self.set_screen(crate::Screen::Loading);
    }

    /// Landing after a gateway jump: on the island, with the way back built beside it.
    pub(crate) fn finish_gateway(&mut self, at: Vec3) -> Vec3 {
        if let Some(g) = self.ne.pending_gateway.take() {
            if self.reg.get(self.world.block_or_air(g)).name != "end_gateway" {
                self.build_gateway(g);
            }
        }
        let (x, z) = (at.x.floor() as i32, at.z.floor() as i32);
        let y = self.world.gen.end_top(x, z).map(|t| t + 1).unwrap_or(at.y as i32);
        Vec3::new(x as f32 + 0.5, y as f32, z as f32 + 0.5)
    }

    // ------------------------------------------------------------------ striders

    fn strider_index(&self) -> Option<usize> {
        let phase = self.ne.strider?;
        self.mobs.list.iter().position(|m| m.alive() && m.phase == phase && self.mobs.reg.defs[m.def].name == "strider")
    }

    /// Right-click a strider to climb on its back.
    pub(crate) fn mount_strider(&mut self) -> bool {
        let Some((i, t)) = self.mobs.pick(self.body.eye(), self.forward(), 3.5) else { return false };
        if t > self.target.map(|h| h.distance).unwrap_or(f32::MAX) || self.mob_name(i) != "strider" || self.ne.strider.is_some() {
            return false;
        }
        self.ne.strider = Some(self.mobs.list[i].phase);
        self.body.flying = false;
        self.toast("Agachar para descer do strider", 3.0);
        self.sfx(Sfx::Place(Material::Cloth));
        true
    }

    /// On a strider: it walks where you steer, over lava as over ground. Sneak to get off.
    pub(crate) fn strider_step(&mut self, wish: Vec3, sneak: bool, dt: f32) -> bool {
        let Some(i) = self.strider_index() else {
            self.ne.strider = None;
            return false;
        };
        if sneak {
            self.ne.strider = None;
            self.body.pos += Vec3::Y * 0.3;
            self.unstick();
            return false;
        }
        let lava = self.reg.id_or_air("lava");
        let pos = self.mobs.list[i].body.pos;
        let mut next = pos;
        let flat = Vec3::new(wish.x, 0.0, wish.z);
        if flat.length_squared() > 0.01 {
            let step = flat.normalize() * 3.2 * dt;
            let to = pos + step;
            // Stand on the lava's top or on ground, stepping up or down a block at most.
            let (x, z) = (to.x.floor() as i32, to.z.floor() as i32);
            let base = pos.y.floor() as i32;
            let stand = (-2..=1).rev().map(|d| base + d - 1).find(|&y| {
                let b = self.world.block_or_air(IVec3::new(x, y, z));
                let free = |k: i32| !self.reg.get(self.world.block_or_air(IVec3::new(x, y + k, z))).solid && self.world.block_or_air(IVec3::new(x, y + k, z)) != lava;
                (b == lava || self.reg.get(b).solid) && free(1) && free(2)
            });
            if let Some(y) = stand {
                next = Vec3::new(to.x, y as f32 + 1.0, to.z);
            }
            let m = &mut self.mobs.list[i];
            m.yaw = flat.x.atan2(flat.z);
            m.walk += dt * 6.0;
            m.walk_speed = 3.0;
        } else {
            self.mobs.list[i].walk_speed = 0.0;
        }
        self.mobs.list[i].body.pos = next;
        self.body.pos = next + Vec3::Y * 1.4;
        self.body.vel = Vec3::ZERO;
        self.body.on_ground = true;
        self.body.fall_distance = 0.0;
        self.fall_peak = self.body.pos.y;
        true
    }

    // ------------------------------------------------------------------ elytra

    fn wearing_elytra(&self) -> bool {
        self.surv.armor[1].is_some_and(|s| self.items.get(s.item).name == "elytra" && s.damage + 1 < self.items.get(s.item).armor.map(|a| a.durability).unwrap_or(1))
    }

    /// Gliding on elytra: press jump while falling to open them. Look down to dive and gain
    /// speed, look up to climb and lose it; rockets push you along. Returns true when it moved
    /// the player this tick.
    pub(crate) fn elytra_step(&mut self, jump: bool, dt: f32) -> bool {
        let pressed = jump && !self.ne.jump_was;
        self.ne.jump_was = jump;
        if !self.ne.gliding {
            if pressed && self.wearing_elytra() && !self.body.on_ground && !self.body.in_water && !self.body.flying && self.body.vel.y < 0.5 {
                self.ne.gliding = true;
                self.sfx(Sfx::Place(Material::Cloth));
            } else {
                return false;
            }
        }
        if self.body.on_ground || self.body.in_water || self.body.flying || !self.wearing_elytra() {
            self.ne.gliding = false;
            self.ne.boost = 0.0;
            return false;
        }
        // The classic glide, worked out at 20 steps a second in blocks per step.
        let look = self.forward();
        let steps = (dt * 20.0).ceil().max(1.0) as i32;
        let frac = dt * 20.0 / steps as f32;
        let mut v = self.body.vel / 20.0;
        let speed_before = v.length();
        for _ in 0..steps {
            let h = (look.x * look.x + look.z * look.z).sqrt();
            let cos2 = h * h;
            v.y += -0.08 * (1.0 - cos2 * 0.75) * frac;
            if v.y < 0.0 && h > 0.0 {
                let lift = v.y * -0.1 * cos2 * frac;
                v.y += lift;
                v.x += look.x / h * lift;
                v.z += look.z / h * lift;
            }
            let hs = (v.x * v.x + v.z * v.z).sqrt();
            if look.y > 0.0 && h > 0.0 {
                let climb = hs * look.y * 0.04 * frac;
                v.y += climb * 3.2;
                v.x -= look.x / h * climb;
                v.z -= look.z / h * climb;
            }
            if h > 0.0 {
                v.x += (look.x / h * hs - v.x) * 0.1 * frac;
                v.z += (look.z / h * hs - v.z) * 0.1 * frac;
            }
            if self.ne.boost > 0.0 {
                v += (look * 0.1 + (look * 1.5 - v) * 0.5) * frac;
            }
            v.x *= 1.0 - 0.01 * frac;
            v.y *= 1.0 - 0.02 * frac;
            v.z *= 1.0 - 0.01 * frac;
        }
        self.ne.boost = (self.ne.boost - dt).max(0.0);
        if self.ne.boost > 0.0 {
            let glow = self.glow_layer();
            self.particles.glow(self.body.pos, -look * 2.0, glow, [1.2, 0.8, 0.4], 0.15, 0.4);
        }
        let vel = v * 20.0;
        self.body.fly(&self.world, vel, dt);
        // Flying into a wall at speed hurts.
        let after = self.body.vel.length() / 20.0;
        if speed_before - after > 0.3 && self.survival {
            self.damage(((speed_before - after) * 10.0 - 3.0).max(1.0), None);
        }
        self.fall_peak = self.body.pos.y;
        // The wings wear a little with every second in the air.
        if self.survival && self.surv.rng.f32() < dt {
            if let Some(s) = self.surv.armor[1].as_mut() {
                s.damage += 1;
            }
        }
        true
    }

    // ------------------------------------------------------------------ test scenes

    /// Test scene: a flat floor in front of the player with the named mobs lined up on it.
    pub(crate) fn test_arena(&mut self, floor: &str) {
        let f = self.forward();
        let fwd = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = Vec3::new(-fwd.z, 0.0, fwd.x);
        let base = self.body.pos.floor().as_ivec3() - IVec3::Y;
        let block = self.reg.id_or_air(floor);
        for a in -2..26 {
            for s in -14..=14 {
                let p = (base.as_vec3() + Vec3::splat(0.5) + fwd * a as f32 + side * s as f32).floor().as_ivec3();
                self.world.set_block(IVec3::new(p.x, base.y, p.z), block);
                for h in 1..14 {
                    self.world.set_block(IVec3::new(p.x, base.y + h, p.z), AIR);
                }
            }
        }
    }

    /// Test scenes for screenshots and logs.
    pub(crate) fn nether_end_test(&mut self, what: &str) {
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = Vec3::new(-flat.z, 0.0, flat.x);
        let eye = self.body.eye();
        match what {
            "shots" => {
                // One of each, frozen in flight in front of the player.
                let kinds = [ShotKind::Ghast, ShotKind::Blaze, ShotKind::Skull, ShotKind::Shulker, ShotKind::Dragon];
                for (k, kind) in kinds.into_iter().enumerate() {
                    let from = eye + flat * 9.0 + side * (k as f32 - 2.0) * 2.0 + Vec3::Y;
                    self.throw_shot(kind, from, eye);
                }
                for _ in 0..12 {
                    self.update_shots(0.02);
                    self.particles.update(&self.world, 0.02);
                }
                self.ne.clouds.push(Cloud { pos: self.body.pos + flat * 5.0, life: 9.0 });
                for _ in 0..40 {
                    self.update_clouds(0.05);
                    self.particles.update(&self.world, 0.05);
                }
            }
            "wither" => {
                // A soul sand T with its skulls, the last one just set.
                let base = (self.body.pos + flat * 5.0).floor().as_ivec3();
                let (sand, skull) = (self.reg.id_or_air("soul_sand"), self.reg.id_or_air("wither_skeleton_skull"));
                let s = if flat.x.abs() > flat.z.abs() { IVec3::Z } else { IVec3::X };
                self.world.set_block(base, sand);
                for a in -1..=1 {
                    self.world.set_block(base + IVec3::Y + s * a, sand);
                    self.world.set_block(base + IVec3::Y * 2 + s * a, skull);
                }
                self.after_place(base + IVec3::Y * 2 + s);
                let woke = self.mobs.list.iter().any(|m| m.alive() && self.mobs.reg.defs[m.def].name == WITHER);
                log::info!("wither summoned: {woke}");
                for _ in 0..30 {
                    self.update_wither(0.2);
                }
                log::info!("wither awake: wake left {:.1}", self.ne.wither_wake);
            }
            "dragon" => {
                self.wake_dragon();
                if let Some(i) = self.dragon_index() {
                    let m = &mut self.mobs.list[i];
                    m.body.pos = self.body.pos + flat * 22.0 + Vec3::Y * 6.0;
                    m.yaw = flat.x.atan2(flat.z) + 1.4;
                    m.walk = 1.3;
                    m.walk_speed = 4.0;
                    m.health = m.max_health * 0.6;
                }
                // Then a simulated minute of the fight, to check it keeps flying its rounds.
                let me = self.body.pos;
                let mut far = 0.0f32;
                for k in 0..1200 {
                    self.before_mobs();
                    self.update_dragon(0.05);
                    if let Some(i) = self.dragon_index() {
                        far = far.max(self.mobs.list[i].body.pos.distance(Vec3::ZERO));
                        if k % 200 == 0 {
                            log::info!("dragon t={:.0}s at {:.0?} phase {:?}", k as f32 * 0.05, self.mobs.list[i].body.pos, self.ne.dragon.phase);
                        }
                    }
                }
                log::info!("dragon farthest from the middle: {far:.0}, shots {}", self.ne.shots.len());
                self.body.pos = me;
                self.health = self.max_health;
                self.ne.shots.clear();
                if let Some(i) = self.dragon_index() {
                    let m = &mut self.mobs.list[i];
                    m.body.pos = self.body.pos + flat * 22.0 + Vec3::Y * 6.0;
                    m.yaw = flat.x.atan2(flat.z) + 1.4;
                }
            }
            "gateway" => {
                self.nether_end_boss_defeated(DRAGON);
                let g = self.gateway_spot();
                log::info!("gateway at {g}: {}", self.reg.get(self.world.block_or_air(g)).name);
                let egg = self.reg.get(self.world.block_or_air(IVec3::new(0, END_EXIT_Y + 4, 0))).name.clone();
                log::info!("egg block: {egg}");
            }
            "strider" => {
                // Ride a strider forward for a few seconds, logging what it stands on.
                let at = self.body.pos;
                if self.mobs.spawn("strider", at) {
                    self.ne.strider = Some(self.mobs.list.last().unwrap().phase);
                }
                for k in 0..160 {
                    let dir = [side, flat, -side, -flat][k / 40];
                    self.strider_step(dir, false, 0.05);
                    self.before_mobs();
                    if let (Some(i), Some((p, yaw))) = (self.strider_index(), self.ne.strider_held.take()) {
                        self.mobs.list[i].body.pos = p;
                        self.mobs.list[i].yaw = yaw;
                    }
                    if k % 20 == 0 {
                        let p = self.body.pos;
                        let under = self.reg.get(self.world.block_or_air((p - Vec3::Y * 1.9).floor().as_ivec3())).name.clone();
                        log::info!("strider ride t={:.0}s at {:.1?} over {under}", k as f32 * 0.05, p);
                    }
                }
            }
            "glide" => {
                // Wear elytra high up and glide for a few seconds, logging the flight.
                if let Some(id) = self.items.id("elytra") {
                    self.surv.armor[1] = Some(ItemStack::new(id, 1));
                }
                self.body.flying = false;
                self.body.on_ground = false;
                self.body.pos.y += 40.0;
                self.body.vel = flat * 6.0 - Vec3::Y * 2.0;
                self.elytra_step(false, 0.05);
                self.elytra_step(true, 0.05);
                log::info!("gliding: {}", self.ne.gliding);
                let start = self.body.pos;
                for k in 0..200 {
                    if k == 100 {
                        self.ne.boost = 1.6;
                    }
                    if !self.elytra_step(true, 0.05) {
                        break;
                    }
                    if k % 25 == 0 {
                        let v = self.body.vel;
                        log::info!("glide t={:.1}s pos {:.0?} speed {:.1} m/s, down {:.1}", k as f32 * 0.05, self.body.pos, v.length(), -v.y);
                    }
                }
                let d = self.body.pos - start;
                log::info!("glided {:.0} blocks out, {:.0} down", Vec3::new(d.x, 0.0, d.z).length(), -d.y);
            }
            _ => {}
        }
    }
}
