//! The shield as a real 3D model on the player: a board of planks inside an iron rim with a
//! boss, a strap and a grip behind. It is carried on the arm at the side (face out, as in the
//! classic game), comes up in front of the body while blocking, and is slung on the back while
//! both hands are busy (an instrument, a two-handed weapon). In first person it sits at the
//! bottom corner of the view and rises to cover it while the use button is held.
//!
//! A shield takes a banner's colour when crafted together with one, and a dye then paints a
//! design over it (fess, pale, cross, saltire...); each further dye moves to the next design.
//! Both ride in the stack's spare socket bits. The Terraria shields worn as accessories show
//! on the arm with their own looks (cobalt, obsidian, ankh, the eye shields).
//!
//! Our own art, painted here in code into a skin of its own (16x16 swatches like
//! instruments.rs). The banner colour and the design are separate meshes drawn with the
//! entity shader's dye tint, so one mesh serves all sixteen colours.

use glam::{Mat4, Quat, Vec3};

use crate::entity::Bones;
use crate::instruments::{IBox, Piece};
use crate::Game;

/// Shield looks, in mesh order.
pub const KINDS: [&str; 6] = ["shield", "bastion_shield", "guardian_shield", "ankh_shield", "charge_shield", "watcher_shield"];
/// The designs a dye paints over a coloured shield (index 1.. in `Look::pattern`).
pub const PATTERNS: [&str; 12] = ["Faixa", "Pala", "Cruz", "Aspa", "Chefe", "Bordadura", "Esquartelado", "Asna", "Besante", "Partido", "Talhado", "Banda"];
/// Dye colours, in the classic order (the same numbers as the dye vat's cloth).
pub const COLOURS: [&str; 16] = ["white", "orange", "magenta", "light_blue", "yellow", "lime", "pink", "gray", "light_gray", "cyan", "purple", "blue", "brown", "green", "red", "black"];
const COLOUR_NAMES: [&str; 16] = ["Branco", "Laranja", "Magenta", "Azul-claro", "Amarelo", "Lima", "Rosa", "Cinza", "Cinza-claro", "Ciano", "Roxo", "Azul", "Marrom", "Verde", "Vermelho", "Preto"];
const RGB: [[u8; 3]; 16] = [
    [233, 236, 236],
    [240, 118, 19],
    [189, 68, 179],
    [58, 175, 217],
    [248, 198, 39],
    [112, 185, 25],
    [237, 141, 172],
    [62, 68, 71],
    [142, 142, 134],
    [21, 137, 145],
    [121, 42, 172],
    [53, 57, 157],
    [114, 71, 40],
    [84, 109, 27],
    [160, 39, 34],
    [20, 21, 25],
];

/// Meshes: the frames of each kind, then the cloth face, then one per design.
pub const MESHES: usize = KINDS.len() + 1 + PATTERNS.len();

/// What a shield looks like.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Look {
    /// Index into KINDS.
    pub kind: u8,
    /// Banner colour + 1 (0 = bare planks).
    pub base: u8,
    /// Design + 1 (0 = none), and its colour.
    pub pattern: u8,
    pub pcol: u8,
}

// The stack's socket number: gems use bits 0..5 and 8..20 (voxel::gems); the coat of arms
// takes bit 7 (coloured), the banner colour in bits 20..24, the design in 24..28 and its
// colour in 28..32.
const COLOURED: u32 = 1 << 7;
const BASE_SHIFT: u32 = 20;
const PATTERN_SHIFT: u32 = 24;
const PCOL_SHIFT: u32 = 28;
const ARMS: u32 = COLOURED | !((1 << BASE_SHIFT) - 1);

impl Look {
    /// The look of an item, by name and socket number; None if it is not a shield.
    pub fn of(name: &str, sockets: u32) -> Option<Look> {
        let kind = KINDS.iter().position(|k| *k == name)? as u8;
        if kind != 0 || sockets & COLOURED == 0 {
            return Some(Look { kind, ..Default::default() });
        }
        let pattern = (((sockets >> PATTERN_SHIFT) & 15) as u8).min(PATTERNS.len() as u8);
        Some(Look { kind, base: ((sockets >> BASE_SHIFT) & 15) as u8 + 1, pattern, pcol: ((sockets >> PCOL_SHIFT) & 15) as u8 })
    }

    /// These coat of arms bits merged into a stack's socket number.
    pub fn into_sockets(self, sockets: u32) -> u32 {
        let keep = sockets & !ARMS;
        if self.base == 0 {
            return keep;
        }
        keep | COLOURED | ((self.base as u32 - 1) & 15) << BASE_SHIFT | (self.pattern as u32 & 15) << PATTERN_SHIFT | (self.pcol as u32 & 15) << PCOL_SHIFT
    }

    /// Packed small for the network (multiplayer guests see each other's shields).
    pub fn pack(self) -> u32 {
        (self.kind as u32 + 1) | (self.base as u32) << 4 | (self.pattern as u32) << 9 | (self.pcol as u32) << 13
    }

    pub fn unpack(v: u32) -> Option<Look> {
        let kind = (v & 15) as u8;
        (kind > 0 && (kind as usize) <= KINDS.len()).then(|| Look {
            kind: kind - 1,
            base: ((v >> 4) & 31).min(16) as u8,
            pattern: ((v >> 9) & 15).min(PATTERNS.len() as u32) as u8,
            pcol: ((v >> 13) & 15) as u8,
        })
    }

    /// The tooltip line: "Brasão: Vermelho, Cruz Branco".
    pub fn describe(self) -> Option<String> {
        if self.base == 0 {
            return None;
        }
        let mut s = format!("Brasão: {}", COLOUR_NAMES[(self.base - 1) as usize]);
        if self.pattern > 0 {
            s += &format!(", {} {}", PATTERNS[(self.pattern - 1) as usize], COLOUR_NAMES[self.pcol as usize]);
        }
        Some(s)
    }
}

/// The colour of dye `i` (0..16), 0..1.
pub fn rgb(i: u8) -> [f32; 3] {
    let c = RGB[(i as usize).min(15)];
    [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0]
}

/// Dye colour index of a banner or dye item name.
pub fn colour_of(name: &str, suffix: &str) -> Option<u8> {
    let c = name.strip_suffix(suffix)?;
    COLOURS.iter().position(|k| *k == c).map(|i| i as u8)
}

// ---------------------------------------------------------------------------------------------
// The models. Model pixels: the board stands in the XY plane with its face toward +Z, the grip
// is at the origin behind it (the hand is at -Z).

const OAK: u8 = 0;
const IRON: u8 = 1;
const BOSS: u8 = 2;
const LEATHER: u8 = 3;
const GRIP: u8 = 4;
const CLOTH: u8 = 5;
const COBALT: u8 = 6;
const COBALT_DARK: u8 = 7;
const OBSIDIAN: u8 = 8;
const AMETHYST: u8 = 9;
const GOLD: u8 = 10;
const CRIMSON: u8 = 11;
const FLESH: u8 = 12;
const EYE: u8 = 13;
const IRIS_RED: u8 = 14;
const PUPIL: u8 = 15;
const BONE: u8 = 16;
const IRIS_TEAL: u8 = 17;
const SWATCHES: u8 = 18;

/// The rim's width.
const RIM: f32 = 0.8;
/// Board front, cloth front, design front.
const FACE_Z: f32 = 1.2;
const CLOTH_Z: f32 = 1.32;
const DESIGN_Z: f32 = 1.42;
/// The heater shield's top and bottom.
const TOP: f32 = 10.0;
const BOTTOM: f32 = -10.0;

fn bx(lo: [f32; 3], hi: [f32; 3], mat: u8) -> IBox {
    IBox { lo: Vec3::from(lo), hi: Vec3::from(hi), mat }
}

/// Half the width of a shape at height `y`.
#[derive(Clone, Copy)]
enum Shape {
    /// Straight sides, rounding in to a point below.
    Heater,
    /// Long point below (the cobalt kite).
    Kite,
    /// A disc of this radius.
    Round(f32),
}

impl Shape {
    fn span(self) -> (f32, f32) {
        match self {
            Shape::Heater => (BOTTOM, TOP),
            Shape::Kite => (-12.0, 9.0),
            Shape::Round(r) => (-r, r),
        }
    }

    fn half(self, y: f32) -> f32 {
        match self {
            Shape::Heater => {
                if y >= -3.0 {
                    6.0
                } else {
                    let t = ((-3.0 - y) / (-3.0 - BOTTOM)).clamp(0.0, 1.0);
                    6.0 * (1.0 - t * t).max(0.0).sqrt()
                }
            }
            Shape::Kite => {
                if y >= 2.0 {
                    5.5
                } else {
                    5.5 * ((y + 12.0) / 14.0).clamp(0.0, 1.0)
                }
            }
            Shape::Round(r) => (r * r - y * y).max(0.0).sqrt(),
        }
    }

    /// Rows of the shape `step` tall: (bottom, top, half width), rounded to half pixels so the
    /// edge steps like the rest of the blocky world. Rows of the same width are merged.
    fn rows(self, step: f32) -> Vec<(f32, f32, f32)> {
        let (lo, hi) = self.span();
        let mut out: Vec<(f32, f32, f32)> = Vec::new();
        let mut y = lo;
        while y < hi - 1e-3 {
            let y1 = (y + step).min(hi);
            let w = (self.half((y + y1) / 2.0) * 2.0).round() / 2.0;
            match out.last_mut() {
                Some(last) if last.2 == w && (last.1 - y).abs() < 1e-4 => last.1 = y1,
                _ => out.push((y, y1, w)),
            }
            y = y1;
        }
        out.retain(|r| r.2 > 0.0);
        out
    }

    /// The rows split where the rim's top and bottom bands end, each marked whether it lies
    /// within them (all rim) or between (face with rim at the sides).
    fn banded(self) -> Vec<(f32, f32, f32, bool)> {
        let (lo, hi) = self.span();
        let (a, b) = (lo + RIM, hi - RIM);
        let mut out = Vec::new();
        for (y0, y1, w) in self.rows(1.0) {
            let mut cuts = vec![y0];
            cuts.extend([a, b].into_iter().filter(|&c| c > y0 + 1e-3 && c < y1 - 1e-3));
            cuts.push(y1);
            for p in cuts.windows(2) {
                let mid = (p[0] + p[1]) / 2.0;
                out.push((p[0], p[1], w, mid < a || mid > b || w <= RIM * 2.0 + 0.2));
            }
        }
        out
    }

    /// Inside the face, within the rim.
    fn inside(self, x: f32, y: f32) -> bool {
        let (lo, hi) = self.span();
        y > lo + RIM && y < hi - RIM && x.abs() < self.half(y) - RIM
    }
}

/// The board: face rows inside a rim that stands a little proud of it front and back.
fn board(shape: Shape, face: u8, rim: u8, out: &mut Vec<IBox>) {
    for (y0, y1, w, whole) in shape.banded() {
        if whole {
            out.push(bx([-w, y0, -0.3], [w, y1, FACE_Z + 0.35], rim));
            continue;
        }
        out.push(bx([-w + RIM, y0, 0.0], [w - RIM, y1, FACE_Z], face));
        out.push(bx([-w, y0, -0.3], [-w + RIM, y1, FACE_Z + 0.35], rim));
        out.push(bx([w - RIM, y0, -0.3], [w, y1, FACE_Z + 0.35], rim));
    }
}

/// A stepped disc standing out of the face from `z0` to `z1`.
fn disc(cx: f32, cy: f32, r: f32, z0: f32, z1: f32, mat: u8, out: &mut Vec<IBox>) {
    for (y0, y1, w) in Shape::Round(r).rows(0.5) {
        out.push(bx([cx - w, cy + y0, z0], [cx + w, cy + y1, z1], mat));
    }
}

/// The grip and straps behind the board.
fn back(out: &mut Vec<IBox>) {
    out.extend([
        // The grip, across the board, held off it on two iron posts.
        bx([-2.6, -0.7, -1.9], [2.6, 0.7, -1.1], GRIP),
        bx([-2.6, -0.8, -1.1], [-1.8, 0.8, -0.3], IRON),
        bx([1.8, -0.8, -1.1], [2.6, 0.8, -0.3], IRON),
        // The arm strap higher up, the forearm goes through it.
        bx([-3.6, 4.0, -0.7], [3.6, 5.4, -0.3], LEATHER),
        bx([-3.6, 4.0, -1.6], [-2.8, 5.4, -0.7], LEATHER),
        bx([2.8, 4.0, -1.6], [3.6, 5.4, -0.7], LEATHER),
        bx([-2.8, 4.0, -1.9], [2.8, 5.4, -1.6], LEATHER),
    ]);
}

/// Frame `k` (KINDS): everything that is not the banner's cloth.
pub fn frame(k: usize) -> Vec<Piece> {
    let mut b = Vec::new();
    match k {
        0 => {
            board(Shape::Heater, OAK, IRON, &mut b);
            // A round iron boss in the middle, riveted.
            disc(0.0, 0.5, 2.2, FACE_Z - 0.2, FACE_Z + 0.75, BOSS, &mut b);
            disc(0.0, 0.5, 1.0, FACE_Z + 0.75, FACE_Z + 1.2, IRON, &mut b);
        }
        1 => {
            // Cobalt kite with a raised cross ridge.
            board(Shape::Kite, COBALT, COBALT_DARK, &mut b);
            b.push(bx([-0.6, -10.0, FACE_Z - 0.1], [0.6, 8.0, FACE_Z + 0.5], COBALT_DARK));
            b.push(bx([-4.6, 3.4, FACE_Z - 0.1], [4.6, 4.6, FACE_Z + 0.5], COBALT_DARK));
            disc(0.0, 4.0, 1.3, FACE_Z + 0.5, FACE_Z + 0.9, COBALT, &mut b);
        }
        2 => {
            // Black obsidian set in dark iron, a violet gem in the middle.
            board(Shape::Heater, OBSIDIAN, BOSS, &mut b);
            for (i, w) in [0.6f32, 1.4, 2.0, 1.4, 0.6].iter().enumerate() {
                let y = 0.5 + (i as f32 - 2.0) * 0.9;
                b.push(bx([-w, y - 0.45, FACE_Z - 0.1], [*w, y + 0.45, FACE_Z + 0.7], AMETHYST));
            }
            b.push(bx([-4.5, 7.4, FACE_Z - 0.1], [4.5, 8.2, FACE_Z + 0.3], BOSS));
        }
        3 => {
            // Crimson field in a gold rim with a gold ankh.
            board(Shape::Heater, CRIMSON, GOLD, &mut b);
            let z0 = FACE_Z - 0.1;
            let z1 = FACE_Z + 0.5;
            // Loop.
            b.push(bx([-2.2, 3.0, z0], [-1.2, 7.0, z1], GOLD));
            b.push(bx([1.2, 3.0, z0], [2.2, 7.0, z1], GOLD));
            b.push(bx([-1.4, 6.6, z0], [1.4, 7.6, z1], GOLD));
            // Bar and stem.
            b.push(bx([-3.8, 1.6, z0], [3.8, 2.8, z1], GOLD));
            b.push(bx([-0.7, -7.0, z0], [0.7, 1.6, z1], GOLD));
            b.push(bx([-1.4, -7.6, z0], [1.4, -6.6, z1], GOLD));
        }
        4 | 5 => {
            // Round eye shields: flesh (the charging shield) or bone (the watcher's).
            let (face, rim, iris) = if k == 4 { (FLESH, CRIMSON, IRIS_RED) } else { (BONE, BOSS, IRIS_TEAL) };
            board(Shape::Round(7.5), face, rim, &mut b);
            disc(0.0, 0.0, 4.2, FACE_Z - 0.1, FACE_Z + 0.6, EYE, &mut b);
            disc(0.0, 0.0, 2.4, FACE_Z + 0.6, FACE_Z + 0.9, iris, &mut b);
            disc(0.0, 0.0, 1.1, FACE_Z + 0.9, FACE_Z + 1.05, PUPIL, &mut b);
        }
        _ => {}
    }
    back(&mut b);
    vec![Piece { part: 0, place: Mat4::IDENTITY, boxes: b }]
}

/// The banner's cloth over the plank face of the classic shield (drawn tinted).
pub fn cloth() -> Vec<Piece> {
    let mut b = Vec::new();
    for (y0, y1, w, whole) in Shape::Heater.banded() {
        if whole {
            continue;
        }
        b.push(bx([-w + RIM, y0, FACE_Z - 0.05], [w - RIM, y1, CLOTH_Z], CLOTH));
    }
    vec![Piece { part: 0, place: Mat4::IDENTITY, boxes: b }]
}

/// Middle of the field, where crosses meet.
const MID: f32 = 1.0;

/// Whether design `p` (0-based into PATTERNS) covers the point (x, y) of the face.
pub fn covers(p: usize, x: f32, y: f32) -> bool {
    let (ax, dy) = (x.abs(), y - MID);
    let shape = Shape::Heater;
    match p {
        0 => dy.abs() < 1.8,
        1 => ax < 1.6,
        2 => dy.abs() < 1.4 || ax < 1.4,
        3 => (ax - dy.abs() * 0.62).abs() < 1.0,
        4 => y > 5.2,
        5 => {
            let (lo, hi) = shape.span();
            shape.half(y) - RIM - ax < 1.3 || y > hi - RIM - 1.3 || y < lo + RIM + 2.0
        }
        6 => (x < 0.0) == (dy > 0.0),
        7 => {
            let ridge = MID + 3.5 - ax * 0.9;
            y < ridge && y > ridge - 2.6
        }
        8 => x * x + dy * dy < 3.4 * 3.4,
        9 => x < 0.0,
        10 => dy > x * 1.3,
        11 => (dy - x * 1.3).abs() < 1.7,
        _ => false,
    }
}

/// Design `p`, as runs of half-pixel cells over the face (drawn tinted).
pub fn design(p: usize) -> Vec<Piece> {
    const C: f32 = 0.5;
    let shape = Shape::Heater;
    let mut b = Vec::new();
    let mut y = BOTTOM;
    while y < TOP {
        let cy = y + C / 2.0;
        let mut run: Option<f32> = None;
        let mut x = -6.0;
        while x <= 6.0 {
            let cx = x + C / 2.0;
            let on = x < 6.0 && shape.inside(cx, cy) && shape.inside(x + 0.01, cy) && shape.inside(x + C - 0.01, cy) && covers(p, cx, cy);
            match (on, run) {
                (true, None) => run = Some(x),
                (false, Some(x0)) => {
                    b.push(bx([x0, y, CLOTH_Z - 0.04], [x, y + C, DESIGN_Z], CLOTH));
                    run = None;
                }
                _ => {}
            }
            x += C;
        }
        y += C;
    }
    vec![Piece { part: 0, place: Mat4::IDENTITY, boxes: b }]
}

/// All the meshes in order (see MESHES).
pub fn models() -> Vec<Vec<Piece>> {
    let mut v: Vec<Vec<Piece>> = (0..KINDS.len()).map(frame).collect();
    v.push(cloth());
    v.extend((0..PATTERNS.len()).map(design));
    v
}

/// Paint the shield skin (64 x 128 model pixels, `scale` texels each).
pub fn paint_skin(scale: u32) -> Vec<u8> {
    let (w, h) = (64 * scale, 128 * scale);
    let mut px = vec![0u8; (w * h * 4) as usize];
    let n = 16 * scale;
    let hash = |x: u32, y: u32, s: u32| {
        let mut v = x.wrapping_mul(0x9E37_79B1) ^ y.wrapping_mul(0x85EB_CA77) ^ s.wrapping_mul(0x5BD1_E995);
        v ^= v >> 15;
        v = v.wrapping_mul(0x2C1B_3C6D);
        v ^= v >> 12;
        (v & 0xffff) as f32 / 65535.0
    };
    let mix = |a: [f32; 3], b: [f32; 3], t: f32| [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
    let metal = |r: f32, u: f32, v: f32, dark: [f32; 3], light: [f32; 3]| {
        let sheen = ((u * 0.5 + v * 0.3).sin() * 0.5 + 0.5) * 0.35;
        mix(dark, light, (r * 0.4 + sheen).min(1.0))
    };
    for m in 0..SWATCHES as u32 {
        let (ox, oy) = ((m % 4) * n, (m / 4) * n);
        for y in 0..n {
            for x in 0..n {
                // u runs along a box's longer side, v across it; both 0..16 model pixels.
                let (u, v) = (x as f32 / scale as f32, y as f32 / scale as f32);
                let r = hash(x, y, m);
                let c: [f32; 3] = match m as u8 {
                    OAK => {
                        // Boards side by side with dark seams, grain running along them.
                        let plank = (u / 3.0).floor();
                        let seam = (u / 3.0).fract() < 0.1;
                        let tone = hash(plank as u32, 7, 3) * 0.25;
                        let grain = ((v * 1.3 + (u * 2.1 + plank).sin() * 0.8).sin() * 0.5 + 0.5).powf(3.0);
                        if seam {
                            [74.0, 52.0, 30.0]
                        } else {
                            mix(mix([170.0, 128.0, 76.0], [140.0, 102.0, 58.0], tone * 2.0), [112.0, 80.0, 44.0], grain * 0.5 + r * 0.15)
                        }
                    }
                    IRON => {
                        let rivet = (u - 1.5).rem_euclid(4.0) < 0.9 && (v % 16.0) < 0.9;
                        if rivet { [196.0, 196.0, 190.0] } else { metal(r, u, v, [96.0, 98.0, 104.0], [158.0, 160.0, 164.0]) }
                    }
                    BOSS => metal(r, u, v, [58.0, 60.0, 66.0], [112.0, 114.0, 120.0]),
                    LEATHER => {
                        let stitch = (v - 0.5).abs() < 0.12 && (u * 2.0).fract() < 0.5;
                        if stitch { [176.0, 150.0, 108.0] } else { mix([104.0, 66.0, 38.0], [78.0, 46.0, 26.0], r * 0.6) }
                    }
                    GRIP => {
                        // Leather wound round the grip.
                        let wind = ((u + v * 0.6) * 2.4).sin() > 0.6;
                        if wind { [62.0, 40.0, 24.0] } else { mix([96.0, 64.0, 38.0], [82.0, 54.0, 30.0], r) }
                    }
                    CLOTH => {
                        // A woven cloth, light so the dye shows true.
                        let weave = ((u * 6.0).sin() * (v * 6.0).sin()) * 0.5 + 0.5;
                        let k = 168.0 + weave * 26.0 + r * 18.0;
                        [k, k, k]
                    }
                    COBALT => metal(r, u, v, [34.0, 74.0, 168.0], [86.0, 142.0, 226.0]),
                    COBALT_DARK => metal(r, u, v, [20.0, 40.0, 104.0], [46.0, 84.0, 160.0]),
                    OBSIDIAN => {
                        let fleck = r > 0.93;
                        if fleck { [110.0, 74.0, 156.0] } else { mix([22.0, 16.0, 34.0], [44.0, 32.0, 66.0], r * 0.5 + ((u * 0.7 - v).sin() * 0.5 + 0.5) * 0.4) }
                    }
                    AMETHYST => {
                        let facet = ((u + v) * 0.9).sin() > 0.3;
                        mix([140.0, 70.0, 210.0], if facet { [226.0, 170.0, 255.0] } else { [98.0, 40.0, 160.0] }, 0.5 + r * 0.2)
                    }
                    GOLD => metal(r, u, v, [178.0, 128.0, 30.0], [252.0, 214.0, 90.0]),
                    CRIMSON => mix([120.0, 22.0, 30.0], [84.0, 12.0, 20.0], r * 0.5 + ((u * 0.5 + v * 0.8).sin() * 0.5 + 0.5) * 0.3),
                    FLESH => {
                        // Veined flesh.
                        let vein = ((u * 1.1 + (v * 0.7).sin() * 2.0).sin() * (v * 0.9 + (u * 0.5).cos()).sin()).abs() < 0.06;
                        if vein { [120.0, 20.0, 30.0] } else { mix([196.0, 92.0, 92.0], [164.0, 66.0, 70.0], r * 0.6) }
                    }
                    EYE => {
                        let vein = ((u * 2.0 + v).sin() * (v * 1.7 - u * 0.3).cos()).abs() < 0.05;
                        if vein { [206.0, 90.0, 90.0] } else { mix([238.0, 232.0, 222.0], [212.0, 204.0, 194.0], r * 0.5) }
                    }
                    IRIS_RED => mix([170.0, 30.0, 26.0], [226.0, 70.0, 40.0], r * 0.5 + ((u * 3.0).sin() * 0.5 + 0.5) * 0.3),
                    PUPIL => mix([14.0, 10.0, 12.0], [34.0, 26.0, 30.0], r),
                    BONE => mix([226.0, 216.0, 186.0], [188.0, 176.0, 146.0], r * 0.5 + ((v * 1.4).sin() * 0.5 + 0.5) * 0.3),
                    _ => mix([30.0, 156.0, 150.0], [90.0, 222.0, 206.0], r * 0.5 + ((u * 3.0).sin() * 0.5 + 0.5) * 0.3),
                };
                let i = (((oy + y) * w + ox + x) * 4) as usize;
                px[i] = c[0].clamp(0.0, 255.0) as u8;
                px[i + 1] = c[1].clamp(0.0, 255.0) as u8;
                px[i + 2] = c[2].clamp(0.0, 255.0) as u8;
                px[i + 3] = 255;
            }
        }
    }
    px
}

// ---------------------------------------------------------------------------------------------
// Where it is held.

/// Where a shield is carried.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mount {
    /// In a hand (left = the off hand).
    Hand { left: bool },
    /// Slung on the back.
    Back,
}

impl Mount {
    fn code(self) -> u32 {
        match self {
            Mount::Hand { left: true } => 0,
            Mount::Hand { left: false } => 1,
            Mount::Back => 2,
        }
    }

    fn from_code(c: u32) -> Mount {
        match c {
            1 => Mount::Hand { left: false },
            2 => Mount::Back,
            _ => Mount::Hand { left: true },
        }
    }
}

/// A worn shield for the network: look, where, and whether it is up (0 = none).
pub fn net_pack(worn: &[(Look, Mount)], raised: bool) -> u32 {
    match worn.first() {
        Some((l, m)) => l.pack() | m.code() << 17 | (raised as u32) << 19,
        None => 0,
    }
}

pub fn net_unpack(v: u32) -> Option<(Look, Mount, bool)> {
    let look = Look::unpack(v & 0x1_FFFF)?;
    Some((look, Mount::from_code((v >> 17) & 3), v >> 19 & 1 == 1))
}

/// The shield on the arm bone `arm` (shoulder at `pivot`) of a model whose body bone is `body`:
/// at the side with its face out, turning to face ahead in front of the fist as `raise` goes
/// to 1 (blocking). Model pixels to the bones' space.
pub fn on_arm(arm: Mat4, body: Mat4, pivot: Vec3, left: bool, raise: f32) -> Mat4 {
    let side = if left { 1.0 } else { -1.0 };
    // Just outside the forearm, the grip in the fist.
    let anchor = pivot + Vec3::new(side * 2.3, -8.0, 0.0);
    let idle = Quat::from_rotation_y(side * std::f32::consts::FRAC_PI_2);
    if raise <= 0.0 {
        return arm * Mat4::from_translation(anchor) * Mat4::from_quat(idle);
    }
    // Blending in the body's space: the arm's own turn toward a board square to the front.
    let rel = body.inverse() * arm;
    let (_, arm_rot, _) = rel.to_scale_rotation_translation();
    let up = Quat::from_rotation_y(-side * 0.3);
    let r = (arm_rot * idle).slerp(up, raise.clamp(0.0, 1.0));
    let fist = rel.transform_point3(pivot + Vec3::new(side * 2.3 * (1.0 - raise), -9.5, 0.0));
    body * Mat4::from_translation(fist + Vec3::new(-side * 1.0, 1.0, 2.2) * raise) * Mat4::from_quat(r)
}

/// The shield slung on the back of body bone `body`, face out, tipped a little.
pub fn on_back(body: Mat4) -> Mat4 {
    body * Mat4::from_translation(Vec3::new(0.0, 17.0, -2.9)) * Mat4::from_rotation_y(std::f32::consts::PI) * Mat4::from_rotation_z(0.18) * Mat4::from_scale(Vec3::splat(0.9))
}

/// First person: at the bottom corner of the view on its side (left for the off hand),
/// rising toward the middle as `raise` goes to 1. `eye` relative to the camera's block.
pub fn first_person(eye: Vec3, yaw: f32, pitch: f32, left: bool, raise: f32, sway: Vec3) -> Mat4 {
    let side = if left { -1.0 } else { 1.0 };
    let cam = Mat4::from_translation(eye) * Mat4::from_rotation_y(yaw) * Mat4::from_rotation_x(pitch);
    let k = raise.clamp(0.0, 1.0);
    let k = k * k * (3.0 - 2.0 * k);
    let at = Vec3::new(side * 0.56, -0.42, -0.86).lerp(Vec3::new(side * 0.24, -0.24, -0.66), k);
    // Turned to show its face toward the middle of the view, squarer while blocking.
    let idle = Quat::from_rotation_y(-side * 0.85) * Quat::from_rotation_x(-0.1);
    let up = Quat::from_rotation_y(-side * 0.22);
    cam * Mat4::from_translation(at + sway) * Mat4::from_quat(idle.slerp(up, k)) * Mat4::from_scale(Vec3::splat(0.5 / 16.0))
}

/// The instances that draw `look` placed by `place`: the frame, and for a coloured shield the
/// cloth and the design, each tinted. Mesh numbers are relative to the first shield mesh.
pub fn parts(look: Look, place: Mat4, light: [f32; 4]) -> Vec<(usize, Bones)> {
    let mut b = Bones { light, ..Default::default() };
    b.m[0] = place.to_cols_array_2d();
    let mut out = vec![(look.kind as usize, b)];
    if look.kind == 0 && look.base > 0 {
        let c = rgb(look.base - 1);
        out.push((KINDS.len(), Bones { tint: [c[0], c[1], c[2], 1.0], ..b }));
        if look.pattern > 0 {
            let c = rgb(look.pcol);
            out.push((KINDS.len() + look.pattern as usize, Bones { tint: [c[0], c[1], c[2], 1.0], ..b }));
        }
    }
    out
}

impl Game {
    /// Both hands are taken (an instrument, a two-handed weapon): a shield goes on the back.
    pub(crate) fn hands_full(&self) -> bool {
        self.held_instrument().is_some() || self.held_weapon().is_some_and(|w| w.two_handed)
    }

    /// The shields the player shows, and where: one in the main hand, the off hand's (or a
    /// Terraria shield worn as an accessory) on the left arm, either on the back while both
    /// hands are busy.
    pub(crate) fn shields_worn(&self) -> Vec<(Look, Mount)> {
        let mut out = Vec::new();
        let look = |st: &voxel::item::ItemStack| Look::of(&self.items.get(st.item).name, st.sockets);
        if let Some(l) = self.held_stack().and_then(|s| look(&s)) {
            out.push((l, Mount::Hand { left: false }));
        }
        let off = self.surv.offhand.and_then(|s| look(&s)).or_else(|| {
            if self.surv.offhand.is_some() {
                return None;
            }
            // Accessories: the first shield worn.
            self.t5.acc[..self.acc_slots()].iter().flatten().find_map(|s| look(s).filter(|l| l.kind != 0))
        });
        if let Some(l) = off {
            out.push((l, if self.hands_full() { Mount::Back } else { Mount::Hand { left: true } }));
        }
        out
    }

    /// 0..1, how far the shield is up (eased, so it rises rather than snaps).
    pub(crate) fn update_shield_raise(&mut self, dt: f32) {
        let target = if self.extras.blocking || std::env::var("MC_BLOCK").is_ok() { 1.0 } else { 0.0 };
        if self.args.screenshot.is_some() {
            self.extras.raise = target;
            return;
        }
        self.extras.raise += (target - self.extras.raise) * (dt * 14.0).min(1.0);
    }

    /// Shield instances for a player model's `bones` (third person, the inventory figure,
    /// guests): `raise` lifts the one in hand.
    pub(crate) fn shield_instances(&self, worn: &[(Look, Mount)], bones: &Bones, pivots: (Vec3, Vec3), raise: f32) -> Vec<(usize, Bones)> {
        let Some(first) = self.entities.shield_mesh(0) else { return Vec::new() };
        let body = Mat4::from_cols_array_2d(&bones.m[1]);
        let mut out = Vec::new();
        for &(look, mount) in worn {
            let place = match mount {
                Mount::Back => on_back(body),
                Mount::Hand { left } => {
                    let (i, pv) = if left { (3, pivots.1) } else { (2, pivots.0) };
                    // Only the classic shield blocks; Terraria shields stay at the side.
                    let up = if look.kind == 0 { raise } else { 0.0 };
                    on_arm(Mat4::from_cols_array_2d(&bones.forearm(i)), body, pv, left, up)
                }
            };
            out.extend(parts(look, place, bones.light).into_iter().map(|(m, b)| (first + m, b)));
        }
        out
    }

    /// The shields in view in first person (one held in either hand; accessories and one on
    /// the back don't show), and whether the main hand holds one.
    pub(crate) fn shield_first_person(&self, eye: Vec3, light: [f32; 4], sway: Vec3) -> (Vec<(usize, Bones)>, bool) {
        let Some(first) = self.entities.shield_mesh(0) else { return (Vec::new(), false) };
        let look = |st: Option<voxel::item::ItemStack>| st.and_then(|s| Look::of(&self.items.get(s.item).name, s.sockets));
        let main = look(self.held_stack());
        let off = look(self.surv.offhand).filter(|_| !self.hands_full());
        let mut out = Vec::new();
        for (l, left) in [(main, false), (off, true)] {
            let Some(l) = l else { continue };
            // Only the classic shield blocks.
            let raise = if l.kind == 0 && (!left || main.is_none()) { self.extras.raise } else { 0.0 };
            let place = first_person(eye, self.yaw, self.pitch, left, raise, sway);
            out.extend(parts(l, place, light).into_iter().map(|(m, b)| (first + m, b)));
        }
        (out, main.is_some())
    }

    /// The model's shoulder pivots, right then left arm (model pixels).
    pub(crate) fn arm_pivots(def: &voxel::mob::MobDef) -> (Vec3, Vec3) {
        let pv = |i: usize| def.parts.get(i).map_or(Vec3::ZERO, |p| Vec3::new(p.pivot.0 as f32, p.pivot.1 as f32, p.pivot.2 as f32));
        (pv(2), pv(3))
    }

    /// Test hook (MC_SHIELD): `item[:colour[:design:colour]]` into the off hand or main hand.
    pub(crate) fn test_shield(&mut self, spec: &str, main: bool) {
        let mut it = spec.split(':');
        let Some(id) = it.next().and_then(|n| self.items.id(n)) else { return };
        let colour = |s: Option<&str>| s.and_then(|c| COLOURS.iter().position(|k| *k == c)).map(|c| c as u8);
        let mut look = Look::default();
        if let Some(c) = colour(it.next()) {
            look.base = c + 1;
            if let (Some(p), Some(c)) = (it.next().and_then(|p| p.parse::<u8>().ok()), colour(it.next())) {
                look.pattern = p.min(PATTERNS.len() as u8);
                look.pcol = c;
            }
        }
        let st = voxel::item::ItemStack { sockets: look.into_sockets(0), ..voxel::item::ItemStack::new(id, 1) };
        if main {
            self.surv.inv.slots[self.selected] = Some(st);
        } else {
            self.surv.offhand = Some(st);
        }
    }

    /// Crafting: a shield with a banner takes the banner's colour; a coloured shield with a dye
    /// gets the next design in that dye's colour. None when the grid is anything else.
    pub(crate) fn shield_craft(&self, grid: &[Option<voxel::item::ItemStack>]) -> Option<voxel::item::ItemStack> {
        let full: Vec<&voxel::item::ItemStack> = grid.iter().flatten().collect();
        if full.len() != 2 {
            return None;
        }
        let name = |s: &voxel::item::ItemStack| self.items.get(s.item).name.clone();
        let shield = full.iter().find(|s| name(s) == "shield")?;
        let other = full.iter().find(|s| name(s) != "shield")?;
        let mut look = Look::of("shield", shield.sockets)?;
        let on = name(other);
        if let Some(c) = colour_of(&on, "_banner") {
            look = Look { base: c + 1, pattern: 0, pcol: 0, ..look };
        } else if let (Some(c), true) = (colour_of(&on, "_dye"), look.base > 0) {
            look.pattern = look.pattern % PATTERNS.len() as u8 + 1;
            look.pcol = c;
        } else {
            return None;
        }
        Some(voxel::item::ItemStack { count: 1, sockets: look.into_sockets(shield.sockets), ..**shield })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coat_of_arms_round_trips_and_leaves_gems_alone() {
        let gems = 0b1010_1010_1010_1010_1010 & !(1 << 7);
        for base in 0..=16u8 {
            for pattern in [0u8, 1, 7, 12] {
                for pcol in [0u8, 9, 15] {
                    let l = Look { kind: 0, base, pattern: if base == 0 { 0 } else { pattern }, pcol: if base == 0 || pattern == 0 { 0 } else { pcol } };
                    let s = l.into_sockets(gems);
                    assert_eq!(s & ((1 << 20) - 1) & !(1 << 7), gems);
                    assert_eq!(Look::of("shield", s), Some(l), "{l:?}");
                    assert_eq!(Look::unpack(l.pack()), Some(l));
                }
            }
        }
        // A plain shield's number is untouched.
        assert_eq!(Look::default().into_sockets(gems), gems);
        assert_eq!(Look::of("bastion_shield", 0).map(|l| l.kind), Some(1));
        assert_eq!(Look::of("stick", 0), None);
        assert_eq!(colour_of("light_blue_banner", "_banner"), Some(3));
        assert_eq!(colour_of("light_gray_dye", "_dye"), Some(8));
        assert_eq!(colour_of("blue_dye", "_banner"), None);
    }

    #[test]
    fn worn_shields_survive_the_network() {
        let l = Look { kind: 0, base: 15, pattern: 3, pcol: 0 };
        for m in [Mount::Hand { left: true }, Mount::Hand { left: false }, Mount::Back] {
            for up in [false, true] {
                assert_eq!(net_unpack(net_pack(&[(l, m)], up)), Some((l, m, up)));
            }
        }
        assert_eq!(net_unpack(0), None);
    }

    #[test]
    fn models_have_boxes_in_their_swatches() {
        let all = models();
        assert_eq!(all.len(), MESHES);
        for (i, m) in all.iter().enumerate() {
            let n: usize = m.iter().map(|p| p.boxes.len()).sum();
            assert!(n > 0, "mesh {i} is empty");
            assert!(m.iter().flat_map(|p| &p.boxes).all(|b| b.mat < SWATCHES && b.lo.cmple(b.hi).all()));
        }
        // Every design covers some of the face but not all of it.
        for p in 0..PATTERNS.len() {
            let mut on = 0;
            let mut total = 0;
            for yi in -18..18 {
                for xi in -10..10 {
                    let (x, y) = (xi as f32 * 0.5 + 0.25, yi as f32 * 0.5 + 0.25);
                    if Shape::Heater.inside(x, y) {
                        total += 1;
                        on += covers(p, x, y) as u32;
                    }
                }
            }
            assert!(on > 10 && on < total, "{} covers {on} of {total}", PATTERNS[p]);
        }
    }

    #[test]
    fn held_shield_sits_outside_the_arm_and_faces_ahead_when_raised() {
        let pivot = Vec3::new(6.0, 22.0, 0.0);
        let idle = on_arm(Mat4::IDENTITY, Mat4::IDENTITY, pivot, true, 0.0);
        // Face out to the left side (+X), the board beside the forearm.
        assert!(idle.transform_vector3(Vec3::Z).x > 0.9);
        assert!(idle.transform_point3(Vec3::ZERO).x > pivot.x + 2.0);
        // An arm swung forward: raised, the face points ahead (+Z).
        let arm = Mat4::from_translation(pivot) * Mat4::from_rotation_x(-1.0) * Mat4::from_translation(-pivot);
        let up = on_arm(arm, Mat4::IDENTITY, pivot, true, 1.0);
        assert!(up.transform_vector3(Vec3::Z).z > 0.9);
        // On the back: face out behind.
        assert!(on_back(Mat4::IDENTITY).transform_vector3(Vec3::Z).z < -0.8);
    }
}
