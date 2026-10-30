//! Signs: wooden boards standing on a post or hung on a wall, with up to four short lines of
//! text. Placing one opens the editor; right click edits it again. The text is cut into the
//! front of the board (see `sign_letters`), and is saved per world in `signs.ron`.

use std::collections::HashMap;
use std::path::Path;

use glam::{IVec3, Quat, Vec3};
use voxel::block::Shape;
use voxel::shapes::DIRS6;

use crate::particles::GpuParticle;
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

    /// The letters of the signs around as tiny boxes fixed to the front of each board, so the
    /// text stays put on the wood as you walk past, hides behind blocks and takes the light of
    /// the place. Each glyph pixel is near-black with a thin lighter edge under it where the
    /// stroke ends, like letters cut into the wood with light falling into the groove.
    pub(crate) fn sign_letters(&self, cam_block: IVec3) -> Vec<GpuParticle> {
        let mut out = Vec::new();
        let eye = self.body.eye();
        let cam = cam_block.as_vec3();
        for (p, text) in &self.signs {
            let at = IVec3::from(*p);
            if text.trim().is_empty() || (at.as_vec3() + Vec3::splat(0.5)).distance(eye) > SHOW_DIST {
                continue;
            }
            let Some(v) = self.is_sign(at) else { continue };
            let d = DIRS6[(v & 3) as usize];
            // The board's back points along `dir`; its front looks the other way.
            let dir = Vec3::new(d[0] as f32, 0.0, d[2] as f32);
            let n = -dir;
            let wall = v & 4 != 0;
            let mid = at.as_vec3() + Vec3::new(0.5, if wall { 0.5 } else { 11.0 / 16.0 }, 0.5) + if wall { dir * (7.0 / 16.0) } else { Vec3::ZERO };
            let face = mid + n / 16.0;
            if (eye - face).dot(n) <= 0.0 {
                continue;
            }
            let l = self.world.light(at);
            let light = ((l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0);
            let right = Vec3::Y.cross(n);
            let rot = Quat::from_mat3(&glam::Mat3::from_cols(right, Vec3::Y, n));
            let origin = face + n * 0.001 - cam;
            for b in letter_boxes(text) {
                let c = origin + right * b.center[0] + Vec3::Y * b.center[1];
                out.push(GpuParticle::plain(c, rot, Vec3::new(b.size[0], b.size[1], DEPTH), b.color, light));
            }
        }
        out
    }
}

/// Signs farther than this (blocks) leave their text out.
const SHOW_DIST: f32 = 20.0;
/// How far the letter boxes reach out of the wood (half inside it, hidden).
const DEPTH: f32 = 0.004;
/// The usable part of the board, in blocks (16×8 pixels less a pixel all round).
const BOARD_W: f32 = 14.0 / 16.0;
const BOARD_H: f32 = 7.0 / 16.0;
/// Font pixels from one line to the next (8 for the glyph, 2 between).
const LINE_H: f32 = 10.0;
/// Size of a font pixel on the board when the longest line is 15 average letters, like the
/// game it comes from: every sign writes at the same size unless a line would run off it.
const PIXEL: f32 = BOARD_W / 90.0;
/// The dark of a letter, and the lit lower wall of its groove.
const INK: [f32; 3] = [0.045, 0.032, 0.022];
const LIP: [f32; 3] = [0.30, 0.21, 0.13];
/// The part of a pixel's height the lit lower wall takes.
const LIP_H: f32 = 0.3;

pub(crate) struct LetterBox {
    /// Centre on the board, in blocks from its middle (right, up).
    pub center: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 3],
}

fn glyph(c: char) -> Option<[u8; 8]> {
    use font8x8::UnicodeFonts;
    font8x8::BASIC_FONTS.get(c).or_else(|| font8x8::LATIN_FONTS.get(c)).or_else(|| font8x8::BASIC_FONTS.get('?'))
}

/// Width of a character in font pixels, the same as the interface text (ui::FontAtlas).
fn advance(g: Option<[u8; 8]>) -> f32 {
    let maxx = g.iter().flatten().fold(None, |m: Option<u32>, &bits| if bits == 0 { m } else { Some(m.unwrap_or(0).max(7 - bits.leading_zeros())) });
    maxx.map_or(4.0, |m| m as f32 + 2.0)
}

/// The boxes that write `text` on a board, centred: runs of pixels along each glyph row
/// merged into one box, each row split into its dark top and, where the stroke ends below,
/// a lighter lip.
pub(crate) fn letter_boxes(text: &str) -> Vec<LetterBox> {
    let lines: Vec<Vec<(char, Option<[u8; 8]>)>> = text.split('\n').take(LINES).map(|l| l.chars().map(|c| (c, if c == ' ' { None } else { glyph(c) })).collect()).collect();
    let width = |l: &[(char, Option<[u8; 8]>)]| l.iter().map(|&(c, g)| if c == ' ' { 4.0 } else { advance(g) }).sum::<f32>() - 1.0;
    let widest = lines.iter().map(|l| width(l)).fold(1.0f32, f32::max);
    let tall = lines.len() as f32 * LINE_H - 2.0;
    let k = PIXEL.min(BOARD_W / widest).min(BOARD_H / tall);
    let mut out = Vec::new();
    let top = tall * k / 2.0;
    for (li, line) in lines.iter().enumerate() {
        let mut x = -width(line) * k / 2.0;
        let y0 = top - li as f32 * LINE_H * k;
        for &(c, g) in line {
            let adv = if c == ' ' { 4.0 } else { advance(g) };
            if let Some(g) = g {
                for row in 0..8 {
                    let bits = g[row];
                    let below = if row < 7 { g[row + 1] } else { 0 };
                    let y = y0 - row as f32 * k;
                    let parts = [(bits, 0.0, 1.0 - LIP_H, INK), (bits & below, 1.0 - LIP_H, LIP_H, INK), (bits & !below, 1.0 - LIP_H, LIP_H, LIP)];
                    for (mask, from, h, color) in parts {
                        let mut col = 0;
                        while col < 8 {
                            if mask & (1 << col) == 0 {
                                col += 1;
                                continue;
                            }
                            let start = col;
                            while col < 8 && mask & (1 << col) != 0 {
                                col += 1;
                            }
                            let len = (col - start) as f32;
                            out.push(LetterBox {
                                center: [x + (start as f32 + len / 2.0) * k, y - (from + h / 2.0) * k],
                                size: [len * k, h * k],
                                color,
                            });
                        }
                    }
                }
            }
            x += adv * k;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_fit_on_the_board() {
        for text in ["Ferramentas e\narmas\n/tp 0 66 114", "WWWWWWWWWWWWWWW\nMMMMMMMMMMMMMMM\nWWWWWWWWWWWWWWW\nMMMMMMMMMMMMMMM", "a"] {
            let boxes = letter_boxes(text);
            assert!(!boxes.is_empty());
            for b in &boxes {
                assert!(b.center[0].abs() + b.size[0] / 2.0 <= BOARD_W / 2.0 + 1e-4, "{text}: too wide");
                assert!(b.center[1].abs() + b.size[1] / 2.0 <= BOARD_H / 2.0 + 1e-4, "{text}: too tall");
            }
        }
    }

    #[test]
    fn short_signs_write_at_the_normal_size() {
        let a = letter_boxes("Oi");
        let b = letter_boxes("Oi\nOi\nOi\nOi");
        assert!((a[0].size[1] - b[0].size[1]).abs() < 1e-6);
    }
}
