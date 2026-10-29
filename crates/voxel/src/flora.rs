//! Plants with a body (flowers, mushrooms, saplings, grass, crops, cane, kelp, corals, vines):
//! no plant is drawn as two crossed pictures. Each plant near the camera is built from small
//! cards and boxes with its own shape (layered petals, a real stem, leaves, a round mushroom
//! cap, a sunflower two blocks tall). Far away the richer ones swap to a few boxes (`build_far`:
//! a stem and a head, a stalk and a cap), so a meadow past a few chunks costs about what the
//! crossed pictures did (see `SectionMesh::near_quads`).
//!
//! Stacked plants (cane, bamboo, kelp, vines) bend as one stem: their quads carry the stack bits
//! like the crossed plants did (`stacked`).
//!
//! The shapes are made here once per block, in block units, and quantized to 1/32 of a block:
//! the vertex format of these quads (`FLAG_FLORA` in mesh.rs) keeps x and z in 1/32 steps and the
//! height inside the block in its own bits, so the shader knows how high each corner is and the
//! plant bends in the wind from a fixed base. Each part names a picture (`Part`) that the game
//! paints from the plant's own texture (game/src/flora_tex.rs), so a resource pack's colours
//! carry over.

use glam::{vec3, Vec3};
use std::f32::consts::{PI, TAU};

/// The picture a quad shows. All but `Sprite` are painted by the game from the plant's texture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Part {
    PetalRound,
    PetalPointed,
    PetalCup,
    PetalFrilly,
    PetalNarrow,
    PetalNotched,
    Bell,
    Center,
    Floret,
    Seeds,
    Stem,
    Leaf,
    LeafLong,
    Blade,
    Frond,
    Cap,
    CapUnder,
    MushStem,
    Wood,
    Foliage,
    Needles,
    HeadBack,
    /// A solid patch of the flower's colour (heads seen from afar, corals, berries).
    Bloom,
    /// The block's own picture.
    Sprite,
}

impl Part {
    pub fn key(self) -> &'static str {
        match self {
            Part::PetalRound => "petal_round",
            Part::PetalPointed => "petal_pointed",
            Part::PetalCup => "petal_cup",
            Part::PetalFrilly => "petal_frilly",
            Part::PetalNarrow => "petal_narrow",
            Part::PetalNotched => "petal_notched",
            Part::Bell => "bell",
            Part::Center => "center",
            Part::Floret => "floret",
            Part::Seeds => "seeds",
            Part::Stem => "stem",
            Part::Leaf => "leaf",
            Part::LeafLong => "leaf_long",
            Part::Blade => "blade",
            Part::Frond => "frond",
            Part::Cap => "cap",
            Part::CapUnder => "cap_under",
            Part::MushStem => "mush_stem",
            Part::Wood => "wood",
            Part::Foliage => "foliage",
            Part::Needles => "needles",
            Part::HeadBack => "head_back",
            Part::Bloom => "bloom",
            Part::Sprite => "sprite",
        }
    }

    pub fn from_key(k: &str) -> Option<Part> {
        const ALL: [Part; 24] = [
            Part::PetalRound,
            Part::PetalPointed,
            Part::PetalCup,
            Part::PetalFrilly,
            Part::PetalNarrow,
            Part::PetalNotched,
            Part::Bell,
            Part::Center,
            Part::Floret,
            Part::Seeds,
            Part::Stem,
            Part::Leaf,
            Part::LeafLong,
            Part::Blade,
            Part::Frond,
            Part::Cap,
            Part::CapUnder,
            Part::MushStem,
            Part::Wood,
            Part::Foliage,
            Part::Needles,
            Part::HeadBack,
            Part::Bloom,
            Part::Sprite,
        ];
        ALL.into_iter().find(|p| p.key() == k)
    }

    /// Takes the biome colour when the plant's picture does (grass, vines: their pictures are
    /// grey, so these parts are painted grey too).
    pub fn follows_tint(self) -> bool {
        matches!(self, Part::Blade | Part::Frond | Part::Sprite | Part::Stem | Part::Leaf | Part::LeafLong | Part::Foliage | Part::Needles)
    }
}

/// One quad of a plant, in block units (x, z in 0..1, y from 0 up to almost 2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawQuad {
    /// Corners, counter-clockwise seen from the front.
    pub p: [Vec3; 4],
    pub uv: [[u8; 2]; 4],
    pub part: Part,
    /// A card seen from both sides (petals, leaves); boxes are closed and one-sided.
    pub two: bool,
    /// How little it bends: 0 like grass .. 3 not at all.
    pub stiff: u8,
    /// Glows (petals of light-giving flowers).
    pub glow: bool,
}

/// A plant ready to mesh: corners in 1/32 block (x, z 1..31, y 0..63).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloraQuad {
    pub pos: [[u16; 3]; 4],
    pub uv: [[u8; 2]; 4],
    pub layer: u16,
    /// Shading face for the front and the back (mesh face order).
    pub face: [u8; 2],
    pub two: bool,
    pub stiff: u8,
    pub glow: bool,
    pub tint: bool,
}

/// Highest corner, in 1/32 block (just under two blocks).
pub const MAX_Y: u16 = 63;

/// A plant block's bodies, ready for `MeshTables::set_flora`.
#[derive(Clone, Debug)]
pub struct FloraModel {
    pub id: crate::block::BlockId,
    pub near: Vec<FloraQuad>,
    /// Drawn instead of `near` far from the camera; None: `near` at every distance.
    pub far: Option<Vec<FloraQuad>>,
    /// Keeps its facing (`Kind::fixed`).
    pub fixed: bool,
    /// Bends with the blocks above and below (`Kind::stacked`).
    pub stacked: bool,
}

/// The shape family of a plant block, from its name. None: it stays crossed pictures (stacked
/// plants like sugar cane and kelp, and anything that is not a plant).
pub fn kind_of(name: &str) -> Option<Kind> {
    use Kind::*;
    Some(match name {
        "poppy" => Poppy,
        "dandelion" => Dandelion,
        "cornflower" => Cornflower,
        "allium" => Allium,
        "azure_bluet" => Bluet,
        "oxeye_daisy" | "white_flower" => Daisy,
        "blue_orchid" => Orchid,
        "lily_of_the_valley" => Lily,
        "open_eyeblossom" => EyeOpen,
        "closed_eyeblossom" => EyeClosed,
        "sunflower" => Sunflower,
        "torchflower" => Torchflower,
        "pitcher_plant" => Pitcher,
        "purple_flower" | "mandacaru_flower" | "ember_bloom" | "gloom_bloom" => Radial,
        "red_mushroom" => RedMushroom,
        "brown_mushroom" => BrownMushroom,
        "glow_mushroom" => GlowMushroom,
        "short_grass" | "aether_tall_grass" | "capim_dourado" | "seagrass" | "pondweed" | "glow_algae" => Grass,
        "samambaia" | "fern" | "sculk_tendrils" => Fern,
        "sugar_cane" => Cane,
        "bamboo" => Bamboo,
        "kelp" => Kelp,
        "cattail" => Cattail,
        "cattail_top" => CattailTop,
        "vine" => Vine,
        "cave_vines" => CaveVine,
        "pale_hanging_moss" => HangingMoss,
        "sea_anemone" => Anemone,
        "sea_pickle" => Pickle,
        "echo_bulb" => Bulb,
        n if n.ends_with("_coral") => Coral,
        n if n.ends_with("_cluster") && n != "amethyst_cluster" => Cluster,
        "spruce_sapling" | "araucaria_sapling" => Conifer,
        "dead_bush" | "jurema" | "blueberry_bush" => Star,
        n if n.ends_with("_tulip") => Tulip,
        n if n.starts_with("bromelia") => Bromelia,
        n if n.ends_with("_sapling") || n == "mangrove_propagule" => Sapling,
        n if n.starts_with("sweet_berry_bush") => Star,
        n if n.contains("_stage") && !n.starts_with("pumpkin_stem") && !n.starts_with("melon_stem") => Crop,
        _ => return None,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Poppy,
    Dandelion,
    Cornflower,
    Allium,
    Bluet,
    Daisy,
    Tulip,
    Orchid,
    Lily,
    EyeOpen,
    EyeClosed,
    Sunflower,
    Torchflower,
    Pitcher,
    Radial,
    Bromelia,
    RedMushroom,
    BrownMushroom,
    GlowMushroom,
    Sapling,
    Conifer,
    Grass,
    Fern,
    Star,
    Crop,
    Cane,
    Bamboo,
    Kelp,
    Cattail,
    CattailTop,
    Vine,
    CaveVine,
    HangingMoss,
    Coral,
    Anemone,
    Pickle,
    Cluster,
    Bulb,
}

impl Kind {
    /// Faces a fixed way (the sunflower looks east); the others turn with their position.
    pub fn fixed(self) -> bool {
        self == Kind::Sunflower
    }

    /// Grows in stacks (cane, bamboo, kelp, vines): fills its block from bottom to top and
    /// bends together with the blocks above and below.
    pub fn stacked(self) -> bool {
        use Kind::*;
        matches!(self, Cane | Bamboo | Kelp | Cattail | CattailTop | Vine | CaveVine | HangingMoss)
    }
}

struct B {
    q: Vec<RawQuad>,
    stiff: u8,
    glow: bool,
}

const C: Vec3 = Vec3::new(0.5, 0.0, 0.5);
const FULL_UV: [[u8; 2]; 4] = [[0, 16], [16, 16], [16, 0], [0, 0]];

fn flat(yaw: f32) -> Vec3 {
    vec3(yaw.cos(), 0.0, yaw.sin())
}

impl B {
    fn quad(&mut self, p: [Vec3; 4], uv: [[u8; 2]; 4], part: Part, two: bool, glow: bool) {
        self.q.push(RawQuad { p, uv, part, two, stiff: self.stiff, glow });
    }

    /// A two-sided card: `w` wide along `right`, `h` long along `up`, standing on `base`.
    fn card(&mut self, base: Vec3, right: Vec3, up: Vec3, w: f32, h: f32, part: Part, glow: bool) {
        let r = right.normalize() * (w * 0.5);
        let u = up.normalize() * h;
        self.quad([base - r, base + r, base + r + u, base - r + u], FULL_UV, part, true, glow);
    }

    /// A petal (or leaf) from `hub` toward `yaw`, raised `tilt` radians above the ground.
    fn petal(&mut self, hub: Vec3, yaw: f32, tilt: f32, len: f32, w: f32, part: Part) {
        let dir = flat(yaw) * tilt.cos() + Vec3::Y * tilt.sin();
        let side = flat(yaw + PI / 2.0);
        let glow = self.glow;
        self.card(hub, side, dir, w, len, part, glow);
    }

    /// A ring of `n` petals, the first at `yaw0`.
    fn ring(&mut self, hub: Vec3, n: usize, yaw0: f32, tilt: f32, len: f32, w: f32, part: Part, out: f32) {
        for k in 0..n {
            let yaw = yaw0 + k as f32 * TAU / n as f32;
            self.petal(hub + flat(yaw) * out, yaw, tilt, len, w, part);
        }
    }

    /// A closed box (no bottom unless `bottom`), its pictures one texel per 1/32 block.
    fn boxy(&mut self, lo: Vec3, hi: Vec3, part: Part, top: Part, bottom: Option<Part>, glow: bool) {
        let s = hi - lo;
        let span = |a: f32| ((a * 32.0).round() as i32).clamp(2, 16) as u8;
        // Corners per face as in mesh.rs FACES (+x, -x, +y, -y, +z, -z).
        const CORNERS: [[[u8; 3]; 4]; 6] = [
            [[1, 0, 1], [1, 0, 0], [1, 1, 0], [1, 1, 1]],
            [[0, 0, 0], [0, 0, 1], [0, 1, 1], [0, 1, 0]],
            [[0, 1, 1], [1, 1, 1], [1, 1, 0], [0, 1, 0]],
            [[0, 0, 0], [1, 0, 0], [1, 0, 1], [0, 0, 1]],
            [[0, 0, 1], [1, 0, 1], [1, 1, 1], [0, 1, 1]],
            [[1, 0, 0], [0, 0, 0], [0, 1, 0], [1, 1, 0]],
        ];
        for (f, cs) in CORNERS.iter().enumerate() {
            let part = match f {
                2 => top,
                3 => match bottom {
                    Some(b) => b,
                    None => continue,
                },
                _ => part,
            };
            let (su, sv) = match f {
                0 | 1 => (span(s.z), span(s.y)),
                2 | 3 => (span(s.x), span(s.z)),
                _ => (span(s.x), span(s.y)),
            };
            // Centre the texels on the picture so thin boxes show its middle.
            let (u0, v0) = ((16 - su) / 2, (16 - sv) / 2);
            let uv = [[u0, v0 + sv], [u0 + su, v0 + sv], [u0 + su, v0], [u0, v0]];
            let p = cs.map(|c| lo + vec3(c[0] as f32 * s.x, c[1] as f32 * s.y, c[2] as f32 * s.z));
            self.quad(p, uv, part, false, glow);
        }
    }

    /// A square stem `t` thick from `y0` to `y1` at (x, z).
    fn stem(&mut self, x: f32, z: f32, y0: f32, y1: f32, t: f32) {
        self.boxy(vec3(x - t / 2.0, y0, z - t / 2.0), vec3(x + t / 2.0, y1, z + t / 2.0), Part::Stem, Part::Stem, None, false);
    }

    /// A small cushion in the middle of a flower.
    fn center(&mut self, hub: Vec3, w: f32, h: f32, glow: bool) {
        self.boxy(hub - vec3(w / 2.0, 0.0, w / 2.0), hub + vec3(w / 2.0, h, w / 2.0), Part::Center, Part::Center, Some(Part::Center), glow);
    }

    fn sprite_plane(&mut self, a: Vec3, b: Vec3, h: f32) {
        self.quad([a, b, b + Vec3::Y * h, a + Vec3::Y * h], FULL_UV, Part::Sprite, true, false);
    }
}

/// The quads of a plant of this kind. `glow`: the plant gives light (its petals glow).
pub fn build(kind: Kind, glow: bool) -> Vec<RawQuad> {
    use Kind::*;
    let mut b = B { q: Vec::new(), stiff: 0, glow };
    let s = 1.0 / 32.0;
    match kind {
        Poppy => {
            b.stem(0.5, 0.5, 0.0, 0.6, 2.0 * s);
            b.petal(C + vec3(0.0, 0.03, 0.0), 0.4, 0.45, 0.34, 0.16, Part::LeafLong);
            b.petal(C + vec3(0.0, 0.03, 0.0), 0.4 + 2.5, 0.35, 0.3, 0.16, Part::LeafLong);
            b.petal(C + vec3(0.0, 0.26, 0.0), 1.9, 0.7, 0.2, 0.12, Part::Leaf);
            let hub = C + Vec3::Y * 0.58;
            b.ring(hub, 4, PI / 4.0, 0.55, 0.27, 0.32, Part::PetalFrilly, 0.02);
            b.ring(hub + Vec3::Y * 0.02, 4, 0.0, 1.05, 0.21, 0.26, Part::PetalFrilly, 0.02);
            b.center(hub, 3.0 * s, 3.0 * s, false);
        }
        Dandelion => {
            for k in 0..5 {
                b.petal(C + Vec3::Y * 0.02, k as f32 * 1.26 + 0.3, 0.18, 0.38, 0.14, Part::LeafLong);
            }
            b.stem(0.5, 0.5, 0.0, 0.62, 2.0 * s);
            let hub = C + Vec3::Y * 0.62;
            // A puff of thin petals, rings rising toward the middle.
            b.ring(hub, 12, 0.0, 0.55, 0.15, 0.1, Part::PetalNarrow, 0.02);
            b.ring(hub + Vec3::Y * 0.01, 9, 0.2, 1.0, 0.14, 0.1, Part::PetalNarrow, 0.015);
            b.ring(hub + Vec3::Y * 0.02, 5, 0.5, 1.4, 0.11, 0.09, Part::PetalNarrow, 0.0);
            b.center(hub - Vec3::Y * 0.02, 3.0 * s, 2.0 * s, false);
        }
        Cornflower => {
            b.stem(0.5, 0.5, 0.0, 0.66, 2.0 * s);
            b.petal(C + Vec3::Y * 0.05, 0.2, 1.0, 0.3, 0.08, Part::LeafLong);
            b.petal(C + Vec3::Y * 0.2, 2.4, 0.9, 0.26, 0.08, Part::LeafLong);
            b.petal(C + Vec3::Y * 0.36, 4.3, 1.0, 0.2, 0.07, Part::LeafLong);
            let hub = C + Vec3::Y * 0.66;
            b.ring(hub, 8, 0.0, 0.35, 0.21, 0.16, Part::PetalNotched, 0.02);
            b.ring(hub + Vec3::Y * 0.01, 6, 0.4, 0.9, 0.14, 0.12, Part::PetalNotched, 0.01);
            b.center(hub - Vec3::Y * 0.02, 3.0 * s, 3.0 * s, false);
        }
        Allium => {
            b.stem(0.5, 0.5, 0.0, 0.7, 2.0 * s);
            b.petal(C + Vec3::Y * 0.02, 0.8, 0.75, 0.38, 0.1, Part::LeafLong);
            b.petal(C + Vec3::Y * 0.02, 3.6, 0.6, 0.34, 0.1, Part::LeafLong);
            let c = C + Vec3::Y * 0.83;
            // Overlapping boxes cut the corners off, so the ball reads round from every side.
            for (w, d, h) in [(0.36, 0.22, 0.24), (0.22, 0.36, 0.24), (0.3, 0.3, 0.32), (0.18, 0.18, 0.4)] {
                b.boxy(c - vec3(w / 2.0, h / 2.0, d / 2.0), c + vec3(w / 2.0, h / 2.0, d / 2.0), Part::Floret, Part::Floret, Some(Part::Floret), glow);
            }
        }
        Bluet => {
            for k in 0..4 {
                b.petal(C, k as f32 * 1.57 + 0.5, 0.25, 0.2, 0.12, Part::Leaf);
            }
            for (x, z, h, yaw) in [(0.36, 0.4, 0.34, 0.2), (0.64, 0.38, 0.44, 0.9), (0.5, 0.67, 0.38, 0.5)] {
                b.stem(x, z, 0.0, h, 2.0 * s);
                let hub = vec3(x, h, z);
                b.ring(hub, 4, yaw, 0.3, 0.12, 0.12, Part::PetalRound, 0.01);
                b.center(hub, 2.0 * s, 2.0 * s, false);
            }
        }
        Daisy => {
            b.stem(0.5, 0.5, 0.0, 0.7, 2.0 * s);
            b.petal(C + Vec3::Y * 0.03, 0.6, 0.35, 0.26, 0.16, Part::Leaf);
            b.petal(C + Vec3::Y * 0.03, 3.3, 0.3, 0.24, 0.16, Part::Leaf);
            b.petal(C + Vec3::Y * 0.3, 2.0, 0.8, 0.22, 0.08, Part::LeafLong);
            let hub = C + Vec3::Y * 0.7;
            b.ring(hub, 14, 0.0, 0.3, 0.22, 0.12, Part::PetalNarrow, 0.05);
            b.center(hub - Vec3::Y * 0.01, 5.0 * s, 2.0 * s, false);
        }
        Tulip => {
            b.stem(0.5, 0.5, 0.0, 0.56, 2.0 * s);
            b.petal(C + Vec3::Y * 0.02, 0.3, 1.05, 0.46, 0.18, Part::LeafLong);
            b.petal(C + Vec3::Y * 0.02, 3.5, 1.15, 0.42, 0.17, Part::LeafLong);
            let hub = C + Vec3::Y * 0.54;
            b.ring(hub, 3, 0.0, 1.32, 0.28, 0.22, Part::PetalCup, 0.07);
            b.ring(hub + Vec3::Y * 0.02, 3, PI / 3.0, 1.42, 0.26, 0.2, Part::PetalCup, 0.05);
        }
        Orchid => {
            for k in 0..3 {
                b.petal(C + Vec3::Y * 0.02, k as f32 * 2.1 + 0.4, 0.3, 0.3, 0.2, Part::Leaf);
            }
            for (x, z, h, yaw) in [(0.4, 0.42, 0.5, 0.0), (0.62, 0.45, 0.62, 1.2), (0.48, 0.64, 0.42, 2.3)] {
                b.stem(x, z, 0.0, h, 2.0 * s);
                let hub = vec3(x, h, z);
                b.ring(hub, 5, yaw, 0.4, 0.15, 0.13, Part::PetalPointed, 0.01);
                b.center(hub, 2.0 * s, 2.0 * s, false);
            }
        }
        Lily => {
            // Two broad leaves to the sides, a stalk arching out between them with white bells
            // hanging under it.
            b.petal(C + vec3(-0.04, 0.0, 0.0), PI / 2.0 + 0.3, 1.2, 0.62, 0.26, Part::Leaf);
            b.petal(C + vec3(-0.04, 0.0, 0.0), -PI / 2.0 - 0.2, 1.25, 0.56, 0.24, Part::Leaf);
            b.stem(0.44, 0.5, 0.0, 0.72, 2.0 * s);
            b.boxy(vec3(0.43, 0.7, 0.48), vec3(0.86, 0.73, 0.52), Part::Stem, Part::Stem, Some(Part::Stem), false);
            for (k, x) in [0.53f32, 0.65, 0.77].iter().enumerate() {
                let y = 0.7 - k as f32 * 0.04;
                b.boxy(vec3(x - 0.01, y - 0.06, 0.49), vec3(x + 0.02, y, 0.51), Part::Stem, Part::Stem, None, false);
                b.boxy(vec3(x - 0.05, y - 0.17, 0.45), vec3(x + 0.06, y - 0.06, 0.56), Part::Bell, Part::Bell, Some(Part::Bell), glow);
            }
        }
        EyeOpen | EyeClosed => {
            b.stem(0.5, 0.5, 0.0, 0.55, 2.0 * s);
            b.petal(C + Vec3::Y * 0.02, 0.9, 0.4, 0.28, 0.15, Part::Leaf);
            b.petal(C + Vec3::Y * 0.02, 4.0, 0.35, 0.26, 0.15, Part::Leaf);
            let hub = C + Vec3::Y * 0.54;
            if kind == EyeOpen {
                b.ring(hub, 5, 0.3, 0.45, 0.24, 0.17, Part::PetalPointed, 0.02);
                b.center(hub, 5.0 * s, 3.0 * s, true);
            } else {
                b.ring(hub, 4, 0.0, 1.42, 0.24, 0.15, Part::PetalPointed, 0.04);
            }
        }
        Sunflower => {
            b.stiff = 1;
            b.boxy(vec3(0.44, 0.0, 0.45), vec3(0.54, 1.46, 0.55), Part::Stem, Part::Stem, None, false);
            for (y, yaw, len) in [(0.3, PI / 4.0, 0.42), (0.62, 1.25 * PI, 0.4), (0.94, 0.75 * PI, 0.36), (1.22, 1.75 * PI, 0.3)] {
                b.petal(vec3(0.49, y, 0.5), yaw, 0.22, len, 0.24, Part::Leaf);
            }
            b.boxy(vec3(0.46, 1.4, 0.46), vec3(0.6, 1.5, 0.54), Part::Stem, Part::Stem, Some(Part::Stem), false);
            // The head looks east: a green back, a brown seed disc and two rings of petals.
            let hub = vec3(0.66, 1.52, 0.5);
            b.boxy(vec3(0.57, 1.25, 0.23), vec3(0.64, 1.79, 0.77), Part::HeadBack, Part::HeadBack, Some(Part::HeadBack), false);
            b.boxy(vec3(0.62, 1.27, 0.25), vec3(0.69, 1.77, 0.75), Part::Seeds, Part::Seeds, Some(Part::Seeds), false);
            for k in 0..24 {
                let a = k as f32 * TAU / 24.0;
                let dir = vec3(0.0, a.sin(), a.cos());
                let back = k % 2 == 1;
                let out = (dir + Vec3::X * if back { 0.05 } else { 0.25 }).normalize();
                let side = dir.cross(Vec3::X);
                let (r, len) = if back { (0.22, 0.2) } else { (0.24, 0.17) };
                b.card(hub - Vec3::X * if back { 0.05 } else { 0.0 } + dir * r, side, out, 0.13, len, Part::PetalPointed, false);
            }
        }
        Torchflower => {
            b.stem(0.5, 0.5, 0.0, 0.5, 2.0 * s);
            b.petal(C + Vec3::Y * 0.02, 0.5, 0.4, 0.26, 0.15, Part::Leaf);
            b.petal(C + Vec3::Y * 0.02, 3.6, 0.4, 0.24, 0.15, Part::Leaf);
            let hub = C + Vec3::Y * 0.48;
            b.ring(hub, 6, 0.0, 1.25, 0.28, 0.14, Part::PetalPointed, 0.04);
            b.ring(hub, 3, 0.5, 1.45, 0.36, 0.13, Part::PetalPointed, 0.01);
        }
        Pitcher => {
            for k in 0..4 {
                b.petal(C, k as f32 * 1.57 + 0.4, 1.15, 0.6, 0.2, Part::LeafLong);
            }
            let hub = C + Vec3::Y * 0.22;
            b.stem(0.5, 0.5, 0.0, 0.24, 3.0 * s);
            b.ring(hub, 4, PI / 4.0, 1.38, 0.5, 0.2, Part::PetalCup, 0.07);
            b.petal(hub + Vec3::Y * 0.48 + flat(PI / 4.0) * 0.1, PI / 4.0, 0.35, 0.18, 0.2, Part::PetalPointed);
        }
        Radial => {
            b.stem(0.5, 0.5, 0.0, 0.62, 2.0 * s);
            b.petal(C + Vec3::Y * 0.03, 0.5, 0.4, 0.28, 0.16, Part::Leaf);
            b.petal(C + Vec3::Y * 0.03, 3.4, 0.35, 0.26, 0.16, Part::Leaf);
            let hub = C + Vec3::Y * 0.62;
            b.ring(hub, 6, 0.0, 0.5, 0.22, 0.18, Part::PetalRound, 0.02);
            b.ring(hub + Vec3::Y * 0.01, 6, PI / 6.0, 0.95, 0.15, 0.14, Part::PetalRound, 0.01);
            b.center(hub, 4.0 * s, 3.0 * s, glow);
        }
        Bromelia => {
            for k in 0..8 {
                let tilt = if k % 2 == 0 { 0.55 } else { 0.85 };
                b.petal(C, k as f32 * TAU / 8.0 + 0.2, tilt, 0.42, 0.13, Part::LeafLong);
            }
            b.ring(C + Vec3::Y * 0.1, 4, 0.0, 1.35, 0.4, 0.12, Part::PetalPointed, 0.02);
        }
        RedMushroom | BrownMushroom | GlowMushroom => {
            b.stiff = 2;
            let (stem_w, stem_h, caps): (f32, f32, &[(f32, f32, f32)]) = match kind {
                RedMushroom => (0.13, 0.36, &[(0.62, 0.32, 0.44), (0.5, 0.44, 0.54), (0.32, 0.54, 0.6)]),
                BrownMushroom => (0.12, 0.3, &[(0.7, 0.28, 0.35), (0.54, 0.35, 0.41), (0.3, 0.41, 0.44)]),
                _ => (0.08, 0.46, &[(0.52, 0.44, 0.53), (0.38, 0.53, 0.6), (0.2, 0.6, 0.63)]),
            };
            b.boxy(C - vec3(stem_w / 2.0, 0.0, stem_w / 2.0), C + vec3(stem_w / 2.0, stem_h, stem_w / 2.0), Part::MushStem, Part::MushStem, None, false);
            for (i, &(w, y0, y1)) in caps.iter().enumerate() {
                let under = if i == 0 { Some(Part::CapUnder) } else { None };
                b.boxy(vec3(0.5 - w / 2.0, y0, 0.5 - w / 2.0), vec3(0.5 + w / 2.0, y1, 0.5 + w / 2.0), Part::Cap, Part::Cap, under, glow);
            }
        }
        Sapling | Conifer => {
            b.stiff = 1;
            if kind == Sapling {
                b.boxy(vec3(0.46, 0.0, 0.46), vec3(0.54, 0.5, 0.54), Part::Wood, Part::Wood, None, false);
                b.boxy(vec3(0.53, 0.26, 0.48), vec3(0.66, 0.3, 0.52), Part::Wood, Part::Wood, Some(Part::Wood), false);
                for (lo, hi) in [
                    (vec3(0.28, 0.4, 0.3), vec3(0.72, 0.68, 0.7)),
                    (vec3(0.36, 0.68, 0.38), vec3(0.62, 0.8, 0.64)),
                    (vec3(0.6, 0.22, 0.38), vec3(0.8, 0.38, 0.6)),
                    (vec3(0.22, 0.52, 0.5), vec3(0.36, 0.64, 0.68)),
                ] {
                    b.boxy(lo, hi, Part::Foliage, Part::Foliage, Some(Part::Foliage), false);
                }
            } else {
                b.boxy(vec3(0.46, 0.0, 0.46), vec3(0.54, 0.24, 0.54), Part::Wood, Part::Wood, None, false);
                for (w, y0, y1) in [(0.56, 0.16, 0.34), (0.44, 0.34, 0.5), (0.32, 0.5, 0.66), (0.18, 0.66, 0.82)] {
                    b.boxy(vec3(0.5 - w / 2.0, y0, 0.5 - w / 2.0), vec3(0.5 + w / 2.0, y1, 0.5 + w / 2.0), Part::Needles, Part::Needles, Some(Part::Needles), false);
                }
            }
        }
        Grass => {
            for k in 0..11 {
                let yaw = k as f32 * 2.4;
                let r = 0.06 + (k % 3) as f32 * 0.1;
                let tilt = 1.08 + (k % 4) as f32 * 0.1;
                let len = 0.42 + ((k * 7) % 5) as f32 * 0.07;
                b.petal(C + flat(yaw) * r, yaw, tilt, len, 0.2, Part::Blade);
            }
        }
        Fern => {
            for k in 0..6 {
                let yaw = k as f32 * TAU / 6.0 + 0.3;
                let tilt = if k % 2 == 0 { 0.7 } else { 0.95 };
                b.petal(C + flat(yaw) * 0.03, yaw, tilt, 0.52, 0.26, Part::Frond);
            }
        }
        Star => {
            for k in 0..3 {
                let r = flat(k as f32 * PI / 3.0) * 0.44;
                b.sprite_plane(C - r, C + r, 0.95);
            }
        }
        Crop => {
            // A "#": four pictures, two each way.
            for o in [0.27f32, 0.73] {
                b.sprite_plane(vec3(0.06, 0.0, o), vec3(0.94, 0.0, o), 1.0);
                b.sprite_plane(vec3(o, 0.0, 0.94), vec3(o, 0.0, 0.06), 1.0);
            }
        }
        Cane => {
            // Three stalks of different thickness, a couple of long leaves.
            for (x, z, w) in [(0.3, 0.34, 0.13), (0.68, 0.3, 0.11), (0.5, 0.7, 0.12)] {
                b.boxy(vec3(x - w / 2.0, 0.0, z - w / 2.0), vec3(x + w / 2.0, 1.0, z + w / 2.0), Part::Stem, Part::Stem, None, false);
            }
            b.petal(vec3(0.3, 0.45, 0.34), 3.6, 0.85, 0.38, 0.12, Part::LeafLong);
            b.petal(vec3(0.68, 0.2, 0.3), 5.6, 0.9, 0.34, 0.12, Part::LeafLong);
            b.petal(vec3(0.5, 0.65, 0.7), 1.6, 0.8, 0.32, 0.11, Part::LeafLong);
        }
        Bamboo => {
            b.boxy(vec3(0.4, 0.0, 0.4), vec3(0.6, 1.0, 0.6), Part::Stem, Part::Stem, None, false);
            b.petal(vec3(0.6, 0.72, 0.5), 0.2, 0.45, 0.32, 0.14, Part::LeafLong);
            b.petal(vec3(0.4, 0.4, 0.5), 3.4, 0.5, 0.28, 0.13, Part::LeafLong);
        }
        Kelp => {
            b.boxy(vec3(0.47, 0.0, 0.47), vec3(0.53, 1.0, 0.53), Part::Stem, Part::Stem, None, false);
            for (k, y) in [0.12f32, 0.45, 0.78].iter().enumerate() {
                b.petal(vec3(0.5, *y, 0.5), k as f32 * 2.1 + 0.4, 0.75, 0.32, 0.18, Part::Leaf);
            }
        }
        Cattail | CattailTop => {
            let top = if kind == CattailTop { 0.5 } else { 1.0 };
            b.boxy(vec3(0.46, 0.0, 0.46), vec3(0.54, top, 0.54), Part::Stem, Part::Stem, None, false);
            if kind == CattailTop {
                b.boxy(vec3(0.43, 0.5, 0.43), vec3(0.57, 0.84, 0.57), Part::Wood, Part::Wood, Some(Part::Wood), false);
                b.boxy(vec3(0.48, 0.84, 0.48), vec3(0.52, 0.97, 0.52), Part::Stem, Part::Stem, None, false);
                b.petal(vec3(0.5, 0.0, 0.5), 1.0, 1.35, 0.5, 0.12, Part::LeafLong);
            } else {
                for k in 0..3 {
                    b.petal(vec3(0.5, 0.0, 0.5), k as f32 * 2.1 + 0.3, 1.32, 0.95, 0.13, Part::LeafLong);
                }
            }
        }
        Vine | HangingMoss => {
            // Strands from top to bottom with leaves (or fringes of moss) along them.
            let strands: &[(f32, f32)] = if kind == Vine { &[(0.3, 0.4), (0.62, 0.32), (0.5, 0.7)] } else { &[(0.32, 0.36), (0.66, 0.42), (0.44, 0.68), (0.7, 0.7)] };
            for (k, &(x, z)) in strands.iter().enumerate() {
                b.boxy(vec3(x - 0.025, 0.0, z - 0.025), vec3(x + 0.025, 1.0, z + 0.025), Part::Stem, Part::Stem, None, false);
                let y = 0.2 + (k as f32 * 0.31) % 0.6;
                if kind == Vine {
                    b.petal(vec3(x, y, z), k as f32 * 2.3, 0.3, 0.18, 0.16, Part::Leaf);
                    b.petal(vec3(x, y + 0.3, z), k as f32 * 2.3 + 3.0, 0.2, 0.16, 0.15, Part::Leaf);
                } else {
                    b.petal(vec3(x, 1.0, z), k as f32 * 1.7, -1.25, 0.55, 0.16, Part::Frond);
                }
            }
        }
        CaveVine => {
            b.boxy(vec3(0.47, 0.0, 0.47), vec3(0.53, 1.0, 0.53), Part::Stem, Part::Stem, None, false);
            b.petal(vec3(0.5, 0.75, 0.5), 0.4, 0.25, 0.2, 0.16, Part::Leaf);
            b.petal(vec3(0.5, 0.35, 0.5), 3.5, 0.2, 0.2, 0.16, Part::Leaf);
            for (x, y, z) in [(0.57, 0.5, 0.5), (0.42, 0.14, 0.46)] {
                b.boxy(vec3(x - 0.06, y - 0.12, z - 0.06), vec3(x + 0.06, y, z + 0.06), Part::Bloom, Part::Bloom, Some(Part::Bloom), true);
            }
        }
        Coral => {
            // A branching coral: a short trunk, arms, rounded tips.
            for (lo, hi) in [
                (vec3(0.44, 0.0, 0.44), vec3(0.56, 0.32, 0.56)),
                (vec3(0.24, 0.24, 0.45), vec3(0.76, 0.33, 0.55)),
                (vec3(0.45, 0.2, 0.26), vec3(0.55, 0.3, 0.74)),
                (vec3(0.24, 0.3, 0.44), vec3(0.36, 0.72, 0.56)),
                (vec3(0.64, 0.3, 0.44), vec3(0.76, 0.6, 0.56)),
                (vec3(0.44, 0.28, 0.64), vec3(0.56, 0.82, 0.76)),
                (vec3(0.45, 0.25, 0.25), vec3(0.55, 0.52, 0.35)),
            ] {
                b.boxy(lo, hi, Part::Bloom, Part::Center, Some(Part::Bloom), glow);
            }
        }
        Anemone => {
            b.boxy(vec3(0.34, 0.0, 0.34), vec3(0.66, 0.16, 0.66), Part::Bloom, Part::Center, None, glow);
            b.ring(C + Vec3::Y * 0.14, 10, 0.0, 1.15, 0.32, 0.08, Part::PetalNarrow, 0.12);
            b.ring(C + Vec3::Y * 0.15, 6, 0.3, 1.4, 0.36, 0.08, Part::PetalNarrow, 0.06);
        }
        Pickle => {
            for (x, z, w, h) in [(0.32, 0.32, 0.15, 0.3), (0.64, 0.4, 0.15, 0.4), (0.46, 0.67, 0.13, 0.24)] {
                b.boxy(vec3(x - w / 2.0, 0.0, z - w / 2.0), vec3(x + w / 2.0, h, z + w / 2.0), Part::Bloom, Part::Center, None, true);
            }
        }
        Cluster => {
            // Fruit hanging from the tree above on short stalks.
            b.boxy(vec3(0.47, 0.6, 0.47), vec3(0.53, 1.0, 0.53), Part::Wood, Part::Wood, None, false);
            for (x, y, z) in [(0.38, 0.5, 0.42), (0.6, 0.52, 0.4), (0.5, 0.42, 0.6), (0.36, 0.34, 0.6), (0.62, 0.32, 0.58), (0.5, 0.24, 0.46)] {
                b.boxy(vec3(x - 0.07, y - 0.12, z - 0.07), vec3(x + 0.07, y + 0.02, z + 0.07), Part::Bloom, Part::Bloom, Some(Part::Bloom), glow);
            }
        }
        Bulb => {
            b.stem(0.5, 0.5, 0.0, 0.3, 3.0 * s);
            b.petal(C + Vec3::Y * 0.02, 0.6, 0.4, 0.26, 0.15, Part::Leaf);
            b.petal(C + Vec3::Y * 0.02, 3.7, 0.4, 0.24, 0.15, Part::Leaf);
            b.boxy(vec3(0.34, 0.28, 0.34), vec3(0.66, 0.56, 0.66), Part::Bloom, Part::Bloom, Some(Part::Bloom), true);
            b.boxy(vec3(0.4, 0.56, 0.4), vec3(0.6, 0.64, 0.6), Part::Bloom, Part::Bloom, None, true);
        }
    }
    b.q
}

/// A cheaper body for far away (a few boxes), for plants whose near one is rich; None: the
/// near body is cheap enough to be drawn at every distance.
pub fn build_far(kind: Kind, glow: bool) -> Option<Vec<RawQuad>> {
    use Kind::*;
    let mut b = B { q: Vec::new(), stiff: 0, glow };
    // A stem and a head box of the flower's colour: (head height, head width, head depth).
    let flower = |b: &mut B, y: f32, w: f32, h: f32| {
        b.boxy(vec3(0.47, 0.0, 0.47), vec3(0.53, y, 0.53), Part::Stem, Part::Stem, None, false);
        b.boxy(vec3(0.5 - w / 2.0, y, 0.5 - w / 2.0), vec3(0.5 + w / 2.0, y + h, 0.5 + w / 2.0), Part::Bloom, Part::Bloom, Some(Part::Bloom), glow);
    };
    match kind {
        Poppy => flower(&mut b, 0.56, 0.34, 0.14),
        Dandelion => flower(&mut b, 0.6, 0.24, 0.12),
        Cornflower => flower(&mut b, 0.64, 0.28, 0.1),
        Allium => flower(&mut b, 0.7, 0.32, 0.3),
        Daisy | Radial => flower(&mut b, 0.66, 0.34, 0.08),
        Tulip => flower(&mut b, 0.54, 0.2, 0.24),
        Orchid | EyeOpen | EyeClosed | Torchflower => flower(&mut b, 0.52, 0.26, 0.14),
        Lily => {
            b.boxy(vec3(0.38, 0.0, 0.42), vec3(0.5, 0.6, 0.58), Part::Leaf, Part::Leaf, None, false);
            b.boxy(vec3(0.52, 0.48, 0.45), vec3(0.8, 0.6, 0.55), Part::Bloom, Part::Bloom, Some(Part::Bloom), glow);
        }
        Bluet => {
            b.boxy(vec3(0.3, 0.0, 0.3), vec3(0.7, 0.3, 0.7), Part::Leaf, Part::Leaf, None, false);
            b.boxy(vec3(0.32, 0.3, 0.32), vec3(0.68, 0.4, 0.68), Part::Bloom, Part::Bloom, None, glow);
        }
        Pitcher | Bromelia => {
            b.boxy(vec3(0.3, 0.0, 0.3), vec3(0.7, 0.3, 0.7), Part::Leaf, Part::Leaf, None, false);
            b.boxy(vec3(0.42, 0.2, 0.42), vec3(0.58, 0.62, 0.58), Part::Bloom, Part::Bloom, None, glow);
        }
        Sunflower => {
            b.stiff = 1;
            b.boxy(vec3(0.44, 0.0, 0.45), vec3(0.54, 1.46, 0.55), Part::Stem, Part::Stem, None, false);
            b.boxy(vec3(0.57, 1.18, 0.16), vec3(0.68, 1.86, 0.84), Part::Bloom, Part::Bloom, Some(Part::HeadBack), false);
        }
        RedMushroom | BrownMushroom | GlowMushroom => {
            b.stiff = 2;
            let (w, y0, y1) = match kind {
                RedMushroom => (0.56, 0.32, 0.56),
                BrownMushroom => (0.66, 0.28, 0.4),
                _ => (0.46, 0.44, 0.62),
            };
            b.boxy(vec3(0.44, 0.0, 0.44), vec3(0.56, y0, 0.56), Part::MushStem, Part::MushStem, None, false);
            b.boxy(vec3(0.5 - w / 2.0, y0, 0.5 - w / 2.0), vec3(0.5 + w / 2.0, y1, 0.5 + w / 2.0), Part::Cap, Part::Cap, Some(Part::CapUnder), glow);
        }
        Sapling => {
            b.stiff = 1;
            b.boxy(vec3(0.46, 0.0, 0.46), vec3(0.54, 0.42, 0.54), Part::Wood, Part::Wood, None, false);
            b.boxy(vec3(0.28, 0.4, 0.3), vec3(0.72, 0.76, 0.7), Part::Foliage, Part::Foliage, Some(Part::Foliage), false);
        }
        Conifer => {
            b.stiff = 1;
            b.boxy(vec3(0.24, 0.06, 0.24), vec3(0.76, 0.46, 0.76), Part::Needles, Part::Needles, Some(Part::Needles), false);
            b.boxy(vec3(0.36, 0.46, 0.36), vec3(0.64, 0.8, 0.64), Part::Needles, Part::Needles, None, false);
        }
        Grass | Fern => {
            let part = if kind == Grass { Part::Blade } else { Part::Frond };
            for k in 0..3 {
                let yaw = k as f32 * TAU / 3.0 + 0.4;
                b.petal(C + flat(yaw) * 0.1, yaw, 1.15, 0.5, 0.3, part);
            }
        }
        Cane => b.boxy(vec3(0.38, 0.0, 0.38), vec3(0.62, 1.0, 0.62), Part::Stem, Part::Stem, None, false),
        Kelp => {
            b.boxy(vec3(0.46, 0.0, 0.46), vec3(0.54, 1.0, 0.54), Part::Stem, Part::Stem, None, false);
            b.petal(vec3(0.5, 0.4, 0.5), 0.4, 0.75, 0.32, 0.2, Part::Leaf);
        }
        Vine | HangingMoss => {
            for (x, z) in [(0.36, 0.4), (0.62, 0.58)] {
                b.boxy(vec3(x - 0.04, 0.0, z - 0.04), vec3(x + 0.04, 1.0, z + 0.04), Part::Stem, Part::Stem, None, false);
            }
        }
        Coral => {
            b.boxy(vec3(0.42, 0.0, 0.42), vec3(0.58, 0.3, 0.58), Part::Bloom, Part::Bloom, None, glow);
            b.boxy(vec3(0.26, 0.3, 0.3), vec3(0.74, 0.72, 0.7), Part::Bloom, Part::Center, None, glow);
        }
        Anemone => {
            b.boxy(vec3(0.34, 0.0, 0.34), vec3(0.66, 0.16, 0.66), Part::Bloom, Part::Center, None, glow);
            b.ring(C + Vec3::Y * 0.14, 3, 0.4, 1.2, 0.34, 0.14, Part::PetalNarrow, 0.1);
        }
        Cattail => {
            b.boxy(vec3(0.46, 0.0, 0.46), vec3(0.54, 1.0, 0.54), Part::Stem, Part::Stem, None, false);
            b.petal(vec3(0.5, 0.0, 0.5), 0.3, 1.32, 0.95, 0.16, Part::LeafLong);
        }
        CattailTop => {
            b.boxy(vec3(0.46, 0.0, 0.46), vec3(0.54, 0.5, 0.54), Part::Stem, Part::Stem, None, false);
            b.boxy(vec3(0.43, 0.5, 0.43), vec3(0.57, 0.86, 0.57), Part::Wood, Part::Wood, None, false);
        }
        CaveVine => {
            b.boxy(vec3(0.46, 0.0, 0.46), vec3(0.54, 1.0, 0.54), Part::Stem, Part::Stem, None, false);
            b.boxy(vec3(0.5, 0.36, 0.44), vec3(0.62, 0.5, 0.56), Part::Bloom, Part::Bloom, Some(Part::Bloom), true);
        }
        Pickle => {
            b.boxy(vec3(0.26, 0.0, 0.26), vec3(0.42, 0.3, 0.42), Part::Bloom, Part::Center, None, true);
            b.boxy(vec3(0.56, 0.0, 0.32), vec3(0.72, 0.4, 0.48), Part::Bloom, Part::Center, None, true);
        }
        Cluster => {
            b.boxy(vec3(0.47, 0.6, 0.47), vec3(0.53, 1.0, 0.53), Part::Wood, Part::Wood, None, false);
            b.boxy(vec3(0.32, 0.2, 0.32), vec3(0.68, 0.6, 0.68), Part::Bloom, Part::Bloom, Some(Part::Bloom), glow);
        }
        Bulb => {
            b.boxy(vec3(0.46, 0.0, 0.46), vec3(0.54, 0.3, 0.54), Part::Stem, Part::Stem, None, false);
            b.boxy(vec3(0.34, 0.28, 0.34), vec3(0.66, 0.6, 0.66), Part::Bloom, Part::Bloom, None, true);
        }
        _ => return None,
    }
    Some(b.q)
}

/// Shading face for a direction: up, down, or the nearest side.
fn face_of(n: Vec3) -> u8 {
    let n = n.normalize_or_zero();
    if n.y > 0.6 {
        2
    } else if n.y < -0.6 {
        3
    } else if n.x.abs() > n.z.abs() {
        if n.x > 0.0 { 0 } else { 1 }
    } else if n.z > 0.0 {
        4
    } else {
        5
    }
}

/// Quantize to 1/32 block, with each part's picture layer from `layer`; parts it has no layer
/// for are left out.
pub fn finish(raw: &[RawQuad], tint: bool, layer: &dyn Fn(Part) -> Option<u16>) -> Vec<FloraQuad> {
    raw.iter()
        .filter_map(|q| {
            let l = layer(q.part)?;
            let pos = q.p.map(|p| {
                [
                    (p.x * 32.0).round().clamp(1.0, 31.0) as u16,
                    (p.y * 32.0).round().clamp(0.0, MAX_Y as f32) as u16,
                    (p.z * 32.0).round().clamp(1.0, 31.0) as u16,
                ]
            });
            let n = (q.p[1] - q.p[0]).cross(q.p[2] - q.p[0]);
            Some(FloraQuad {
                pos,
                uv: q.uv,
                layer: l,
                face: [face_of(n), face_of(-n)],
                two: q.two,
                stiff: q.stiff,
                glow: q.glow,
                tint: tint && q.part.follows_tint(),
            })
        })
        .collect()
}

/// Every part a plant of this kind uses.
pub fn parts(raw: &[RawQuad]) -> Vec<Part> {
    let mut v: Vec<Part> = raw.iter().map(|q| q.part).collect();
    v.sort();
    v.dedup();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Kind; 38] = [
        Kind::Poppy,
        Kind::Dandelion,
        Kind::Cornflower,
        Kind::Allium,
        Kind::Bluet,
        Kind::Daisy,
        Kind::Tulip,
        Kind::Orchid,
        Kind::Lily,
        Kind::EyeOpen,
        Kind::EyeClosed,
        Kind::Sunflower,
        Kind::Torchflower,
        Kind::Pitcher,
        Kind::Radial,
        Kind::Bromelia,
        Kind::RedMushroom,
        Kind::BrownMushroom,
        Kind::GlowMushroom,
        Kind::Sapling,
        Kind::Conifer,
        Kind::Grass,
        Kind::Fern,
        Kind::Star,
        Kind::Crop,
        Kind::Cane,
        Kind::Bamboo,
        Kind::Kelp,
        Kind::Cattail,
        Kind::CattailTop,
        Kind::Vine,
        Kind::CaveVine,
        Kind::HangingMoss,
        Kind::Coral,
        Kind::Anemone,
        Kind::Pickle,
        Kind::Cluster,
        Kind::Bulb,
    ];

    fn quads(raw: &[RawQuad]) -> usize {
        raw.iter().map(|q| if q.two { 2 } else { 1 }).sum()
    }

    #[test]
    fn far_bodies_cost_about_what_crossed_pictures_did() {
        for k in ALL {
            let near = build(k, false);
            let far = build_far(k, false);
            let far_n = far.as_ref().map(|f| quads(f)).unwrap_or(quads(&near));
            // Crossed pictures were 8 quads (two planes, both sides).
            assert!(far_n <= 12, "{k:?} far: {far_n} quads");
            if let Some(f) = far {
                assert!(quads(&f) < quads(&near), "{k:?}");
            }
        }
    }

    #[test]
    fn stacked_plants_fill_their_block_and_stay_inside_it() {
        for k in ALL.into_iter().filter(|k| k.stacked()) {
            for raw in [Some(build(k, false)), build_far(k, false)].into_iter().flatten() {
                let lo = raw.iter().flat_map(|q| q.p).map(|p| p.y).fold(9.0f32, f32::min);
                let hi = raw.iter().flat_map(|q| q.p).map(|p| p.y).fold(0.0f32, f32::max);
                assert!(lo < 0.01 && hi <= 1.0 + 1e-4, "{k:?}: {lo}..{hi}");
            }
        }
        assert_eq!(kind_of("kelp"), Some(Kind::Kelp));
        assert_eq!(kind_of("tube_coral"), Some(Kind::Coral));
        assert_eq!(kind_of("amethyst_cluster"), None);
    }

    #[test]
    fn plants_fit_their_column_and_stay_cheap() {
        for k in ALL {
            let raw = build(k, false);
            assert!(!raw.is_empty(), "{k:?}");
            for q in &raw {
                for p in q.p {
                    assert!(p.x > -0.01 && p.x < 1.01 && p.z > -0.01 && p.z < 1.01, "{k:?} leaves its block: {p}");
                    assert!(p.y > -0.01 && p.y * 32.0 <= MAX_Y as f32 + 0.5, "{k:?} too tall: {p}");
                }
            }
            // Two-sided cards count twice; a plant stays well under a hundred quads.
            let quads: usize = raw.iter().map(|q| if q.two { 2 } else { 1 }).sum();
            assert!(quads <= 90, "{k:?}: {quads} quads");
            let tall = raw.iter().flat_map(|q| q.p).map(|p| p.y).fold(0.0f32, f32::max);
            if k == Kind::Sunflower {
                assert!(tall > 1.7, "the sunflower stands two blocks tall: {tall}");
            }
        }
    }

    #[test]
    fn flowers_are_bigger_than_the_old_crossed_pictures() {
        for name in ["poppy", "dandelion", "cornflower", "allium", "oxeye_daisy", "red_tulip", "lily_of_the_valley"] {
            let raw = build(kind_of(name).unwrap(), false);
            let tall = raw.iter().flat_map(|q| q.p).map(|p| p.y).fold(0.0f32, f32::max);
            assert!(tall >= 0.68, "{name}: {tall}");
        }
        assert_eq!(kind_of("fire"), None);
        assert_eq!(kind_of("wheat_stage3"), Some(Kind::Crop));
    }

    #[test]
    fn faces_point_out_of_boxes() {
        let raw = build(Kind::RedMushroom, false);
        let top = raw.iter().find(|q| !q.two && q.p.iter().all(|p| (p.y - 0.6).abs() < 1e-4)).unwrap();
        let n = (top.p[1] - top.p[0]).cross(top.p[2] - top.p[0]);
        assert!(n.y > 0.0);
    }
}
