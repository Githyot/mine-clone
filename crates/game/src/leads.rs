//! Leads and name tags (vanilla ideas). A lead ties an animal to you: it follows close behind,
//! and right click on a fence ties it to the post. A name tag gives a creature a name you type,
//! shown over its head; named and leashed creatures stay around.

use glam::Vec3;
use voxel::item::ItemStack;

use crate::{Game, Screen};

pub const NAME_LEN: usize = 16;

impl Game {
    /// Right click with a lead or a name tag on a creature, or with leashed animals on a fence.
    pub(crate) fn lead_use(&mut self) -> bool {
        let Some(held) = self.held_name() else {
            return self.lead_to_post();
        };
        if held != "lead" && held != "name_tag" {
            return self.lead_to_post();
        }
        let Some((i, t)) = self.mobs.pick(self.body.eye(), self.forward(), 4.0) else {
            return held == "lead" && self.lead_to_post();
        };
        let block = self.target.map(|h| h.distance).unwrap_or(f32::MAX);
        if t > block + 0.3 {
            return held == "lead" && self.lead_to_post();
        }
        let def = &self.mobs.reg.defs[self.mobs.list[i].def];
        if def.boss || def.town {
            return false;
        }
        if held == "name_tag" {
            self.naming = Some((i, self.mobs.list[i].name.clone().unwrap_or_default()));
            self.set_screen(Screen::Sign);
            return true;
        }
        // A lead: only on animals, not on monsters.
        if def.kind == voxel::mob::Kind::Hostile {
            return false;
        }
        let m = &mut self.mobs.list[i];
        if m.leashed {
            // Untie it; the lead comes back.
            m.leashed = false;
            m.post = None;
            let at = m.body.pos + Vec3::Y;
            if let Some(it) = self.items.id("lead") {
                self.pop_drop(at, ItemStack::new(it, 1));
            }
            return true;
        }
        m.leashed = true;
        m.post = None;
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        self.anim.start_swing();
        true
    }

    /// Right click on a fence with animals on your lead: tie them all to it.
    fn lead_to_post(&mut self) -> bool {
        let Some(hit) = self.target else { return false };
        let name = self.reg.get(self.world.block_or_air(hit.block)).name.clone();
        if !name.ends_with("_fence") && !name.ends_with("_wall") {
            return false;
        }
        let me = self.body.pos;
        let mut tied = false;
        for m in self.mobs.list.iter_mut() {
            if m.alive() && m.leashed && m.post.is_none() && m.body.pos.distance(me) < 12.0 {
                m.post = Some(hit.block);
                tied = true;
            }
        }
        tied
    }

    /// Leashed animals keep close to whoever holds them; a lead pulled too far snaps.
    pub(crate) fn update_leads(&mut self, dt: f32) {
        // Test hook: the creatures around get names, and the nearest ones a lead.
        if std::env::var("MC_LEAD").is_ok() && !self.mobs.list.iter().any(|m| m.name.is_some()) && !self.mobs.list.is_empty() {
            for (k, m) in self.mobs.list.iter_mut().enumerate() {
                m.name = Some(["Pipoca", "Bolinha", "Dolly", "Trovão"][k % 4].to_string());
                m.leashed = k < 2;
            }
        }
        let me = self.body.pos;
        let mut snapped = Vec::new();
        let mut ropes = Vec::new();
        for m in self.mobs.list.iter_mut() {
            if !m.leashed {
                continue;
            }
            if !m.alive() {
                m.leashed = false;
                snapped.push(m.body.pos + Vec3::Y * 0.5);
                continue;
            }
            let anchor = match m.post {
                Some(p) => p.as_vec3() + Vec3::new(0.5, 0.6, 0.5),
                None => me + Vec3::Y * 0.9,
            };
            let to = anchor - m.body.pos;
            let flat = Vec3::new(to.x, 0.0, to.z);
            let d = flat.length();
            if d > 12.0 {
                m.leashed = false;
                m.post = None;
                snapped.push(m.body.pos + Vec3::Y * 0.5);
                continue;
            }
            if d > 3.0 {
                m.go_to(anchor - flat.normalize() * 1.5, 0.2);
            }
            if d > 6.0 {
                let pull = flat.normalize() * (d - 6.0) * 4.0 * dt;
                m.body.vel += Vec3::new(pull.x, 0.0, pull.z) * 10.0;
            }
            ropes.push((m.body.pos + Vec3::Y * 0.6, anchor));
        }
        if let Some(it) = self.items.id("lead") {
            for at in snapped {
                self.pop_drop(at, ItemStack::new(it, 1));
            }
        }
        // The rope: a sagging line of little brown bits.
        let layer = self.block_look(self.reg.id_or_air("brown_wool")).0;
        for (a, b) in ropes {
            let n = ((a.distance(b) / 0.07) as usize).clamp(2, 200);
            for k in 0..=n {
                let t = k as f32 / n as f32;
                let sag = (t * (1.0 - t)) * 4.0 * (0.4 + a.distance(b) * 0.05);
                let p = a.lerp(b, t) - Vec3::Y * sag;
                self.particles.mote(p, layer, [0.5, 0.36, 0.2], 0.06, 0.0, 0.05);
            }
        }
    }

    pub(crate) fn naming_key(&mut self, code: winit::keyboard::KeyCode) {
        use winit::keyboard::KeyCode;
        let Some((_, t)) = self.naming.as_mut() else { return };
        match code {
            KeyCode::Backspace => {
                t.pop();
            }
            KeyCode::Enter | KeyCode::NumpadEnter => self.close_naming(true),
            KeyCode::Escape => self.close_naming(false),
            _ => {}
        }
    }

    pub(crate) fn naming_type(&mut self, text: &str) {
        let Some((_, t)) = self.naming.as_mut() else { return };
        for c in text.chars() {
            if !c.is_control() && t.chars().count() < NAME_LEN {
                t.push(c);
            }
        }
    }

    fn close_naming(&mut self, done: bool) {
        if let Some((i, name)) = self.naming.take() {
            let name = name.trim().to_string();
            if done && !name.is_empty() {
                if let Some(m) = self.mobs.list.get_mut(i).filter(|m| m.alive()) {
                    m.name = Some(name);
                    if self.survival {
                        let sel = self.selected;
                        self.surv.inv.take(sel, 1);
                    }
                }
            }
        }
        self.set_screen(Screen::Playing);
    }

    /// The name tag editor: a little tag with the name being typed.
    pub(crate) fn draw_naming(&mut self, w: f32, h: f32) {
        let s = self.ui.scale;
        let text = self.naming.as_ref().map(|n| n.1.clone()).unwrap_or_default();
        let (bw, bh) = (120.0 * s, 24.0 * s);
        let (x, y) = ((w - bw) / 2.0, (h - bh) / 2.0 - 20.0 * s);
        self.ui.rect(0.0, 0.0, w, h, [0, 0, 0, 120]);
        self.ui.rect(x - 2.0 * s, y - 2.0 * s, bw + 4.0 * s, bh + 4.0 * s, [60, 60, 66, 255]);
        self.ui.rect(x, y, bw, bh, [214, 206, 186, 255]);
        let blink = (self.now * 2.0) as i64 % 2 == 0;
        let shown = if blink { format!("{text}_") } else { text.clone() };
        let tw = self.ui.text_width(&text, s);
        self.ui.text(w / 2.0 - tw / 2.0, y + 8.0 * s, &shown, [30, 24, 16, 255]);
        self.ui.text_centered(w / 2.0, y + bh + 10.0 * s, "Digite o nome. Enter: pronto. Esc: cancela", [255, 255, 255, 255]);
    }

    /// Names over the heads of named creatures nearby.
    pub(crate) fn draw_mob_names(&mut self, w: f32, h: f32) {
        let Some((vp, cam)) = self.ui_view_proj.filter(|_| self.screen == crate::Screen::Playing) else { return };
        let eye = self.body.eye();
        let project = |v: Vec3| {
            let c = vp * (v - cam.as_vec3()).extend(1.0);
            (c.w > 0.1).then(|| (((c.x / c.w) * 0.5 + 0.5) * w, (0.5 - (c.y / c.w) * 0.5) * h))
        };
        let named: Vec<(Vec3, String)> = self
            .mobs
            .list
            .iter()
            .filter(|m| m.alive() && m.body.pos.distance(eye) < if m.still { 6.0 } else { 16.0 })
            .filter_map(|m| {
                let def = &self.mobs.reg.defs[m.def];
                if !self.in_sight(&[m.body.pos + Vec3::Y * def.height * def.scale.max(0.3) * 0.9, m.body.center()]) {
                    return None;
                }
                m.name.clone().map(|n| (m.body.pos + Vec3::Y * (def.height * def.scale.max(0.3) + 0.35), n))
            })
            .collect();
        for (at, name) in named {
            let Some(p) = project(at) else { continue };
            let s = self.ui.scale * 0.8;
            let tw = self.ui.text_width(&name, s);
            self.ui.rect(p.0 - tw / 2.0 - 2.0 * s, p.1 - 2.0 * s, tw + 4.0 * s, 11.0 * s, [0, 0, 0, 110]);
            self.ui.text_scaled(p.0 - tw / 2.0, p.1, &name, [255, 255, 255, 255], s);
        }
    }
}

