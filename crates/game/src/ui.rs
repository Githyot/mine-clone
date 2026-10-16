//! Immediate-mode 2D drawing for the HUD and menus: coloured rectangles, pixel text and block icons.

use font8x8::UnicodeFonts;
use voxel::block::Shape;
use voxel::{BlockId, BlockRegistry};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UiVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [u8; 4],
    /// -2 = plain colour, -1 = font atlas, >= 0 = block texture layer.
    pub layer: i32,
}

/// 16×16 grid of 8×8 glyphs for code points 0..256 (ASCII + Latin-1, enough for Portuguese).
pub struct FontAtlas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    /// Advance width of each glyph in font pixels.
    pub advance: [u8; 256],
}

impl FontAtlas {
    pub fn new() -> FontAtlas {
        let (w, h) = (128u32, 128u32);
        let mut pixels = vec![0u8; (w * h) as usize];
        let mut advance = [4u8; 256];
        for code in 0u32..256 {
            let ch = char::from_u32(code).unwrap();
            let glyph = font8x8::BASIC_FONTS.get(ch).or_else(|| font8x8::LATIN_FONTS.get(ch));
            let Some(g) = glyph else { continue };
            let (gx, gy) = ((code % 16) * 8, (code / 16) * 8);
            let mut maxx = 0;
            let mut any = false;
            for (row, bits) in g.iter().enumerate() {
                for col in 0..8 {
                    if bits & (1 << col) != 0 {
                        pixels[((gy + row as u32) * w + gx + col) as usize] = 255;
                        maxx = maxx.max(col);
                        any = true;
                    }
                }
            }
            advance[code as usize] = if any { maxx as u8 + 2 } else { 4 };
        }
        FontAtlas { width: w, height: h, pixels, advance }
    }
}

pub const WHITE: [u8; 4] = [255, 255, 255, 255];
pub const GRAY: [u8; 4] = [170, 170, 170, 255];
pub const YELLOW: [u8; 4] = [255, 255, 85, 255];

pub struct UiBatch {
    pub verts: Vec<UiVertex>,
    pub scale: f32,
    pub width: f32,
    pub height: f32,
    advance: [u8; 256],
    /// Interface art from the resource packs, by name (see `textures::GuiAtlas`).
    pub gui: std::collections::HashMap<String, [f32; 4]>,
}

impl UiBatch {
    pub fn new(font: &FontAtlas) -> UiBatch {
        UiBatch { verts: Vec::new(), scale: 2.0, width: 1.0, height: 1.0, advance: font.advance, gui: Default::default() }
    }

    pub fn begin(&mut self, width: f32, height: f32, gui_scale: u32) {
        self.verts.clear();
        self.width = width;
        self.height = height;
        let auto = (width / 320.0).min(height / 240.0).floor().clamp(1.0, 4.0);
        self.scale = if gui_scale == 0 { auto } else { (gui_scale as f32).min(auto.max(1.0)) };
    }

    /// Any four corners (clockwise from top-left), so icons can be drawn in perspective.
    pub fn quad(&mut self, p: [[f32; 2]; 4], uv: [[f32; 2]; 4], color: [u8; 4], layer: i32) {
        let v = |i: usize| UiVertex { pos: p[i], uv: uv[i], color, layer };
        self.verts.extend_from_slice(&[v(0), v(1), v(2), v(0), v(2), v(3)]);
    }

    /// A rectangle with a colour per corner (top-left, top-right, bottom-right, bottom-left),
    /// blended smoothly across.
    pub fn gradient(&mut self, x: f32, y: f32, w: f32, h: f32, c: [[u8; 4]; 4]) {
        let p = [[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
        let v = |i: usize| UiVertex { pos: p[i], uv: [0.0; 2], color: c[i], layer: -2 };
        self.verts.extend_from_slice(&[v(0), v(1), v(2), v(0), v(2), v(3)]);
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [u8; 4]) {
        self.quad([[x, y], [x + w, y], [x + w, y + h], [x, y + h]], [[0.0; 2]; 4], color, -2);
    }

    pub fn frame(&mut self, x: f32, y: f32, w: f32, h: f32, t: f32, color: [u8; 4]) {
        self.rect(x, y, w, t, color);
        self.rect(x, y + h - t, w, t, color);
        self.rect(x, y, t, h, color);
        self.rect(x + w - t, y, t, h, color);
    }

    pub fn text_width(&self, s: &str, scale: f32) -> f32 {
        s.chars().map(|c| self.advance[(c as u32).min(255) as usize] as f32 * scale).sum()
    }

    fn glyphs(&mut self, x: f32, y: f32, s: &str, scale: f32, color: [u8; 4]) {
        let mut cx = x;
        for c in s.chars() {
            let code = if (c as u32) < 256 { c as u32 } else { '?' as u32 };
            if c != ' ' {
                let (u, v) = ((code % 16) as f32 / 16.0, (code / 16) as f32 / 16.0);
                let d = 1.0 / 16.0;
                let sz = 8.0 * scale;
                self.quad([[cx, y], [cx + sz, y], [cx + sz, y + sz], [cx, y + sz]], [[u, v], [u + d, v], [u + d, v + d], [u, v + d]], color, -1);
            }
            cx += self.advance[code as usize] as f32 * scale;
        }
    }

    /// Text with the classic drop shadow.
    pub fn text(&mut self, x: f32, y: f32, s: &str, color: [u8; 4]) {
        let sc = self.scale;
        let shadow = [color[0] / 4, color[1] / 4, color[2] / 4, color[3]];
        self.glyphs(x + sc, y + sc, s, sc, shadow);
        self.glyphs(x, y, s, sc, color);
    }

    pub fn text_scaled(&mut self, x: f32, y: f32, s: &str, color: [u8; 4], scale: f32) {
        let shadow = [color[0] / 4, color[1] / 4, color[2] / 4, color[3]];
        self.glyphs(x + scale, y + scale, s, scale, shadow);
        self.glyphs(x, y, s, scale, color);
    }

    /// Text without a shadow, for dark text on light panels.
    pub fn label(&mut self, x: f32, y: f32, s: &str, color: [u8; 4], scale: f32) {
        self.glyphs(x, y, s, scale, color);
    }

    pub fn text_centered(&mut self, cx: f32, y: f32, s: &str, color: [u8; 4]) {
        let w = self.text_width(s, self.scale);
        self.text(cx - w / 2.0, y, s, color);
    }

    /// Whether the resource packs supply the interface picture `name`.
    pub fn has_sprite(&self, name: &str) -> bool {
        self.gui.contains_key(name)
    }

    /// Draw part of a pack's interface picture: `part` is (x0, y0, x1, y1) as fractions of the
    /// picture. Returns false (drawing nothing) when no pack has it.
    pub fn sprite_part(&mut self, name: &str, part: [f32; 4], x: f32, y: f32, w: f32, h: f32, color: [u8; 4]) -> bool {
        let Some(&[u0, v0, u1, v1]) = self.gui.get(name) else { return false };
        let u = |f: f32| u0 + (u1 - u0) * f;
        let v = |f: f32| v0 + (v1 - v0) * f;
        let uv = [[u(part[0]), v(part[1])], [u(part[2]), v(part[1])], [u(part[2]), v(part[3])], [u(part[0]), v(part[3])]];
        self.quad([[x, y], [x + w, y], [x + w, y + h], [x, y + h]], uv, color, -3);
        true
    }

    /// A whole interface picture from the packs.
    pub fn sprite(&mut self, name: &str, x: f32, y: f32, w: f32, h: f32) -> bool {
        self.sprite_part(name, [0.0, 0.0, 1.0, 1.0], x, y, w, h, WHITE)
    }

    /// A block as inventory icons show it: seen from above, front-left and front-right, built
    /// from its boxes so stairs, slabs, fences and walls keep their shape. Plants are flat.
    pub fn block_icon(&mut self, reg: &BlockRegistry, id: BlockId, x: f32, y: f32, size: f32) {
        let b = reg.get(id);
        let full = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        if matches!(b.shape, Shape::Cross | Shape::Torch) {
            let l = b.faces[0] as i32;
            self.quad([[x, y], [x + size, y], [x + size, y + size], [x, y + size]], full, WHITE, l);
            return;
        }
        let mut boxes: Vec<voxel::shapes::PBox> = Vec::new();
        if b.shape.is_boxes() {
            // Stairs climb away from the viewer; fences, walls and panes show two arms.
            let (variant, conn) = match b.shape {
                Shape::Stairs | Shape::Roof => (3, 0),
                Shape::Fence | Shape::Wall | Shape::Pane => (0, voxel::shapes::CONN_PX | voxel::shapes::CONN_NX),
                _ => (0, 0),
            };
            voxel::shapes::boxes(b.shape, variant, conn, false, &mut boxes);
        } else {
            boxes.push([0, 0, 0, 16, 16, 16]);
        }
        // Far boxes first so near ones cover them.
        boxes.sort_by_key(|q| (q[0] as u16 + q[2] as u16, q[1]));
        let cx = x + size / 2.0;
        let w = size * 0.5;
        let h = size * 0.25;
        let y0 = y + size * 0.02;
        let side = size * 0.58;
        // A point of the block (in pixels, 0..16) on the screen.
        let at = |px: f32, py: f32, pz: f32| [cx + w * (px - pz) / 16.0, y0 + h * (px + pz) / 16.0 + side * (16.0 - py) / 16.0];
        for q in boxes {
            let [x0, y0b, z0, x1, y1, z1] = q.map(|v| v as f32);
            let t = |v: f32| v / 16.0;
            // Front (+z), seen on the left.
            self.quad(
                [at(x0, y1, z1), at(x1, y1, z1), at(x1, y0b, z1), at(x0, y0b, z1)],
                [[t(x0), t(16.0 - y1)], [t(x1), t(16.0 - y1)], [t(x1), t(16.0 - y0b)], [t(x0), t(16.0 - y0b)]],
                [205, 205, 205, 255],
                b.faces[4] as i32,
            );
            // Side (+x), seen on the right.
            self.quad(
                [at(x1, y1, z1), at(x1, y1, z0), at(x1, y0b, z0), at(x1, y0b, z1)],
                [[t(16.0 - z1), t(16.0 - y1)], [t(16.0 - z0), t(16.0 - y1)], [t(16.0 - z0), t(16.0 - y0b)], [t(16.0 - z1), t(16.0 - y0b)]],
                [150, 150, 150, 255],
                b.faces[0] as i32,
            );
            // Top.
            self.quad(
                [at(x0, y1, z0), at(x1, y1, z0), at(x1, y1, z1), at(x0, y1, z1)],
                [[t(x0), t(z0)], [t(x1), t(z0)], [t(x1), t(z1)], [t(x0), t(z1)]],
                WHITE,
                b.faces[2] as i32,
            );
        }
    }

    /// A clickable button; returns true when the mouse is over it.
    pub fn button(&mut self, x: f32, y: f32, w: f32, h: f32, label: &str, mouse: (f32, f32)) -> bool {
        let hover = mouse.0 >= x && mouse.0 < x + w && mouse.1 >= y && mouse.1 < y + h;
        let s = self.scale;
        self.rect(x, y, w, h, if hover { [40, 40, 40, 255] } else { [0, 0, 0, 255] });
        self.rect(x + s, y + s, w - 2.0 * s, h - 2.0 * s, if hover { [126, 136, 191, 255] } else { [111, 111, 111, 255] });
        self.rect(x + s, y + s, w - 2.0 * s, s, if hover { [168, 178, 230, 255] } else { [160, 160, 160, 255] });
        self.rect(x + s, y + h - 2.0 * s, w - 2.0 * s, s, if hover { [90, 100, 150, 255] } else { [70, 70, 70, 255] });
        // Long labels shrink, then lose their end, so they never spill out of the button.
        let color = if hover { YELLOW } else { [224, 224, 224, 255] };
        let room = w - 6.0 * s;
        let mut sc = s;
        while sc > 1.0 && self.text_width(label, sc) > room {
            sc -= 1.0;
        }
        let mut text = label.to_string();
        if self.text_width(&text, sc) > room {
            while !text.is_empty() && self.text_width(&format!("{text}..."), sc) > room {
                text.pop();
            }
            text.push_str("...");
        }
        let tw = self.text_width(&text, sc);
        self.text_scaled(x + (w - tw) / 2.0, y + (h - 8.0 * sc) / 2.0, &text, color, sc);
        hover
    }
}
