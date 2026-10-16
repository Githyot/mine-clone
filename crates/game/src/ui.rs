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
}

impl UiBatch {
    pub fn new(font: &FontAtlas) -> UiBatch {
        UiBatch { verts: Vec::new(), scale: 2.0, width: 1.0, height: 1.0, advance: font.advance }
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

    /// A small 3D-looking cube (or a flat sprite for plants), like inventory icons.
    pub fn block_icon(&mut self, reg: &BlockRegistry, id: BlockId, x: f32, y: f32, size: f32) {
        let b = reg.get(id);
        let full = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        if matches!(b.shape, Shape::Cross | Shape::Torch) {
            let l = b.faces[0] as i32;
            self.quad([[x, y], [x + size, y], [x + size, y + size], [x, y + size]], full, WHITE, l);
            return;
        }
        let cx = x + size / 2.0;
        let w = size * 0.5;
        let h = size * 0.25;
        let y0 = y + size * 0.02;
        let side = size * 0.58;
        let t = [cx, y0];
        let r = [cx + w, y0 + h];
        let c = [cx, y0 + 2.0 * h];
        let l = [cx - w, y0 + h];
        let down = |p: [f32; 2]| [p[0], p[1] + side];
        // Left face (front texture), right face (side texture), then the top.
        self.quad([l, c, down(c), down(l)], full, [205, 205, 205, 255], b.faces[4] as i32);
        self.quad([c, r, down(r), down(c)], full, [150, 150, 150, 255], b.faces[0] as i32);
        self.quad([t, r, c, l], [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]], WHITE, b.faces[2] as i32);
    }

    /// A clickable button; returns true when the mouse is over it.
    pub fn button(&mut self, x: f32, y: f32, w: f32, h: f32, label: &str, mouse: (f32, f32)) -> bool {
        let hover = mouse.0 >= x && mouse.0 < x + w && mouse.1 >= y && mouse.1 < y + h;
        let s = self.scale;
        self.rect(x, y, w, h, if hover { [40, 40, 40, 255] } else { [0, 0, 0, 255] });
        self.rect(x + s, y + s, w - 2.0 * s, h - 2.0 * s, if hover { [126, 136, 191, 255] } else { [111, 111, 111, 255] });
        self.rect(x + s, y + s, w - 2.0 * s, s, if hover { [168, 178, 230, 255] } else { [160, 160, 160, 255] });
        self.rect(x + s, y + h - 2.0 * s, w - 2.0 * s, s, if hover { [90, 100, 150, 255] } else { [70, 70, 70, 255] });
        self.text_centered(x + w / 2.0, y + (h - 8.0 * s) / 2.0, label, if hover { YELLOW } else { [224, 224, 224, 255] });
        hover
    }
}
