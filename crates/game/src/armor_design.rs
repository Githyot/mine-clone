//! Every armour set has its own theme (HP, 2026-10-29: "as armaduras ainda são muito iguais"):
//! its own silhouette built from layered 3D pieces (pauldrons in lames, crests, horns, capes,
//! wings, shells...), one signature piece that names the set at a glance (the platinum helm is a
//! dragon's head), a pattern pressed into the plates and its own colours for trims, cloth, horn
//! and the parts that glow.
//!
//! The pieces are boxes placed freely (turned, mirrored, chained into curves) on the player's
//! bones. They don't get a texture net each: their faces take swatches ("tiles") painted in the
//! lower half of the armour texture, 16 swatches of 16×16 model pixels. The bottom row of
//! swatches glows in the dark (entity.wgsl).

use crate::entity::ArmorStyle;
use glam::{EulerRot, Mat4, Quat, Vec3};

/// The swatch a piece's faces take; its number is its place in the 4×4 grid.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
    /// The set's metal, brushed.
    Metal = 0,
    /// The metal polished bright.
    Polished = 1,
    /// The metal in shadow (undersides, inner lames, straps).
    Dark = 2,
    /// The set's second colour.
    Accent = 3,
    /// Fittings (gold unless the set says otherwise), engraved.
    Trim = 4,
    /// Horsehair, plumes and feathers: streaky.
    Hair = 5,
    /// Horn, bone, teeth and claws: ringed.
    Horn = 6,
    /// Woven cloth (capes, tabards, robes, hats).
    Cloth = 7,
    /// Fur and fleece.
    Fur = 8,
    /// Overlapping scales in the set's metal.
    Scales = 9,
    /// A leaf with its veins.
    Leaf = 10,
    /// Dark glass with a streak of sky in it.
    Glass = 11,
    /// Light: glowing eyes, lamps, halos, cores (glows in the dark).
    Glow = 12,
    /// Fire, yellow at the tip (glows in the dark).
    Flame = 13,
    /// A cut gem that shines from inside (glows in the dark).
    GemGlow = 14,
    /// A cut gem.
    Gem = 15,
}

impl Tile {
    /// Top-left corner of the swatch in the armour texture, in model pixels.
    pub fn origin(self) -> (f32, f32) {
        let i = self as u32;
        ((i % 4 * 16) as f32, (64 + i / 4 * 16) as f32)
    }
    /// Swatches that are one picture (a gem, a flame) stretch over the whole face; the others
    /// are cut to the face's size so their pattern keeps its scale.
    pub fn stretch(self) -> bool {
        matches!(self, Tile::Flame | Tile::Glow | Tile::GemGlow | Tile::Gem)
    }
}

/// The colours of a set.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub base: [u8; 3],
    pub accent: [u8; 3],
    pub trim: [u8; 3],
    pub hair: [u8; 3],
    pub horn: [u8; 3],
    pub cloth: [u8; 3],
    pub fur: [u8; 3],
    pub leaf: [u8; 3],
    pub glass: [u8; 3],
    pub glow: [u8; 3],
    pub gem: [u8; 3],
}

/// How the front of the helmet is cut (the 8×8 face of the helmet's net).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Visor {
    /// Open below the brow.
    Open,
    /// A closed helm with an eye slit.
    Slit,
    /// An eye slit and breathing slots under it.
    Breaths,
    /// Corinthian: a T between the cheek plates.
    TSlot,
    /// Cheek guards down both sides.
    Cheeks,
    /// A nasal bar down the open face.
    Nasal,
    /// Two eye holes in a mask.
    Eyes,
    /// A band open over the eyes only (a mask over the mouth).
    Band,
    /// Closed: the face is a piece of its own (glass, a glowing mask).
    Closed,
}

/// One extra piece: a box of half-size `half` centred on the origin of `place`, riding on body
/// part `part`, worn with armour piece `slot` (0 helmet, 1 chestplate, 2 leggings, 3 boots).
#[derive(Clone, Copy, Debug)]
pub struct Piece {
    pub slot: usize,
    pub part: usize,
    pub place: Mat4,
    pub half: Vec3,
    pub tile: Tile,
}

pub struct Design {
    pub style: ArmorStyle,
    pub visor: Visor,
    pub pal: Palette,
    pub pieces: Vec<Piece>,
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    [0, 1, 2].map(|i| {
        (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    })
}

fn dk(a: [u8; 3], k: f32) -> [u8; 3] {
    a.map(|c| (c as f32 * k).clamp(0.0, 255.0) as u8)
}

const GOLD: [u8; 3] = [226, 184, 72];
const IVORY: [u8; 3] = [228, 216, 186];
const WHITE: [u8; 3] = [236, 236, 230];

/// Places pieces. Coordinates are model pixels on the player model (the head is x -4..4,
/// y 24..32, z -4..4; the front is +Z; the right arm and leg, parts 2 and 4, are on -X).
/// Most pieces are written for the right side and mirrored.
struct B {
    v: Vec<Piece>,
    slot: usize,
}

fn rot(deg: [f32; 3]) -> Quat {
    let r = |d: f32| d.to_radians();
    Quat::from_euler(EulerRot::YXZ, r(deg[1]), r(deg[0]), r(deg[2]))
}

impl B {
    fn slot(&mut self, s: usize) -> &mut Self {
        self.slot = s;
        self
    }
    fn mark(&self) -> usize {
        self.v.len()
    }
    fn push(&mut self, part: usize, place: Mat4, size: Vec3, tile: Tile) {
        self.v.push(Piece {
            slot: self.slot,
            part,
            place,
            half: size.abs() * 0.5,
            tile,
        });
    }
    /// An upright box between two corners.
    fn bx(&mut self, part: usize, lo: [f32; 3], hi: [f32; 3], tile: Tile) {
        let (lo, hi) = (Vec3::from(lo), Vec3::from(hi));
        self.push(part, Mat4::from_translation((lo + hi) * 0.5), hi - lo, tile);
    }
    /// A box of `size` centred on `c`, turned by `deg` (degrees about X, Y, Z).
    fn rb(&mut self, part: usize, c: [f32; 3], size: [f32; 3], deg: [f32; 3], tile: Tile) {
        self.push(
            part,
            Mat4::from_rotation_translation(rot(deg), Vec3::from(c)),
            Vec3::from(size),
            tile,
        );
    }
    /// A curve of boxes (a horn, a tail, a feather): starting at `start` pointing along +Y turned
    /// by `deg`, each segment `[length, thickness, bend about X, bend about Z]` bends before it
    /// grows. Returns where the tip ends.
    fn chain(
        &mut self,
        part: usize,
        start: [f32; 3],
        deg: [f32; 3],
        segs: &[[f32; 4]],
        tile: Tile,
    ) -> Vec3 {
        let mut q = rot(deg);
        let mut p = Vec3::from(start);
        for s in segs {
            q *= rot([s[2], 0.0, s[3]]);
            let d = q * Vec3::Y;
            self.push(
                part,
                Mat4::from_rotation_translation(q, p + d * s[0] * 0.5),
                Vec3::new(s[1], s[0] + s[1] * 0.3, s[1]),
                tile,
            );
            p += d * s[0];
        }
        p
    }
    /// A flat feather or blade: a chain whose boxes are `w` wide and `t` thick.
    fn blade(
        &mut self,
        part: usize,
        start: [f32; 3],
        deg: [f32; 3],
        segs: &[[f32; 3]],
        w: f32,
        t: f32,
        tile: Tile,
    ) {
        let mut q = rot(deg);
        let mut p = Vec3::from(start);
        let n = segs.len().max(1) as f32;
        for (i, s) in segs.iter().enumerate() {
            q *= rot([s[1], 0.0, s[2]]);
            let d = q * Vec3::Y;
            let ww = w * (1.0 - i as f32 / n * 0.45);
            self.push(
                part,
                Mat4::from_rotation_translation(q, p + d * s[0] * 0.5),
                Vec3::new(ww, s[0] + 0.2, t),
                tile,
            );
            p += d * s[0];
        }
    }
    /// A glowing crack zigzagging down a plate facing +Z: each `[length, angle]` goes down at
    /// that many degrees off vertical.
    fn crack(&mut self, part: usize, start: [f32; 3], segs: &[[f32; 2]]) {
        let mut p = Vec3::from(start);
        for s in segs {
            let a = s[1].to_radians();
            let d = Vec3::new(a.sin(), -a.cos(), 0.0);
            self.rb(
                part,
                (p + d * s[0] * 0.5).to_array(),
                [0.5, s[0] + 0.3, 0.35],
                [0.0, 0.0, s[1]],
                Tile::Flame,
            );
            p += d * s[0];
        }
    }
    /// A ring of `n` boxes of `size` (along the ring, height, thickness) around `c`, tilted by
    /// `tilt` degrees about X.
    fn ring(
        &mut self,
        part: usize,
        c: [f32; 3],
        r: f32,
        n: usize,
        size: [f32; 3],
        tilt: f32,
        tile: Tile,
    ) {
        let base = Mat4::from_translation(Vec3::from(c)) * Mat4::from_rotation_x(tilt.to_radians());
        for k in 0..n {
            let a = k as f32 / n as f32 * std::f32::consts::TAU;
            let m = base
                * Mat4::from_translation(Vec3::new(a.cos() * r, 0.0, a.sin() * r))
                * Mat4::from_rotation_y(-(a + std::f32::consts::FRAC_PI_2));
            self.push(part, m, Vec3::from(size), tile);
        }
    }
    /// Mirror every piece placed since `from` onto the other side (x → -x, right ↔ left).
    fn mirror(&mut self, from: usize) {
        let flip = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
        let added: Vec<Piece> = self.v[from..]
            .iter()
            .map(|p| Piece {
                part: match p.part {
                    2 => 3,
                    3 => 2,
                    4 => 5,
                    5 => 4,
                    k => k,
                },
                place: flip * p.place,
                ..*p
            })
            .collect();
        self.v.extend(added);
    }

    // ---- Kits shared by many sets ----

    /// Pauldrons: `n` lames stacked down each shoulder, sloping out by `tilt` degrees, alternating
    /// two swatches.
    fn pauldrons(&mut self, n: usize, w: f32, tilt: f32, a: Tile, b: Tile) {
        let m = self.mark();
        self.slot(1);
        for i in 0..n {
            let f = i as f32;
            self.rb(
                2,
                [-6.4 - 0.4 * f, 25.5 - 1.45 * f, 0.0],
                [w + 0.5 * f, 1.5, 7.0 + 0.35 * f],
                [0.0, 0.0, tilt + 7.0 * f],
                if i % 2 == 0 { a } else { b },
            );
        }
        self.mirror(m);
    }
    /// A plate over the forearm at the wrist.
    fn bracers(&mut self, t: Tile) {
        let m = self.mark();
        self.slot(1);
        self.bx(2, [-8.7, 12.3, -2.7], [-3.3, 15.2, 2.7], t);
        self.mirror(m);
    }
    /// A raised breastplate with a ridge.
    fn breastplate(&mut self, t: Tile, ridge: Tile) {
        self.slot(1);
        self.bx(1, [-4.3, 15.0, 3.0], [4.3, 23.6, 3.6], t);
        self.bx(1, [-0.5, 15.0, 3.6], [0.5, 23.2, 4.0], ridge);
    }
    /// A plate round the neck.
    fn gorget(&mut self, t: Tile) {
        self.slot(1);
        self.bx(1, [-4.0, 24.4, -3.4], [4.0, 25.9, 3.4], t);
    }
    /// A cape from the shoulders, `len` long, hanging a little away from the back.
    fn cape(&mut self, len: f32, t: Tile) {
        self.slot(1);
        let tilt = 7.0f32;
        let zc = -3.3 - (len * 0.5) * tilt.to_radians().sin() - 0.3;
        self.rb(
            1,
            [0.0, 25.0 - len * 0.5, zc],
            [9.4, len, 0.5],
            [tilt, 0.0, 0.0],
            t,
        );
        self.bx(1, [-5.2, 24.0, -3.8], [5.2, 25.6, -3.0], t);
    }
    /// A belt with a buckle.
    fn belt(&mut self, t: Tile, buckle: Tile) {
        self.slot(2);
        self.bx(1, [-4.95, 11.4, -2.95], [4.95, 12.8, 2.95], t);
        self.bx(1, [-1.0, 11.1, 2.95], [1.0, 13.1, 3.45], buckle);
    }
    /// Two front plates hanging from the waist over the thighs.
    fn tassets(&mut self, t: Tile, n: usize) {
        let m = self.mark();
        self.slot(2);
        for i in 0..n {
            let f = i as f32;
            self.rb(
                1,
                [-2.5, 10.4 - f * 1.4, 3.0 + f * 0.25],
                [4.6 - f * 0.3, 1.6, 0.6],
                [-8.0 - 4.0 * f, 0.0, 0.0],
                t,
            );
        }
        self.mirror(m);
    }
    /// Knee cops with a little wing on the outside.
    fn knees(&mut self, t: Tile, wing: bool) {
        let m = self.mark();
        self.slot(2);
        self.bx(4, [-4.3, 5.2, 2.5], [0.3, 7.6, 3.3], t);
        if wing {
            self.rb(4, [-4.6, 6.4, 1.8], [0.5, 2.6, 2.2], [0.0, 0.0, 12.0], t);
        }
        self.mirror(m);
    }
    /// A robe from the waist to the shins.
    fn skirt(&mut self, top: f32, bottom: f32, t: Tile, hem: Option<Tile>) {
        self.slot(2);
        self.bx(1, [-5.1, bottom, -3.1], [5.1, top, 3.1], t);
        if let Some(h) = hem {
            self.bx(1, [-5.3, bottom - 0.2, -3.3], [5.3, bottom + 1.0, 3.3], h);
        }
    }
    /// Toe caps on the boots.
    fn toes(&mut self, t: Tile) {
        let m = self.mark();
        self.slot(3);
        self.bx(4, [-4.8, -1.0, 2.9], [0.8, 1.6, 4.4], t);
        self.mirror(m);
    }
    /// A turned-down cuff round the top of the boots.
    fn cuffs(&mut self, t: Tile) {
        let m = self.mark();
        self.slot(3);
        self.bx(4, [-5.3, 3.6, -3.3], [1.3, 5.3, 3.3], t);
        self.mirror(m);
    }
    /// A band round the helmet at the brow.
    fn brow(&mut self, t: Tile) {
        self.slot(0);
        self.bx(0, [-5.35, 29.3, -5.35], [5.35, 30.5, 5.35], t);
    }
    /// A pointed hat: stacked boxes narrowing upward, the tip bending back by `lean` per tier.
    fn hat(&mut self, brim: f32, tiers: usize, lean: f32, t: Tile) -> Vec3 {
        self.slot(0);
        if brim > 0.0 {
            self.bx(
                0,
                [-5.5 - brim, 32.6, -5.5 - brim],
                [5.5 + brim, 33.4, 5.5 + brim],
                t,
            );
        }
        let segs: Vec<[f32; 4]> = (0..tiers)
            .map(|i| {
                [
                    3.0,
                    9.6 - i as f32 * (8.4 / tiers as f32),
                    if i == 0 { 0.0 } else { -lean },
                    0.0,
                ]
            })
            .collect();
        self.chain(0, [0.0, 32.4, 0.0], [0.0, 0.0, 0.0], &segs, t)
    }
    /// A hood over the helmet, peaked at the back.
    fn hood(&mut self, t: Tile, peak: bool) {
        self.slot(0);
        self.bx(0, [-5.6, 23.4, -5.7], [5.6, 33.6, -4.6], t);
        self.bx(0, [-5.7, 23.0, -5.6], [-4.7, 33.6, 5.3], t);
        self.bx(0, [4.7, 23.0, -5.6], [5.7, 33.6, 5.3], t);
        self.bx(0, [-5.6, 32.7, -5.6], [5.6, 33.8, 5.6], t);
        self.bx(0, [-5.6, 30.9, 4.6], [5.6, 33.8, 5.9], t);
        if peak {
            self.rb(0, [0.0, 32.0, -6.4], [3.2, 3.2, 1.4], [-35.0, 0.0, 0.0], t);
        }
    }
}

/// The palette every set starts from: derived from its metal.
fn base_palette(c: [u8; 3]) -> Palette {
    Palette {
        base: c,
        accent: dk(c, 0.62),
        trim: GOLD,
        hair: [150, 40, 36],
        horn: IVORY,
        cloth: dk(c, 0.7),
        fur: [196, 172, 134],
        leaf: [76, 150, 58],
        glass: [34, 52, 70],
        glow: [255, 210, 120],
        gem: [210, 40, 60],
    }
}

/// The design of one material (`entity::ARMOR_MATERIALS` names).
pub fn design(name: &str, color: [u8; 3]) -> Design {
    use ArmorStyle::*;
    use Tile::*;
    let mut b = B {
        v: Vec::new(),
        slot: 0,
    };
    let mut p = base_palette(color);
    let (style, visor) = match name {
        // ---- Minecraft ----
        "leather" => {
            // A ranger: a fleece mantle, a feathered cap, a quiver on the back.
            p.fur = [214, 196, 160];
            p.hair = [70, 128, 74];
            p.accent = [96, 58, 32];
            b.slot(0)
                .bx(0, [-5.45, 29.4, -5.45], [5.45, 30.6, 5.45], Fur);
            b.blade(
                0,
                [-5.3, 30.6, -1.5],
                [-38.0, 0.0, 22.0],
                &[[2.5, 0.0, 0.0], [2.5, -10.0, 0.0], [2.0, -12.0, 0.0]],
                1.3,
                0.4,
                Hair,
            );
            b.slot(1).bx(1, [-5.6, 23.3, -3.7], [5.6, 25.8, 3.7], Fur);
            let m = b.mark();
            b.bx(2, [-9.4, 23.7, -3.4], [-2.6, 25.7, 3.4], Fur);
            b.mirror(m);
            b.rb(
                1,
                [0.0, 18.4, 3.15],
                [1.0, 15.5, 0.45],
                [0.0, 0.0, 38.0],
                Accent,
            );
            b.rb(
                1,
                [2.3, 19.5, -4.3],
                [2.6, 10.0, 2.6],
                [0.0, 0.0, -24.0],
                Accent,
            );
            for (dx, dz) in [(0.0, 0.0), (0.8, 0.6), (-0.6, -0.5)] {
                b.rb(
                    1,
                    [4.6 + dx, 24.4, -4.3 + dz],
                    [0.5, 2.4, 1.2],
                    [0.0, 0.0, -24.0],
                    Hair,
                );
            }
            b.slot(2);
            b.bx(1, [-4.95, 11.4, -2.95], [4.95, 12.6, 2.95], Accent);
            let m = b.mark();
            b.bx(1, [-5.0, 9.6, 1.2], [-3.0, 12.2, 3.7], Accent);
            b.mirror(m);
            b.cuffs(Fur);
            (Leather, Visor::Open)
        }
        "chainmail" => {
            // A man-at-arms: mail coif under a padded cap, a red tabard with a gold cross.
            p.cloth = [168, 36, 40];
            p.accent = [104, 70, 42];
            b.brow(Cloth);
            b.slot(0)
                .bx(0, [-5.35, 21.6, -5.5], [5.35, 24.2, -4.6], Metal);
            let m = b.mark();
            b.bx(0, [-5.5, 21.6, -4.6], [-4.6, 24.0, 2.0], Metal);
            b.mirror(m);
            b.slot(1);
            for z in [3.0, -3.5] {
                b.bx(1, [-3.7, 8.6, z], [3.7, 24.4, z + 0.5], Cloth);
            }
            b.bx(1, [-0.6, 15.4, 3.5], [0.6, 21.6, 3.85], Trim);
            b.bx(1, [-2.3, 18.4, 3.5], [2.3, 19.6, 3.85], Trim);
            b.bx(1, [-5.25, 12.6, -3.3], [5.25, 13.8, 3.7], Accent);
            b.knees(Dark, false);
            b.cuffs(Accent);
            (Mail, Visor::Open)
        }
        "iron" => {
            // A knight: a great helm with a blue plume, plate in layers.
            p.hair = [58, 86, 168];
            b.slot(0)
                .bx(0, [-0.55, 23.2, 5.0], [0.55, 32.2, 5.8], Polished);
            b.bx(0, [-5.45, 29.3, 4.9], [5.45, 30.3, 5.7], Polished);
            b.chain(
                0,
                [0.0, 32.8, -1.0],
                [-15.0, 0.0, 0.0],
                &[
                    [2.0, 1.4, 0.0, 0.0],
                    [2.6, 1.7, -28.0, 0.0],
                    [2.6, 1.5, -30.0, 0.0],
                    [2.4, 1.1, -28.0, 0.0],
                ],
                Hair,
            );
            b.pauldrons(3, 6.6, 18.0, Metal, Polished);
            b.gorget(Polished);
            b.breastplate(Polished, Metal);
            b.bracers(Metal);
            b.slot(1).bx(1, [-5.3, 10.8, -3.3], [5.3, 12.2, 3.3], Metal);
            b.tassets(Metal, 2);
            b.knees(Polished, true);
            b.toes(Metal);
            b.cuffs(Polished);
            (Plate, Visor::Breaths)
        }
        "golden" => {
            // A king: a crown with laurel wings at the temples, a sun on the chest, a red cape.
            p.cloth = [150, 26, 40];
            p.trim = [250, 214, 110];
            p.gem = [200, 24, 44];
            b.slot(0).bx(0, [-5.5, 32.0, -5.5], [5.5, 33.6, 5.5], Trim);
            for k in 0..8 {
                let a = k as f32 / 8.0 * std::f32::consts::TAU;
                b.rb(
                    0,
                    [a.sin() * 5.1, 34.4, a.cos() * 5.1],
                    [1.3, 1.3, 1.3],
                    [45.0, a.to_degrees(), 45.0],
                    Trim,
                );
            }
            b.bx(0, [-0.8, 32.2, 5.5], [0.8, 33.6, 6.0], Gem);
            let m = b.mark();
            for (i, a) in [0.0, 22.0, 44.0].iter().enumerate() {
                b.blade(
                    0,
                    [-5.6, 30.0 + i as f32 * 0.4, -0.5 - i as f32 * 0.6],
                    [-40.0 - a, 0.0, 30.0],
                    &[[2.4, 0.0, 0.0], [2.2, -8.0, 0.0]],
                    1.4,
                    0.35,
                    Polished,
                );
            }
            b.mirror(m);
            b.pauldrons(2, 6.4, 22.0, Polished, Trim);
            b.slot(1).bx(1, [-2.0, 17.0, 3.0], [2.0, 21.0, 3.6], Trim);
            b.bx(1, [-0.8, 18.2, 3.6], [0.8, 19.8, 4.1], Gem);
            b.cape(19.0, Cloth);
            b.belt(Trim, Gem);
            b.slot(2).bx(1, [-1.8, 6.0, 2.9], [1.8, 11.4, 3.4], Accent);
            b.toes(Polished);
            b.cuffs(Trim);
            (Ornate, Visor::Open)
        }
        "diamond" => {
            // A crystal knight: a crown of shards, shard clusters growing from the shoulders.
            p.gem = [150, 246, 240];
            b.slot(0);
            for (x, z, h, tz, tx) in [
                (0.0, 0.0, 5.0, 0.0, 0.0),
                (-2.6, 0.5, 3.4, 18.0, 0.0),
                (2.6, 0.5, 3.4, -18.0, 0.0),
                (-1.4, -2.6, 2.8, 10.0, -16.0),
                (1.4, -2.6, 2.8, -10.0, -16.0),
            ] {
                b.rb(
                    0,
                    [x, 33.0 + h * 0.4, z],
                    [1.4, h, 1.4],
                    [tx, 45.0, tz],
                    Gem,
                );
            }
            b.bx(0, [-5.4, 29.2, 4.9], [5.4, 30.2, 5.6], Polished);
            b.pauldrons(2, 6.6, 20.0, Polished, Metal);
            let m = b.mark();
            b.slot(1);
            for (dx, dz, h, tz, tx) in [
                (0.0, 0.0, 4.0, 20.0, 0.0),
                (-1.2, 1.6, 2.8, 38.0, 20.0),
                (-0.8, -1.8, 3.0, 30.0, -22.0),
            ] {
                b.rb(
                    2,
                    [-6.8 + dx, 27.0 + h * 0.3, dz],
                    [1.3, h, 1.3],
                    [tx, 45.0, tz],
                    Gem,
                );
            }
            b.mirror(m);
            b.bx(1, [-1.4, 17.4, 3.0], [1.4, 20.6, 3.7], Gem);
            b.breastplate(Metal, Polished);
            let m = b.mark();
            b.slot(2)
                .rb(4, [-2.0, 7.2, 3.4], [1.2, 2.6, 1.2], [30.0, 45.0, 0.0], Gem);
            b.mirror(m);
            b.knees(Polished, false);
            b.toes(Polished);
            (Facet, Visor::Slit)
        }
        "netherite" => {
            // A brute: curled ram's horns, spiked pauldrons, spines down the back.
            p.horn = [150, 128, 108];
            p.glow = [255, 120, 50];
            let m = b.mark();
            b.slot(0).chain(
                0,
                [-4.8, 30.5, 0.5],
                [0.0, 0.0, 72.0],
                &[
                    [2.4, 2.4, 0.0, 0.0],
                    [2.4, 2.1, 12.0, 36.0],
                    [2.2, 1.8, 14.0, 40.0],
                    [2.0, 1.4, 10.0, 38.0],
                    [1.8, 1.0, 0.0, 30.0],
                ],
                Horn,
            );
            b.mirror(m);
            b.bx(0, [-5.4, 29.0, 4.9], [5.4, 30.1, 5.7], Dark);
            b.pauldrons(3, 7.0, 15.0, Metal, Dark);
            let m = b.mark();
            b.slot(1);
            for (dz, h) in [(-2.0, 3.0), (0.0, 4.0), (2.0, 3.0)] {
                b.rb(
                    2,
                    [-7.2, 26.6 + h * 0.3, dz],
                    [1.0, h, 1.0],
                    [0.0, 0.0, 26.0],
                    Horn,
                );
            }
            b.mirror(m);
            for (y, h) in [(22.5, 3.0), (18.5, 3.4), (14.5, 2.6)] {
                b.rb(1, [0.0, y, -4.0], [1.0, h, 1.0], [-55.0, 0.0, 0.0], Horn);
            }
            b.crack(
                1,
                [-1.0, 22.6, 3.1],
                &[[3.0, 20.0], [2.6, -24.0], [3.0, 18.0]],
            );
            b.bracers(Dark);
            b.knees(Dark, false);
            let m = b.mark();
            b.slot(2)
                .rb(4, [-2.0, 6.6, 4.2], [1.0, 2.6, 1.0], [70.0, 0.0, 0.0], Horn);
            b.slot(3).rb(
                4,
                [-2.0, 1.0, -3.8],
                [1.0, 2.2, 1.0],
                [-70.0, 0.0, 0.0],
                Horn,
            );
            b.mirror(m);
            b.toes(Dark);
            b.cuffs(Metal);
            (Spiked, Visor::Breaths)
        }
        "copper" => {
            // A legionary: the red transverse crest, banded shoulders, a red cape and a strap skirt.
            p.hair = [176, 40, 34];
            p.cloth = [150, 34, 32];
            p.accent = [104, 70, 42];
            b.slot(0);
            for k in 0..7 {
                let a = (k as f32 / 6.0 - 0.5) * 2.4;
                b.rb(
                    0,
                    [a.sin() * 5.2, 32.6 + a.cos() * 2.0, 0.0],
                    [1.7, 3.0, 1.3],
                    [0.0, 0.0, -a.to_degrees()],
                    Hair,
                );
            }
            b.rb(
                0,
                [0.0, 23.6, -5.6],
                [10.4, 1.0, 2.6],
                [-25.0, 0.0, 0.0],
                Polished,
            );
            let m = b.mark();
            b.bx(0, [-5.6, 23.0, 0.8], [-4.9, 28.6, 4.8], Polished);
            b.mirror(m);
            b.pauldrons(4, 6.4, 25.0, Metal, Polished);
            b.cape(17.0, Cloth);
            for (x, y) in [(-2.4, 21.0), (2.4, 21.0), (0.0, 17.6)] {
                b.bx(1, [x - 0.9, y - 0.9, 3.0], [x + 0.9, y + 0.9, 3.6], Trim);
            }
            b.slot(2);
            for i in 0..8 {
                let x = -4.7 + i as f32 * 1.2;
                b.bx(1, [x, 7.2, 2.9], [x + 0.9, 11.6, 3.35], Accent);
                b.bx(1, [x, 7.2, -3.35], [x + 0.9, 11.6, -2.9], Accent);
            }
            b.belt(Metal, Trim);
            b.cuffs(Accent);
            (Roman, Visor::Cheeks)
        }
        "bronze" => {
            // A hoplite: a tall black crest on a stalk, a round shield slung on the back.
            p.hair = [36, 30, 28];
            p.accent = [140, 40, 30];
            b.slot(0).bx(0, [-0.5, 33.0, -1.0], [0.5, 34.3, 1.0], Metal);
            for k in 0..8 {
                let a = (k as f32 / 7.0 - 0.3) * 2.3;
                b.rb(
                    0,
                    [0.0, 33.2 + a.cos() * 3.0, -a.sin() * 5.5],
                    [1.2, 3.0, 1.7],
                    [-a.to_degrees(), 0.0, 0.0],
                    Hair,
                );
            }
            b.slot(1);
            b.bx(1, [-4.6, 12.4, -4.3], [4.6, 23.6, -3.5], Accent);
            b.bx(1, [-5.6, 13.4, -4.3], [5.6, 22.6, -3.5], Accent);
            b.rb(
                1,
                [0.0, 18.0, -3.9],
                [8.6, 8.6, 0.8],
                [0.0, 0.0, 45.0],
                Accent,
            );
            b.bx(1, [-1.6, 16.4, -4.8], [1.6, 19.6, -4.2], Trim);
            b.ring(1, [0.0, 18.0, -4.45], 5.5, 12, [3.0, 0.9, 0.9], 90.0, Trim);
            b.pauldrons(2, 6.2, 20.0, Accent, Metal);
            b.slot(2);
            for i in 0..8 {
                let x = -4.7 + i as f32 * 1.2;
                b.bx(1, [x, 7.6, 2.9], [x + 0.9, 11.4, 3.35], Accent);
            }
            b.knees(Polished, false);
            b.cuffs(Accent);
            (Greek, Visor::TSlot)
        }
        "lead" => {
            // A sapper: a brimmed helm with a headlamp, square pauldrons, a tank on the back.
            p.glow = [255, 236, 170];
            p.accent = [86, 66, 44];
            b.slot(0).bx(0, [-6.0, 30.0, -6.0], [6.0, 30.8, 6.0], Metal);
            b.bx(0, [-1.5, 30.8, 4.9], [1.5, 33.0, 6.8], Dark);
            b.bx(0, [-1.1, 31.2, 6.8], [1.1, 32.6, 7.2], Glow);
            b.slot(1);
            let m = b.mark();
            b.bx(2, [-9.7, 22.4, -3.6], [-2.6, 26.0, 3.6], Metal);
            b.bx(2, [-9.9, 21.0, -3.4], [-5.0, 22.4, 3.4], Dark);
            b.mirror(m);
            b.bx(1, [-3.2, 13.2, -6.0], [3.2, 23.6, -3.2], Dark);
            b.bx(1, [-3.4, 23.6, -5.6], [3.4, 24.4, -3.6], Metal);
            b.bx(1, [-0.6, 24.4, -5.0], [0.6, 25.4, -4.0], Trim);
            b.belt(Accent, Metal);
            let m = b.mark();
            b.slot(2).bx(1, [-5.2, 9.0, 1.4], [-3.2, 11.4, 3.7], Accent);
            b.mirror(m);
            b.knees(Metal, true);
            b.toes(Dark);
            b.cuffs(Metal);
            (Banded, Visor::Slit)
        }
        "silver" => {
            // A gothic knight: a pointed hounskull visor, a tall spike, fluted rondels, poulaines.
            p.accent = [40, 52, 104];
            p.gem = [80, 120, 230];
            b.slot(0);
            b.rb(
                0,
                [0.0, 27.0, 5.8],
                [4.2, 4.2, 4.2],
                [0.0, 45.0, 0.0],
                Polished,
            );
            b.rb(
                0,
                [0.0, 27.0, 7.4],
                [1.8, 2.6, 1.8],
                [0.0, 45.0, 0.0],
                Polished,
            );
            b.chain(
                0,
                [0.0, 33.0, 0.0],
                [0.0, 0.0, 0.0],
                &[
                    [1.6, 1.6, 0.0, 0.0],
                    [1.8, 1.0, 0.0, 0.0],
                    [1.6, 0.6, 0.0, 0.0],
                ],
                Polished,
            );
            b.pauldrons(2, 6.4, 20.0, Polished, Metal);
            let m = b.mark();
            b.slot(1).bx(2, [-7.8, 21.6, 3.0], [-4.2, 25.2, 3.6], Trim);
            b.bx(2, [-6.5, 22.9, 3.6], [-5.5, 23.9, 4.0], Gem);
            b.mirror(m);
            b.breastplate(Polished, Polished);
            b.bx(1, [-0.9, 18.0, 4.0], [0.9, 19.8, 4.4], Gem);
            b.tassets(Polished, 3);
            b.knees(Polished, true);
            let m = b.mark();
            b.slot(3).chain(
                4,
                [-2.0, 0.2, 3.2],
                [80.0, 0.0, 0.0],
                &[
                    [2.0, 1.6, 0.0, 0.0],
                    [2.0, 1.0, 8.0, 0.0],
                    [1.6, 0.6, 8.0, 0.0],
                ],
                Polished,
            );
            b.mirror(m);
            b.cuffs(Polished);
            (Gothic, Visor::Slit)
        }
        "tungsten" => {
            // A samurai: kuwagata horns in a V, a tiered neck guard, a face mask, big sode.
            p.accent = [168, 40, 40];
            p.hair = [228, 226, 220];
            b.slot(0);
            let m = b.mark();
            b.blade(
                0,
                [-0.8, 30.8, 5.6],
                [0.0, 0.0, 22.0],
                &[[2.2, 0.0, 0.0], [2.4, 0.0, -10.0], [2.2, 0.0, -10.0]],
                1.0,
                0.4,
                Trim,
            );
            for i in 0..3 {
                let f = i as f32;
                b.rb(
                    0,
                    [-5.6 - f * 0.6, 23.4 - f * 1.2, -1.5 - f * 0.4],
                    [1.0, 1.6, 8.0 + f],
                    [0.0, 0.0, 25.0 + f * 8.0],
                    if i % 2 == 0 { Metal } else { Accent },
                );
            }
            b.rb(
                0,
                [-5.8, 28.6, 4.0],
                [0.8, 3.2, 2.6],
                [0.0, -25.0, 0.0],
                Accent,
            );
            b.mirror(m);
            for i in 0..3 {
                let f = i as f32;
                b.rb(
                    0,
                    [0.0, 23.4 - f * 1.2, -5.6 - f * 0.6],
                    [10.0 + f * 1.4, 1.6, 1.0],
                    [-25.0 - f * 8.0, 0.0, 0.0],
                    if i % 2 == 0 { Metal } else { Accent },
                );
            }
            b.bx(0, [-3.8, 23.0, 4.9], [3.8, 26.6, 5.7], Dark);
            b.bx(0, [-2.2, 26.0, 5.7], [2.2, 26.6, 6.0], Hair);
            b.slot(1);
            let m = b.mark();
            for i in 0..4 {
                let f = i as f32;
                b.rb(
                    2,
                    [-9.0 - f * 0.3, 23.6 - f * 1.7, 0.0],
                    [0.9, 1.8, 7.6],
                    [0.0, 0.0, 10.0],
                    if i % 2 == 0 { Metal } else { Accent },
                );
            }
            b.mirror(m);
            b.slot(2);
            for i in 0..3 {
                let f = i as f32;
                b.bx(
                    1,
                    [-5.4 - f * 0.3, 10.8 - f * 1.8, -3.4 - f * 0.3],
                    [5.4 + f * 0.3, 12.6 - f * 1.8, 3.4 + f * 0.3],
                    if i % 2 == 0 { Metal } else { Accent },
                );
            }
            b.cuffs(Accent);
            (Samurai, Visor::Open)
        }
        "platinum" => {
            // A dragon knight: the helm is a dragon's head (the face looks out between its jaws),
            // folded wings on the back, a tail, claws on the shoulders and boots.
            p.accent = [46, 70, 140];
            p.horn = [232, 214, 170];
            p.gem = [120, 226, 255];
            p.glow = [140, 230, 255];
            b.slot(0);
            // Upper jaw and snout over the brow, with a ridge, nostrils and teeth.
            b.bx(0, [-3.4, 29.3, 4.4], [3.4, 32.3, 9.4], Scales);
            b.bx(0, [-2.8, 29.8, 9.4], [2.8, 31.9, 10.6], Scales);
            b.bx(0, [-1.0, 32.3, 2.0], [1.0, 33.3, 9.2], Polished);
            for x in [-1.8, 1.0] {
                b.bx(0, [x, 30.8, 10.6], [x + 0.8, 31.5, 10.9], Dark);
            }
            for x in [-2.6, -1.1, 0.4, 1.9] {
                b.bx(0, [x, 28.5, 9.3], [x + 0.7, 29.3, 10.0], Horn);
            }
            // Lower jaw under the chin, teeth pointing up.
            b.bx(0, [-3.4, 22.0, 3.6], [3.4, 23.4, 9.0], Scales);
            for x in [-2.6, -1.1, 0.4, 1.9] {
                b.bx(0, [x, 23.4, 8.1], [x + 0.7, 24.3, 8.8], Horn);
            }
            let m = b.mark();
            // Glowing eyes under a brow ridge, cheek plates, a fin behind the jaw, swept horns.
            b.bx(0, [-3.7, 31.0, 5.2], [-3.1, 31.9, 6.9], GemGlow);
            b.rb(
                0,
                [-2.9, 32.6, 6.0],
                [1.4, 0.8, 3.0],
                [0.0, 0.0, 18.0],
                Polished,
            );
            b.bx(0, [-5.8, 22.8, 0.6], [-5.0, 31.0, 5.0], Polished);
            for (i, a) in [0.0, 20.0, 40.0].iter().enumerate() {
                b.blade(
                    0,
                    [-5.6, 28.5 - i as f32 * 1.6, -1.0],
                    [-60.0 - a, 0.0, 20.0],
                    &[[2.4, 0.0, 0.0], [1.8, -10.0, 0.0]],
                    1.2,
                    0.35,
                    Accent,
                );
            }
            b.chain(
                0,
                [-3.0, 32.4, -1.6],
                [-58.0, 0.0, 12.0],
                &[
                    [2.8, 2.0, 0.0, 0.0],
                    [2.8, 1.6, -14.0, 4.0],
                    [2.8, 1.2, -14.0, 6.0],
                    [2.4, 0.8, -12.0, 4.0],
                    [1.6, 0.5, -10.0, 0.0],
                ],
                Horn,
            );
            b.mirror(m);
            for (i, z) in [3.0, 0.4, -2.2, -4.8].iter().enumerate() {
                b.rb(
                    0,
                    [0.0, 34.0 - i as f32 * 0.3, *z],
                    [0.6, 2.2 - i as f32 * 0.2, 1.8],
                    [-30.0, 0.0, 0.0],
                    Accent,
                );
            }
            // Scaled pauldrons, each gripped by three claws.
            b.pauldrons(2, 6.8, 18.0, Scales, Polished);
            let m = b.mark();
            b.slot(1);
            for dz in [-2.0, 0.0, 2.0] {
                b.chain(
                    2,
                    [-8.6, 25.0, dz],
                    [0.0, 0.0, 80.0],
                    &[
                        [1.4, 0.8, 0.0, 0.0],
                        [1.4, 0.6, 0.0, 50.0],
                        [1.0, 0.4, 0.0, 40.0],
                    ],
                    Horn,
                );
            }
            // A half-spread wing: the bone rises from the shoulder blade, three fingers fan down
            // from its tip with the membrane between them.
            let tip = b.chain(
                1,
                [-1.8, 22.0, -3.8],
                [-24.0, 0.0, 52.0],
                &[[5.0, 1.2, 0.0, 0.0], [4.6, 0.9, 0.0, -34.0]],
                Polished,
            );
            for (a, len) in [(150.0f32, 11.0f32), (128.0, 9.0), (106.0, 6.5)] {
                b.chain(
                    1,
                    tip.to_array(),
                    [-18.0, 0.0, a],
                    &[[len, 0.6, 0.0, 0.0]],
                    Polished,
                );
            }
            for (a, len) in [(161.0f32, 11.0f32), (139.0, 9.6), (117.0, 7.6)] {
                let d = rot([-18.0, 0.0, a]) * Vec3::Y;
                b.rb(
                    1,
                    (tip + d * len * 0.5 + Vec3::new(0.0, 0.0, -0.1)).to_array(),
                    [len * 0.38, len, 0.3],
                    [-18.0, 0.0, a],
                    Accent,
                );
            }
            b.mirror(m);
            b.breastplate(Scales, Polished);
            b.bx(1, [-1.2, 18.4, 4.0], [1.2, 20.6, 4.5], GemGlow);
            b.bracers(Scales);
            // The tail, from the small of the back down to the heels, with a spade tip.
            b.slot(2);
            let tip = b.chain(
                1,
                [0.0, 12.6, -3.0],
                [-150.0, 0.0, 0.0],
                &[
                    [3.0, 2.0, 0.0, 0.0],
                    [3.0, 1.7, 14.0, 0.0],
                    [3.0, 1.4, 16.0, 6.0],
                    [3.0, 1.1, 16.0, 6.0],
                    [2.6, 0.8, 12.0, 4.0],
                ],
                Scales,
            );
            b.rb(
                1,
                tip.to_array(),
                [2.4, 2.4, 0.4],
                [-60.0, 0.0, 45.0],
                Accent,
            );
            for (i, z) in [-4.0, -6.4].iter().enumerate() {
                b.rb(
                    1,
                    [0.0, 10.5 - i as f32 * 2.2, *z],
                    [0.5, 1.6, 1.4],
                    [-50.0, 0.0, 0.0],
                    Accent,
                );
            }
            b.belt(Polished, GemGlow);
            b.knees(Scales, true);
            let m = b.mark();
            b.slot(3);
            for dx in [-3.6, -2.0, -0.4] {
                b.chain(
                    4,
                    [dx, 0.4, 3.6],
                    [80.0, 0.0, 0.0],
                    &[[1.2, 0.8, 0.0, 0.0], [1.0, 0.5, 40.0, 0.0]],
                    Horn,
                );
            }
            b.mirror(m);
            b.toes(Scales);
            b.cuffs(Polished);
            (ArmorStyle::Scale, Visor::Open)
        }
        // ---- Terraria-inspired hard mode metals ----
        "cobalt" => {
            // A tide knight: a fin crest, scallop-shell pauldrons, pearls, a sea-green cape.
            p.accent = [22, 52, 120];
            p.cloth = [30, 96, 120];
            p.gem = [226, 236, 246];
            b.slot(0);
            for (i, (y, z, h)) in [
                (33.6, 3.0, 1.6),
                (34.2, 1.0, 2.6),
                (34.8, -1.2, 3.6),
                (34.6, -3.4, 3.2),
                (34.0, -5.0, 2.2),
            ]
            .iter()
            .enumerate()
            {
                b.rb(
                    0,
                    [0.0, *y, *z],
                    [0.7, *h, 2.4],
                    [-24.0 - i as f32 * 6.0, 0.0, 0.0],
                    Polished,
                );
            }
            let m = b.mark();
            b.blade(
                0,
                [-5.5, 29.0, 2.0],
                [-60.0, 0.0, 40.0],
                &[[2.6, 0.0, 0.0], [2.0, -20.0, 0.0]],
                2.0,
                0.35,
                Accent,
            );
            b.mirror(m);
            b.bx(0, [-0.7, 29.4, 5.4], [0.7, 30.6, 5.9], Gem);
            let m = b.mark();
            b.slot(1);
            for k in 0..5 {
                let a = -40.0 + k as f32 * 20.0;
                b.blade(
                    2,
                    [-6.2, 25.6, 0.0],
                    [a, 0.0, 70.0],
                    &[[3.6, 0.0, 0.0]],
                    1.6,
                    0.6,
                    if k % 2 == 0 { Polished } else { Metal },
                );
            }
            b.bx(2, [-7.2, 22.0, 3.0], [-5.8, 23.4, 3.5], Gem);
            b.mirror(m);
            b.cape(18.0, Cloth);
            b.breastplate(Metal, Polished);
            b.bx(1, [-1.0, 20.6, 4.0], [1.0, 22.4, 4.4], Gem);
            b.tassets(Polished, 2);
            b.knees(Polished, true);
            let m = b.mark();
            b.slot(3).blade(
                4,
                [-4.6, 3.0, 0.0],
                [0.0, 0.0, 50.0],
                &[[2.4, 0.0, 0.0]],
                2.2,
                0.35,
                Accent,
            );
            b.mirror(m);
            b.toes(Polished);
            (Wave, Visor::Slit)
        }
        "palladium" => {
            // A lion: a mane round the face, round ears, lion-mask pauldrons.
            p.fur = [150, 78, 30];
            p.accent = [140, 70, 30];
            p.gem = [40, 160, 80];
            b.slot(0);
            b.bx(0, [-6.6, 33.0, 4.4], [6.6, 35.0, 6.0], Fur);
            b.bx(0, [-6.6, 21.6, 4.4], [6.6, 23.2, 6.0], Fur);
            let m = b.mark();
            b.bx(0, [-6.8, 21.6, -2.0], [-5.0, 35.0, 6.0], Fur);
            b.rb(
                0,
                [-3.2, 34.4, 0.6],
                [2.2, 2.0, 0.9],
                [0.0, 0.0, 12.0],
                Metal,
            );
            b.mirror(m);
            b.bx(0, [-6.4, 22.4, -6.4], [6.4, 34.4, -4.8], Fur);
            b.bx(0, [-0.7, 29.4, 5.0], [0.7, 30.6, 5.7], Gem);
            b.pauldrons(2, 6.8, 20.0, Metal, Accent);
            let m = b.mark();
            b.slot(1);
            b.bx(2, [-10.4, 21.2, -2.6], [-9.0, 25.4, 2.6], Fur);
            b.bx(2, [-10.0, 22.0, -1.8], [-9.2, 24.6, 1.8], Polished);
            b.bx(2, [-10.2, 23.4, -1.2], [-9.9, 24.0, -0.5], Gem);
            b.bx(2, [-10.2, 23.4, 0.5], [-9.9, 24.0, 1.2], Gem);
            b.mirror(m);
            b.breastplate(Polished, Accent);
            b.bracers(Accent);
            b.belt(Accent, Polished);
            b.knees(Polished, false);
            b.cuffs(Fur);
            (Mane, Visor::Open)
        }
        "mythril" => {
            // An elf: leaf wings swept back from the temples, a circlet, leaf pauldrons, a cape.
            p.leaf = [110, 200, 150];
            p.cloth = [40, 96, 76];
            p.gem = [210, 250, 230];
            b.brow(Trim);
            b.slot(0)
                .bx(0, [-0.8, 29.1, 5.35], [0.8, 30.7, 5.8], GemGlow);
            let m = b.mark();
            for (i, a) in [0.0, 14.0, 28.0].iter().enumerate() {
                b.blade(
                    0,
                    [-5.6, 30.6 - i as f32 * 1.2, 1.0],
                    [-62.0 - a, 0.0, 26.0],
                    &[[2.4, 0.0, 0.0], [2.4, -6.0, 0.0], [2.2, -8.0, 0.0]],
                    1.7,
                    0.35,
                    Leaf,
                );
            }
            b.mirror(m);
            let m = b.mark();
            b.slot(1);
            for (k, a) in [-30.0, 0.0, 30.0].iter().enumerate() {
                b.blade(
                    2,
                    [-6.0, 25.6 - k as f32 * 0.4, *a * 0.06],
                    [*a, 0.0, 64.0 + k as f32 * 8.0],
                    &[[2.6, 0.0, 0.0], [2.2, 0.0, -10.0]],
                    2.2,
                    0.45,
                    Leaf,
                );
            }
            b.mirror(m);
            b.cape(20.0, Cloth);
            b.breastplate(Leaf, Trim);
            b.slot(2);
            let m = b.mark();
            for (k, x) in [-3.8, -1.4].iter().enumerate() {
                b.blade(
                    1,
                    [*x, 11.8, 3.2],
                    [180.0 - 8.0, 0.0, 6.0 - k as f32 * 6.0],
                    &[[2.4, 0.0, 0.0], [1.8, 0.0, 0.0]],
                    2.0,
                    0.4,
                    Leaf,
                );
            }
            b.mirror(m);
            b.knees(Polished, false);
            b.cuffs(Leaf);
            (Elven, Visor::Open)
        }
        "orichalcum" => {
            // A blossom: a crown of petals, petal pauldrons and skirt, a flower on the chest.
            p.accent = [250, 170, 214];
            p.trim = [250, 214, 96];
            b.slot(0);
            for k in 0..7 {
                let a = k as f32 / 7.0 * 360.0;
                let r = a.to_radians();
                b.blade(
                    0,
                    [r.sin() * 2.6, 33.0, r.cos() * 2.6],
                    [0.0, a, 0.0],
                    &[[2.4, 26.0, 0.0], [1.6, 20.0, 0.0]],
                    2.2,
                    0.4,
                    Accent,
                );
            }
            b.bx(0, [-1.2, 33.0, -1.2], [1.2, 34.6, 1.2], Trim);
            b.slot(1);
            let m = b.mark();
            for (k, a) in [-38.0, 0.0, 38.0].iter().enumerate() {
                b.blade(
                    2,
                    [-6.0, 25.8, 0.0],
                    [*a, 0.0, 78.0 + (k % 2) as f32 * 10.0],
                    &[[2.6, 0.0, 0.0], [2.0, 0.0, 12.0]],
                    2.8,
                    0.45,
                    if k == 1 { Accent } else { Polished },
                );
            }
            b.mirror(m);
            for k in 0..5 {
                let a = (k as f32 / 5.0 * 360.0).to_radians();
                b.rb(
                    1,
                    [a.sin() * 1.4, 19.0 + a.cos() * 1.4, 3.4],
                    [1.5, 2.2, 0.5],
                    [0.0, 0.0, -(k as f32 / 5.0 * 360.0)],
                    Accent,
                );
            }
            b.bx(1, [-0.7, 18.3, 3.6], [0.7, 19.7, 4.0], Trim);
            b.slot(2);
            for k in 0..10 {
                let a = k as f32 / 10.0 * 360.0;
                let r = a.to_radians();
                b.blade(
                    1,
                    [r.sin() * 4.6, 12.0, r.cos() * 2.8],
                    [180.0 + 14.0, a, 0.0],
                    &[[2.4, 0.0, 0.0], [1.6, -10.0, 0.0]],
                    2.2,
                    0.4,
                    if k % 2 == 0 { Accent } else { Polished },
                );
            }
            b.cuffs(Accent);
            (Petal, Visor::Open)
        }
        "adamantite" => {
            // An oni: a demon mask with two tall horns, fangs and heavy brows.
            p.horn = [236, 214, 170];
            p.glow = [255, 214, 90];
            p.accent = [90, 16, 26];
            b.slot(0);
            let m = b.mark();
            b.chain(
                0,
                [-3.0, 32.6, 1.0],
                [8.0, 0.0, 14.0],
                &[
                    [2.6, 1.8, 0.0, 0.0],
                    [2.6, 1.4, 8.0, -10.0],
                    [2.2, 1.0, 10.0, -12.0],
                    [1.6, 0.6, 10.0, -10.0],
                ],
                Horn,
            );
            b.rb(
                0,
                [-2.4, 30.3, 5.4],
                [3.4, 1.2, 1.0],
                [0.0, 0.0, -16.0],
                Accent,
            );
            b.bx(0, [-2.6, 28.4, 5.05], [-1.4, 29.2, 5.3], Glow);
            b.chain(
                0,
                [-2.2, 24.6, 5.4],
                [180.0, 0.0, 0.0],
                &[[1.6, 0.8, 0.0, 0.0], [1.0, 0.5, 0.0, 0.0]],
                Horn,
            );
            b.mirror(m);
            b.bx(0, [-2.0, 24.2, 5.0], [2.0, 25.2, 5.6], Dark);
            b.pauldrons(3, 7.2, 14.0, Metal, Accent);
            let m = b.mark();
            b.slot(1);
            for (dz, h) in [(-1.6, 3.2), (1.6, 3.2)] {
                b.rb(
                    2,
                    [-7.4, 27.0 + h * 0.3, dz],
                    [1.2, h, 1.2],
                    [0.0, 0.0, 10.0],
                    Horn,
                );
            }
            for y in [13.0, 14.4] {
                b.rb(2, [-8.8, y, 0.0], [1.2, 0.8, 0.8], [0.0, 0.0, 90.0], Horn);
            }
            b.mirror(m);
            b.bracers(Accent);
            b.slot(1).bx(1, [-1.6, 17.0, 3.0], [1.6, 21.0, 3.6], Accent);
            b.bx(1, [-0.6, 18.6, 3.6], [0.6, 19.6, 4.0], Glow);
            b.tassets(Accent, 2);
            b.knees(Metal, true);
            b.toes(Accent);
            b.cuffs(Metal);
            (Spiked, Visor::Eyes)
        }
        "titanium" => {
            // A sleek machine: a glowing visor band, swept antenna fins, a core in the chest.
            p.glow = [110, 220, 255];
            p.accent = [60, 64, 78];
            b.slot(0).bx(0, [-4.6, 27.9, 4.9], [4.6, 29.3, 5.4], Glow);
            let m = b.mark();
            b.blade(
                0,
                [-5.3, 29.0, -1.0],
                [-70.0, 0.0, 14.0],
                &[[3.6, 0.0, 0.0], [2.4, -10.0, 0.0]],
                1.0,
                0.4,
                Polished,
            );
            b.blade(
                0,
                [-5.3, 31.0, -1.6],
                [-58.0, 0.0, 10.0],
                &[[2.8, 0.0, 0.0]],
                0.7,
                0.4,
                Polished,
            );
            b.mirror(m);
            b.bx(0, [-0.4, 33.0, -4.0], [0.4, 33.8, 3.0], Glow);
            b.slot(1);
            let m = b.mark();
            b.rb(
                2,
                [-6.6, 25.2, 0.0],
                [7.4, 1.6, 7.6],
                [0.0, 0.0, 26.0],
                Polished,
            );
            b.rb(
                2,
                [-9.2, 22.4, 0.0],
                [0.6, 3.2, 6.0],
                [0.0, 0.0, 8.0],
                Accent,
            );
            b.bx(2, [-9.0, 20.0, -0.4], [-8.6, 22.6, 0.4], Glow);
            b.mirror(m);
            b.bx(1, [-1.4, 17.8, 3.0], [1.4, 20.6, 3.7], Accent);
            b.bx(1, [-0.9, 18.3, 3.7], [0.9, 20.1, 4.0], Glow);
            for x in [-3.2, 2.6] {
                b.bx(1, [x, 12.2, 3.0], [x + 0.6, 17.4, 3.4], Glow);
            }
            b.bracers(Polished);
            b.slot(2);
            let m = b.mark();
            b.bx(4, [-4.1, 2.4, 2.6], [-3.6, 11.0, 3.0], Glow);
            b.mirror(m);
            b.knees(Polished, true);
            b.toes(Polished);
            b.cuffs(Accent);
            (Hex, Visor::Slit)
        }
        "radiant" => {
            // Holy: a floating halo, small wings at the temples, a white tabard with a gold sun.
            p.glow = [255, 236, 150];
            p.cloth = [240, 236, 222];
            p.hair = [246, 244, 236];
            b.slot(0)
                .ring(0, [0.0, 36.0, -0.6], 4.6, 14, [2.2, 0.7, 0.7], -10.0, Glow);
            let m = b.mark();
            for (i, a) in [0.0, 16.0, 32.0].iter().enumerate() {
                b.blade(
                    0,
                    [-5.5, 29.6 + i as f32 * 0.6, -0.2 - i as f32 * 0.6],
                    [-30.0 - a, 0.0, 40.0],
                    &[[2.0, 0.0, 0.0], [1.8, -12.0, 0.0]],
                    1.2,
                    0.3,
                    Hair,
                );
            }
            b.mirror(m);
            b.slot(1);
            let m = b.mark();
            for (k, a) in [-20.0, 0.0, 20.0, 40.0].iter().enumerate() {
                b.blade(
                    2,
                    [-7.0, 25.0, -(k as f32) * 0.7],
                    [-40.0 - a, 0.0, 66.0],
                    &[[2.4, 0.0, 0.0], [2.0, -10.0, 0.0]],
                    1.4,
                    0.35,
                    Hair,
                );
            }
            b.mirror(m);
            b.pauldrons(2, 6.4, 22.0, Polished, Trim);
            for z in [3.0, -3.5] {
                b.bx(1, [-3.4, 8.0, z], [3.4, 24.4, z + 0.5], Cloth);
            }
            b.bx(1, [-1.8, 17.0, 3.5], [1.8, 20.6, 3.9], Trim);
            b.bx(1, [-0.8, 18.0, 3.9], [0.8, 19.6, 4.2], Glow);
            b.belt(Trim, Glow);
            b.knees(Polished, true);
            let m = b.mark();
            b.slot(3).blade(
                4,
                [-4.8, 3.6, -1.0],
                [-40.0, 0.0, 50.0],
                &[[2.0, 0.0, 0.0]],
                1.2,
                0.3,
                Hair,
            );
            b.mirror(m);
            b.cuffs(Trim);
            (Ornate, Visor::Slit)
        }
        "verdite" => {
            // A living plant: a crown of leaves with a bud, leaves bursting from the shoulders,
            // vines round the legs and roots at the toes.
            p.leaf = [96, 196, 74];
            p.accent = [92, 64, 40];
            p.gem = [236, 120, 170];
            b.slot(0);
            for k in 0..6 {
                let a = k as f32 / 6.0 * 360.0 + 30.0;
                let r = a.to_radians();
                b.blade(
                    0,
                    [r.sin() * 3.6, 32.8, r.cos() * 3.6],
                    [0.0, a, 0.0],
                    &[[2.6, 22.0, 0.0], [2.2, 20.0, 0.0]],
                    2.0,
                    0.4,
                    Leaf,
                );
            }
            b.bx(0, [-1.0, 33.0, -1.0], [1.0, 35.4, 1.0], Leaf);
            b.rb(0, [0.0, 36.0, 0.0], [1.8, 1.8, 1.8], [0.0, 45.0, 0.0], Gem);
            b.slot(1);
            let m = b.mark();
            for (k, (a, y)) in [(-40.0, 25.4), (0.0, 25.8), (40.0, 25.4), (-10.0, 23.0)]
                .iter()
                .enumerate()
            {
                b.blade(
                    2,
                    [-6.4, *y, *a * 0.05],
                    [*a, 0.0, 30.0 + k as f32 * 12.0],
                    &[[2.4, 0.0, 0.0], [2.2, 0.0, 14.0]],
                    2.0,
                    0.4,
                    Leaf,
                );
            }
            b.mirror(m);
            b.ring(1, [0.0, 17.0, 0.0], 4.6, 10, [3.0, 0.7, 0.7], 18.0, Accent);
            b.slot(2);
            let m = b.mark();
            b.ring(4, [-2.0, 4.0, 0.0], 2.9, 6, [3.0, 0.7, 0.6], -14.0, Accent);
            b.blade(
                4,
                [-2.0, 9.6, 2.8],
                [30.0, 0.0, 10.0],
                &[[1.8, 0.0, 0.0]],
                1.6,
                0.35,
                Leaf,
            );
            b.mirror(m);
            let m = b.mark();
            b.slot(3);
            for dx in [-4.0, -1.0] {
                b.chain(
                    4,
                    [dx, 0.2, 2.8],
                    [100.0, 0.0, 0.0],
                    &[[1.4, 0.8, 0.0, 0.0], [1.2, 0.6, 20.0, 10.0]],
                    Accent,
                );
            }
            b.mirror(m);
            (Bark, Visor::Open)
        }
        "lumite" => {
            // Celestial: a glowing third eye, a crown of floating shards behind the head, shards
            // floating over the shoulders, glowing seams.
            p.glow = [140, 250, 230];
            p.gem = [120, 250, 230];
            p.accent = [26, 44, 60];
            b.slot(0).rb(
                0,
                [0.0, 30.0, 5.4],
                [1.6, 1.6, 0.6],
                [0.0, 0.0, 45.0],
                GemGlow,
            );
            for k in 0..5 {
                let a = (k as f32 / 4.0 - 0.5) * 140.0;
                let r = a.to_radians();
                b.rb(
                    0,
                    [r.sin() * 5.4, 32.0 + r.cos() * 4.6, -6.2],
                    [1.0, 3.0 - (k as f32 - 2.0).abs() * 0.5, 1.0],
                    [0.0, 0.0, -a],
                    GemGlow,
                );
            }
            b.slot(1);
            let m = b.mark();
            b.rb(
                2,
                [-7.0, 29.0, 0.0],
                [1.4, 3.2, 1.4],
                [20.0, 45.0, 14.0],
                GemGlow,
            );
            b.rb(
                2,
                [-8.6, 27.4, 1.8],
                [0.9, 1.8, 0.9],
                [-10.0, 45.0, 28.0],
                GemGlow,
            );
            b.mirror(m);
            b.pauldrons(2, 6.6, 18.0, Metal, Accent);
            b.bx(1, [-0.4, 11.6, 3.0], [0.4, 24.0, 3.4], Glow);
            b.bx(1, [-4.6, 17.6, 3.0], [4.6, 18.2, 3.4], Glow);
            b.knees(Accent, false);
            b.toes(Accent);
            (Cosmic, Visor::Slit)
        }
        // ---- Terraria-inspired early and evil sets ----
        "meteor" => {
            // A space traveller: a glass dome, a lamp on an antenna, rocks on the shoulders, air
            // tanks on the back.
            p.glass = [40, 70, 96];
            p.glow = [255, 140, 60];
            p.accent = [60, 50, 46];
            b.slot(0).bx(0, [-4.2, 23.6, 4.9], [4.2, 31.6, 5.7], Glass);
            b.bx(0, [-5.6, 22.6, -5.6], [5.6, 24.0, 5.6], Polished);
            b.chain(
                0,
                [-3.0, 33.0, -2.0],
                [0.0, 0.0, 10.0],
                &[[3.4, 0.4, 0.0, 0.0]],
                Polished,
            );
            b.rb(
                0,
                [-3.6, 36.6, -2.0],
                [0.9, 0.9, 0.9],
                [45.0, 45.0, 0.0],
                Glow,
            );
            b.slot(1);
            let m = b.mark();
            b.rb(
                2,
                [-6.6, 25.6, 0.4],
                [4.6, 2.6, 4.4],
                [12.0, 24.0, 20.0],
                Accent,
            );
            b.rb(
                2,
                [-7.6, 26.6, -1.4],
                [2.6, 2.0, 2.4],
                [30.0, 10.0, 40.0],
                Accent,
            );
            b.rb(
                2,
                [-6.8, 26.9, 0.9],
                [0.5, 1.6, 2.4],
                [12.0, 24.0, 20.0],
                Flame,
            );
            b.bx(1, [-3.4, 13.0, -6.0], [-0.6, 23.0, -3.2], Polished);
            b.mirror(m);
            b.bx(1, [-2.0, 17.0, 3.0], [2.0, 20.6, 3.6], Accent);
            b.bx(1, [-1.2, 17.6, 3.6], [1.2, 20.0, 3.9], Flame);
            b.belt(Accent, Polished);
            b.knees(Accent, false);
            b.cuffs(Polished);
            (Rock, Visor::Closed)
        }
        "umbral" => {
            // Shadow: a hood with burning purple eyes, a ragged cape, thorns on the shoulders.
            p.cloth = [44, 30, 64];
            p.glow = [210, 120, 255];
            p.horn = [60, 40, 90];
            b.hood(Cloth, true);
            b.slot(0);
            b.bx(0, [-4.6, 23.4, 4.6], [4.6, 29.0, 5.4], Cloth);
            for x in [-2.8, 1.2] {
                b.bx(0, [x, 29.4, 5.0], [x + 1.6, 30.2, 5.5], Glow);
            }
            b.slot(1);
            let m = b.mark();
            for (dz, h, t) in [(-1.6, 4.0, 30.0), (0.4, 5.0, 18.0), (2.0, 3.4, 8.0)] {
                b.rb(
                    2,
                    [-7.0, 27.0 + h * 0.3, dz],
                    [0.8, h, 0.8],
                    [0.0, 0.0, t],
                    Horn,
                );
            }
            b.mirror(m);
            for (k, len) in [15.0, 19.0, 16.0, 21.0, 14.0, 18.0].iter().enumerate() {
                let x = -4.2 + k as f32 * 1.68;
                b.rb(
                    1,
                    [x, 25.0 - len * 0.5, -3.9 - len * 0.04],
                    [1.6, *len, 0.4],
                    [6.0, 0.0, 0.0],
                    Cloth,
                );
            }
            b.bracers(Dark);
            b.knees(Dark, false);
            b.toes(Dark);
            (Veins, Visor::Closed)
        }
        "sanguinite" => {
            // Flesh and bone: a crown of teeth, one great eye on the brow, a rib cage over the
            // chest, toothy lumps on the shoulders.
            p.horn = [232, 214, 190];
            p.gem = [255, 210, 60];
            p.accent = [92, 14, 20];
            b.slot(0);
            for k in 0..9 {
                let a = k as f32 / 9.0 * 360.0;
                let r = a.to_radians();
                b.rb(
                    0,
                    [r.sin() * 4.6, 33.8, r.cos() * 4.6],
                    [1.0, 2.4, 1.0],
                    [r.cos() * 18.0, 0.0, -r.sin() * 18.0],
                    Horn,
                );
            }
            b.rb(
                0,
                [0.0, 31.0, 5.4],
                [3.0, 2.2, 0.8],
                [0.0, 0.0, 0.0],
                Accent,
            );
            b.bx(0, [-1.0, 30.4, 5.8], [1.0, 31.6, 6.1], GemGlow);
            b.slot(1);
            let m = b.mark();
            for (i, y) in [22.0, 19.6, 17.2, 14.8].iter().enumerate() {
                b.rb(
                    1,
                    [-2.5, *y, 3.5],
                    [4.2 - i as f32 * 0.3, 0.9, 0.7],
                    [0.0, 0.0, 14.0],
                    Horn,
                );
            }
            b.rb(
                2,
                [-6.6, 25.6, 0.0],
                [5.4, 2.6, 5.8],
                [0.0, 0.0, 16.0],
                Accent,
            );
            for dz in [-1.6, 0.0, 1.6] {
                b.rb(2, [-7.8, 27.0, dz], [0.7, 1.6, 0.7], [0.0, 0.0, 22.0], Horn);
            }
            b.mirror(m);
            b.bx(1, [-0.6, 13.6, 3.2], [0.6, 23.4, 3.8], Horn);
            b.tassets(Accent, 2);
            b.knees(Horn, false);
            b.toes(Accent);
            (Veins, Visor::Eyes)
        }
        "hive" => {
            // A bee: antennae, a fuzzy collar, glassy wings on the back, a stinger.
            p.accent = [44, 30, 18];
            p.fur = [250, 206, 70];
            p.glass = [190, 214, 226];
            let m = b.mark();
            b.slot(0).chain(
                0,
                [-1.8, 33.0, 3.0],
                [24.0, 0.0, 18.0],
                &[[2.4, 0.5, 0.0, 0.0], [2.2, 0.5, 30.0, 0.0]],
                Accent,
            );
            b.rb(
                0,
                [-3.3, 37.0, 5.8],
                [1.2, 1.2, 1.2],
                [0.0, 45.0, 0.0],
                Accent,
            );
            b.mirror(m);
            b.slot(1).bx(1, [-5.6, 23.3, -3.7], [5.6, 25.6, 3.7], Fur);
            let m = b.mark();
            b.rb(
                1,
                [-4.2, 21.6, -4.2],
                [7.0, 3.4, 0.3],
                [0.0, 30.0, 30.0],
                Glass,
            );
            b.rb(
                1,
                [-3.4, 18.6, -4.0],
                [5.0, 2.6, 0.3],
                [0.0, 26.0, -10.0],
                Glass,
            );
            b.mirror(m);
            b.pauldrons(1, 6.4, 20.0, Fur, Fur);
            b.slot(2).chain(
                1,
                [0.0, 11.6, -3.0],
                [-130.0, 0.0, 0.0],
                &[
                    [1.6, 1.6, 0.0, 0.0],
                    [1.6, 0.9, 10.0, 0.0],
                    [1.2, 0.4, 10.0, 0.0],
                ],
                Accent,
            );
            b.belt(Accent, Polished);
            b.cuffs(Fur);
            (Stripes, Visor::Open)
        }
        "jungle" => {
            // Tribal: a fan of feathers on the head, a tooth necklace, a leaf skirt.
            p.hair = [196, 46, 46];
            p.leaf = [80, 160, 60];
            p.accent = [96, 66, 38];
            b.brow(Accent);
            b.slot(0);
            for k in 0..7 {
                let a = (k as f32 / 6.0 - 0.5) * 120.0;
                b.blade(
                    0,
                    [a.to_radians().sin() * 3.6, 32.0, -2.4],
                    [-12.0, 0.0, -a],
                    &[[2.6, 0.0, 0.0], [2.4, -6.0, 0.0]],
                    1.2,
                    0.3,
                    if k % 2 == 0 { Hair } else { Leaf },
                );
            }
            b.slot(1);
            for k in 0..7 {
                let a = (k as f32 / 6.0 - 0.5) * 2.0;
                b.rb(
                    1,
                    [a.sin() * 4.0, 23.6 - a.cos() * 1.6, 3.4],
                    [0.6, 1.4, 0.6],
                    [0.0, 0.0, -a.to_degrees()],
                    Horn,
                );
            }
            let m = b.mark();
            b.blade(
                2,
                [-6.2, 25.6, 0.0],
                [0.0, 0.0, 72.0],
                &[[2.6, 0.0, 0.0], [2.0, 0.0, 14.0]],
                2.4,
                0.4,
                Leaf,
            );
            b.mirror(m);
            b.slot(2);
            for k in 0..12 {
                let a = k as f32 / 12.0 * 360.0;
                let r = a.to_radians();
                b.blade(
                    1,
                    [r.sin() * 4.6, 12.2, r.cos() * 2.8],
                    [180.0 + 10.0, a, 0.0],
                    &[[2.6, 0.0, 0.0], [2.2, -6.0, 0.0]],
                    1.8,
                    0.35,
                    Leaf,
                );
            }
            b.cuffs(Leaf);
            (Bark, Visor::Open)
        }
        "fossil" => {
            // Fossil: a raptor-skull brow, ammonite spirals on the shoulders, a spine down the back.
            p.horn = [222, 206, 170];
            p.accent = [140, 120, 90];
            b.slot(0);
            b.bx(0, [-4.0, 29.2, 4.8], [4.0, 31.6, 7.0], Horn);
            for x in [-3.0, -1.3, 0.5, 2.2] {
                b.bx(0, [x, 28.4, 6.2], [x + 0.7, 29.2, 6.9], Horn);
            }
            let m = b.mark();
            b.bx(0, [-3.6, 30.0, 7.0], [-2.0, 31.0, 7.2], Dark);
            b.chain(
                0,
                [-4.6, 32.0, -2.0],
                [-70.0, 0.0, 20.0],
                &[[1.8, 1.0, 0.0, 0.0], [1.4, 0.6, -20.0, 0.0]],
                Horn,
            );
            b.mirror(m);
            b.slot(1);
            let m = b.mark();
            b.ring(2, [-9.0, 23.4, 0.0], 2.4, 8, [2.0, 1.0, 1.0], 0.0, Horn);
            b.ring(2, [-9.0, 23.4, 0.0], 1.2, 5, [1.4, 0.9, 0.9], 0.0, Accent);
            b.mirror(m);
            for i in 0..6 {
                b.bx(
                    1,
                    [-0.9, 13.0 + i as f32 * 2.0, -3.8],
                    [0.9, 14.2 + i as f32 * 2.0, -3.0],
                    Horn,
                );
                b.rb(
                    1,
                    [0.0, 13.6 + i as f32 * 2.0, -4.4],
                    [0.6, 1.2, 1.0],
                    [-40.0, 0.0, 0.0],
                    Horn,
                );
            }
            b.pauldrons(1, 6.4, 18.0, Horn, Horn);
            b.knees(Horn, false);
            let m = b.mark();
            b.slot(3);
            for dx in [-3.6, -0.8] {
                b.chain(
                    4,
                    [dx, 0.4, 3.0],
                    [80.0, 0.0, 0.0],
                    &[[1.2, 0.7, 0.0, 0.0], [1.0, 0.5, 40.0, 0.0]],
                    Horn,
                );
            }
            b.mirror(m);
            (Bone, Visor::Open)
        }
        "necro" => {
            // A skeleton: a skull helm with teeth, a rib cage, bone pauldrons, a tattered cloak.
            p.horn = [226, 222, 206];
            p.cloth = [46, 40, 40];
            b.slot(0);
            for x in [-3.0, -1.6, -0.2, 1.2, 2.6] {
                b.bx(0, [x - 0.5, 23.2, 4.9], [x + 0.5, 25.0, 5.6], Horn);
            }
            b.bx(0, [-0.6, 26.4, 5.0], [0.6, 27.6, 5.4], Dark);
            b.slot(1);
            let m = b.mark();
            for (i, y) in [22.4, 20.0, 17.6, 15.2].iter().enumerate() {
                b.rb(
                    1,
                    [-2.4, *y, 3.4],
                    [3.8 - i as f32 * 0.3, 0.9, 0.6],
                    [0.0, 0.0, 16.0],
                    Horn,
                );
            }
            b.rb(
                2,
                [-6.6, 25.4, 0.0],
                [4.8, 2.4, 5.8],
                [0.0, 0.0, 18.0],
                Horn,
            );
            for dz in [-1.8, 0.0, 1.8] {
                b.rb(2, [-8.4, 26.2, dz], [0.7, 2.2, 0.7], [0.0, 0.0, 50.0], Horn);
            }
            b.mirror(m);
            b.bx(1, [-0.5, 13.0, 3.0], [0.5, 23.4, 3.6], Horn);
            for (k, len) in [16.0, 20.0, 15.0, 19.0, 17.0].iter().enumerate() {
                let x = -3.8 + k as f32 * 1.9;
                b.rb(
                    1,
                    [x, 25.0 - len * 0.5, -3.9 - len * 0.05],
                    [1.8, *len, 0.4],
                    [7.0, 0.0, 0.0],
                    Cloth,
                );
            }
            b.knees(Horn, false);
            b.toes(Horn);
            (Bone, Visor::Eyes)
        }
        "ninja" => {
            // A ninja: a headband whose tails stream out behind, a sash, wrapped shins.
            p.accent = [176, 32, 36];
            b.brow(Accent);
            b.slot(0);
            b.blade(
                0,
                [-0.6, 29.9, -5.4],
                [-100.0, 0.0, 12.0],
                &[[2.6, 0.0, 0.0], [2.6, 12.0, 4.0], [2.4, 14.0, 6.0]],
                1.1,
                0.3,
                Accent,
            );
            b.blade(
                0,
                [0.6, 29.9, -5.4],
                [-108.0, 0.0, -14.0],
                &[[2.6, 0.0, 0.0], [2.4, 10.0, -6.0], [2.2, 14.0, -6.0]],
                1.0,
                0.3,
                Accent,
            );
            b.slot(1);
            b.rb(
                1,
                [0.0, 18.6, 0.0],
                [10.8, 1.2, 6.6],
                [0.0, 0.0, -30.0],
                Dark,
            );
            let m = b.mark();
            b.bx(2, [-9.3, 21.8, -3.3], [-2.7, 25.6, 3.3], Dark);
            b.mirror(m);
            b.slot(2)
                .bx(1, [-5.1, 11.2, -3.1], [5.1, 13.0, 3.1], Accent);
            b.blade(
                1,
                [3.6, 11.4, 3.2],
                [180.0, 0.0, -10.0],
                &[[2.6, 0.0, 0.0], [2.2, 0.0, 6.0]],
                1.2,
                0.3,
                Accent,
            );
            let m = b.mark();
            b.slot(3);
            for y in [1.8, 3.4] {
                b.rb(4, [-2.0, y, 0.0], [5.6, 0.9, 5.6], [0.0, 0.0, 8.0], Dark);
            }
            b.mirror(m);
            (Wrap, Visor::Band)
        }
        // ---- End-game sets ----
        "spectral" => {
            // A spectre: a hood over a pale glowing mask, a robe that thins into a wisp.
            p.cloth = [36, 60, 98];
            p.glow = [190, 230, 255];
            b.hood(Cloth, false);
            b.slot(0)
                .bx(0, [-4.0, 23.6, 4.8], [4.0, 30.4, 5.4], Polished);
            for x in [-2.8, 1.2] {
                b.bx(0, [x, 27.6, 5.3], [x + 1.6, 28.6, 5.6], Glow);
            }
            b.bx(0, [-1.0, 24.6, 5.3], [1.0, 25.2, 5.6], Dark);
            b.slot(1).bx(1, [-5.8, 22.6, -3.8], [5.8, 25.6, 3.8], Cloth);
            b.skirt(12.6, 3.0, Cloth, None);
            b.slot(2).chain(
                1,
                [0.0, 4.6, -2.6],
                [-150.0, 0.0, 0.0],
                &[
                    [2.4, 3.6, 0.0, 0.0],
                    [2.4, 2.4, 20.0, 10.0],
                    [2.2, 1.4, 20.0, 10.0],
                    [2.0, 0.7, 20.0, 0.0],
                ],
                Glow,
            );
            b.cuffs(Cloth);
            (Cosmic, Visor::Closed)
        }
        "sporite" => {
            // A mushroom: a wide cap with glowing spots and gills, little caps on the shoulders.
            p.accent = [210, 196, 170];
            p.glow = [90, 160, 255];
            b.slot(0);
            b.bx(0, [-7.4, 32.6, -7.4], [7.4, 34.0, 7.4], Metal);
            b.bx(0, [-7.2, 32.0, -7.2], [7.2, 32.6, 7.2], Dark);
            b.bx(0, [-5.8, 34.0, -5.8], [5.8, 35.6, 5.8], Metal);
            b.bx(0, [-3.4, 35.6, -3.4], [3.4, 36.6, 3.4], Metal);
            for (x, y, z) in [
                (-5.0, 34.0, 5.0),
                (3.0, 35.6, 4.2),
                (5.6, 34.0, -1.6),
                (-2.0, 36.6, -1.0),
                (-6.4, 34.0, -3.0),
                (1.0, 35.6, -5.0),
                (4.0, 34.0, 6.4),
            ] {
                b.bx(
                    0,
                    [x - 0.7, y - 0.2, z - 0.7],
                    [x + 0.7, y + 0.35, z + 0.7],
                    Glow,
                );
            }
            b.slot(1);
            let m = b.mark();
            b.bx(2, [-10.0, 25.4, -3.6], [-3.0, 26.6, 3.6], Metal);
            b.bx(2, [-8.6, 26.6, -2.2], [-4.4, 27.4, 2.2], Metal);
            b.bx(2, [-8.4, 26.4, 1.6], [-7.4, 27.0, 2.6], Glow);
            b.bx(2, [-6.0, 27.2, -1.0], [-5.0, 27.7, 0.0], Glow);
            b.mirror(m);
            b.bx(1, [-1.0, 17.6, 3.0], [1.0, 19.6, 3.5], Glow);
            b.knees(Accent, false);
            b.cuffs(Accent);
            (Spots, Visor::Open)
        }
        "beetle" => {
            // A rhinoceros beetle: a great curved horn, wing cases on the back, a shell all over.
            p.horn = [60, 30, 70];
            p.accent = [40, 20, 50];
            b.slot(0).chain(
                0,
                [0.0, 30.6, 5.0],
                [72.0, 0.0, 0.0],
                &[
                    [2.2, 3.0, 0.0, 0.0],
                    [2.0, 2.6, -28.0, 0.0],
                    [1.8, 2.0, -30.0, 0.0],
                    [1.4, 1.3, -30.0, 0.0],
                ],
                Polished,
            );
            let m = b.mark();
            b.rb(
                0,
                [-2.6, 33.2, 0.0],
                [1.0, 1.8, 1.0],
                [0.0, 0.0, 20.0],
                Polished,
            );
            b.mirror(m);
            b.slot(1);
            let m = b.mark();
            b.rb(
                1,
                [-2.6, 17.4, -4.0],
                [5.4, 15.0, 1.6],
                [6.0, -8.0, 3.0],
                Polished,
            );
            b.rb(
                1,
                [-2.6, 17.4, -3.2],
                [4.6, 13.8, 0.4],
                [6.0, -8.0, 3.0],
                Glass,
            );
            b.mirror(m);
            b.pauldrons(2, 7.0, 16.0, Polished, Accent);
            b.breastplate(Polished, Accent);
            b.knees(Polished, true);
            b.toes(Polished);
            (Shell, Visor::Slit)
        }
        "turtle" => {
            // A turtle: a domed shell on the back, a beaked brow, shell plates on the shoulders.
            p.accent = [140, 120, 52];
            p.horn = [200, 186, 120];
            b.slot(0).rb(
                0,
                [0.0, 29.6, 5.6],
                [8.0, 1.6, 2.0],
                [-20.0, 0.0, 0.0],
                Accent,
            );
            b.rb(0, [0.0, 28.8, 6.6], [2.2, 1.6, 1.6], [0.0, 45.0, 0.0], Horn);
            b.slot(1);
            b.bx(1, [-6.2, 10.4, -5.0], [6.2, 24.4, -3.2], Scales);
            b.bx(1, [-5.4, 11.4, -6.4], [5.4, 23.4, -5.0], Scales);
            b.bx(1, [-3.8, 13.0, -7.4], [3.8, 21.8, -6.4], Accent);
            b.bx(1, [-6.6, 10.0, -4.4], [6.6, 11.0, -3.0], Horn);
            let m = b.mark();
            b.rb(
                2,
                [-6.6, 25.2, 0.0],
                [6.6, 2.0, 7.2],
                [0.0, 0.0, 18.0],
                Scales,
            );
            b.mirror(m);
            b.knees(Accent, false);
            b.toes(Horn);
            (Hex, Visor::Open)
        }
        "solar" => {
            // The sun: a crown of flame rays, a blazing core, flames licking up from the shoulders.
            p.glow = [255, 200, 90];
            p.accent = [120, 30, 6];
            b.slot(0);
            for k in 0..9 {
                let a = (k as f32 / 8.0 - 0.5) * 200.0;
                let r = a.to_radians();
                let len = if k % 2 == 0 { 4.4 } else { 3.0 };
                b.rb(
                    0,
                    [
                        r.sin() * (5.4 + len * 0.5),
                        29.0 + r.cos() * (5.4 + len * 0.5),
                        -5.4,
                    ],
                    [1.0, len, 0.6],
                    [0.0, 0.0, -a],
                    Flame,
                );
            }
            b.bx(0, [-4.6, 27.9, 4.9], [4.6, 29.3, 5.4], Flame);
            b.slot(1);
            let m = b.mark();
            for (dz, h, t) in [(-1.8, 3.0, 26.0), (0.0, 4.2, 12.0), (1.8, 3.0, 20.0)] {
                b.rb(
                    2,
                    [-7.0, 26.0 + h * 0.4, dz],
                    [1.6, h, 1.4],
                    [0.0, 0.0, t],
                    Flame,
                );
            }
            b.mirror(m);
            b.pauldrons(2, 6.8, 18.0, Metal, Accent);
            b.bx(1, [-1.8, 17.2, 3.0], [1.8, 20.8, 3.7], Flame);
            b.breastplate(Metal, Accent);
            b.tassets(Accent, 2);
            b.knees(Metal, true);
            let m = b.mark();
            b.slot(3).rb(
                4,
                [-4.8, 3.2, -1.0],
                [0.6, 2.4, 1.6],
                [0.0, 0.0, 30.0],
                Flame,
            );
            b.mirror(m);
            (Flare, Visor::Slit)
        }
        "vortex" => {
            // A hunter: a swept fin hood, a glowing visor, a scarf streaming back, energy rings.
            p.glow = [120, 255, 200];
            p.accent = [16, 60, 56];
            b.slot(0).bx(0, [-4.6, 27.9, 4.9], [4.6, 29.3, 5.4], Glow);
            b.blade(
                0,
                [0.0, 32.6, 3.0],
                [-70.0, 0.0, 0.0],
                &[[3.0, 0.0, 0.0], [3.0, -12.0, 0.0], [2.6, -14.0, 0.0]],
                1.0,
                2.2,
                Polished,
            );
            b.slot(1)
                .bx(1, [-5.6, 23.0, -3.7], [5.6, 25.8, 3.7], Accent);
            b.blade(
                1,
                [2.0, 24.6, -3.8],
                [-110.0, 0.0, -14.0],
                &[[3.0, 0.0, 0.0], [3.0, 14.0, -6.0], [2.6, 14.0, -6.0]],
                2.0,
                0.35,
                Accent,
            );
            let m = b.mark();
            b.ring(2, [-6.0, 19.0, 0.0], 3.6, 10, [2.4, 0.5, 0.5], 0.0, Glow);
            b.mirror(m);
            b.pauldrons(1, 6.6, 26.0, Polished, Polished);
            b.slot(2);
            let m = b.mark();
            b.ring(4, [-2.0, 6.0, 0.0], 3.2, 8, [2.6, 0.5, 0.5], 0.0, Glow);
            b.mirror(m);
            b.toes(Accent);
            (Swirl, Visor::Slit)
        }
        "nebula" => {
            // A star mage: a floating crescent behind the head with an orb, a high collar, a robe.
            p.glow = [255, 140, 230];
            p.gem = [200, 120, 255];
            p.cloth = [70, 24, 96];
            p.trim = [240, 190, 240];
            b.slot(0);
            for k in 0..9 {
                let a = (k as f32 / 8.0 - 0.5) * 220.0;
                let r = a.to_radians();
                b.rb(
                    0,
                    [r.sin() * 5.6, 30.0 + r.cos() * 5.6, -6.4],
                    [2.0, 1.4 - (k as f32 - 4.0).abs() * 0.12, 0.6],
                    [0.0, 0.0, -a],
                    Glow,
                );
            }
            b.rb(
                0,
                [0.0, 37.2, -6.4],
                [1.6, 1.6, 1.6],
                [45.0, 45.0, 0.0],
                GemGlow,
            );
            b.slot(1);
            b.bx(1, [-5.4, 24.0, -3.6], [5.4, 28.0, -2.8], Trim);
            let m = b.mark();
            b.rb(
                1,
                [-5.0, 26.0, -0.6],
                [0.8, 4.0, 5.0],
                [0.0, 20.0, -14.0],
                Trim,
            );
            b.mirror(m);
            b.pauldrons(1, 6.4, 22.0, Cloth, Cloth);
            b.skirt(12.6, 2.6, Cloth, Some(Glow));
            b.cuffs(Trim);
            (Cosmic, Visor::Open)
        }
        "stardust" => {
            // A summoner of stars: a five-pointed star on the brow, stars on the shoulders.
            p.glow = [150, 220, 255];
            p.accent = [20, 48, 96];
            b.slot(0);
            for k in 0..5 {
                let a = k as f32 * 72.0;
                let r = a.to_radians();
                b.rb(
                    0,
                    [r.sin() * 1.7, 31.6 + r.cos() * 1.7, 5.4],
                    [1.0, 2.6, 0.6],
                    [0.0, 0.0, -a],
                    Glow,
                );
            }
            for k in 0..4 {
                let a = (k as f32 / 3.0 - 0.5) * 120.0;
                let r = a.to_radians();
                b.rb(
                    0,
                    [r.sin() * 4.6, 33.0 + r.cos() * 2.0, -3.0],
                    [0.7, 3.0, 0.7],
                    [-20.0, 0.0, -a],
                    Polished,
                );
            }
            b.slot(1);
            let m = b.mark();
            for k in 0..4 {
                let a = k as f32 * 90.0;
                b.rb(2, [-6.6, 27.6, 0.0], [0.7, 3.4, 0.7], [0.0, 90.0, a], Glow);
            }
            b.mirror(m);
            b.pauldrons(2, 6.6, 20.0, Metal, Accent);
            b.bx(1, [-0.3, 12.0, 3.0], [0.3, 24.0, 3.4], Glow);
            b.knees(Accent, true);
            b.toes(Polished);
            (Stars, Visor::Slit)
        }
        // ---- The mod pack's dimensions ----
        "zanite" => {
            // Gem-set plate: big amethyst cabochons and crystal spikes at the temples.
            p.gem = [190, 120, 255];
            b.slot(0);
            let m = b.mark();
            b.rb(
                0,
                [-5.6, 32.0, 0.0],
                [1.2, 3.4, 1.2],
                [0.0, 45.0, 30.0],
                Gem,
            );
            b.mirror(m);
            b.bx(0, [-1.0, 29.3, 4.9], [1.0, 31.0, 5.8], Gem);
            b.pauldrons(2, 6.6, 18.0, Polished, Metal);
            let m = b.mark();
            b.slot(1).rb(
                2,
                [-6.4, 26.6, 0.0],
                [2.2, 1.4, 2.2],
                [0.0, 45.0, 18.0],
                Gem,
            );
            b.mirror(m);
            b.breastplate(Metal, Polished);
            b.bx(1, [-1.6, 17.2, 3.9], [1.6, 20.4, 4.6], Gem);
            let m = b.mark();
            b.slot(2).bx(4, [-3.0, 5.6, 3.0], [-1.0, 7.4, 3.8], Gem);
            b.mirror(m);
            b.knees(Polished, false);
            b.toes(Polished);
            (Facet, Visor::Slit)
        }
        "gravitite" => {
            // Floating plates: a ring orbiting the head, pauldrons and knees hovering on light.
            p.glow = [255, 150, 240];
            b.slot(0).ring(
                0,
                [0.0, 34.6, 0.0],
                6.6,
                12,
                [3.0, 0.8, 0.8],
                14.0,
                Polished,
            );
            b.bx(0, [-4.6, 27.9, 4.9], [4.6, 29.3, 5.4], Glow);
            b.slot(1);
            let m = b.mark();
            b.rb(
                2,
                [-7.8, 27.0, 0.0],
                [6.0, 1.6, 6.6],
                [0.0, 0.0, 22.0],
                Polished,
            );
            b.rb(
                2,
                [-7.4, 26.0, 0.0],
                [4.0, 0.4, 4.4],
                [0.0, 0.0, 22.0],
                Glow,
            );
            b.mirror(m);
            b.breastplate(Polished, Glow);
            b.slot(2);
            let m = b.mark();
            b.bx(4, [-4.0, 5.4, 4.2], [0.0, 7.6, 4.8], Polished);
            b.bx(4, [-3.2, 5.9, 3.4], [-0.8, 7.1, 3.6], Glow);
            b.mirror(m);
            b.toes(Polished);
            (Plain, Visor::Slit)
        }
        "valkyrie" => {
            // A valkyrie: great white wings on the helm, feathered pauldrons, a blue cape.
            p.hair = [246, 246, 240];
            p.cloth = [46, 76, 150];
            b.slot(0);
            let m = b.mark();
            for (i, a) in [0.0, 14.0, 28.0, 42.0].iter().enumerate() {
                b.blade(
                    0,
                    [-5.5, 29.4 + i as f32 * 0.5, 0.6 - i as f32 * 0.8],
                    [-20.0 - a, 0.0, 52.0],
                    &[[3.0, 0.0, 0.0], [3.0, -10.0, 0.0], [2.6, -10.0, 0.0]],
                    2.0,
                    0.35,
                    Hair,
                );
            }
            b.mirror(m);
            b.bx(0, [-0.5, 24.6, 5.0], [0.5, 30.4, 5.6], Trim);
            b.slot(1);
            let m = b.mark();
            for (k, a) in [-30.0, -10.0, 10.0, 30.0].iter().enumerate() {
                b.blade(
                    2,
                    [-6.6, 25.6, *a * 0.06],
                    [*a, 0.0, 80.0 + (k % 2) as f32 * 8.0],
                    &[[2.6, 0.0, 0.0], [2.0, 0.0, 10.0]],
                    1.6,
                    0.4,
                    Hair,
                );
            }
            b.mirror(m);
            b.cape(20.0, Cloth);
            b.breastplate(Polished, Trim);
            b.belt(Trim, Polished);
            let m = b.mark();
            b.slot(3).blade(
                4,
                [-4.8, 3.6, -1.0],
                [-30.0, 0.0, 48.0],
                &[[2.0, 0.0, 0.0]],
                1.2,
                0.3,
                Hair,
            );
            b.mirror(m);
            (Feather, Visor::Nasal)
        }
        "phoenix" => {
            // A phoenix: a crest of fire feathers, flaming pauldrons, a tail of long fire feathers.
            p.hair = [250, 150, 40];
            p.accent = [140, 30, 10];
            b.slot(0);
            for k in 0..5 {
                let a = (k as f32 / 4.0 - 0.5) * 60.0;
                b.blade(
                    0,
                    [a.to_radians().sin() * 2.0, 32.6, -1.0],
                    [-30.0, 0.0, -a],
                    &[[2.4, 0.0, 0.0], [2.4, -14.0, 0.0], [2.2, -14.0, 0.0]],
                    1.3,
                    0.35,
                    Flame,
                );
            }
            b.bx(0, [-0.6, 28.4, 5.0], [0.6, 32.6, 5.7], Hair);
            b.slot(1);
            let m = b.mark();
            for (k, a) in [-24.0, 0.0, 24.0].iter().enumerate() {
                b.blade(
                    2,
                    [-6.6, 25.6, *a * 0.06],
                    [*a, 0.0, 60.0 + k as f32 * 10.0],
                    &[[2.4, 0.0, 0.0], [2.2, 0.0, -14.0]],
                    1.8,
                    0.4,
                    if k == 1 { Flame } else { Hair },
                );
            }
            b.mirror(m);
            b.breastplate(Hair, Flame);
            b.slot(2);
            for k in 0..5 {
                let a = (k as f32 / 4.0 - 0.5) * 50.0;
                b.blade(
                    1,
                    [a.to_radians().sin() * 2.0, 12.0, -3.2],
                    [-160.0, 0.0, a],
                    &[[3.0, 0.0, 0.0], [3.0, 10.0, 0.0], [2.6, 12.0, 0.0]],
                    1.4,
                    0.35,
                    if k % 2 == 0 { Flame } else { Hair },
                );
            }
            b.knees(Accent, false);
            b.cuffs(Hair);
            (Feather, Visor::Open)
        }
        "resonarium" => {
            // A resonator: a tuning fork on the helm, speaker rings on the shoulders, glowing crystals.
            p.glow = [80, 240, 230];
            p.gem = [80, 240, 230];
            b.slot(0);
            for x in [-1.6, 1.0] {
                b.bx(0, [x, 33.0, -0.3], [x + 0.6, 37.6, 0.3], Polished);
            }
            b.bx(0, [-1.6, 33.0, -0.4], [1.6, 33.8, 0.4], Polished);
            b.bx(0, [-0.8, 29.4, 5.0], [0.8, 30.6, 5.6], GemGlow);
            b.slot(1);
            let m = b.mark();
            b.ring(
                2,
                [-9.4, 22.6, 0.0],
                2.6,
                10,
                [1.8, 0.8, 0.8],
                0.0,
                Polished,
            );
            b.rb(
                2,
                [-9.4, 22.6, 0.0],
                [0.8, 2.4, 2.4],
                [0.0, 0.0, 0.0],
                GemGlow,
            );
            b.mirror(m);
            b.pauldrons(1, 6.4, 20.0, Metal, Metal);
            b.bx(1, [-1.0, 16.6, 3.0], [1.0, 21.6, 3.6], GemGlow);
            b.knees(Polished, false);
            b.toes(Polished);
            (Rings, Visor::Slit)
        }
        // ---- Wizard robes, one per school ----
        "pyromancer" => {
            p.cloth = [150, 40, 18];
            p.trim = [240, 170, 60];
            let tip = b.hat(1.6, 4, 16.0, Cloth);
            b.slot(0).rb(
                0,
                (tip + Vec3::new(0.0, 0.6, -0.4)).to_array(),
                [1.6, 2.6, 1.6],
                [-20.0, 0.0, 0.0],
                Flame,
            );
            b.bx(0, [-5.6, 33.4, -5.6], [5.6, 34.4, 5.6], Trim);
            b.slot(1);
            let m = b.mark();
            for (dz, h) in [(-1.6, 2.6), (0.0, 3.6), (1.6, 2.6)] {
                b.rb(
                    2,
                    [-6.6, 26.0 + h * 0.4, dz],
                    [1.4, h, 1.2],
                    [0.0, 0.0, 18.0],
                    Flame,
                );
            }
            b.mirror(m);
            b.pauldrons(1, 6.4, 20.0, Cloth, Cloth);
            b.skirt(12.6, 2.4, Cloth, Some(Trim));
            b.cuffs(Trim);
            (Robe, Visor::Open)
        }
        "cryomancer" => {
            p.cloth = [70, 130, 190];
            p.gem = [200, 240, 255];
            p.fur = [236, 244, 250];
            b.hood(Cloth, true);
            b.slot(0);
            for (i, x) in [-4.4, -2.6, -0.8, 1.0, 2.8, 4.4].iter().enumerate() {
                b.rb(
                    0,
                    [*x, 30.0 - (i % 2) as f32 * 0.6, 5.7],
                    [0.8, 2.2 + (i % 2) as f32, 0.6],
                    [0.0, 45.0, 0.0],
                    Gem,
                );
            }
            b.slot(1).bx(1, [-5.6, 23.3, -3.7], [5.6, 25.6, 3.7], Fur);
            let m = b.mark();
            b.rb(
                2,
                [-7.0, 27.4, 0.0],
                [1.2, 3.4, 1.2],
                [0.0, 45.0, 20.0],
                Gem,
            );
            b.rb(
                2,
                [-8.4, 26.4, 1.4],
                [0.9, 2.2, 0.9],
                [20.0, 45.0, 40.0],
                Gem,
            );
            b.mirror(m);
            b.skirt(12.6, 2.4, Cloth, Some(Fur));
            b.cuffs(Fur);
            (Robe, Visor::Open)
        }
        "electromancer" => {
            p.cloth = [60, 52, 30];
            p.trim = [210, 130, 60];
            p.glow = [255, 240, 120];
            let tip = b.hat(1.2, 3, 4.0, Cloth);
            b.slot(0);
            for i in 0..3 {
                b.ring(
                    0,
                    [0.0, 34.0 + i as f32 * 2.0, 0.0],
                    3.8 - i as f32 * 0.8,
                    8,
                    [3.0 - i as f32 * 0.6, 0.6, 0.6],
                    0.0,
                    Trim,
                );
            }
            for (a, z) in [(-20.0, 30.0), (20.0, -30.0)] {
                b.rb(
                    0,
                    (tip + Vec3::new(a * 0.04, 0.6, 0.0)).to_array(),
                    [0.5, 2.6, 0.5],
                    [0.0, 0.0, z],
                    Glow,
                );
            }
            b.slot(1);
            let m = b.mark();
            for i in 0..3 {
                b.ring(
                    2,
                    [-6.0, 21.0 + i as f32 * 1.4, 0.0],
                    3.6,
                    8,
                    [2.8, 0.6, 0.6],
                    0.0,
                    Trim,
                );
            }
            b.rb(
                2,
                [-6.4, 27.4, 0.0],
                [0.5, 3.0, 0.5],
                [0.0, 0.0, 20.0],
                Glow,
            );
            b.mirror(m);
            b.skirt(12.6, 2.4, Cloth, Some(Trim));
            b.cuffs(Trim);
            (Robe, Visor::Open)
        }
        "priest" => {
            p.cloth = [236, 226, 196];
            p.trim = [226, 184, 72];
            p.accent = [176, 40, 40];
            b.slot(0);
            b.rb(0, [0.0, 35.6, 1.4], [8.4, 6.0, 3.4], [0.0, 0.0, 0.0], Cloth);
            b.rb(
                0,
                [0.0, 35.6, -1.6],
                [8.4, 6.0, 3.0],
                [0.0, 0.0, 0.0],
                Cloth,
            );
            for z in [1.4, -1.6] {
                b.rb(0, [0.0, 38.6, z], [5.8, 5.8, 2.6], [0.0, 0.0, 45.0], Cloth);
            }
            b.bx(0, [-0.5, 33.6, 3.1], [0.5, 38.4, 3.4], Trim);
            b.bx(0, [-1.8, 36.4, 3.1], [1.8, 37.2, 3.4], Trim);
            b.slot(1);
            let m = b.mark();
            b.bx(1, [-3.2, 6.0, 3.0], [-1.6, 24.6, 3.5], Accent);
            b.bx(1, [-3.2, 6.0, 3.5], [-1.6, 6.8, 3.7], Trim);
            b.mirror(m);
            b.bx(1, [-5.6, 23.3, -3.7], [5.6, 25.6, 3.7], Trim);
            b.skirt(12.6, 1.6, Cloth, Some(Trim));
            b.cuffs(Trim);
            (Robe, Visor::Open)
        }
        "voidwalker" => {
            p.cloth = [36, 16, 60];
            p.glow = [200, 110, 255];
            p.gem = [140, 60, 230];
            b.hood(Cloth, true);
            b.slot(0).bx(0, [-4.6, 23.4, 4.6], [4.6, 29.6, 5.2], Dark);
            b.rb(0, [0.0, 28.0, 5.3], [2.0, 1.2, 0.4], [0.0, 0.0, 0.0], Glow);
            b.slot(1);
            let m = b.mark();
            b.rb(
                2,
                [-9.0, 28.4, -1.0],
                [1.0, 2.6, 1.0],
                [10.0, 45.0, 20.0],
                GemGlow,
            );
            b.rb(
                2,
                [-10.0, 24.0, 2.4],
                [0.8, 1.8, 0.8],
                [-20.0, 45.0, 30.0],
                GemGlow,
            );
            b.mirror(m);
            b.bx(1, [-5.6, 22.8, -3.7], [5.6, 25.6, 3.7], Cloth);
            b.skirt(12.6, 2.0, Cloth, Some(Glow));
            b.cuffs(Dark);
            (Robe, Visor::Closed)
        }
        "bloodmage" => {
            p.cloth = [110, 14, 24];
            p.horn = [60, 30, 30];
            p.gem = [210, 20, 40];
            b.hood(Cloth, true);
            let m = b.mark();
            b.slot(0).chain(
                0,
                [-3.0, 33.4, 1.0],
                [-10.0, 0.0, 20.0],
                &[
                    [1.6, 1.2, 0.0, 0.0],
                    [1.4, 0.8, -20.0, 0.0],
                    [1.0, 0.5, -20.0, 0.0],
                ],
                Horn,
            );
            b.mirror(m);
            b.slot(1).rb(
                1,
                [0.0, 18.0, 3.2],
                [1.6, 15.6, 0.5],
                [0.0, 0.0, 36.0],
                Accent,
            );
            for (x, y) in [(-3.0, 22.0), (0.0, 18.0), (3.0, 14.0)] {
                b.rb(1, [x, y, 3.6], [1.0, 1.0, 0.6], [0.0, 0.0, 45.0], Gem);
            }
            b.pauldrons(1, 6.4, 20.0, Cloth, Cloth);
            b.skirt(12.6, 2.4, Cloth, Some(Dark));
            b.cuffs(Dark);
            (Robe, Visor::Open)
        }
        "cultist" => {
            p.cloth = [16, 92, 98];
            p.trim = [220, 196, 120];
            p.horn = [214, 206, 180];
            b.hood(Cloth, true);
            b.slot(0).bx(0, [-4.2, 23.4, 4.9], [4.2, 30.4, 5.5], Horn);
            for x in [-2.8, 1.4] {
                b.bx(0, [x, 27.6, 5.4], [x + 1.4, 28.4, 5.6], Dark);
            }
            for x in [-2.0, 0.0, 2.0] {
                b.chain(
                    0,
                    [x, 23.4, 5.2],
                    [180.0, 0.0, 0.0],
                    &[
                        [1.4, 0.6, 0.0, 0.0],
                        [1.2, 0.5, 18.0, 0.0],
                        [1.0, 0.4, -24.0, 0.0],
                    ],
                    Cloth,
                );
            }
            b.slot(1)
                .ring(1, [0.0, 23.0, 0.0], 4.6, 12, [2.6, 0.5, 0.5], 22.0, Trim);
            b.rb(1, [0.0, 19.6, 3.6], [2.2, 2.2, 0.5], [0.0, 0.0, 45.0], Trim);
            b.skirt(12.6, 1.6, Cloth, Some(Trim));
            b.cuffs(Trim);
            (Robe, Visor::Closed)
        }
        "archevoker" => {
            p.cloth = [92, 74, 46];
            p.trim = [226, 184, 72];
            p.gem = [60, 200, 120];
            b.brow(Trim);
            b.slot(0).bx(0, [-0.8, 29.1, 5.35], [0.8, 30.7, 5.8], Gem);
            b.slot(1);
            b.bx(1, [-5.6, 24.6, -3.8], [5.6, 31.6, -3.0], Trim);
            let m = b.mark();
            b.rb(
                1,
                [-5.4, 27.6, -0.8],
                [0.8, 6.0, 5.0],
                [0.0, 22.0, -10.0],
                Trim,
            );
            b.mirror(m);
            b.cape(22.0, Cloth);
            b.pauldrons(1, 6.6, 20.0, Trim, Trim);
            b.skirt(12.6, 1.6, Cloth, Some(Trim));
            b.cuffs(Trim);
            (Robe, Visor::Open)
        }
        "gladiator" => {
            // A murmillo: a wide-brimmed grille helm with a fish crest, one segmented sword arm
            // (the right), one great shoulder guard (the left), a single greave.
            p.hair = [176, 40, 34];
            p.accent = [104, 70, 42];
            b.slot(0)
                .bx(0, [-6.8, 26.6, -6.8], [6.8, 27.3, 6.8], Polished);
            for (i, z) in [4.0, 2.0, 0.0, -2.0, -4.0].iter().enumerate() {
                let h = [1.8, 2.8, 3.4, 3.0, 2.0][i];
                b.rb(
                    0,
                    [0.0, 33.0 + h * 0.5, *z],
                    [1.0, h, 2.2],
                    [-12.0, 0.0, 0.0],
                    if i == 2 { Hair } else { Polished },
                );
            }
            for x in [-3.0, -1.5, 0.0, 1.5, 3.0] {
                b.bx(0, [x - 0.3, 23.2, 5.0], [x + 0.3, 29.0, 5.5], Dark);
            }
            b.bx(0, [-4.0, 25.8, 5.0], [4.0, 26.4, 5.5], Dark);
            b.slot(1);
            for i in 0..6 {
                let f = i as f32;
                b.bx(
                    2,
                    [-9.0 - (i % 2) as f32 * 0.2, 12.4 + f * 2.1, -3.0],
                    [-3.0, 14.0 + f * 2.1, 3.0],
                    if i % 2 == 0 { Polished } else { Metal },
                );
            }
            b.rb(
                3,
                [7.0, 25.4, 0.0],
                [7.8, 2.0, 8.0],
                [0.0, 0.0, -14.0],
                Polished,
            );
            b.rb(
                3,
                [10.0, 26.6, 0.0],
                [0.8, 4.2, 6.6],
                [0.0, 0.0, -8.0],
                Polished,
            );
            b.slot(2).bx(1, [-5.1, 10.2, -3.1], [5.1, 13.0, 3.1], Trim);
            for i in 0..6 {
                let x = -4.2 + i as f32 * 1.4;
                b.bx(1, [x, 6.8, 2.9], [x + 1.0, 10.4, 3.35], Accent);
            }
            b.slot(3)
                .bx(5, [-0.8, 2.4, -2.9], [4.8, 9.6, 3.4], Polished);
            b.bx(5, [-0.6, 7.0, 3.4], [4.6, 8.0, 3.7], Trim);
            (Roman, Visor::Open)
        }
        "magmarite" => {
            // Volcanic: molten-tipped horns, rock spikes on the shoulders, magma seams.
            p.horn = [52, 40, 36];
            p.glow = [255, 150, 40];
            let m = b.mark();
            b.slot(0).chain(
                0,
                [-3.6, 32.6, 0.0],
                [0.0, 0.0, 28.0],
                &[
                    [2.4, 1.8, 0.0, 0.0],
                    [2.0, 1.4, 0.0, -18.0],
                    [1.8, 1.0, 0.0, -18.0],
                ],
                Horn,
            );
            b.rb(
                0,
                [-5.5, 38.4, 0.0],
                [0.7, 1.4, 0.7],
                [0.0, 0.0, -6.0],
                Flame,
            );
            b.mirror(m);
            b.bx(0, [-4.6, 27.9, 4.9], [4.6, 29.2, 5.4], Flame);
            b.slot(1);
            let m = b.mark();
            for (dz, h, t) in [(-1.8, 3.4, 20.0), (0.2, 4.4, 8.0), (2.0, 2.8, 30.0)] {
                b.rb(
                    2,
                    [-7.0, 26.6 + h * 0.3, dz],
                    [1.4, h, 1.4],
                    [dz * 5.0, 0.0, t],
                    Horn,
                );
            }
            b.mirror(m);
            b.pauldrons(2, 7.0, 16.0, Dark, Metal);
            b.crack(
                1,
                [-2.4, 23.0, 3.1],
                &[[2.6, 25.0], [2.4, -20.0], [2.8, 30.0], [2.0, -15.0]],
            );
            b.crack(
                1,
                [2.6, 21.0, 3.1],
                &[[2.4, -30.0], [2.6, 20.0], [2.2, -25.0]],
            );
            b.knees(Dark, true);
            b.toes(Dark);
            b.cuffs(Metal);
            (Volcanic, Visor::Slit)
        }
        // ---- Vanity outfits (cloth, no armour points) ----
        "tuxedo" | "vanity_tuxedo" => {
            p.accent = [176, 30, 40];
            b.slot(0).bx(0, [-6.6, 32.6, -6.6], [6.6, 33.4, 6.6], Dark);
            b.bx(0, [-4.4, 33.4, -4.4], [4.4, 39.6, 4.4], Dark);
            b.bx(0, [-4.6, 33.6, -4.6], [4.6, 34.8, 4.6], Accent);
            b.slot(1);
            b.bx(1, [-1.0, 23.4, 3.0], [1.0, 24.4, 3.5], Accent);
            b.rb(
                1,
                [-1.4, 23.9, 3.25],
                [1.6, 1.6, 0.5],
                [0.0, 0.0, 45.0],
                Accent,
            );
            b.rb(
                1,
                [1.4, 23.9, 3.25],
                [1.6, 1.6, 0.5],
                [0.0, 0.0, 45.0],
                Accent,
            );
            let m = b.mark();
            b.rb(
                1,
                [-2.4, 7.6, -3.4],
                [4.0, 10.0, 0.4],
                [6.0, 0.0, -4.0],
                Dark,
            );
            b.mirror(m);
            (Robe, Visor::Open)
        }
        "pirate" | "vanity_pirate" => {
            p.accent = [30, 26, 24];
            p.trim = [226, 184, 72];
            b.slot(0)
                .bx(0, [-5.6, 32.6, -5.6], [5.6, 35.0, 5.6], Accent);
            for k in 0..3 {
                let a = k as f32 * 120.0;
                let r = a.to_radians();
                b.rb(
                    0,
                    [r.sin() * 5.2, 34.4, r.cos() * 5.2],
                    [8.6, 3.2, 0.7],
                    [-20.0, a, 0.0],
                    Accent,
                );
                b.rb(
                    0,
                    [r.sin() * 5.5, 35.9, r.cos() * 5.5],
                    [8.6, 0.4, 0.8],
                    [-20.0, a, 0.0],
                    Trim,
                );
            }
            b.slot(1).rb(
                1,
                [0.0, 13.0, 0.0],
                [10.8, 1.6, 6.6],
                [0.0, 0.0, -8.0],
                Cloth,
            );
            b.skirt(12.6, 4.0, Cloth, Some(Trim));
            b.cuffs(Accent);
            (Robe, Visor::Open)
        }
        "explorer" | "vanity_explorer" => {
            p.accent = [120, 90, 54];
            p.cloth = [92, 70, 40];
            b.slot(0)
                .bx(0, [-7.6, 32.4, -7.6], [7.6, 33.0, 7.6], Accent);
            b.bx(0, [-4.8, 33.0, -4.8], [4.8, 36.6, 4.8], Accent);
            b.bx(0, [-5.0, 33.0, -5.0], [5.0, 34.0, 5.0], Dark);
            b.bx(0, [-0.6, 36.0, -4.4], [0.6, 36.7, 4.4], Dark);
            b.slot(1)
                .bx(1, [-3.8, 13.4, -6.4], [3.8, 23.2, -3.2], Cloth);
            b.bx(1, [-3.4, 19.6, -6.8], [3.4, 22.6, -6.4], Accent);
            let m = b.mark();
            b.bx(1, [-3.6, 13.0, -3.4], [-2.6, 25.4, 3.4], Dark);
            b.mirror(m);
            b.belt(Dark, Polished);
            (Robe, Visor::Open)
        }
        "royal" | "vanity_royal" => {
            p.trim = [240, 200, 80];
            p.fur = [244, 242, 236];
            p.cloth = [150, 26, 40];
            p.gem = [60, 90, 220];
            b.slot(0).bx(0, [-5.0, 32.6, -5.0], [5.0, 34.2, 5.0], Trim);
            for k in 0..8 {
                let a = k as f32 / 8.0 * std::f32::consts::TAU;
                b.rb(
                    0,
                    [a.sin() * 4.6, 35.0, a.cos() * 4.6],
                    [1.2, 1.6, 1.2],
                    [0.0, a.to_degrees(), 0.0],
                    Trim,
                );
            }
            b.bx(0, [-0.7, 32.8, 5.0], [0.7, 34.0, 5.4], Gem);
            b.cape(22.0, Cloth);
            b.slot(1).bx(1, [-5.8, 22.8, -3.9], [5.8, 25.8, 3.9], Fur);
            (Robe, Visor::Open)
        }
        "vanity_wizard" => {
            p.trim = [240, 210, 110];
            let tip = b.hat(2.0, 5, 12.0, Cloth);
            b.slot(0)
                .rb(0, tip.to_array(), [1.4, 1.4, 1.4], [45.0, 45.0, 0.0], Trim);
            b.bx(0, [-5.6, 33.4, -5.6], [5.6, 34.4, 5.6], Trim);
            b.skirt(12.6, 1.6, Cloth, Some(Trim));
            (Robe, Visor::Open)
        }
        "vanity_sailor" => {
            p.cloth = [236, 236, 236];
            p.accent = [30, 40, 96];
            b.slot(0).bx(0, [-5.8, 32.6, -5.8], [5.8, 34.2, 5.8], Cloth);
            b.bx(0, [-5.9, 32.4, -5.9], [5.9, 33.2, 5.9], Accent);
            b.slot(1)
                .bx(1, [-4.6, 17.6, -3.8], [4.6, 25.4, -3.1], Accent);
            b.rb(1, [0.0, 21.6, 3.3], [1.6, 1.6, 0.5], [0.0, 0.0, 45.0], Hair);
            (Robe, Visor::Open)
        }
        "vanity_ranger" => {
            p.cloth = [40, 84, 40];
            b.hood(Cloth, true);
            b.cape(22.0, Cloth);
            b.belt(Accent, Polished);
            (Robe, Visor::Open)
        }
        "vanity_shadow" => {
            p.cloth = [30, 24, 40];
            b.hood(Cloth, true);
            b.slot(0).bx(0, [-4.6, 23.2, 4.6], [4.6, 27.6, 5.4], Cloth);
            b.slot(1).blade(
                1,
                [2.0, 24.0, -3.8],
                [-110.0, 0.0, -14.0],
                &[[3.0, 0.0, 0.0], [3.0, 14.0, -6.0]],
                2.0,
                0.35,
                Cloth,
            );
            (Robe, Visor::Open)
        }
        // Dyed armour and anything new keep the classic shape.
        _ => (crate::entity::armor_style(name), Visor::Open),
    };
    let visor = match (style, visor) {
        (_, v) if v != Visor::Open => v,
        (Greek, _) => Visor::TSlot,
        (Roman, _) if name == "copper" => Visor::Cheeks,
        (Plate | Banded | Gothic | Volcanic, _) => Visor::Slit,
        (Spiked, _) => Visor::Breaths,
        _ => Visor::Open,
    };
    Design {
        style,
        visor,
        pal: p,
        pieces: b.v,
    }
}

/// A little value noise for swatches: 0..1, smooth over model pixels.
fn noise(x: f32, y: f32, seed: i32) -> f32 {
    let h = |i: i32, j: i32| {
        let n = (i.wrapping_mul(374761393)
            ^ j.wrapping_mul(668265263)
            ^ seed.wrapping_mul(1274126177)) as u32;
        let n = (n ^ (n >> 13)).wrapping_mul(1274126177);
        (n >> 8) as f32 / (1u32 << 24) as f32
    };
    let (i, j) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - x.floor(), y - y.floor());
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let a = h(i, j) + (h(i + 1, j) - h(i, j)) * sx;
    let b = h(i, j + 1) + (h(i + 1, j + 1) - h(i, j + 1)) * sx;
    a + (b - a) * sy
}

/// Paint the 16 swatches into the lower half of an armour picture `w` texels wide with `sc`
/// texels per model pixel. `glint` is how shiny the set is (0.18 matte .. 0.62 mirror).
pub fn paint_tiles(pal: &Palette, glint: f32, pixels: &mut [u8], w: u32, sc: u32) {
    let f = |c: [u8; 3]| [c[0] as f32, c[1] as f32, c[2] as f32];
    for t in 0..16u32 {
        let tile: Tile = TILES[t as usize];
        let (ox, oy) = tile.origin();
        for ty in 0..16 * sc {
            for tx in 0..16 * sc {
                let fx = (tx as f32 + 0.5) / sc as f32;
                let fy = (ty as f32 + 0.5) / sc as f32;
                let (gx, gy) = (ox as u32 * sc + tx, oy as u32 * sc + ty);
                let h = ((gx as i32 * 73 + gy as i32 * 151) ^ (gx as i32 * gy as i32 * 17)) & 15;
                let speck = 1.0 + (h as f32 - 7.5) / 7.5 * 0.05;
                let mut wh = 0.0f32;
                let (mut col, k) = match tile {
                    Tile::Metal => {
                        let brush = noise(fx * 0.5, fy * 4.0, 3) * 0.1;
                        (
                            f(pal.base),
                            (0.94 + brush) * speck * (1.06 - fy / 16.0 * 0.12),
                        )
                    }
                    Tile::Polished => {
                        let streak = ((fx + fy * 0.6) * 0.45).sin().abs();
                        wh = glint * 0.45 * (1.0 - streak).powi(6);
                        (f(mix(pal.base, WHITE, 0.14)), 1.06 - fy / 16.0 * 0.1)
                    }
                    Tile::Dark => {
                        let rivet = ((fx % 4.0) - 2.0).hypot((fy % 4.0) - 2.0) < 0.45;
                        (
                            f(dk(pal.base, 0.58)),
                            if rivet { 1.35 } else { 0.94 * speck },
                        )
                    }
                    Tile::Accent => (
                        f(pal.accent),
                        speck
                            * (1.04 - fy / 16.0 * 0.1)
                            * (0.94 + noise(fx * 0.7, fy * 0.7, 5) * 0.12),
                    ),
                    Tile::Trim => {
                        let line = ((fx * 1.3 + (fy * 0.9).sin() * 1.2).sin()).abs() < 0.16;
                        wh = if (fx + fy) % 5.0 < 0.5 {
                            glint * 0.5
                        } else {
                            0.0
                        };
                        (f(pal.trim), if line { 0.74 } else { 1.04 * speck })
                    }
                    Tile::Hair => (f(pal.hair), 0.72 + noise(fx * 2.6, fy * 0.25, 7) * 0.5),
                    Tile::Horn => {
                        let ring = (fy * 2.4).sin() * 0.08;
                        (f(pal.horn), (1.04 - fy / 16.0 * 0.25 + ring) * speck)
                    }
                    Tile::Cloth => {
                        let weave = if (tx + ty) % 2 == 0 { 1.03 } else { 0.97 };
                        let fold = 0.92 + 0.08 * (fx * 0.9).sin();
                        (f(pal.cloth), weave * fold)
                    }
                    Tile::Fur => (f(pal.fur), 0.74 + noise(fx * 2.2, fy * 1.1, 11) * 0.42),
                    Tile::Scales => {
                        let row = (fy / 1.4).floor();
                        let sx = fx / 1.8 + if row as i32 % 2 == 0 { 0.0 } else { 0.5 };
                        let (lx, ly) = (sx.fract() - 0.5, (fy / 1.4).fract());
                        let d = (lx * lx * 1.3 + (ly - 0.15).powi(2) * 0.8).sqrt();
                        let mut kk = 1.14 - ly * 0.34;
                        if d > 0.5 && ly > 0.42 {
                            kk *= 0.62;
                        }
                        if ly < 0.18 {
                            wh = glint * 0.35;
                        }
                        (f(pal.base), kk)
                    }
                    Tile::Leaf => {
                        let vein =
                            (fx - 8.0).abs() < 0.3 || ((fx - 8.0).abs() - (fy % 3.0)).abs() < 0.22;
                        (
                            f(pal.leaf),
                            if vein {
                                1.22
                            } else {
                                0.9 + noise(fx, fy, 13) * 0.16
                            },
                        )
                    }
                    Tile::Glass => {
                        let streak = ((fx - fy * 0.8) % 7.0).abs() < 0.9;
                        wh = if streak { 0.35 } else { 0.0 };
                        (f(pal.glass), 0.8 + fy / 16.0 * 0.3)
                    }
                    Tile::Glow => {
                        let d = ((fx - 8.0).powi(2) + (fy - 8.0).powi(2)).sqrt() / 8.0;
                        wh = (0.35 * (1.0 - d)).max(0.0);
                        (f(pal.glow), 1.0)
                    }
                    Tile::Flame => {
                        let up = 1.0 - fy / 16.0;
                        let tongue = noise(fx * 0.8, fy * 0.5 + 3.0, 17);
                        let hot = (up * 0.8 + tongue * 0.4).clamp(0.0, 1.0);
                        wh = (hot - 0.6).max(0.0) * 0.8;
                        (
                            f(mix(
                                dk(pal.glow, 0.7),
                                mix(pal.glow, [255, 250, 200], 0.6),
                                hot,
                            )),
                            1.0,
                        )
                    }
                    Tile::GemGlow | Tile::Gem => {
                        let (a, c) = (fx / 16.0 - fy / 16.0, fx / 16.0 + fy / 16.0 - 1.0);
                        let facet = match (a > 0.0, c > 0.0) {
                            (true, true) => 0.8,
                            (true, false) => 1.18,
                            (false, true) => 0.92,
                            (false, false) => 1.06,
                        };
                        let d = ((fx - 5.0).powi(2) + (fy - 5.0).powi(2)).sqrt();
                        wh = (0.6 - d / 3.0).max(0.0);
                        if h == 15 {
                            wh = wh.max(0.5);
                        }
                        (f(pal.gem), facet)
                    }
                };
                if matches!(tile, Tile::Metal | Tile::Accent | Tile::Scales) && h == 15 {
                    wh = wh.max(glint * 0.25);
                }
                let o = ((gy * w + gx) * 4) as usize;
                for c in 0..3 {
                    col[c] = (col[c] * k).min(255.0);
                    pixels[o + c] = (col[c] + (255.0 - col[c]) * wh) as u8;
                }
                pixels[o + 3] = 255;
            }
        }
    }
}

const TILES: [Tile; 16] = [
    Tile::Metal,
    Tile::Polished,
    Tile::Dark,
    Tile::Accent,
    Tile::Trim,
    Tile::Hair,
    Tile::Horn,
    Tile::Cloth,
    Tile::Fur,
    Tile::Scales,
    Tile::Leaf,
    Tile::Glass,
    Tile::Glow,
    Tile::Flame,
    Tile::GemGlow,
    Tile::Gem,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_armour_set_has_its_own_look() {
        let mut seen = std::collections::HashMap::new();
        for (name, c) in crate::entity::ARMOR_MATERIALS {
            if name.starts_with("dye_") {
                continue;
            }
            let d = design(name, c);
            assert!(!d.pieces.is_empty(), "{name} has no pieces of its own");
            assert!(
                d.pieces.iter().any(|p| p.slot == 0),
                "{name}: the helmet has nothing of its own"
            );
            // No two sets share the same silhouette.
            let key: Vec<i32> = d
                .pieces
                .iter()
                .map(|p| {
                    (p.place.w_axis.x * 10.0 + p.place.w_axis.y * 100.0 + p.half.x * 1000.0) as i32
                })
                .collect();
            if let Some(other) = seen.insert(key, name) {
                let same_outfit =
                    name.trim_start_matches("vanity_") == other.trim_start_matches("vanity_");
                assert!(same_outfit, "{name} looks like {other}");
            }
            for p in &d.pieces {
                assert!(p.part < 6 && p.slot < 4, "{name}: bad piece");
                let c = p.place.w_axis;
                assert!(
                    c.x.abs() < 16.0 && c.y > -4.0 && c.y < 46.0 && c.z.abs() < 16.0,
                    "{name}: piece far from the body at {c:?}"
                );
            }
        }
    }

    #[test]
    fn platinum_helm_is_a_dragon() {
        let d = design("platinum", [196, 222, 230]);
        // Jaws reach out in front of the face, horns sweep back behind it, the eyes glow.
        let helm: Vec<_> = d.pieces.iter().filter(|p| p.slot == 0).collect();
        assert!(helm.iter().any(|p| p.place.w_axis.z > 9.0));
        assert!(helm
            .iter()
            .any(|p| p.place.w_axis.z < -6.0 && p.tile == Tile::Horn));
        assert!(helm.iter().any(|p| p.tile == Tile::GemGlow));
    }
}
