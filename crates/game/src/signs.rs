//! Signs: wooden boards standing on a post or hung on a wall, with up to four short lines of
//! text. Placing one opens the editor; right click edits it again. The text is drawn over the
//! board while you are close and in front of it, and is saved per world in `signs.ron`.

use std::collections::HashMap;
use std::path::Path;

use glam::{IVec3, Vec3};
use voxel::block::Shape;
use voxel::shapes::DIRS6;

use crate::{Game, Screen};

pub const LINES: usize = 4;
pub const LINE_LEN: usize = 15;

pub fn load(dir: &Path) -> HashMap<[i32; 3], String> {
    std::fs::read_to_string(dir.join("signs.ron")).ok().and_then(|t| ron::from_str(&t).ok()).unwrap_or_default()
}

pub fn save(dir: &Path, signs: &HashMap<[i32; 3], String>) {
    let _ = std::fs::create_dir_all(dir);
    if let Ok(t) = ron::ser::to_string_pretty(signs, ron::ser::PrettyConfig::default()) {
        let _ = std::fs::write(dir.join("signs.ron"), t);
    }
}

impl Game {
    fn is_sign(&self, p: IVec3) -> Option<u8> {
        let id = self.world.block_or_air(p);
        let b = self.reg.get(id);
        (b.shape == Shape::Sign).then_some(b.variant)
    }

    /// Open the editor for the sign at `p`.
    pub(crate) fn edit_sign(&mut self, p: IVec3) -> bool {
        if self.is_sign(p).is_none() {
            return false;
        }
        self.sign_edit = Some(p);
        self.signs.entry(p.to_array()).or_default();
        self.set_screen(Screen::Sign);
        true
    }

    pub(crate) fn sign_key(&mut self, code: winit::keyboard::KeyCode) {
        use winit::keyboard::KeyCode;
        let Some(p) = self.sign_edit else { return };
        let t = self.signs.entry(p.to_array()).or_default();
        match code {
            KeyCode::Backspace => {
                t.pop();
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {
                if t.split('\n').count() < LINES {
                    t.push('\n');
                } else {
                    self.close_sign();
                }
            }
            KeyCode::Escape => self.close_sign(),
            _ => {}
        }
    }

    pub(crate) fn sign_type(&mut self, text: &str) {
        let Some(p) = self.sign_edit else { return };
        let t = self.signs.entry(p.to_array()).or_default();
        for c in text.chars() {
            let line = t.rsplit('\n').next().unwrap_or("");
            if !c.is_control() && line.chars().count() < LINE_LEN {
                t.push(c);
            }
        }
    }

    fn close_sign(&mut self) {
        if let Some(p) = self.sign_edit.take() {
            if self.signs.get(&p.to_array()).is_some_and(|t| t.trim().is_empty()) {
                self.signs.remove(&p.to_array());
            }
        }
        self.set_screen(Screen::Playing);
    }

    /// The editor: the board, big, with its lines and a cursor.
    pub(crate) fn draw_sign_editor(&mut self, w: f32, h: f32) {
        let s = self.ui.scale;
        let text = self.sign_edit.and_then(|p| self.signs.get(&p.to_array()).cloned()).unwrap_or_default();
        let (bw, bh) = (110.0 * s, 56.0 * s);
        let (x, y) = ((w - bw) / 2.0, (h - bh) / 2.0 - 20.0 * s);
        self.ui.rect(0.0, 0.0, w, h, [0, 0, 0, 120]);
        self.ui.rect(x - 2.0 * s, y - 2.0 * s, bw + 4.0 * s, bh + 4.0 * s, [70, 48, 26, 255]);
        self.ui.rect(x, y, bw, bh, [160, 122, 74, 255]);
        let blink = (self.now * 2.0) as i64 % 2 == 0;
        let lines: Vec<&str> = text.split('\n').collect();
        for (i, l) in lines.iter().enumerate() {
            let shown = if i + 1 == lines.len() && blink { format!("{l}_") } else { l.to_string() };
            let tw = self.ui.text_width(l, s);
            self.ui.text(w / 2.0 - tw / 2.0, y + 6.0 * s + i as f32 * 12.0 * s, &shown, [30, 20, 10, 255]);
        }
        self.ui.text_centered(w / 2.0, y + bh + 10.0 * s, "Digite o texto. Enter: nova linha / pronto. Esc: fecha", [255, 255, 255, 255]);
    }

    /// The text of the signs around, drawn over their boards.
    pub(crate) fn draw_sign_texts(&mut self, w: f32, h: f32) {
        let Some((vp, cam)) = self.ui_view_proj else { return };
        let eye = self.body.eye();
        let near: Vec<([i32; 3], String)> =
            self.signs.iter().filter(|(p, t)| !t.is_empty() && IVec3::from(**p).as_vec3().distance(eye) < 12.0).map(|(p, t)| (*p, t.clone())).collect();
        let project = |v: Vec3| {
            let c = vp * (v - cam.as_vec3()).extend(1.0);
            (c.w > 0.1).then(|| (((c.x / c.w) * 0.5 + 0.5) * w, (0.5 - (c.y / c.w) * 0.5) * h))
        };
        for (p, text) in near {
            let at = IVec3::from(p);
            let Some(v) = self.is_sign(at) else {
                self.signs.remove(&p);
                continue;
            };
            let d = DIRS6[(v & 3) as usize];
            let dir = Vec3::new(d[0] as f32, 0.0, d[2] as f32);
            let wall = v & 4 != 0;
            let mid = at.as_vec3() + Vec3::new(0.5, if wall { 0.5 } else { 11.0 / 16.0 }, 0.5) + if wall { dir * (7.0 / 16.0) } else { Vec3::ZERO };
            let face = mid - dir * (1.2 / 16.0);
            if (eye - face).dot(-dir) <= 0.0 {
                continue;
            }
            let (Some(a), Some(b)) = (project(face), project(face + Vec3::Y * 0.5)) else { continue };
            let block = (a.1 - b.1).abs() * 2.0;
            let sc = block * 0.85 / 90.0;
            if sc < 0.25 {
                continue;
            }
            let lines: Vec<&str> = text.split('\n').collect();
            let lh = 9.5 * sc;
            let top = a.1 - lh * lines.len() as f32 / 2.0;
            for (i, l) in lines.iter().enumerate() {
                let tw = self.ui.text_width(l, sc);
                self.ui.text_scaled(a.0 - tw / 2.0, top + i as f32 * lh, l, [40, 28, 14, 255], sc);
            }
        }
    }
}
