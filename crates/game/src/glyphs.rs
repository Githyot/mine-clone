//! Glyph spells (Ars Nouveau's idea, our own glyphs): learn glyphs by reading them (right click),
//! then write up to three spells in the grimoire (Shift + right click with it in hand). A spell
//! starts with a form (projectile, touch, self), then effects run in order where it lands, and
//! augments make it stronger, wider, split into more or longer lasting. Hold right click with
//! the grimoire to cast the spell in use; G switches between the three.

use std::collections::BTreeSet;

use glam::{IVec3, Vec3};
use voxel::block::{Shape, ToolKind};
use voxel::item::ToolDef;
use voxel::physics::Terrain;
use voxel::AIR;

use crate::audio::{Material, Sfx};
use crate::ui::WHITE;
use crate::{Game, Screen};

pub const SPELL_SLOTS: usize = 3;
const MAX_GLYPHS: usize = 9;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Form,
    Effect,
    Augment,
}

/// (id, name, kind, mana, colour)
const GLYPHS: &[(&str, &str, Kind, f32, [f32; 3])] = &[
    ("projectile", "Projétil", Kind::Form, 2.0, [1.0, 1.0, 1.2]),
    ("touch", "Toque", Kind::Form, 1.0, [1.0, 1.0, 1.2]),
    ("self", "Si Mesmo", Kind::Form, 1.0, [1.0, 1.0, 1.2]),
    ("harm", "Dano", Kind::Effect, 3.0, [1.5, 0.4, 0.4]),
    ("ignite", "Fogo", Kind::Effect, 3.0, [1.6, 0.6, 0.15]),
    ("freeze", "Gelo", Kind::Effect, 3.0, [0.5, 0.85, 1.6]),
    ("heal", "Cura", Kind::Effect, 4.0, [0.45, 1.5, 0.5]),
    ("break", "Quebrar", Kind::Effect, 2.0, [0.9, 0.8, 0.6]),
    ("knockback", "Empurrão", Kind::Effect, 2.0, [1.2, 1.2, 0.6]),
    ("pull", "Puxão", Kind::Effect, 2.0, [0.8, 0.6, 1.3]),
    ("leap", "Salto", Kind::Effect, 2.0, [0.7, 1.4, 1.2]),
    ("light", "Luz", Kind::Effect, 1.0, [1.6, 1.4, 0.6]),
    ("blink", "Teleporte", Kind::Effect, 5.0, [1.1, 0.45, 1.6]),
    ("explode", "Explosão", Kind::Effect, 6.0, [1.6, 0.8, 0.2]),
    ("harvest", "Colheita", Kind::Effect, 2.0, [0.6, 1.4, 0.4]),
    ("amplify", "Amplificar", Kind::Augment, 3.0, [1.0, 1.0, 1.0]),
    ("aoe", "Área", Kind::Augment, 3.0, [1.0, 1.0, 1.0]),
    ("split", "Dividir", Kind::Augment, 3.0, [1.0, 1.0, 1.0]),
    ("accelerate", "Acelerar", Kind::Augment, 1.0, [1.0, 1.0, 1.0]),
    ("extend", "Prolongar", Kind::Augment, 2.0, [1.0, 1.0, 1.0]),
];

fn glyph(id: &str) -> Option<&'static (&'static str, &'static str, Kind, f32, [f32; 3])> {
    GLYPHS.iter().find(|g| g.0 == id)
}

/// A spell in flight (projectile form).
struct GlyphBolt {
    pos: Vec3,
    vel: Vec3,
    life: f32,
    spell: Vec<String>,
    tint: [f32; 3],
}

#[derive(Default)]
pub struct Grimoire {
    pub learned: BTreeSet<String>,
    pub spells: [Vec<String>; SPELL_SLOTS],
    pub active: usize,
    bolts: Vec<GlyphBolt>,
    cd: f32,
}

/// Where a spell lands.
#[derive(Clone, Copy)]
enum Target {
    Mob(usize),
    Block(IVec3, IVec3),
    Player,
    Air(Vec3),
}

/// The augments of a spell, counted.
#[derive(Default, Clone, Copy)]
struct Aug {
    amp: f32,
    aoe: f32,
    split: u32,
    accel: f32,
    ext: f32,
}

fn augments(spell: &[String]) -> Aug {
    let mut a = Aug::default();
    for g in spell {
        match g.as_str() {
            "amplify" => a.amp += 1.0,
            "aoe" => a.aoe += 1.0,
            "split" => a.split += 1,
            "accelerate" => a.accel += 1.0,
            "extend" => a.ext += 1.0,
            _ => {}
        }
    }
    a
}

/// Mana a spell costs.
fn cost(spell: &[String]) -> f32 {
    1.0 + spell.iter().filter_map(|g| glyph(g)).map(|g| g.3).sum::<f32>()
}

/// "Projétil · Fogo · Dano" — the spell written out.
fn spell_text(spell: &[String]) -> String {
    if spell.is_empty() {
        return "(vazia)".to_string();
    }
    spell.iter().filter_map(|g| glyph(g)).map(|g| g.1).collect::<Vec<_>>().join(" · ")
}

/// The colour of a spell's light: its first effect's.
fn tint(spell: &[String]) -> [f32; 3] {
    spell.iter().filter_map(|g| glyph(g)).find(|g| g.2 == Kind::Effect).map(|g| g.4).unwrap_or([1.1, 1.0, 1.4])
}

impl Game {
    /// Right click with a glyph: learn it for good (the page is used up).
    pub(crate) fn learn_glyph(&mut self, item: &str) -> bool {
        let Some(id) = item.strip_prefix("glyph_") else { return false };
        let Some(g) = glyph(id) else { return false };
        if self.t5.glyphs.learned.contains(id) {
            self.toast(format!("Você já conhece o glifo {}", g.1), 2.5);
            return true;
        }
        self.t5.glyphs.learned.insert(id.to_string());
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        self.sfx(Sfx::Cast(3));
        self.anim.start_swing();
        self.toast(format!("Glifo aprendido: {} (escreva magias no Grimório: Shift + botão direito)", g.1), 4.0);
        true
    }

    pub(crate) fn open_grimoire(&mut self) {
        self.set_screen(Screen::Grimoire);
    }

    /// G with the grimoire in hand: the next spell.
    pub(crate) fn glyph_cycle(&mut self) {
        if self.held_stack().map(|s| self.items.get(s.item).name.as_str() == "glyph_book") != Some(true) {
            return;
        }
        let g = &mut self.t5.glyphs;
        g.active = (g.active + 1) % SPELL_SLOTS;
        let t = format!("Magia {}: {}", g.active + 1, spell_text(&g.spells[g.active]));
        self.toast(t, 2.5);
    }

    /// Test hook: the first spell cast, its bolts a moment into their flight.
    pub(crate) fn test_glyph_cast(&mut self) {
        self.t5.glyphs.cd = 0.0;
        self.update_glyph_casting(0.0, true);
        // Far enough out that the trail near the hand has faded.
        for k in 0..12 {
            self.update_glyph_bolts(0.04);
            if k == 4 {
                self.particles.update(&self.world, 1.0);
            }
        }
    }

    fn knows(&self, id: &str) -> bool {
        !self.survival || self.t5.glyphs.learned.contains(id)
    }

    pub(crate) fn update_glyph_casting(&mut self, dt: f32, use_held: bool) {
        self.t5.glyphs.cd -= dt;
        self.update_glyph_bolts(dt);
        if !use_held || self.screen != Screen::Playing || self.t5.glyphs.cd > 0.0 {
            return;
        }
        if self.held_stack().map(|s| self.items.get(s.item).name.as_str() == "glyph_book") != Some(true) {
            return;
        }
        if self.keys.contains(&winit::keyboard::KeyCode::ShiftLeft) || self.keys.contains(&winit::keyboard::KeyCode::ShiftRight) {
            return;
        }
        let spell = self.t5.glyphs.spells[self.t5.glyphs.active].clone();
        let Some(form) = spell.first().and_then(|g| glyph(g)).filter(|g| g.2 == Kind::Form) else {
            self.toast("Esta magia está vazia: abra o Grimório com Shift + botão direito", 3.0);
            self.t5.glyphs.cd = 1.0;
            return;
        };
        let (power, mana_k, speed_k) = self.caster_mods();
        let aug = augments(&spell);
        let c = cost(&spell) * mana_k;
        if self.survival && self.magic.mana < c {
            self.t5.glyphs.cd = 0.2;
            return;
        }
        if self.survival {
            self.magic.mana -= c;
        }
        self.magic.power = power;
        self.t5.glyphs.cd = (0.45 - aug.accel * 0.08).max(0.15) / speed_k;
        let eye = self.body.eye();
        let fwd = self.forward();
        let col = tint(&spell);
        match form.0 {
            "projectile" => {
                let n = 1 + aug.split * 2;
                let right = fwd.cross(Vec3::Y).normalize_or_zero();
                for k in 0..n {
                    let a = (k as f32 - (n - 1) as f32 / 2.0) * 0.14;
                    let dir = (fwd * a.cos() + right * a.sin()).normalize();
                    let speed = 22.0 * (1.0 + aug.accel * 0.4);
                    self.t5.glyphs.bolts.push(GlyphBolt { pos: eye + fwd * 0.6, vel: dir * speed, life: 1.6 + aug.ext * 0.8, spell: spell.clone(), tint: col });
                }
            }
            "touch" => {
                let reach = 5.0 + aug.ext;
                let target = self.touch_target(reach);
                self.apply_effects(&spell, target, false);
            }
            _ => self.apply_effects(&spell, Target::Player, true),
        }
        self.sfx(Sfx::Cast(3));
        self.anim.start_swing();
    }

    /// What a touch spell reaches: the first mob or block within `reach`.
    fn touch_target(&self, reach: f32) -> Target {
        let eye = self.body.eye();
        let fwd = self.forward();
        let mut p = eye;
        let mut t = 0.0;
        let mut last = eye.floor().as_ivec3();
        while t < reach {
            p += fwd * 0.1;
            t += 0.1;
            if let Some(k) = self.mob_at(p) {
                return Target::Mob(k);
            }
            let b = p.floor().as_ivec3();
            if b != last {
                if self.world.block_or_air(b) != AIR && !self.world.is_liquid(self.world.block_or_air(b)) {
                    return Target::Block(b, last - b);
                }
                last = b;
            }
        }
        Target::Air(p)
    }

    fn update_glyph_bolts(&mut self, dt: f32) {
        if self.t5.glyphs.bolts.is_empty() {
            return;
        }
        let layer = self.glow_layer();
        let mut bolts = std::mem::take(&mut self.t5.glyphs.bolts);
        let mut landed: Vec<(Vec<String>, Target)> = Vec::new();
        bolts.retain_mut(|b| {
            for _ in 0..2 {
                self.particles.glow(b.pos, -b.vel * 0.04, layer, b.tint, 0.09, 0.35);
            }
            if dt == 0.0 {
                return true;
            }
            b.life -= dt;
            if b.life <= 0.0 {
                landed.push((std::mem::take(&mut b.spell), Target::Air(b.pos)));
                return false;
            }
            let step = b.vel * dt;
            let n = (step.length() / 0.2).ceil().max(1.0) as usize;
            for _ in 0..n {
                let prev = b.pos;
                b.pos += step / n as f32;
                if let Some(k) = self.mob_at(b.pos) {
                    landed.push((std::mem::take(&mut b.spell), Target::Mob(k)));
                    return false;
                }
                let cell = b.pos.floor().as_ivec3();
                let id = self.world.block_or_air(cell);
                if id != AIR && !self.world.is_liquid(id) && (self.world.solid(cell) || self.reg.get(id).shape == Shape::Cross) {
                    landed.push((std::mem::take(&mut b.spell), Target::Block(cell, prev.floor().as_ivec3() - cell)));
                    return false;
                }
                if self.world.is_liquid(id) && b.spell.iter().any(|g| g == "freeze") {
                    landed.push((std::mem::take(&mut b.spell), Target::Block(cell, IVec3::Y)));
                    return false;
                }
            }
            true
        });
        self.t5.glyphs.bolts = bolts;
        for (spell, target) in landed {
            self.apply_effects(&spell, target, false);
        }
    }

    fn target_point(&self, t: Target) -> Vec3 {
        match t {
            Target::Mob(k) => self.mobs.list.get(k).map(|m| m.body.center()).unwrap_or(self.body.pos),
            Target::Block(b, n) => b.as_vec3() + Vec3::splat(0.5) + n.as_vec3() * 0.5,
            Target::Player => self.body.pos + Vec3::Y,
            Target::Air(p) => p,
        }
    }

    /// The effects of a spell, in order, where it landed.
    fn apply_effects(&mut self, spell: &[String], target: Target, on_self: bool) {
        let aug = augments(spell);
        let at = self.target_point(target);
        let layer = self.glow_layer();
        let col = tint(spell);
        for _ in 0..14 {
            let v = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.3, self.surv.rng.f32() - 0.5) * (3.0 + aug.aoe * 2.0);
            self.particles.glow(at, v, layer, col, 0.08, 0.4);
        }
        // Who it touches: the target, and with Área everyone close by.
        let radius = if aug.aoe > 0.0 { 1.5 + aug.aoe * 1.5 } else { 0.0 };
        let mut mobs: Vec<usize> = match target {
            Target::Mob(k) => vec![k],
            _ => Vec::new(),
        };
        if radius > 0.0 {
            for (k, m) in self.mobs.list.iter().enumerate() {
                if m.alive() && !mobs.contains(&k) && m.body.center().distance(at) < radius && !(on_self && self.mobs.reg.defs[m.def].town) {
                    mobs.push(k);
                }
            }
        }
        let player = matches!(target, Target::Player);
        let caster = self.body.pos;
        let k = 1.0 + aug.amp * 0.5;
        for g in spell.iter() {
            match g.as_str() {
                "harm" => {
                    for &m in &mobs {
                        self.hurt_mob(m, 5.0 * k, caster, 3);
                    }
                    if player {
                        self.damage(3.0 * k, None);
                    }
                }
                "ignite" => {
                    for &m in &mobs {
                        self.hurt_mob(m, 1.0, caster, 0);
                        self.mobs.list[m].on_fire = self.mobs.list[m].on_fire.max(5.0 * (1.0 + aug.ext));
                    }
                }
                "freeze" => {
                    for &m in &mobs {
                        self.hurt_mob(m, 1.5 * k, caster, 1);
                        self.mobs.list[m].chill = self.mobs.list[m].chill.max(4.0 * (1.0 + aug.ext));
                    }
                    if let Target::Block(b, _) = target {
                        self.freeze_around(b, radius.max(1.0) as i32);
                    }
                }
                "heal" => {
                    for &m in &mobs {
                        let mob = &mut self.mobs.list[m];
                        mob.health = (mob.health + 4.0 * k).min(mob.max_health);
                    }
                    if player || mobs.is_empty() && on_self {
                        self.health = (self.health + 4.0 * k).min(self.max_health);
                    }
                }
                "break" => {
                    if let Target::Block(b, n) = target {
                        let r = if aug.aoe > 0.0 { 1 } else { 0 };
                        self.glyph_break(b, n, r, 3 + aug.amp as u8);
                    }
                }
                "knockback" => {
                    for &m in &mobs {
                        let dir = (self.mobs.list[m].body.pos - caster).normalize_or_zero();
                        self.mobs.list[m].body.vel += dir * 9.0 * k + Vec3::Y * 4.0;
                    }
                    if player {
                        self.body.vel += self.forward() * 12.0 * k;
                    }
                }
                "pull" => {
                    for &m in &mobs {
                        let dir = (caster - self.mobs.list[m].body.pos).normalize_or_zero();
                        self.mobs.list[m].body.vel += dir * 9.0 * k + Vec3::Y * 3.0;
                    }
                    if let (Target::Block(b, _), false) = (target, player) {
                        // Pulls the caster to the block, like a short hook.
                        let to = (b.as_vec3() - caster).normalize_or_zero();
                        self.body.vel += to * 10.0 * k;
                    }
                }
                "leap" => {
                    for &m in &mobs {
                        self.mobs.list[m].body.vel.y += 10.0 * k;
                    }
                    if player {
                        self.body.vel.y = self.body.vel.y.max(10.0 * k);
                        self.fall_peak = self.body.pos.y;
                    }
                }
                "light" => {
                    let spot = match target {
                        Target::Block(b, n) => b + n,
                        _ => at.floor().as_ivec3(),
                    };
                    let torch = self.reg.id_or_air("torch");
                    if torch != AIR && self.world.block_or_air(spot) == AIR && self.world.solid(spot - IVec3::Y) {
                        self.world.set_block(spot, torch);
                    } else {
                        // No floor for a torch: a little light hangs in the air for a while.
                        for _ in 0..30 {
                            self.particles.glow(at, Vec3::ZERO, layer, [1.6, 1.4, 0.7], 0.15, 6.0);
                        }
                    }
                }
                "blink" => {
                    let to = if player { self.body.pos + self.forward() * (8.0 + 4.0 * aug.amp) } else { at };
                    let mut to = to;
                    let mut n = 0;
                    while n < 6 && (self.world.solid(to.floor().as_ivec3()) || self.world.solid((to + Vec3::Y).floor().as_ivec3())) {
                        to += Vec3::Y;
                        n += 1;
                    }
                    self.body.pos = Vec3::new(to.x, to.y.floor(), to.z);
                    self.body.vel = Vec3::ZERO;
                    self.fall_peak = self.body.pos.y;
                }
                "explode" => {
                    self.explode(at, 1.5 + aug.amp);
                }
                "harvest" => {
                    let b = match target {
                        Target::Block(b, _) => b,
                        _ => at.floor().as_ivec3(),
                    };
                    let r = 1 + aug.aoe as i32;
                    for dx in -r..=r {
                        for dy in -1..=1 {
                            for dz in -r..=r {
                                let p = b + IVec3::new(dx, dy, dz);
                                let id = self.world.block_or_air(p);
                                if id != AIR && self.reg.get(id).shape == Shape::Cross && self.world.break_block(p) {
                                    self.glyph_drops(p, id, 0);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        self.sfx(Sfx::SpellHit(3));
    }

    fn freeze_around(&mut self, b: IVec3, r: i32) {
        let water = self.reg.id_or_air("water");
        let lava = self.reg.id_or_air("lava");
        let (ice, obsidian) = (self.reg.id_or_air("ice"), self.reg.id_or_air("obsidian"));
        for dx in -r..=r {
            for dz in -r..=r {
                for dy in -1..=1 {
                    let p = b + IVec3::new(dx, dy, dz);
                    let id = self.world.block_or_air(p);
                    // Any water level freezes; still lava cools to obsidian.
                    if self.reg.get(id).base == water || id == water {
                        self.world.set_block(p, ice);
                    } else if id == lava {
                        self.world.set_block(p, obsidian);
                    }
                }
            }
        }
    }

    /// Break blocks with a spell: as hard as a pickaxe of `tier` can take.
    fn glyph_break(&mut self, b: IVec3, n: IVec3, r: i32, tier: u8) {
        for dx in -r..=r {
            for dy in -r..=r {
                for dz in -r..=r {
                    // The area spreads across the face that was hit.
                    let off = IVec3::new(dx, dy, dz);
                    if r > 0 && (off * n.abs()) != IVec3::ZERO {
                        continue;
                    }
                    let p = b + off;
                    let id = self.world.block_or_air(p);
                    let def = self.reg.get(id);
                    if id == AIR || def.hardness < 0.0 || def.hardness > 3.0 + tier as f32 * 4.0 || self.world.is_liquid(id) || def.tier > tier {
                        continue;
                    }
                    if self.world.break_block(p) {
                        let (layer, tint) = self.block_look(id);
                        self.particles.block_bits(p, layer, tint, 8, 1.0);
                        self.sfx(Sfx::Break(Material::of(&self.reg.get(id).name)));
                        self.glyph_drops(p, id, tier);
                    }
                }
            }
        }
    }

    fn glyph_drops(&mut self, p: IVec3, id: voxel::BlockId, tier: u8) {
        if !self.survival {
            return;
        }
        let tool = ToolDef { kind: ToolKind::Pickaxe, tier, speed: 6.0, damage: 1.0, durability: 1 };
        let drops = self.items.drops(&self.reg, id, Some(&tool), &mut self.surv.rng);
        for d in drops {
            self.pop_drop(p.as_vec3() + Vec3::splat(0.5), d);
        }
    }

    /// The grimoire screen: three spells, the glyphs you know, and what each spell costs.
    pub(crate) fn draw_grimoire(&mut self, w: f32, h: f32, mouse: (f32, f32)) {
        // The page is tall: a smaller scale when the window is short, so it all fits.
        let saved = self.ui.scale;
        self.ui.scale = (h / 330.0).floor().clamp(1.0, saved);
        self.draw_grimoire_page(w, h, mouse);
        self.ui.scale = saved;
    }

    fn draw_grimoire_page(&mut self, w: f32, h: f32, mouse: (f32, f32)) {
        let s = self.ui.scale;
        self.ui.rect(0.0, 0.0, w, h, [10, 6, 20, 190]);
        let pw = (360.0 * s).min(w - 16.0 * s);
        let x0 = ((w - pw) / 2.0).floor();
        let mut y = 12.0 * s;
        self.ui.text_centered(w / 2.0, y, "Grimório de Glifos", [230, 210, 255, 255]);
        y += 14.0 * s;
        self.ui.text_centered(w / 2.0, y, "Clique nos glifos para escrever; comece por uma forma.", [170, 160, 190, 255]);
        y += 10.0 * s;
        self.ui.text_centered(w / 2.0, y, "Esc fecha. Com o grimório na mão, G troca a magia.", [170, 160, 190, 255]);
        y += 14.0 * s;
        let bh = 16.0 * s;
        let tw = (pw - 8.0 * s) / SPELL_SLOTS as f32;
        for i in 0..SPELL_SLOTS {
            let label = if i == self.t5.glyphs.active { format!("> Magia {} <", i + 1) } else { format!("Magia {}", i + 1) };
            if self.ui.button(x0 + i as f32 * (tw + 4.0 * s), y, tw, bh, &label, mouse) && self.clicked {
                self.t5.glyphs.active = i;
                self.sfx(Sfx::Click);
            }
        }
        y += bh + 8.0 * s;
        let active = self.t5.glyphs.active;
        let spell = self.t5.glyphs.spells[active].clone();
        self.ui.rect(x0, y, pw, 34.0 * s, [40, 26, 60, 230]);
        self.ui.frame(x0, y, pw, 34.0 * s, s, [120, 90, 170, 255]);
        let text = spell_text(&spell);
        self.ui.text(x0 + 6.0 * s, y + 5.0 * s, &text, WHITE);
        let info = if spell.is_empty() { String::new() } else { format!("Mana: {:.0}", cost(&spell)) };
        self.ui.text(x0 + 6.0 * s, y + 19.0 * s, &info, [140, 170, 255, 255]);
        y += 40.0 * s;
        let half = (pw - 4.0 * s) / 2.0;
        if self.ui.button(x0, y, half, bh, "Apagar o último glifo", mouse) && self.clicked {
            self.t5.glyphs.spells[active].pop();
            self.sfx(Sfx::Click);
        }
        if self.ui.button(x0 + half + 4.0 * s, y, half, bh, "Limpar a magia", mouse) && self.clicked {
            self.t5.glyphs.spells[active].clear();
            self.sfx(Sfx::Click);
        }
        y += bh + 10.0 * s;
        for (title, kind) in [("Formas", Kind::Form), ("Efeitos", Kind::Effect), ("Aumentos", Kind::Augment)] {
            self.ui.text(x0, y, title, [200, 180, 240, 255]);
            y += 11.0 * s;
            let cols = 5;
            let cw = (pw - (cols - 1) as f32 * 4.0 * s) / cols as f32;
            let list: Vec<_> = GLYPHS.iter().filter(|g| g.2 == kind).collect();
            for (k, g) in list.iter().enumerate() {
                let (cx, cy) = (x0 + (k % cols) as f32 * (cw + 4.0 * s), y + (k / cols) as f32 * (bh + 3.0 * s));
                let known = self.knows(g.0);
                let label = if known { format!("{} ({:.0})", g.1, g.3) } else { format!("? {}", g.1) };
                let over = self.ui.button(cx, cy, cw, bh, &label, mouse);
                if !known {
                    self.ui.rect(cx, cy, cw, bh, [0, 0, 0, 120]);
                }
                if over && self.clicked && known {
                    let sp = &mut self.t5.glyphs.spells[active];
                    let first = sp.is_empty();
                    if first != (kind == Kind::Form) {
                        let t = if first { "Comece a magia por uma forma" } else { "Só a primeira peça é uma forma" };
                        self.toast(t, 2.0);
                    } else if sp.len() >= MAX_GLYPHS {
                        self.toast("Uma magia tem no máximo 9 glifos", 2.0);
                    } else {
                        sp.push(g.0.to_string());
                        self.sfx(Sfx::Click);
                    }
                }
            }
            y += ((list.len() + cols - 1) / cols) as f32 * (bh + 3.0 * s) + 6.0 * s;
        }
        if self.survival {
            let n = self.t5.glyphs.learned.len();
            let t = format!("Glifos aprendidos: {n} de {}. Fabrique glifos e use-os (botão direito) para aprender.", GLYPHS.len());
            self.ui.text_centered(w / 2.0, y + 2.0 * s, &t, [170, 160, 190, 255]);
        }
    }

    /// The spell in use, under the crosshair while the grimoire is in hand.
    pub(crate) fn draw_glyph_hud(&mut self, w: f32, h: f32) {
        if self.held_stack().map(|s| self.items.get(s.item).name.as_str() == "glyph_book") != Some(true) || self.screen != Screen::Playing {
            return;
        }
        let g = &self.t5.glyphs;
        let t = format!("Magia {}: {}", g.active + 1, spell_text(&g.spells[g.active]));
        let s = self.ui.scale;
        self.ui.text_centered(w / 2.0, h / 2.0 + 14.0 * s, &t, [210, 190, 255, 230]);
    }
}
