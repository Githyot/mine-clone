//! Summoned creatures with real bodies: every minion, sentry and trap the summoner calls is a
//! 3D model built from boxes here (our own art, painted in code into one skin layer) and drawn
//! with the mob pipeline, one bone per moving piece.
//!
//! - Fogo-fátuo: a glowing core with eyes inside three flickering flame tongues, a wispy tail
//!   and motes circling it.
//! - Vespa: thorax, striped abdomen with a stinger that jabs, head with compound eyes,
//!   mandibles and antennae, four buzzing wings and dangling legs.
//! - Diabrete: a little devil with curled horns, pointed ears, glowing eyes and fangs, bat
//!   wings, a tail with an arrow tip, and a fireball it winds up and throws.
//! - Lâmina Voadora: a sword with a glowing fuller, gilded guard and pommel gem, three rune
//!   shards circling it; it hangs point down, then stabs and slashes.
//! - Geleinha: a jellyfish with a layered bell, frilled rim, oral arms and six tentacles of two
//!   segments that trail and curl as the bell pulses.
//! - Dragão Estelar: a serpent dragon. Head with horns, brow, glowing eyes, whiskers and an
//!   opening jaw full of teeth; eight body segments with belly plates and dorsal fins that
//!   follow the path the head flew; clawed legs, two membrane wings that beat, a star-tipped
//!   tail fin.
//! - Sentries: a stone brazier tower whose fire turns to aim, an ice crystal floating over a
//!   frozen rock with shards circling it, a thunderbird totem under its own storm cloud, a
//!   wooden ballista that turns, tilts, looses its bolt and reloads, and a buried powder trap
//!   with a blinking fuse.

use std::f32::consts::{PI, TAU};

use glam::{Mat4, Quat, Vec3};

use crate::entity::Bones;
use crate::instruments::{IBox, Piece};

// Minion models, in the order of `rpg::MINIONS`.
pub const WISP: usize = 0;
pub const WASP: usize = 1;
pub const IMP: usize = 2;
pub const BLADE: usize = 3;
pub const JELLY: usize = 4;
pub const DRAGON: usize = 5;
// Sentry models: the four of `sentries::SENTRIES` in order, then the trap.
pub const EMBER: usize = 6;
pub const FROST: usize = 7;
pub const STORM: usize = 8;
pub const BALLISTA: usize = 9;
pub const TRAP: usize = 10;
// Pets and mounts that were plain cubes (their mobs are drawn with these instead).
pub const PET_ORB: usize = 11;
pub const PET_HEART: usize = 12;
pub const UFO: usize = 13;
pub const COUNT: usize = 14;

/// Body segments of the dragon (bones 2..2+SEGMENTS).
pub const SEGMENTS: usize = 8;
/// Space between the dragon's segments along its path, in blocks.
pub const SEG_GAP: f32 = 0.5;

/// Model for a minion kind (`rpg::MINIONS` index).
pub fn minion_model(kind: usize) -> usize {
    kind.min(DRAGON)
}

/// Model drawn in place of a pet or mount mob whose own model is too plain.
pub fn pet_model(mob: &str) -> Option<usize> {
    match mob {
        "pet_orb" => Some(PET_ORB),
        "pet_heart" => Some(PET_HEART),
        "pet_wisp" => Some(WISP),
        "pet_jelly" => Some(JELLY),
        "mount_ufo" => Some(UFO),
        _ => None,
    }
}

/// Model for a Terraria sentry rod (tr_weapons.rs), by item name.
pub fn rod_model(item: &str) -> Option<usize> {
    match item {
        "ballista_rod" => Some(BALLISTA),
        "flame_sentry_rod" => Some(EMBER),
        "lightning_aura_rod" => Some(STORM),
        "explosive_trap_rod" => Some(TRAP),
        _ => None,
    }
}

// Materials: one 16 × 16 swatch each in the summon skin (4 across, 8 down).
const CORE: u8 = 0;
const SPIRIT: u8 = 1;
const SPIRIT_TIP: u8 = 2;
const EYE: u8 = 3;
const WASP_YELLOW: u8 = 4;
const CHITIN: u8 = 5;
const WING: u8 = 6;
const FLAME: u8 = 7;
const IMP_SKIN: u8 = 8;
const IMP_DARK: u8 = 9;
const HORN: u8 = 10;
const EMBER_GLOW: u8 = 11;
const STEEL: u8 = 12;
const GOLD: u8 = 13;
const LEATHER: u8 = 14;
const RUNE: u8 = 15;
const BELL: u8 = 16;
const JELLY_CORE: u8 = 17;
const SCALE: u8 = 18;
const BACK: u8 = 19;
const BELLY: u8 = 20;
const MEMBRANE: u8 = 21;
const STONE: u8 = 22;
const STONE_CRACK: u8 = 23;
const ICE: u8 = 24;
const ICE_DEEP: u8 = 25;
const WOOD: u8 = 26;
const PAINTED: u8 = 27;
const IRON: u8 = 28;
const ROPE: u8 = 29;
const CLOUD: u8 = 30;
const BOLT: u8 = 31;
const MATERIALS: u8 = 32;

/// A box by its centre and size (model pixels).
fn c(center: [f32; 3], size: [f32; 3], mat: u8) -> IBox {
    let (m, h) = (Vec3::from(center), Vec3::from(size) * 0.5);
    IBox { lo: m - h, hi: m + h, mat }
}

/// The same boxes on the other side (X mirrored).
fn mirror(boxes: &[IBox]) -> Vec<IBox> {
    boxes.iter().map(|b| IBox { lo: Vec3::new(-b.hi.x, b.lo.y, b.lo.z), hi: Vec3::new(-b.lo.x, b.hi.y, b.hi.z), mat: b.mat }).collect()
}

/// Boxes and their mirror image.
fn both(boxes: Vec<IBox>) -> Vec<IBox> {
    let m = mirror(&boxes);
    boxes.into_iter().chain(m).collect()
}

fn piece(part: u32, boxes: Vec<IBox>) -> Piece {
    Piece { part, place: Mat4::IDENTITY, boxes }
}

fn placed(part: u32, place: Mat4, boxes: Vec<IBox>) -> Piece {
    Piece { part, place, boxes }
}

/// Two crossed slabs (a flame tongue seen from any side), from `y0` to `y1`.
fn tongue(w: f32, n: f32, y0: f32, y1: f32, mat: u8) -> Vec<IBox> {
    let (h, n) = (w / 2.0, n / 2.0);
    vec![IBox { lo: Vec3::new(-h, y0, -n), hi: Vec3::new(h, y1, n), mat }, IBox { lo: Vec3::new(-n, y0, -h), hi: Vec3::new(n, y1, h), mat }]
}

/// A tapering horn along +Z from the origin, `w` wide at the root, `len` long.
fn horn(w: f32, len: f32, root: u8, tip: u8) -> Vec<IBox> {
    let n = 4;
    (0..n)
        .map(|k| {
            let t = k as f32 / n as f32;
            let s = w * (1.0 - t * 0.7);
            c([0.0, 0.0, len * (t + 0.5 / n as f32)], [s, s, len / n as f32 + 0.3], if k == n - 1 { tip } else { root })
        })
        .collect()
}

/// About `pivot`, turned by `r`.
fn about(pivot: Vec3, r: Quat) -> Mat4 {
    Mat4::from_translation(pivot) * Mat4::from_quat(r) * Mat4::from_translation(-pivot)
}

/// A rotation turning +Z toward `d` with no roll (Y stays up).
pub fn face(d: Vec3) -> Quat {
    let d = d.normalize_or_zero();
    if d == Vec3::ZERO {
        return Quat::IDENTITY;
    }
    let h = (d.x * d.x + d.z * d.z).sqrt();
    Quat::from_rotation_y(d.x.atan2(d.z)) * Quat::from_rotation_x((-d.y).atan2(h))
}

/// How big each segment of the dragon is against the middle of its body.
fn seg_size(i: usize) -> f32 {
    match i {
        0 => 0.78,
        1 => 0.92,
        _ => 1.0 - (i as f32 - 2.0) / (SEGMENTS as f32 - 2.0) * 0.5,
    }
}

/// The boxes of model `i`, each piece on its bone.
pub fn model(i: usize) -> Vec<Piece> {
    match i {
        WISP => {
            // Bone 0 the core with its face, 1..=3 the flame tongues, 4..=6 motes, 7 the tail.
            let mut core = vec![
                c([0.0, 0.0, 0.0], [5.0, 5.0, 5.0], CORE),
                c([0.0, 0.0, 0.0], [6.0, 4.0, 4.0], CORE),
                c([0.0, 0.0, 0.0], [4.0, 6.0, 4.0], CORE),
                c([0.0, 0.0, 0.0], [4.0, 4.0, 6.0], CORE),
            ];
            core.extend(both(vec![c([1.2, 0.5, 3.0], [1.1, 1.7, 0.3], EYE)]));
            core.push(c([0.0, -1.1, 3.0], [1.4, 0.5, 0.3], EYE));
            let mut flame = tongue(6.6, 3.4, -2.5, 3.5, SPIRIT);
            flame.extend(tongue(7.2, 4.0, -1.0, 1.5, SPIRIT));
            let mid = tongue(4.2, 2.4, 2.0, 7.0, SPIRIT);
            let mut tip = tongue(2.0, 1.2, 5.5, 10.5, SPIRIT_TIP);
            tip.push(c([1.6, 6.5, 0.0], [0.8, 2.6, 0.8], SPIRIT_TIP));
            tip.push(c([-1.3, 5.8, 0.5], [0.8, 2.0, 0.8], SPIRIT_TIP));
            let mote = |k: u32| piece(k, vec![c([0.0, 0.0, 0.0], [1.3, 1.3, 1.3], CORE), c([0.0, 0.0, 0.0], [0.6, 2.2, 0.6], SPIRIT)]);
            let tail = vec![
                c([0.0, -0.5, -4.0], [3.6, 3.0, 3.0], SPIRIT),
                c([0.0, -0.2, -6.5], [2.4, 2.0, 2.6], SPIRIT),
                c([0.0, 0.3, -8.6], [1.4, 1.2, 2.0], SPIRIT_TIP),
                c([0.0, 0.8, -10.0], [0.7, 0.7, 1.2], SPIRIT_TIP),
            ];
            vec![piece(0, core), piece(1, flame), piece(2, mid), piece(3, tip), mote(4), mote(5), mote(6), piece(7, tail)]
        }
        WASP => {
            // 0 thorax, 1 head, 2 abdomen, 3/4 fore wings, 5/6 hind wings, 7 legs.
            let thorax = vec![
                c([0.0, 0.0, 0.0], [5.0, 5.0, 5.4], CHITIN),
                c([0.0, 1.8, 0.6], [5.4, 1.8, 2.4], WASP_YELLOW),
                c([0.0, 0.0, 2.6], [3.6, 3.6, 0.8], CHITIN),
                c([0.0, -0.6, -2.6], [3.0, 3.0, 0.8], CHITIN),
            ];
            let mut head = vec![
                c([0.0, 0.6, 4.6], [4.4, 4.4, 3.6], WASP_YELLOW),
                c([0.0, 2.4, 4.2], [3.6, 0.8, 3.0], CHITIN),
                c([0.0, -1.2, 6.6], [2.0, 1.4, 0.8], WASP_YELLOW),
            ];
            head.extend(both(vec![
                // Compound eyes bulging from the sides, mandibles, antennae.
                c([2.1, 1.0, 4.9], [1.4, 3.2, 3.0], EYE),
                c([0.9, -1.8, 6.9], [0.8, 0.8, 1.8], CHITIN),
                c([0.5, -2.3, 7.7], [0.8, 0.6, 0.6], CHITIN),
                c([1.3, 3.8, 5.6], [0.5, 2.6, 0.5], CHITIN),
                c([1.6, 5.1, 6.6], [0.5, 0.5, 2.0], CHITIN),
                c([1.8, 5.0, 7.8], [0.5, 1.0, 0.5], WASP_YELLOW),
            ]));
            let abdomen = vec![
                c([0.0, 0.0, -4.6], [3.0, 3.0, 2.4], CHITIN),
                c([0.0, 0.0, -6.8], [5.0, 5.0, 2.4], WASP_YELLOW),
                c([0.0, 0.0, -8.6], [5.2, 5.2, 1.4], CHITIN),
                c([0.0, 0.0, -10.2], [4.6, 4.6, 2.0], WASP_YELLOW),
                c([0.0, 0.0, -11.8], [3.8, 3.8, 1.4], CHITIN),
                c([0.0, 0.0, -13.1], [2.6, 2.6, 1.4], WASP_YELLOW),
                c([0.0, 0.0, -14.4], [1.4, 1.4, 1.2], CHITIN),
                c([0.0, 0.0, -15.6], [0.5, 0.5, 1.6], CHITIN),
            ];
            let fore = vec![c([5.2, 2.6, -0.6], [8.4, 0.3, 3.6], WING), c([8.6, 2.6, -1.2], [2.0, 0.3, 2.8], WING)];
            let hind = vec![c([4.0, 2.4, -3.0], [6.0, 0.3, 2.6], WING)];
            let mut legs = Vec::new();
            for z in [-1.6, 0.0, 1.6] {
                legs.extend(both(vec![c([3.2, -1.8, z], [2.2, 0.6, 0.6], CHITIN), c([4.1, -3.6, z + 0.4], [0.6, 3.4, 0.6], CHITIN), c([4.1, -5.4, z + 0.9], [0.6, 0.6, 1.4], WASP_YELLOW)]));
            }
            vec![piece(0, thorax), piece(1, head), piece(2, abdomen), piece(3, fore.clone()), piece(4, mirror(&fore)), piece(5, hind.clone()), piece(6, mirror(&hind)), piece(7, legs)]
        }
        IMP => {
            // 0 torso, 1 head, 2/3 arms, 4/5 legs, 6/7 wings, 8 fireball, 9 tail.
            let torso = vec![
                c([0.0, 0.0, 0.0], [5.4, 5.6, 3.8], IMP_SKIN),
                c([0.0, 1.4, 0.2], [6.6, 2.6, 4.2], IMP_SKIN),
                c([0.0, -0.6, 1.8], [3.6, 3.0, 0.6], IMP_DARK),
                c([0.0, -2.8, 0.0], [5.0, 1.2, 3.8], IMP_DARK),
            ];
            let mut head = vec![
                c([0.0, 6.0, 0.4], [6.0, 5.4, 5.4], IMP_SKIN),
                c([0.0, 7.6, 3.1], [6.2, 1.0, 0.9], IMP_DARK),
                c([0.0, 5.4, 3.5], [1.2, 1.4, 0.8], IMP_SKIN),
                c([0.0, 4.2, 3.15], [3.4, 0.7, 0.3], EYE),
                c([0.0, 3.4, 1.2], [3.6, 1.0, 3.0], IMP_SKIN),
            ];
            head.extend(both(vec![
                c([1.5, 6.6, 3.15], [1.5, 1.1, 0.4], EMBER_GLOW),
                c([0.9, 3.8, 3.25], [0.5, 0.7, 0.3], HORN),
                // Pointed ears.
                c([3.5, 6.6, 0.2], [1.2, 2.0, 1.2], IMP_SKIN),
                c([4.4, 7.4, -0.1], [1.0, 1.6, 0.8], IMP_SKIN),
                c([5.1, 8.2, -0.3], [0.6, 1.0, 0.5], IMP_DARK),
            ]));
            let mut pieces = vec![piece(0, torso), piece(1, head)];
            // Horns curling up, out and back.
            for s in [1.0f32, -1.0] {
                let at = Mat4::from_translation(Vec3::new(1.9 * s, 8.4, 0.8)) * Mat4::from_quat(Quat::from_rotation_y(-0.35 * s + PI) * Quat::from_rotation_x(-1.1));
                pieces.push(placed(1, at, horn(1.6, 5.0, HORN, IMP_DARK)));
            }
            let arm = vec![c([3.9, 0.3, 0.0], [1.7, 4.4, 1.7], IMP_SKIN), c([3.9, -2.4, 0.2], [2.0, 1.4, 2.0], IMP_DARK), c([3.9, -3.3, 0.9], [0.5, 0.8, 0.5], HORN)];
            pieces.push(piece(2, arm.clone()));
            pieces.push(piece(3, mirror(&arm)));
            let leg = vec![c([1.5, -4.3, 0.6], [2.0, 2.8, 2.2], IMP_SKIN), c([1.5, -6.2, 0.0], [1.6, 2.0, 1.6], IMP_SKIN), c([1.5, -7.5, 0.6], [1.8, 0.8, 2.8], IMP_DARK)];
            pieces.push(piece(4, leg.clone()));
            pieces.push(piece(5, mirror(&leg)));
            // Bat wings on the back: an arm bone along the top, three scalloped panels, fingers.
            let wing = vec![
                c([3.8, 2.4, -2.2], [7.6, 0.9, 0.9], IMP_DARK),
                c([8.0, 2.0, -2.2], [1.2, 1.6, 0.9], HORN),
                c([1.6, -0.6, -2.3], [3.2, 5.2, 0.3], MEMBRANE),
                c([4.6, 0.0, -2.3], [2.8, 4.0, 0.3], MEMBRANE),
                c([7.2, 0.9, -2.3], [2.4, 2.2, 0.3], MEMBRANE),
                c([3.2, -0.2, -2.25], [0.4, 4.6, 0.4], IMP_DARK),
                c([6.0, 0.4, -2.25], [0.4, 3.4, 0.4], IMP_DARK),
            ];
            pieces.push(piece(6, wing.clone()));
            pieces.push(piece(7, mirror(&wing)));
            let mut ball = vec![c([0.0, 0.0, 0.0], [2.6, 2.6, 2.6], EMBER_GLOW)];
            ball.extend(tongue(3.4, 3.4, -1.2, 1.2, FLAME));
            ball.extend(tongue(1.6, 1.6, 1.0, 3.0, FLAME));
            pieces.push(piece(8, ball));
            pieces.push(piece(
                9,
                vec![
                    c([0.0, -2.4, -2.6], [1.1, 1.1, 2.4], IMP_SKIN),
                    c([0.0, -3.0, -4.6], [0.9, 0.9, 2.0], IMP_SKIN),
                    c([0.0, -3.2, -6.3], [0.8, 0.8, 1.6], IMP_DARK),
                    c([0.0, -3.2, -7.5], [2.4, 0.5, 1.2], IMP_DARK),
                    c([0.0, -3.2, -8.3], [1.2, 0.5, 0.6], IMP_DARK),
                ],
            ));
            pieces
        }
        BLADE => {
            // Point along +Z, flat of the blade facing ±X. 0 the sword, 1..=3 rune shards.
            let sword = vec![
                c([0.0, 0.0, 7.0], [0.7, 2.6, 14.0], STEEL),
                c([0.0, 0.0, 6.0], [0.9, 0.8, 11.0], RUNE),
                c([0.0, 0.0, 14.5], [0.6, 1.8, 1.0], STEEL),
                c([0.0, 0.0, 15.3], [0.5, 0.9, 0.8], STEEL),
                c([0.0, 0.0, -0.6], [1.6, 7.4, 1.4], GOLD),
                c([0.0, 3.9, -0.1], [1.9, 1.2, 1.9], GOLD),
                c([0.0, -3.9, -0.1], [1.9, 1.2, 1.9], GOLD),
                c([0.0, 0.0, -0.4], [1.8, 1.6, 1.0], RUNE),
                c([0.0, 0.0, -3.2], [1.3, 1.3, 4.0], LEATHER),
                c([0.0, 0.0, -2.4], [1.5, 1.5, 0.5], GOLD),
                c([0.0, 0.0, -5.6], [2.0, 2.0, 1.6], GOLD),
                c([0.0, 0.0, -5.6], [2.2, 0.9, 0.9], RUNE),
            ];
            let shard = |k: u32| piece(k, vec![c([0.0, 0.0, 0.0], [0.8, 2.0, 0.8], RUNE), c([0.0, 0.0, 0.0], [1.4, 0.8, 0.8], RUNE)]);
            vec![piece(0, sword), shard(1), shard(2), shard(3)]
        }
        JELLY => {
            // 0 bell, 1 oral arms, 2..=13 six tentacles of two segments.
            let mut bell = vec![
                c([0.0, 3.0, 0.0], [10.0, 3.0, 10.0], BELL),
                c([0.0, 5.0, 0.0], [8.4, 2.0, 8.4], BELL),
                c([0.0, 6.5, 0.0], [5.6, 1.4, 5.6], BELL),
                c([0.0, 7.4, 0.0], [2.8, 0.8, 2.8], JELLY_CORE),
                c([0.0, 1.2, 0.0], [11.0, 0.8, 11.0], BELL),
                c([0.0, 1.6, 0.0], [8.0, 1.0, 8.0], JELLY_CORE),
            ];
            // A frilled rim: little scallops round the edge.
            for k in 0..8 {
                let a = k as f32 / 8.0 * TAU;
                bell.push(c([a.cos() * 5.4, 0.6, a.sin() * 5.4], [1.6, 1.0, 1.6], JELLY_CORE));
            }
            bell.extend(both(vec![c([2.0, 3.4, 5.05], [1.1, 1.2, 0.3], EYE)]));
            let arms = vec![c([0.0, 0.0, 0.0], [2.6, 2.4, 2.6], JELLY_CORE), c([0.6, -2.4, 0.4], [1.2, 2.6, 1.2], JELLY_CORE), c([-0.7, -2.0, -0.5], [1.0, 2.0, 1.0], BELL)];
            let mut pieces = vec![piece(0, bell), piece(1, arms)];
            for j in 0..6 {
                let (x, z) = jelly_root(j);
                pieces.push(piece(2 + j as u32 * 2, vec![c([x, -1.0, z], [0.9, 4.2, 0.9], BELL), c([x, 0.6, z], [1.2, 0.8, 1.2], JELLY_CORE)]));
                pieces.push(piece(3 + j as u32 * 2, vec![c([x, -5.0, z], [0.7, 4.2, 0.7], BELL), c([x, -7.3, z], [0.5, 0.8, 0.5], JELLY_CORE)]));
            }
            pieces
        }
        DRAGON => {
            // 0 head, 1 jaw, 2..=9 body segments, 10/11 wings, 12 tail fin.
            let mut head = vec![
                c([0.0, 1.0, 0.0], [7.0, 5.0, 7.0], SCALE),
                c([0.0, 3.8, -0.4], [5.6, 1.0, 6.0], BACK),
                c([0.0, 0.8, 5.4], [5.0, 3.0, 5.0], SCALE),
                c([0.0, 2.4, 5.0], [3.6, 0.8, 4.0], BACK),
                c([0.0, 1.0, 8.3], [4.0, 2.4, 1.6], SCALE),
                c([0.0, -0.6, 2.4], [6.0, 1.0, 4.0], BELLY),
                c([0.0, 4.6, -2.6], [1.0, 1.8, 2.2], MEMBRANE),
                c([0.0, 4.2, -4.0], [0.8, 1.4, 1.6], MEMBRANE),
            ];
            head.extend(both(vec![
                c([1.1, 1.9, 9.15], [0.9, 0.7, 0.2], EYE),
                // Glowing eyes under a heavy brow.
                c([3.05, 2.1, 1.6], [0.4, 1.2, 2.2], CORE),
                c([3.15, 2.1, 1.8], [0.3, 0.5, 0.6], EYE),
                c([2.8, 3.2, 1.8], [1.6, 0.8, 2.8], BACK),
                // Upper teeth.
                c([1.7, -0.9, 5.2], [0.5, 0.9, 0.5], HORN),
                c([1.7, -0.9, 7.2], [0.5, 0.9, 0.5], HORN),
                c([1.0, -0.9, 8.6], [0.5, 0.7, 0.5], HORN),
                // Cheek frills.
                c([3.8, 0.6, -1.6], [1.0, 2.6, 3.0], MEMBRANE),
                c([4.4, 0.2, -3.0], [0.8, 1.8, 1.6], MEMBRANE),
                // Whiskers trailing back from the snout.
                c([3.2, 0.2, 6.6], [2.2, 0.4, 0.4], BELLY),
                c([4.3, -0.2, 4.4], [0.4, 0.4, 4.2], BELLY),
                c([4.5, -0.8, 1.8], [0.4, 0.4, 1.8], BELLY),
            ]));
            let mut pieces = vec![piece(0, head)];
            // Two long horns sweeping back and up, and two small ones in front of them.
            for s in [1.0f32, -1.0] {
                let at = Mat4::from_translation(Vec3::new(2.2 * s, 3.6, -1.6)) * Mat4::from_quat(Quat::from_rotation_y(PI - 0.25 * s) * Quat::from_rotation_x(-0.5));
                pieces.push(placed(0, at, horn(1.6, 8.0, HORN, BELLY)));
                let at = Mat4::from_translation(Vec3::new(1.4 * s, 3.4, 2.6)) * Mat4::from_quat(Quat::from_rotation_y(PI - 0.4 * s) * Quat::from_rotation_x(-0.9));
                pieces.push(placed(0, at, horn(0.9, 2.6, HORN, BELLY)));
            }
            let mut jaw = vec![
                c([0.0, -1.7, 3.8], [4.6, 1.4, 8.0], SCALE),
                c([0.0, -2.3, 3.8], [3.6, 0.6, 7.0], BELLY),
                c([0.0, -1.0, 4.4], [2.2, 0.3, 5.0], IMP_SKIN),
                c([0.0, -2.6, 0.2], [1.0, 1.0, 1.4], BACK),
            ];
            jaw.extend(both(vec![c([1.5, -0.7, 4.6], [0.4, 0.8, 0.4], HORN), c([1.5, -0.7, 6.6], [0.4, 0.8, 0.4], HORN), c([0.8, -0.7, 7.5], [0.4, 0.7, 0.4], HORN)]));
            pieces.push(piece(1, jaw));
            for k in 0..SEGMENTS {
                let f = seg_size(k);
                let mut b = vec![
                    c([0.0, 0.0, 0.0], [6.0 * f, 6.0 * f, 10.4 * f], SCALE),
                    c([0.0, -3.0 * f, 0.0], [4.4 * f, 0.8, 7.4 * f], BELLY),
                    c([0.0, 3.0 * f, 0.0], [3.2 * f, 0.8, 7.8 * f], BACK),
                    // Dorsal fin, swept back.
                    c([0.0, 3.0 * f + 1.4 * f, -0.8 * f], [0.8, 2.8 * f, 3.6 * f], MEMBRANE),
                    c([0.0, 3.0 * f + 3.0 * f, -2.2 * f], [0.6, 1.4 * f, 1.6 * f], MEMBRANE),
                ];
                b.extend(both(vec![c([3.0 * f, 0.4 * f, 0.0], [0.6, 3.0 * f, 6.4 * f], BACK)]));
                if k == 0 {
                    // A mane on the neck.
                    b.extend(both(vec![c([2.4, 3.2, -1.0], [1.0, 2.4, 3.6], MEMBRANE), c([3.0, 2.0, -2.4], [0.8, 2.0, 2.4], MEMBRANE)]));
                }
                if k == 1 || k == 4 {
                    // Clawed legs tucked under the body.
                    b.extend(both(vec![
                        c([3.4 * f, -2.0 * f, 0.6], [1.6, 3.0, 2.2], SCALE),
                        c([3.6 * f, -3.8 * f, 1.4], [1.4, 1.6, 1.4], SCALE),
                        c([3.6 * f, -4.8 * f, 2.2], [1.8, 0.8, 2.4], HORN),
                    ]));
                }
                pieces.push(piece(2 + k as u32, b));
            }
            // Wings: a leading arm bone, two fingers back from the wrist, membrane between.
            let wing = vec![
                c([4.6, 0.0, 0.0], [9.2, 1.0, 1.0], BACK),
                c([9.6, 0.4, 0.4], [1.4, 1.4, 1.4], HORN),
                c([10.6, 0.0, -0.4], [2.4, 0.6, 0.6], BACK),
                c([11.6, -0.1, -4.6], [0.6, 0.6, 8.0], BACK),
                c([7.6, -0.1, -4.2], [0.6, 0.6, 7.6], BACK),
                c([4.0, -0.1, -3.0], [0.5, 0.5, 5.6], BACK),
                c([2.0, 0.0, -2.6], [4.0, 0.3, 5.0], MEMBRANE),
                c([5.8, 0.0, -3.8], [3.6, 0.3, 7.4], MEMBRANE),
                c([9.6, 0.0, -4.4], [3.8, 0.3, 8.2], MEMBRANE),
                c([12.4, 0.0, -2.6], [1.4, 0.3, 4.4], MEMBRANE),
            ];
            pieces.push(piece(10, wing.clone()));
            pieces.push(piece(11, mirror(&wing)));
            let f = seg_size(SEGMENTS - 1);
            let mut tail = vec![
                c([0.0, 0.0, -4.0 * f - 1.6], [3.0, 3.0, 3.6], SCALE),
                c([0.0, 0.0, -4.0 * f - 4.4], [2.0, 2.0, 2.6], SCALE),
                c([0.0, 0.0, -4.0 * f - 7.4], [0.6, 6.0, 4.0], MEMBRANE),
                c([0.0, 0.0, -4.0 * f - 6.8], [6.4, 0.6, 3.2], MEMBRANE),
                c([0.0, 0.0, -4.0 * f - 9.8], [1.6, 1.6, 1.6], CORE),
            ];
            tail.extend(both(vec![c([0.0, 2.6, -4.0 * f - 9.0], [0.5, 1.6, 1.6], MEMBRANE)]));
            pieces.push(piece(12, tail));
            pieces
        }
        EMBER => {
            // 0 the stone tower, 1 the brazier that turns, 2..=4 its flames.
            let mut base = vec![
                c([0.0, 1.0, 0.0], [14.0, 2.0, 14.0], STONE),
                c([0.0, 3.0, 0.0], [12.0, 2.0, 12.0], STONE),
                c([0.0, 8.0, 0.0], [8.0, 8.0, 8.0], STONE_CRACK),
                c([0.0, 5.0, 0.0], [9.0, 1.0, 9.0], IRON),
                c([0.0, 11.4, 0.0], [9.0, 1.0, 9.0], IRON),
                c([5.2, 2.4, 4.4], [3.0, 2.6, 3.0], STONE),
                c([-5.0, 2.2, -4.6], [3.4, 2.2, 2.6], STONE),
            ];
            base.extend(both(vec![c([4.2, 8.0, 4.2], [1.0, 6.0, 1.0], IRON)]));
            let mut brazier = vec![
                c([0.0, 13.2, 0.0], [7.0, 1.6, 7.0], IRON),
                c([0.0, 14.6, 0.0], [11.0, 1.6, 11.0], IRON),
                c([0.0, 15.8, 0.0], [12.0, 1.0, 12.0], IRON),
                c([0.0, 16.0, 0.0], [9.6, 1.0, 9.6], EMBER_GLOW),
                // A snout pointing where it shoots.
                c([0.0, 14.8, 6.8], [3.4, 3.0, 3.0], IRON),
                c([0.0, 14.8, 8.4], [2.2, 2.0, 0.3], EMBER_GLOW),
            ];
            brazier.extend(both(vec![c([6.1, 16.6, 6.1], [1.2, 1.6, 1.2], IRON)]));
            // Coals glowing in the bowl, a ring of licks round a tall middle tongue.
            let mut low = vec![c([0.0, 17.0, 0.0], [6.0, 1.4, 6.0], EMBER_GLOW)];
            low.extend(tongue(3.8, 2.2, 16.4, 21.0, FLAME));
            for (x, z, h) in [(2.8, 1.2, 3.4), (-2.6, 1.8, 2.6), (1.4, -2.8, 3.0), (-1.8, -2.2, 3.8), (0.4, 3.0, 2.2)] {
                low.push(c([x, 16.6 + h / 2.0, z], [1.6, h, 1.6], FLAME));
                low.push(c([x * 1.1, 16.8 + h, z * 1.1], [0.8, 1.2, 0.8], IMP_SKIN));
            }
            let mut mid = tongue(2.6, 1.6, 20.0, 24.4, FLAME);
            mid.push(c([-1.4, 22.0, 0.8], [1.0, 3.2, 1.0], FLAME));
            mid.push(c([1.0, 21.4, -1.0], [1.0, 2.4, 1.0], IMP_SKIN));
            let mut tip = tongue(1.4, 0.9, 23.6, 27.4, IMP_SKIN);
            tip.push(c([1.0, 25.2, 0.4], [0.6, 2.0, 0.6], IMP_SKIN));
            vec![piece(0, base), piece(1, brazier), piece(2, low), piece(3, mid), piece(4, tip)]
        }
        FROST => {
            // 0 frozen rock, 1 the floating crystal, 2..=4 shards circling it.
            let base = vec![
                c([0.0, 1.0, 0.0], [12.0, 2.0, 10.0], STONE),
                c([1.6, 2.6, 1.0], [7.0, 2.0, 6.0], STONE),
                c([-2.0, 2.6, -1.4], [6.4, 1.6, 6.0], ICE),
                c([4.2, 4.0, -3.0], [1.6, 4.4, 1.6], ICE),
                c([4.2, 6.6, -3.0], [0.8, 1.2, 0.8], ICE),
                c([-4.6, 3.6, 2.6], [1.4, 3.4, 1.4], ICE_DEEP),
                c([-3.0, 4.0, -4.0], [1.2, 4.0, 1.2], ICE),
                c([2.4, 3.4, 4.0], [1.0, 2.8, 1.0], ICE_DEEP),
            ];
            let crystal = vec![
                c([0.0, 13.0, 0.0], [2.4, 15.0, 2.4], ICE),
                c([0.0, 13.0, 0.0], [4.2, 10.0, 4.2], ICE),
                c([0.0, 13.0, 0.0], [5.8, 5.6, 5.8], ICE_DEEP),
                c([0.0, 13.0, 0.0], [6.4, 2.0, 3.0], ICE),
                c([0.0, 13.0, 0.0], [3.0, 2.0, 6.4], ICE),
                c([0.0, 13.0, 0.0], [1.0, 17.0, 1.0], CORE),
            ];
            let shard = |k: u32| piece(k, vec![c([0.0, 0.0, 0.0], [1.4, 3.4, 1.4], ICE), c([0.0, 0.0, 0.0], [0.6, 4.6, 0.6], ICE_DEEP)]);
            vec![piece(0, base), piece(1, crystal), shard(2), shard(3), shard(4)]
        }
        STORM => {
            // 0 the thunderbird totem, 1 its cloud, 2 the lightning, 3 the ring round the rod.
            let mut totem = vec![
                c([0.0, 1.0, 0.0], [10.0, 2.0, 10.0], STONE),
                c([0.0, 5.0, 0.0], [6.0, 6.0, 6.0], PAINTED),
                c([0.0, 11.0, 0.0], [6.0, 6.0, 6.0], WOOD),
                c([0.0, 15.2, 0.0], [5.0, 2.4, 5.0], PAINTED),
                // Lower face: glowing eyes, a wide mouth.
                c([0.0, 3.4, 3.1], [3.6, 1.2, 0.3], EYE),
                c([0.0, 6.6, 3.2], [5.0, 0.8, 0.4], WOOD),
                // Upper face: a hooked beak.
                c([0.0, 10.6, 3.8], [2.0, 1.8, 1.8], PAINTED),
                c([0.0, 9.6, 4.6], [1.4, 1.0, 1.0], PAINTED),
                c([0.0, 17.8, 0.0], [0.8, 3.0, 0.8], IRON),
                c([0.0, 20.6, 0.0], [0.5, 2.8, 0.5], IRON),
            ];
            totem.extend(both(vec![
                c([1.4, 5.2, 3.15], [1.4, 1.0, 0.3], BOLT),
                c([1.5, 12.0, 3.15], [1.4, 1.4, 0.3], BOLT),
                // Spread wings at the top.
                c([4.4, 14.0, 0.0], [3.0, 1.6, 2.4], PAINTED),
                c([6.6, 15.0, 0.0], [1.8, 2.6, 2.0], PAINTED),
                c([7.6, 16.6, 0.0], [1.0, 2.2, 1.6], WOOD),
            ]));
            let cloud = vec![
                c([0.0, 26.0, 0.0], [9.0, 3.0, 7.0], CLOUD),
                c([-3.4, 27.0, 1.2], [5.0, 3.2, 4.4], CLOUD),
                c([3.2, 27.4, -1.0], [5.4, 3.6, 5.0], CLOUD),
                c([0.4, 28.4, 0.2], [5.0, 2.4, 4.4], CLOUD),
                c([0.0, 24.6, 0.0], [6.0, 0.8, 4.0], ICE_DEEP),
            ];
            let bolt = vec![
                c([0.4, 23.0, 0.0], [0.6, 2.4, 0.6], BOLT),
                c([-0.2, 21.8, 0.0], [1.6, 0.6, 0.6], BOLT),
                c([-0.6, 20.6, 0.0], [0.6, 2.4, 0.6], BOLT),
            ];
            let ring = vec![
                c([2.2, 19.2, 0.0], [0.6, 0.6, 5.0], IRON),
                c([-2.2, 19.2, 0.0], [0.6, 0.6, 5.0], IRON),
                c([0.0, 19.2, 2.2], [5.0, 0.6, 0.6], IRON),
                c([0.0, 19.2, -2.2], [5.0, 0.6, 0.6], IRON),
                c([2.2, 19.2, 2.2], [0.8, 0.8, 0.8], BOLT),
            ];
            vec![piece(0, totem), piece(1, cloud), piece(2, bolt), piece(3, ring)]
        }
        BALLISTA => {
            // 0 the stand, 1 the cradle that turns and tilts, 2 the bolt, 3/4 the two halves of
            // the string (unit long along +X, stretched by the pose).
            let mut stand = vec![
                c([0.0, 0.6, 0.0], [14.0, 1.2, 2.2], WOOD),
                c([0.0, 0.6, 0.0], [2.2, 1.2, 14.0], WOOD),
                c([0.0, 5.6, 0.0], [2.6, 10.0, 2.6], WOOD),
                c([0.0, 10.4, 0.0], [3.6, 0.8, 3.6], IRON),
                c([0.0, 3.0, 0.0], [3.2, 0.8, 3.2], IRON),
            ];
            stand.extend(both(vec![c([6.6, 1.4, 0.0], [1.2, 0.4, 2.6], IRON)]));
            let mut pieces = vec![piece(0, stand)];
            // Braces from the post down to the feet.
            for k in 0..4 {
                let a = k as f32 * PI / 2.0;
                let at = Mat4::from_translation(Vec3::new(0.0, 6.0, 0.0)) * Mat4::from_quat(Quat::from_rotation_y(a) * Quat::from_rotation_x(2.35));
                pieces.push(placed(0, at, vec![c([0.0, 0.0, 3.5], [1.0, 1.0, 7.0], WOOD)]));
            }
            let mut cradle = vec![
                c([0.0, 11.8, 1.0], [3.0, 1.6, 16.0], WOOD),
                c([0.0, 12.8, 1.0], [2.0, 0.4, 15.0], IRON),
                c([0.0, 12.0, 8.0], [6.0, 2.6, 2.0], IRON),
                // The bow limbs, bent back at their tips.
                c([0.0, 12.0, 8.0], [17.0, 1.2, 1.4], WOOD),
                // The winch at the back.
                c([0.0, 11.2, -6.6], [5.0, 2.0, 1.8], IRON),
            ];
            cradle.extend(both(vec![
                c([9.0, 12.0, 7.0], [1.4, 1.2, 2.2], WOOD),
                c([9.6, 12.0, 5.8], [0.9, 1.4, 0.9], IRON),
                c([2.9, 11.2, -6.6], [0.6, 3.4, 0.6], WOOD),
                c([2.9, 12.8, -5.8], [0.6, 0.6, 1.8], WOOD),
            ]));
            pieces.push(piece(1, cradle));
            pieces.push(piece(
                2,
                vec![
                    c([0.0, 13.4, 2.0], [0.8, 0.8, 14.0], WOOD),
                    c([0.0, 13.4, 9.4], [1.6, 1.6, 1.0], IRON),
                    c([0.0, 13.4, 10.3], [0.9, 0.9, 1.0], IRON),
                    c([0.0, 14.4, -4.0], [0.2, 1.6, 2.4], ROPE),
                    c([0.0, 13.4, -4.0], [1.8, 0.2, 2.4], ROPE),
                ],
            ));
            let string = vec![c([0.5, 0.0, 0.0], [1.0, 0.35, 0.35], ROPE)];
            pieces.push(piece(3, string.clone()));
            pieces.push(piece(4, string));
            pieces
        }
        PET_ORB => {
            // A shadow orb: a dark sphere with violet cracks and an eye, 1..=3 shards circling,
            // 4 a halo turning round it.
            let mut orb = vec![
                c([0.0, 0.0, 0.0], [6.0, 6.0, 6.0], CHITIN),
                c([0.0, 0.0, 0.0], [7.0, 4.4, 4.4], CHITIN),
                c([0.0, 0.0, 0.0], [4.4, 7.0, 4.4], CHITIN),
                c([0.0, 0.0, 0.0], [4.4, 4.4, 7.0], CHITIN),
                c([0.0, 0.0, 0.0], [7.2, 0.6, 4.6], RUNE),
                c([0.0, 1.6, 0.0], [0.6, 3.6, 7.2], RUNE),
                c([0.0, 0.2, 3.5], [2.6, 2.6, 0.4], RUNE),
                c([0.0, 0.2, 3.75], [0.8, 1.8, 0.3], EYE),
            ];
            orb.push(c([-2.0, -2.0, 3.0], [0.6, 1.8, 0.6], RUNE));
            let shard = |k: u32| piece(k, vec![c([0.0, 0.0, 0.0], [1.0, 2.6, 1.0], CHITIN), c([0.0, 1.6, 0.0], [0.5, 0.8, 0.5], RUNE)]);
            let halo = (0..6)
                .map(|k| {
                    let a = k as f32 / 6.0 * TAU;
                    c([a.cos() * 5.4, 0.0, a.sin() * 5.4], [1.0, 0.5, 1.0], RUNE)
                })
                .collect();
            vec![piece(0, orb), shard(1), shard(2), shard(3), piece(4, halo)]
        }
        PET_HEART => {
            // A beating heart with its great vessels on top; 1/2 drops circling it.
            let mut heart = vec![
                c([0.0, -0.6, 0.0], [5.8, 3.6, 3.6], IMP_SKIN),
                c([0.0, -2.6, 0.0], [3.8, 2.0, 3.0], IMP_SKIN),
                c([0.3, -4.0, 0.0], [1.8, 1.4, 2.0], IMP_SKIN),
                c([0.6, 3.6, 0.3], [1.6, 3.0, 1.6], IMP_DARK),
                c([1.8, 5.0, 0.3], [3.0, 1.4, 1.4], IMP_DARK),
                c([3.2, 4.4, 0.3], [1.2, 1.6, 1.2], IMP_DARK),
                c([-1.4, 3.4, -0.6], [1.2, 2.4, 1.2], SPIRIT_TIP),
                c([-1.4, 4.8, -0.6], [1.0, 1.0, 1.0], SPIRIT_TIP),
                // Veins over the front.
                c([0.4, 0.0, 1.85], [0.4, 3.4, 0.3], IMP_DARK),
                c([-0.6, -1.4, 1.85], [2.2, 0.4, 0.3], IMP_DARK),
                c([1.4, 1.2, 1.85], [0.4, 1.6, 0.3], IMP_DARK),
            ];
            heart.extend(both(vec![c([1.6, 1.4, 0.0], [3.4, 3.6, 3.4], IMP_SKIN)]));
            let drop = |k: u32| piece(k, vec![c([0.0, 0.0, 0.0], [1.0, 1.2, 1.0], IMP_SKIN), c([0.0, 0.9, 0.0], [0.5, 0.6, 0.5], IMP_SKIN)]);
            vec![piece(0, heart), drop(1), drop(2)]
        }
        UFO => {
            // A rounded saucer in three tiers with a gilded cockpit ring and an antenna; 1 the
            // ring of lights that runs round the rim, 2 the beam emitter underneath.
            let mut hull = vec![
                c([0.0, 2.0, 0.0], [18.0, 2.0, 12.0], IRON),
                c([0.0, 2.0, 0.0], [12.0, 2.0, 18.0], IRON),
                c([0.0, 2.0, 0.0], [16.0, 2.0, 16.0], IRON),
                c([0.0, 3.6, 0.0], [28.0, 1.4, 18.0], STEEL),
                c([0.0, 3.6, 0.0], [18.0, 1.4, 28.0], STEEL),
                c([0.0, 3.6, 0.0], [25.0, 1.4, 25.0], STEEL),
                c([0.0, 4.8, 0.0], [20.0, 1.2, 14.0], STEEL),
                c([0.0, 4.8, 0.0], [14.0, 1.2, 20.0], STEEL),
                c([0.0, 4.8, 0.0], [18.0, 1.2, 18.0], STEEL),
                c([0.0, 5.8, 0.0], [12.0, 1.0, 12.0], IRON),
                c([0.0, 6.5, 0.0], [10.0, 0.6, 10.0], GOLD),
                c([0.0, 5.2, -9.6], [0.4, 4.0, 0.4], IRON),
            ];
            hull.push(c([0.0, 7.4, -9.6], [1.0, 1.0, 1.0], EMBER_GLOW));
            let lights = (0..10)
                .map(|k| {
                    let a = k as f32 / 10.0 * TAU;
                    c([a.cos() * 13.2, 3.6, a.sin() * 13.2], [1.6, 1.0, 1.6], if k % 2 == 0 { CORE } else { EMBER_GLOW })
                })
                .collect();
            vec![piece(0, hull), piece(1, lights), piece(2, vec![c([0.0, 0.6, 0.0], [6.0, 0.8, 6.0], CORE), c([0.0, 0.1, 0.0], [3.0, 0.4, 3.0], ICE)])]
        }
        _ => {
            // TRAP: 0 the iron plate, spikes and powder keg, 1 the fuse light.
            let mut plate = vec![
                c([0.0, 0.4, 0.0], [12.0, 0.8, 12.0], IRON),
                c([0.0, 2.6, 0.0], [6.0, 4.0, 6.0], WOOD),
                c([0.0, 1.4, 0.0], [6.4, 0.8, 6.4], IRON),
                c([0.0, 3.8, 0.0], [6.4, 0.8, 6.4], IRON),
                c([0.0, 4.8, 0.0], [5.0, 0.4, 5.0], WOOD),
                c([0.0, 5.2, 0.0], [2.0, 0.4, 2.0], STONE_CRACK),
                c([0.0, 6.2, 0.0], [0.5, 2.0, 0.5], ROPE),
            ];
            for (x, z) in [(4.6, 4.6), (-4.6, 4.6), (4.6, -4.6), (-4.6, -4.6), (0.0, 5.0), (0.0, -5.0), (5.0, 0.0), (-5.0, 0.0)] {
                plate.push(c([x, 1.6, z], [1.0, 1.8, 1.0], IRON));
                plate.push(c([x, 2.8, z], [0.5, 0.8, 0.5], STEEL));
            }
            vec![piece(0, plate), piece(1, vec![c([0.0, 7.6, 0.0], [1.4, 1.4, 1.4], EMBER_GLOW), c([0.0, 7.6, 0.0], [0.6, 2.2, 0.6], FLAME)])]
        }
    }
}

/// Where tentacle `j` of the jellyfish hangs from the bell.
fn jelly_root(j: usize) -> (f32, f32) {
    let a = j as f32 / 6.0 * TAU + 0.3;
    (a.cos() * 3.8, a.sin() * 3.8)
}

/// Drawn size of each model against its model pixels (blocks per pixel).
fn model_scale(i: usize) -> f32 {
    let k = match i {
        WISP => 0.9,
        WASP => 0.75,
        IMP => 0.8,
        BLADE => 0.85,
        JELLY => 0.8,
        DRAGON => 1.1,
        EMBER | FROST | STORM => 1.0,
        BALLISTA => 1.1,
        PET_ORB | PET_HEART => 0.9,
        _ => 1.0,
    };
    k / 16.0
}

/// What a minion is doing this frame, for its pose.
pub struct MinionPose<'a> {
    /// Where it is, relative to the camera's block.
    pub pos: Vec3,
    /// Which way it faces (any length).
    pub dir: Vec3,
    pub vel: Vec3,
    /// Seconds, running on its own clock.
    pub t: f32,
    /// Attack swing: 1 at the moment it strikes, down to 0.
    pub atk: f32,
    /// Hunting (moves faster, wings beat harder).
    pub hunting: bool,
    /// The dragon's path, newest first, relative to the camera's block.
    pub trail: &'a [Vec3],
    pub light: [f32; 4],
}

fn bones_with(light: [f32; 4], glow: bool) -> Bones {
    Bones { light, tint: [1.0, 1.0, 1.0, if glow { 3.0 } else { 0.0 }], ..Default::default() }
}

/// Bones of minion model `i`.
pub fn minion_bones(i: usize, p: &MinionPose) -> Bones {
    let mut b = bones_with(p.light, true);
    let k = model_scale(i);
    let t = p.t;
    let dir = if p.dir.length_squared() > 1e-4 { p.dir } else { Vec3::Z };
    let speed = p.vel.length();
    let set = |b: &mut Bones, n: usize, m: Mat4| b.m[n] = m.to_cols_array_2d();
    match i {
        WISP => {
            let flat = Vec3::new(dir.x, 0.0, dir.z);
            let root = Mat4::from_translation(p.pos + Vec3::Y * (t * 2.3).sin() * 0.05) * Mat4::from_quat(face(flat)) * Mat4::from_scale(Vec3::splat(k));
            let throb = 1.0 + p.atk * 0.35;
            set(&mut b, 0, root * Mat4::from_scale(Vec3::splat(throb)));
            for n in 1..=3usize {
                let f = n as f32;
                let w = |a: f32, c: f32| (t * a + f * 1.7).sin() * 0.6 + (t * c + f * 0.9).sin() * 0.4;
                // The flame leans back against the way it flies.
                let lean = Quat::from_rotation_x(-(speed * 0.06).min(0.6) * f * 0.5 + w(6.1, 13.7) * 0.07 * f) * Quat::from_rotation_z(w(7.3, 11.9) * 0.07 * f);
                let tall = (1.0 + w(10.0, 16.3) * (0.06 + 0.05 * f)) * throb;
                set(&mut b, n, root * about(Vec3::ZERO, lean) * Mat4::from_scale(Vec3::new(1.0 / tall.sqrt(), tall, 1.0 / tall.sqrt())));
            }
            for n in 0..3usize {
                let a = t * 2.6 + n as f32 * TAU / 3.0;
                let r = 6.0 + (t * 1.7 + n as f32).sin();
                set(&mut b, 4 + n, root * Mat4::from_translation(Vec3::new(a.cos() * r, (a * 1.5).sin() * 2.5, a.sin() * r)) * Mat4::from_rotation_y(a * 2.0));
            }
            let sway = Quat::from_rotation_y((t * 3.1).sin() * 0.35) * Quat::from_rotation_x((t * 2.4).sin() * 0.2);
            set(&mut b, 7, root * about(Vec3::new(0.0, 0.0, -2.0), sway) * Mat4::from_scale(Vec3::new(1.0, 1.0, 1.0 + (speed * 0.05).min(0.6))));
        }
        WASP => {
            let bank = Quat::from_rotation_z((t * 1.3).sin() * 0.08);
            let root = Mat4::from_translation(p.pos + Vec3::Y * (t * 5.0).sin() * 0.03) * Mat4::from_quat(face(dir) * bank) * Mat4::from_scale(Vec3::splat(k));
            set(&mut b, 0, root);
            set(&mut b, 1, root * about(Vec3::new(0.0, 0.0, 2.6), Quat::from_rotation_x((t * 1.9).sin() * 0.08)));
            // The abdomen bobs, and jabs forward under the body to sting.
            let jab = p.atk * 1.1;
            set(&mut b, 2, root * about(Vec3::new(0.0, -0.4, -2.8), Quat::from_rotation_x(0.25 + (t * 4.0).sin() * 0.08 + jab)));
            // Wings: a blur of fast beats.
            let beat = (t * 70.0).sin() * 0.7;
            let hind = (t * 70.0 + 0.6).sin() * 0.6;
            set(&mut b, 3, root * about(Vec3::new(1.2, 2.6, 0.0), Quat::from_rotation_y(-0.25) * Quat::from_rotation_z(beat + 0.2)));
            set(&mut b, 4, root * about(Vec3::new(-1.2, 2.6, 0.0), Quat::from_rotation_y(0.25) * Quat::from_rotation_z(-beat - 0.2)));
            set(&mut b, 5, root * about(Vec3::new(1.2, 2.4, -1.6), Quat::from_rotation_y(-0.45) * Quat::from_rotation_z(hind + 0.15)));
            set(&mut b, 6, root * about(Vec3::new(-1.2, 2.4, -1.6), Quat::from_rotation_y(0.45) * Quat::from_rotation_z(-hind - 0.15)));
            set(&mut b, 7, root * about(Vec3::new(0.0, -1.8, 0.0), Quat::from_rotation_x(-(speed * 0.05).min(0.5) + (t * 3.3).sin() * 0.1)));
        }
        IMP => {
            let flat = Vec3::new(dir.x, 0.0, dir.z);
            let bob = (t * 4.2).sin();
            let root = Mat4::from_translation(p.pos + Vec3::Y * bob * 0.06) * Mat4::from_quat(face(flat) * Quat::from_rotation_x((speed * 0.04).min(0.35))) * Mat4::from_scale(Vec3::splat(k));
            set(&mut b, 0, root);
            set(&mut b, 1, root * about(Vec3::new(0.0, 3.0, 0.0), Quat::from_rotation_z((t * 1.3).sin() * 0.12) * Quat::from_rotation_x(-0.1 + bob * 0.05)));
            // Right arm winds the fireball up overhead and throws it; the left balances.
            let (wind, throw) = if p.atk > 0.5 { ((1.0 - p.atk) * 2.0, 0.0) } else { (1.0, 1.0 - p.atk * 2.0) };
            let swing = if p.atk > 0.0 { -2.6 * wind + 3.2 * throw } else { (t * 3.0).sin() * 0.2 };
            set(&mut b, 2, root * about(Vec3::new(3.4, 2.4, 0.0), Quat::from_rotation_x(swing)));
            set(&mut b, 3, root * about(Vec3::new(-3.4, 2.4, 0.0), Quat::from_rotation_x(-(t * 3.0).sin() * 0.2 - 0.2) * Quat::from_rotation_z(-0.3)));
            let tuck = 0.7 + bob * 0.1;
            set(&mut b, 4, root * about(Vec3::new(1.5, -3.0, 0.0), Quat::from_rotation_x(-tuck)));
            set(&mut b, 5, root * about(Vec3::new(-1.5, -3.0, 0.0), Quat::from_rotation_x(-tuck + 0.25)));
            let flap = (t * if p.hunting { 13.0 } else { 9.0 }).sin();
            set(&mut b, 6, root * about(Vec3::new(1.2, 2.2, -2.2), Quat::from_rotation_y(0.5 + flap * 0.55) * Quat::from_rotation_z(0.25 + flap * 0.25)));
            set(&mut b, 7, root * about(Vec3::new(-1.2, 2.2, -2.2), Quat::from_rotation_y(-0.5 - flap * 0.55) * Quat::from_rotation_z(-0.25 - flap * 0.25)));
            // The fireball sits in the right hand while it winds up.
            if p.atk > 0.0 && p.atk > 0.35 {
                let hand = Vec3::new(3.9, -3.6, 1.0);
                let arm = Mat4::from_cols_array_2d(&b.m[2]);
                let grow = ((1.0 - p.atk) * 3.0).min(1.0);
                set(&mut b, 8, arm * Mat4::from_translation(hand) * Mat4::from_rotation_y(t * 9.0) * Mat4::from_scale(Vec3::splat(grow.max(0.05))));
            } else {
                b.m[8] = [[0.0; 4]; 4];
            }
            set(&mut b, 9, root * about(Vec3::new(0.0, -2.2, -1.8), Quat::from_rotation_y((t * 2.7).sin() * 0.5) * Quat::from_rotation_x((t * 2.1).sin() * 0.2)));
        }
        BLADE => {
            // Waiting: point down, turning slowly. Hunting: point at the target, then stab and
            // whirl through it.
            let rest = Quat::from_rotation_y(t * 0.8) * Quat::from_rotation_x(PI / 2.0 - 0.15);
            let aim = face(dir);
            let r = if p.hunting { aim } else { rest };
            let slash = if p.atk > 0.0 { Quat::from_rotation_x(-(1.0 - p.atk) * TAU) } else { Quat::IDENTITY };
            let lunge = if p.hunting { dir.normalize_or_zero() * (p.atk * PI).sin() * 0.5 } else { Vec3::ZERO };
            let root = Mat4::from_translation(p.pos + lunge + Vec3::Y * (t * 1.9).sin() * 0.05) * Mat4::from_quat(r * slash * Quat::from_rotation_z(t * 0.6)) * Mat4::from_scale(Vec3::splat(k));
            set(&mut b, 0, root);
            for n in 0..3usize {
                let a = -t * 3.0 + n as f32 * TAU / 3.0;
                set(&mut b, 1 + n, root * Mat4::from_translation(Vec3::new(a.cos() * 3.4, a.sin() * 3.4, 1.0 + (t * 2.0 + n as f32).sin() * 3.0)) * Mat4::from_rotation_z(a));
            }
        }
        JELLY => {
            // The bell squeezes and opens; the tentacles trail behind the way it swims.
            let pulse = (t * 3.2).sin();
            let squeeze = 1.0 + pulse * 0.1 + p.atk * 0.15;
            let swim = Vec3::new(dir.x, 0.0, dir.z);
            let tilt = (speed * 0.08).min(0.7);
            let root = Mat4::from_translation(p.pos + Vec3::Y * (pulse * 0.08)) * Mat4::from_quat(face(swim) * Quat::from_rotation_x(tilt)) * Mat4::from_scale(Vec3::splat(k));
            let bell = root * Mat4::from_scale(Vec3::new(squeeze, 2.0 - squeeze, squeeze));
            set(&mut b, 0, bell);
            set(&mut b, 1, root * Mat4::from_rotation_y(t * 0.7) * Mat4::from_scale(Vec3::new(1.0, 1.0 + pulse * 0.1, 1.0)));
            for j in 0..6usize {
                let (x, z) = jelly_root(j);
                let ph = t * 3.2 - 0.6 + j as f32 * 1.1;
                let a1 = Quat::from_rotation_x(ph.sin() * 0.25 - tilt * 0.6) * Quat::from_rotation_z((ph * 0.7).cos() * 0.2 + x * 0.03 * (1.0 - pulse));
                let up = about(Vec3::new(x, 1.0, z), a1);
                set(&mut b, 2 + j * 2, bell * up);
                let a2 = Quat::from_rotation_x((ph - 0.9).sin() * 0.45 - tilt * 0.4) * Quat::from_rotation_z((ph * 0.8 - 0.5).cos() * 0.3);
                set(&mut b, 3 + j * 2, bell * up * about(Vec3::new(x, -3.1, z), a2));
            }
        }
        _ => dragon_bones(&mut b, p, k),
    }
    b
}

/// The dragon: the head where the minion is, the body laid along the path behind it.
fn dragon_bones(b: &mut Bones, p: &MinionPose, k: f32) {
    let t = p.t;
    let dir = if p.dir.length_squared() > 1e-4 { p.dir.normalize() } else { Vec3::Z };
    let pts = dragon_spine(p.pos, dir, p.trail, t);
    let head_dir = (pts[0] - pts[1]).normalize_or_zero();
    let head_dir = if head_dir == Vec3::ZERO { dir } else { (head_dir + dir).normalize_or_zero() };
    let head = Mat4::from_translation(pts[0]) * Mat4::from_quat(face(head_dir) * Quat::from_rotation_x((t * 1.7).sin() * 0.08)) * Mat4::from_scale(Vec3::splat(k * 1.15));
    b.m[0] = head.to_cols_array_2d();
    // The jaw opens wide to bite, and a little as it breathes.
    let open = 0.08 + (t * 1.3).sin().max(0.0) * 0.12 + (p.atk * PI).sin() * 0.75;
    b.m[1] = (head * about(Vec3::new(0.0, -1.0, -0.6), Quat::from_rotation_x(open))).to_cols_array_2d();
    let mut seg = [Mat4::IDENTITY; SEGMENTS];
    for (n, s) in seg.iter_mut().enumerate() {
        let (a, c) = (pts[n + 1], pts[n + 2]);
        let d = (pts[n] - c).normalize_or_zero();
        let roll = Quat::from_rotation_z((t * 2.0 - n as f32 * 0.6).sin() * 0.06);
        *s = Mat4::from_translation(a) * Mat4::from_quat(face(if d == Vec3::ZERO { dir } else { d }) * roll) * Mat4::from_scale(Vec3::splat(k));
        b.m[2 + n] = s.to_cols_array_2d();
    }
    // Wings on the second segment, big and raised in a V: slow strong beats, faster on the
    // hunt.
    let f = seg_size(1);
    let beat = (t * if p.hunting { 7.0 } else { 4.2 }).sin();
    for (n, s) in [(10usize, 1.0f32), (11, -1.0)] {
        let root = Vec3::new(2.6 * f * s, 2.6 * f, 0.8);
        let r = Quat::from_rotation_z(s * (0.55 + beat * 0.55)) * Quat::from_rotation_y(-s * (0.2 + beat * 0.12));
        b.m[n] = (seg[1] * Mat4::from_translation(root) * Mat4::from_quat(r) * Mat4::from_scale(Vec3::splat(1.7))).to_cols_array_2d();
    }
    let last = seg[SEGMENTS - 1];
    b.m[12] = (last * about(Vec3::new(0.0, 0.0, -4.0 * seg_size(SEGMENTS - 1)), Quat::from_rotation_y((t * 3.0).sin() * 0.35))).to_cols_array_2d();
}

/// Points along the dragon from its head back: head, then one per segment, then one beyond
/// the tail. The path behind comes from where the head has been; whatever is missing (just
/// summoned) runs straight back. A slow S wave rolls down the body.
pub fn dragon_spine(head: Vec3, dir: Vec3, trail: &[Vec3], t: f32) -> Vec<Vec3> {
    let n = SEGMENTS + 2;
    let mut out = Vec::with_capacity(n);
    out.push(head);
    let mut path: Vec<Vec3> = Vec::with_capacity(trail.len() + 1);
    path.push(head);
    path.extend(trail.iter().copied().filter(|q| q.is_finite()));
    let mut at = 0usize;
    let mut from = head;
    let mut back = -dir;
    for k in 1..n {
        // Each segment overlaps the one before it a little, so the body never shows gaps.
        let gap = if k == 1 { SEG_GAP * 0.8 } else { SEG_GAP * (0.04 + seg_size(k - 2) * 1.08) };
        let mut left = gap;
        let mut p = from;
        loop {
            if at + 1 < path.len() {
                let q = path[at + 1];
                let d = q.distance(p);
                if d >= left && d > 1e-5 {
                    p += (q - p) * (left / d);
                    back = (q - p).normalize_or(back);
                    break;
                }
                left -= d;
                p = q;
                at += 1;
            } else {
                p += back * left;
                break;
            }
        }
        out.push(p);
        from = p;
    }
    // The wave, across the body and growing toward the tail.
    for (k, q) in out.iter_mut().enumerate().skip(1) {
        let side = dir.cross(Vec3::Y).normalize_or(Vec3::X);
        let w = (t * 2.4 - k as f32 * 0.7).sin() * 0.06 * (k as f32 / n as f32);
        *q += side * w + Vec3::Y * (t * 1.9 - k as f32 * 0.5).cos() * 0.04;
    }
    out
}

/// What a sentry is doing this frame.
pub struct SentryPose {
    /// Its foot, relative to the camera's block.
    pub pos: Vec3,
    /// Toward its target (any length; zero when nothing to shoot).
    pub aim: Vec3,
    pub t: f32,
    /// 1 at the moment it fires, down to 0.
    pub fire: f32,
    pub light: [f32; 4],
}

/// Bones of sentry model `i`.
pub fn sentry_bones(i: usize, p: &SentryPose) -> Bones {
    let glow = matches!(i, FROST | STORM);
    let mut b = bones_with(p.light, glow);
    let k = model_scale(i);
    let t = p.t;
    let root = Mat4::from_translation(p.pos) * Mat4::from_scale(Vec3::splat(k));
    let set = |b: &mut Bones, n: usize, m: Mat4| b.m[n] = m.to_cols_array_2d();
    let flat = Vec3::new(p.aim.x, 0.0, p.aim.z);
    let yaw = if flat.length_squared() > 1e-4 { flat.x.atan2(flat.z) } else { t * 0.3 };
    set(&mut b, 0, root);
    match i {
        EMBER => {
            let kick = Mat4::from_translation(Vec3::new(0.0, 0.0, -p.fire * 1.2));
            let head = root * Mat4::from_rotation_y(yaw) * kick;
            set(&mut b, 1, head);
            for n in 2..=4usize {
                let f = (n - 1) as f32;
                let w = |a: f32, c: f32| (t * a + f * 1.7).sin() * 0.6 + (t * c + f * 0.9).sin() * 0.4;
                let base = Vec3::new(0.0, 16.4, 0.0);
                let lean = Quat::from_rotation_x(w(6.1, 13.1) * 0.035 * f + p.fire * 0.3) * Quat::from_rotation_z(w(7.7, 12.3) * 0.035 * f);
                let tall = 1.0 + w(10.0, 15.1) * (0.06 + 0.05 * f) + p.fire * 0.5;
                set(&mut b, n, head * about(base, lean) * Mat4::from_translation(base) * Mat4::from_scale(Vec3::new(1.0 / tall.sqrt(), tall, 1.0 / tall.sqrt())) * Mat4::from_translation(-base));
            }
        }
        FROST => {
            let bob = Vec3::Y * ((t * 1.6).sin() * 1.0 + 1.0);
            let c = Vec3::new(0.0, 13.0, 0.0);
            let spin = 1.0 + p.fire * 0.12;
            set(&mut b, 1, root * Mat4::from_translation(bob) * about(c, Quat::from_rotation_y(t * 0.7)) * Mat4::from_translation(c) * Mat4::from_scale(Vec3::splat(spin)) * Mat4::from_translation(-c));
            for n in 0..3usize {
                let a = t * 1.8 + n as f32 * TAU / 3.0;
                let at = c + bob + Vec3::new(a.cos() * 6.5, (a * 2.0).sin() * 2.0, a.sin() * 6.5);
                set(&mut b, 2 + n, root * Mat4::from_translation(at) * Mat4::from_rotation_y(-a) * Mat4::from_rotation_z(0.3));
            }
        }
        STORM => {
            let drift = Vec3::new((t * 0.5).sin() * 0.6, (t * 0.9).sin() * 0.5, (t * 0.4).cos() * 0.6);
            set(&mut b, 1, root * Mat4::from_translation(drift) * about(Vec3::new(0.0, 26.0, 0.0), Quat::from_rotation_y(t * 0.25)));
            if p.fire > 0.0 {
                let flick = if (t * 30.0).sin() > -0.3 { 1.0 } else { 0.0 };
                set(&mut b, 2, root * Mat4::from_translation(drift * 0.5) * about(Vec3::new(0.0, 22.0, 0.0), Quat::from_rotation_y(t * 7.0)) * about(Vec3::new(0.0, 20.0, 0.0), Quat::IDENTITY) * Mat4::from_scale(Vec3::splat(flick)));
            } else {
                b.m[2] = [[0.0; 4]; 4];
            }
            set(&mut b, 3, root * about(Vec3::new(0.0, 19.2, 0.0), Quat::from_rotation_y(t * (2.0 + p.fire * 10.0))));
        }
        BALLISTA => {
            let pitch = if p.aim.length_squared() > 1e-4 { (-p.aim.y).atan2(flat.length()).clamp(-0.6, 0.5) } else { 0.0 };
            let cradle = root * about(Vec3::new(0.0, 11.0, 0.0), Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch));
            set(&mut b, 1, cradle);
            // Drawn back at rest with a bolt on; on firing the bolt is gone and the string snaps
            // forward, then it is wound back and the next bolt laid on.
            let drawn = ((0.7 - p.fire) / 0.45).clamp(0.0, 1.0);
            let nock = Vec3::new(0.0, 13.4, 8.0 - drawn * 13.0);
            if p.fire > 0.25 {
                b.m[2] = [[0.0; 4]; 4];
            } else {
                set(&mut b, 2, cradle);
            }
            for (n, s) in [(3usize, 1.0f32), (4, -1.0)] {
                let tip = Vec3::new(9.6 * s, 12.6, 5.8);
                let d = nock - tip;
                let r = Quat::from_rotation_arc(Vec3::X, d.normalize());
                set(&mut b, n, cradle * Mat4::from_translation(tip) * Mat4::from_quat(r) * Mat4::from_scale(Vec3::new(d.length(), 1.0, 1.0)));
            }
        }
        _ => {
            // The trap's fuse light blinks.
            let on = 0.6 + 0.4 * (t * 6.0).sin().max(0.0);
            set(&mut b, 1, root * about(Vec3::new(0.0, 7.6, 0.0), Quat::from_rotation_y(t)) * Mat4::from_translation(Vec3::new(0.0, 7.6, 0.0)) * Mat4::from_scale(Vec3::splat(on)) * Mat4::from_translation(Vec3::new(0.0, -7.6, 0.0)));
        }
    }
    b
}

/// Bones of a pet or mount drawn with model `i` (see `pet_model`): at its mob's feet `pos`
/// (relative to the camera's block), facing `yaw`, `speed` blocks a second.
pub fn pet_bones(i: usize, pos: Vec3, yaw: f32, t: f32, speed: f32, light: [f32; 4]) -> Bones {
    let dir = Vec3::new(yaw.sin(), 0.0, yaw.cos());
    match i {
        WISP | JELLY => {
            let lift = if i == WISP { 0.25 } else { 0.35 };
            minion_bones(i, &MinionPose { pos: pos + Vec3::Y * lift, dir, vel: dir * speed, t, atk: 0.0, hunting: false, trail: &[], light })
        }
        UFO => {
            let mut b = bones_with(light, false);
            let k = model_scale(i);
            let wob = Quat::from_rotation_x((t * 1.3).sin() * 0.04 + (speed * 0.02).min(0.15)) * Quat::from_rotation_z((t * 1.1).cos() * 0.04);
            let root = Mat4::from_translation(pos + Vec3::Y * (0.12 + (t * 2.0).sin() * 0.04)) * Mat4::from_quat(Quat::from_rotation_y(yaw) * wob) * Mat4::from_scale(Vec3::splat(k));
            b.m[0] = root.to_cols_array_2d();
            b.m[1] = (root * Mat4::from_rotation_y(t * 2.5)).to_cols_array_2d();
            let glow = 1.0 + (t * 6.0).sin() * 0.15;
            b.m[2] = (root * Mat4::from_scale(Vec3::new(glow, 1.0, glow))).to_cols_array_2d();
            b
        }
        _ => {
            // The orb and the heart: glowing light pets that hover and bob.
            let mut b = bones_with(light, true);
            let k = model_scale(i);
            let root = Mat4::from_translation(pos + Vec3::Y * (0.3 + (t * 2.1).sin() * 0.05)) * Mat4::from_quat(Quat::from_rotation_y(yaw)) * Mat4::from_scale(Vec3::splat(k));
            if i == PET_HEART {
                // Lub-dub: two quick beats, then a rest.
                let ph = (t * 1.2).fract();
                let beat = (-((ph - 0.05) * 30.0).powi(2)).exp() * 0.16 + (-((ph - 0.22) * 30.0).powi(2)).exp() * 0.1;
                b.m[0] = (root * Mat4::from_rotation_z((t * 0.9).sin() * 0.12) * Mat4::from_scale(Vec3::splat(1.0 + beat))).to_cols_array_2d();
                for n in 0..2usize {
                    let a = t * 1.7 + n as f32 * PI;
                    b.m[1 + n] = (root * Mat4::from_translation(Vec3::new(a.cos() * 5.0, (a * 1.3).sin() * 2.0, a.sin() * 5.0))).to_cols_array_2d();
                }
            } else {
                b.m[0] = (root * Mat4::from_rotation_y((t * 0.6).sin() * 0.5) * Mat4::from_rotation_x((t * 0.8).sin() * 0.15)).to_cols_array_2d();
                for n in 0..3usize {
                    let a = -t * 2.2 + n as f32 * TAU / 3.0;
                    b.m[1 + n] = (root * Mat4::from_translation(Vec3::new(a.cos() * 6.5, (a * 2.0).sin() * 1.5, a.sin() * 6.5)) * Mat4::from_rotation_y(-a) * Mat4::from_rotation_z(0.5)).to_cols_array_2d();
                }
                b.m[4] = (root * Mat4::from_rotation_y(t * 1.4) * Mat4::from_rotation_x(0.35)).to_cols_array_2d();
            }
            b
        }
    }
}

/// Paint the summon skin: `scale` texels per model pixel, in the mob skin layout (64 × 128
/// model pixels), one 16 × 16 swatch per material.
pub fn paint(scale: u32) -> Vec<u8> {
    let (w, h) = (64 * scale, 128 * scale);
    let mut px = vec![0u8; (w * h * 4) as usize];
    let hash = |x: u32, y: u32, s: u32| {
        let mut v = x.wrapping_mul(0x9E37_79B1) ^ y.wrapping_mul(0x85EB_CA77) ^ s.wrapping_mul(0x1656_67B1);
        v ^= v >> 15;
        v = v.wrapping_mul(0x2C1B_3C6D);
        v ^= v >> 12;
        (v & 0xffff) as f32 / 65535.0
    };
    let mix = |a: [f32; 3], b: [f32; 3], t: f32| {
        let t = t.clamp(0.0, 1.0);
        [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
    };
    let n = 16 * scale;
    for m in 0..MATERIALS as u32 {
        let (ox, oy) = ((m % 4) * n, (m / 4) * n);
        for y in 0..n {
            for x in 0..n {
                let (u, v) = (x as f32 / scale as f32, y as f32 / scale as f32);
                let r = hash(x, y, m);
                // Coarser noise, one value per model pixel, for blotches.
                let r1 = hash(u as u32, v as u32, m + 77);
                let wave = |a: f32, b: f32| (u * a + v * b).sin() * 0.5 + 0.5;
                let col: [f32; 3] = match m as u8 {
                    CORE => mix([236.0, 252.0, 255.0], [190.0, 240.0, 255.0], r * 0.6),
                    SPIRIT => mix([90.0, 210.0, 240.0], [150.0, 236.0, 255.0], wave(0.7, 1.3) * 0.6 + r * 0.3),
                    SPIRIT_TIP => mix([40.0, 110.0, 210.0], [70.0, 150.0, 235.0], r * 0.6 + wave(1.1, 0.4) * 0.3),
                    EYE => {
                        // Dark, with a glint in one corner.
                        if (u - 4.0).abs() < 1.0 && (v - 4.0).abs() < 1.0 { [210.0, 220.0, 230.0] } else { mix([18.0, 16.0, 26.0], [36.0, 30.0, 44.0], r) }
                    }
                    WASP_YELLOW => {
                        let fuzz = r > 0.8;
                        let c = mix([236.0, 184.0, 40.0], [210.0, 150.0, 24.0], r1 * 0.6 + wave(0.9, 2.1) * 0.2);
                        if fuzz { mix(c, [255.0, 226.0, 120.0], 0.5) } else { c }
                    }
                    CHITIN => {
                        let sheen = wave(0.5, 0.8);
                        mix([22.0, 20.0, 18.0], [62.0, 56.0, 48.0], sheen * 0.5 + r * 0.25)
                    }
                    WING => {
                        // Pale, faintly iridescent, with dark veins.
                        let vein = (u * 0.9).fract() < 0.1 || ((v + u * 0.3) * 0.5).fract() < 0.06;
                        if vein { [86.0, 84.0, 90.0] } else { mix([206.0, 222.0, 232.0], [226.0, 214.0, 236.0], wave(0.4, 0.3) * 0.7 + r * 0.2) }
                    }
                    FLAME => mix([255.0, 176.0, 56.0], [246.0, 112.0, 28.0], r * 0.5 + v / 16.0 * 0.4),
                    IMP_SKIN => {
                        let spot = r1 > 0.86;
                        let c = mix([196.0, 58.0, 36.0], [160.0, 38.0, 26.0], r * 0.4 + wave(0.8, 1.1) * 0.3);
                        if spot { mix(c, [110.0, 20.0, 18.0], 0.5) } else { c }
                    }
                    IMP_DARK => mix([86.0, 22.0, 28.0], [58.0, 14.0, 20.0], r * 0.5 + wave(1.3, 0.7) * 0.3),
                    HORN => {
                        // Ridged bone, darker toward one end.
                        let ridge = ((v * 1.6).sin() * 0.5 + 0.5) * 0.25;
                        mix([232.0, 218.0, 186.0], [150.0, 128.0, 100.0], u / 16.0 * 0.5 + ridge + r * 0.1)
                    }
                    EMBER_GLOW => mix([255.0, 230.0, 120.0], [255.0, 168.0, 40.0], r * 0.6 + wave(1.5, 1.5) * 0.3),
                    STEEL => {
                        let shine = wave(0.25, 0.6);
                        mix([150.0, 160.0, 176.0], [232.0, 238.0, 246.0], shine * 0.8 + r * 0.1)
                    }
                    GOLD => mix([176.0, 124.0, 34.0], [246.0, 206.0, 96.0], wave(0.7, 0.9) * 0.7 + r * 0.2),
                    LEATHER => {
                        let wrap = ((u + v) * 1.4).sin() > 0.4;
                        let c = mix([92.0, 52.0, 30.0], [70.0, 38.0, 22.0], r * 0.6);
                        if wrap { mix(c, [40.0, 22.0, 14.0], 0.5) } else { c }
                    }
                    RUNE => {
                        // Violet glow with brighter rune strokes.
                        let stroke = ((u * 1.3).sin() * (v * 0.9 + u * 0.2).cos()).abs() < 0.12;
                        if stroke { [250.0, 230.0, 255.0] } else { mix([150.0, 80.0, 230.0], [200.0, 130.0, 255.0], r * 0.5 + wave(0.6, 0.6) * 0.3) }
                    }
                    BELL => {
                        // Translucent-looking pink-violet with paler spots and radial lines.
                        let line = ((u - 8.0).atan2(v - 8.0) * 4.0).sin().abs() < 0.12;
                        let spot = r1 > 0.88;
                        let c = mix([226.0, 138.0, 222.0], [176.0, 112.0, 230.0], wave(0.5, 0.4) * 0.6 + r * 0.2);
                        if spot {
                            mix(c, [255.0, 220.0, 250.0], 0.6)
                        } else if line {
                            mix(c, [140.0, 70.0, 180.0], 0.4)
                        } else {
                            c
                        }
                    }
                    JELLY_CORE => mix([255.0, 120.0, 210.0], [255.0, 190.0, 240.0], r * 0.5 + wave(1.2, 1.2) * 0.3),
                    SCALE => {
                        // Overlapping scales: rows of arcs, each with a light edge.
                        let row = (v / 2.0).floor();
                        let su = (u + row * 1.0) / 2.0;
                        let fu = su.fract() - 0.5;
                        let fv = (v / 2.0).fract();
                        let edge = fv > 0.55 + fu * fu * 1.2;
                        let c = mix([30.0, 150.0, 156.0], [22.0, 112.0, 130.0], r1 * 0.6 + r * 0.15);
                        if edge { mix(c, [120.0, 230.0, 220.0], 0.45) } else { c }
                    }
                    BACK => {
                        // Deep night blue with stars.
                        let star = r > 0.965;
                        if star { [230.0, 240.0, 255.0] } else { mix([24.0, 34.0, 86.0], [36.0, 56.0, 120.0], r1 * 0.5 + wave(0.4, 0.7) * 0.3) }
                    }
                    BELLY => {
                        // Pale gold plates with dark seams.
                        let seam = (v / 3.0).fract() < 0.12;
                        let c = mix([240.0, 214.0, 140.0], [214.0, 178.0, 100.0], r * 0.4 + u / 16.0 * 0.3);
                        if seam { mix(c, [120.0, 90.0, 50.0], 0.6) } else { c }
                    }
                    MEMBRANE => {
                        let star = r > 0.975;
                        let vein = ((u * 0.7 + v * 0.2).sin()).abs() < 0.08;
                        if star {
                            [200.0, 255.0, 250.0]
                        } else if vein {
                            [30.0, 70.0, 110.0]
                        } else {
                            mix([44.0, 110.0, 170.0], [70.0, 170.0, 196.0], wave(0.3, 0.5) * 0.6 + r * 0.2)
                        }
                    }
                    STONE => {
                        let joint = (v / 4.0).fract() < 0.08 || ((u + (v / 4.0).floor() * 3.0) / 6.0).fract() < 0.06;
                        let c = mix([118.0, 116.0, 112.0], [88.0, 86.0, 84.0], r1 * 0.6 + r * 0.25);
                        if joint { mix(c, [50.0, 48.0, 46.0], 0.6) } else { c }
                    }
                    STONE_CRACK => {
                        // Dark basalt with glowing cracks.
                        let crack = ((u * 0.9 + (v * 0.7).sin() * 2.0).sin() * (v * 0.8 + (u * 0.5).cos() * 1.6).sin()).abs() < 0.1;
                        if crack { mix([255.0, 150.0, 40.0], [255.0, 210.0, 90.0], r) } else { mix([56.0, 50.0, 50.0], [34.0, 30.0, 32.0], r1 * 0.6 + r * 0.2) }
                    }
                    ICE => {
                        let streak = ((u * 0.6 - v * 0.9).sin()).abs() < 0.08;
                        let c = mix([196.0, 236.0, 250.0], [150.0, 206.0, 240.0], r1 * 0.5 + r * 0.2);
                        if streak { [245.0, 252.0, 255.0] } else { c }
                    }
                    ICE_DEEP => mix([70.0, 140.0, 214.0], [110.0, 180.0, 236.0], wave(0.5, 0.9) * 0.6 + r * 0.2),
                    WOOD => {
                        let grain = ((v + (u * 0.4).sin() * 0.7) * 2.3).sin() * 0.5 + 0.5;
                        mix([150.0, 104.0, 60.0], [104.0, 70.0, 40.0], grain.powf(2.0) * 0.7 + r * 0.2)
                    }
                    PAINTED => {
                        // Carved wood painted in teal and red bands with a black outline.
                        let band = (v / 4.0).floor() as i32 % 3;
                        let edge = (v / 4.0).fract() < 0.12;
                        let base = match band {
                            0 => [40.0, 140.0, 140.0],
                            1 => [180.0, 60.0, 40.0],
                            _ => [230.0, 210.0, 170.0],
                        };
                        if edge { [30.0, 24.0, 20.0] } else { mix(base, [base[0] * 0.75, base[1] * 0.75, base[2] * 0.75], r * 0.5) }
                    }
                    IRON => {
                        let rivet = (u - 3.0).rem_euclid(6.0) < 1.0 && (v - 1.0).rem_euclid(6.0) < 1.0;
                        if rivet { [160.0, 156.0, 150.0] } else { mix([66.0, 66.0, 72.0], [100.0, 98.0, 96.0], r * 0.5 + r1 * 0.3) }
                    }
                    ROPE => {
                        let twist = ((u * 2.0 + v * 2.0).sin()) > 0.0;
                        if twist { [214.0, 196.0, 150.0] } else { [176.0, 156.0, 112.0] }
                    }
                    CLOUD => mix([96.0, 98.0, 128.0], [150.0, 152.0, 178.0], wave(0.6, 0.45) * 0.5 + r1 * 0.3 + r * 0.1),
                    _ => mix([255.0, 250.0, 200.0], [255.0, 230.0, 90.0], r * 0.6),
                };
                let i = (((oy + y) * w + ox + x) * 4) as usize;
                px[i] = col[0].clamp(0.0, 255.0) as u8;
                px[i + 1] = col[1].clamp(0.0, 255.0) as u8;
                px[i + 2] = col[2].clamp(0.0, 255.0) as u8;
                px[i + 3] = 255;
            }
        }
    }
    px
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_model_has_boxes_on_free_bones() {
        for i in 0..COUNT {
            let m = model(i);
            assert!(!m.is_empty(), "model {i}");
            for p in &m {
                // The last two bone slots are the eyelids and the mouth.
                assert!((p.part as usize) < crate::entity::MOUTH_PART, "model {i} part {}", p.part);
                for b in &p.boxes {
                    assert!(b.lo.cmple(b.hi).all(), "model {i}: inverted box");
                    assert!(b.mat < MATERIALS);
                }
            }
        }
    }

    #[test]
    fn dragon_follows_its_path() {
        // Flying along +X, the body lies back toward -X, segment after segment.
        let trail: Vec<Vec3> = (1..40).map(|k| Vec3::new(-0.2 * k as f32, 0.0, 0.0)).collect();
        let pts = dragon_spine(Vec3::ZERO, Vec3::X, &trail, 0.0);
        assert_eq!(pts.len(), SEGMENTS + 2);
        for w in pts.windows(2) {
            assert!(w[1].x < w[0].x, "{pts:?}");
            let gap = w[0].distance(w[1]);
            assert!(gap > 0.2 && gap < 0.7, "{gap}");
        }
        // Just summoned: no trail yet, it still has a whole body.
        let pts = dragon_spine(Vec3::ZERO, Vec3::Z, &[], 0.0);
        assert!(pts.last().unwrap().z < -3.0);
        let p = MinionPose { pos: Vec3::ZERO, dir: Vec3::Z, vel: Vec3::ZERO, t: 1.0, atk: 0.5, hunting: true, trail: &[], light: [1.0, 0.0, 0.0, 0.0] };
        for i in 0..=DRAGON {
            let b = minion_bones(i, &p);
            assert!(b.m.iter().flatten().flatten().all(|v| v.is_finite()), "minion {i}");
        }
        for i in [PET_ORB, PET_HEART, UFO, WISP, JELLY] {
            let b = pet_bones(i, Vec3::ZERO, 1.0, 3.0, 2.0, [1.0, 0.0, 0.0, 0.0]);
            assert!(b.m.iter().flatten().flatten().all(|v| v.is_finite()), "pet {i}");
        }
        for i in EMBER..=TRAP {
            let b = sentry_bones(i, &SentryPose { pos: Vec3::ZERO, aim: Vec3::new(3.0, 1.0, 2.0), t: 2.0, fire: 0.5, light: [1.0, 0.0, 0.0, 0.0] });
            assert!(b.m.iter().flatten().flatten().all(|v| v.is_finite()), "sentry {i}");
        }
    }
}
