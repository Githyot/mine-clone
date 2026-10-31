//! Armour item icons drawn from the armour as it is worn (HP, 2026-10-31: "se o set de
//! adamantina tem chifre porque o ícone dela como item não tem?").
//!
//! Every set has its own 3D look (armor_design.rs): horns, crests, plumes, masks, wings. Hand-drawn
//! icons drifted away from it, so each icon is now a picture of the worn piece itself: the same
//! boxes and the same armour texture, seen from the front and a little from the side and above,
//! cropped to fill the slot. The pictures go to tools/tex64/armor3d/ and build.py turns them into
//! chunky pixel art like every other item (pixel.py). Re-run after changing a design:
//!
//! ```text
//! cargo test -p game write_armor_icons -- --ignored
//! python3 tools/tex64/build.py --only <the icons>
//! ```

use glam::{Mat3, Vec3};

const PIECES: [&str; 4] = ["helmet", "chestplate", "leggings", "boots"];
/// Output size; the picture is drawn on a GRID × GRID pixel-art grid (pixel.py's), each cell
/// from SS × SS samples.
const OUT: usize = 64;
const GRID: usize = 32;
const SS: usize = 4;
/// Empty border kept round the picture, in grid cells.
const MARGIN: f32 = 1.0;

/// A picture of piece `piece` of set `name`: OUT × OUT RGBA.
fn render(def: &voxel::mob::MobDef, name: &str, color: [u8; 3], piece: usize) -> Vec<u8> {
    let (verts, idx, tex, tw, th) = crate::entity::armor_piece_model(def, name, color, piece);
    // Turned a little so the side pieces (horns, pauldrons, wings) show their depth, looked at a
    // little from above. The legs stand a little apart, so leggings read as two legs under the
    // belt and boots as a pair.
    let (yaw, pitch, gap) = [(-26f32, 14f32, 0.0), (-24.0, 12.0, 0.0), (-10.0, 6.0, 1.6), (-22.0, 20.0, 2.2)][piece];
    let view = Mat3::from_rotation_x(pitch.to_radians()) * Mat3::from_rotation_y(yaw.to_radians());
    let pts: Vec<Vec3> = verts
        .iter()
        .map(|(p, _, _, part)| {
            let mut p = Vec3::from(*p);
            p.x += match part {
                4 => -gap,
                5 => gap,
                _ => 0.0,
            };
            let q = view * p;
            Vec3::new(q.x, -q.y, q.z)
        })
        .collect();
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for p in &pts {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    // Colour (alpha 0 = empty) and which face (a quad: two triangles) each sample shows.
    let draw = |scale: f32, off: (f32, f32), w: usize, h: usize| -> (Vec<[f32; 4]>, Vec<usize>) {
        let mut col = vec![[0f32; 4]; w * h];
        let mut face = vec![usize::MAX; w * h];
        let mut depth = vec![f32::MIN; w * h];
        let sp = |p: Vec3| ((p.x - off.0) * scale, (p.y - off.1) * scale, p.z);
        for (ti, t) in idx.chunks(3).enumerate() {
            let [a, b, c] = [t[0] as usize, t[1] as usize, t[2] as usize];
            let (pa, pb, pc) = (sp(pts[a]), sp(pts[b]), sp(pts[c]));
            let area = (pb.0 - pa.0) * (pc.1 - pa.1) - (pb.1 - pa.1) * (pc.0 - pa.0);
            if area.abs() < 1e-6 {
                continue;
            }
            let x0 = pa.0.min(pb.0).min(pc.0).floor().max(0.0) as usize;
            let x1 = (pa.0.max(pb.0).max(pc.0).ceil().max(0.0) as usize).min(w);
            let y0 = pa.1.min(pb.1).min(pc.1).floor().max(0.0) as usize;
            let y1 = (pa.1.max(pb.1).max(pc.1).ceil().max(0.0) as usize).min(h);
            // Three clearly different tones for top, front and sides, like a block icon; the
            // swatches of the bottom row glow in the game (entity.wgsl), so they stay unshaded.
            let shade = verts[a].2;
            let tone = (0.92 + (shade - 0.8) * 1.3).clamp(0.5, 1.2);
            for y in y0..y1 {
                for x in x0..x1 {
                    let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                    let w0 = ((pb.0 - px) * (pc.1 - py) - (pb.1 - py) * (pc.0 - px)) / area;
                    let w1 = ((pc.0 - px) * (pa.1 - py) - (pc.1 - py) * (pa.0 - px)) / area;
                    let w2 = 1.0 - w0 - w1;
                    if w0 < -1e-4 || w1 < -1e-4 || w2 < -1e-4 {
                        continue;
                    }
                    let z = w0 * pa.2 + w1 * pb.2 + w2 * pc.2;
                    let i = y * w + x;
                    if z <= depth[i] {
                        continue;
                    }
                    let u = w0 * verts[a].1[0] + w1 * verts[b].1[0] + w2 * verts[c].1[0];
                    let v = w0 * verts[a].1[1] + w1 * verts[b].1[1] + w2 * verts[c].1[1];
                    let tx = ((u * tw as f32) as i64).clamp(0, tw as i64 - 1) as usize;
                    let ty = ((v * th as f32) as i64).clamp(0, th as i64 - 1) as usize;
                    let k = (ty * tw as usize + tx) * 4;
                    if tex[k + 3] < 128 {
                        continue;
                    }
                    let lit = if v >= 0.875 && u < 0.75 { 1.0 } else { tone };
                    depth[i] = z;
                    face[i] = ti / 2;
                    let c = |j: usize| (tex[k + j] as f32 * lit).min(255.0);
                    col[i] = [c(0), c(1), c(2), 255.0];
                }
            }
        }
        (col, face)
    };
    // First pass at a fixed scale to find what is actually drawn (helmet nets have see-through
    // texels), then the real pass fitted to that.
    let probe = 16.0;
    let (pw, ph) = (((hi.x - lo.x) * probe).ceil() as usize + 1, ((hi.y - lo.y) * probe).ceil() as usize + 1);
    let (img, _) = draw(probe, (lo.x, lo.y), pw, ph);
    let (mut bx0, mut by0, mut bx1, mut by1) = (pw, ph, 0, 0);
    for y in 0..ph {
        for x in 0..pw {
            if img[y * pw + x][3] > 0.0 {
                bx0 = bx0.min(x);
                bx1 = bx1.max(x + 1);
                by0 = by0.min(y);
                by1 = by1.max(y + 1);
            }
        }
    }
    let mut out = vec![0u8; OUT * OUT * 4];
    if bx1 <= bx0 {
        return out;
    }
    let (cw, ch) = ((bx1 - bx0) as f32 / probe, (by1 - by0) as f32 / probe);
    let (cx, cy) = (lo.x + (bx0 + bx1) as f32 * 0.5 / probe, lo.y + (by0 + by1) as f32 * 0.5 / probe);
    let n = GRID * SS;
    let scale = (GRID as f32 - 2.0 * MARGIN) * SS as f32 / cw.max(ch);
    let half = n as f32 * 0.5 / scale;
    let (big, faces) = draw(scale, (cx - half, cy - half), n, n);
    // Each grid cell: the average colour of its samples, and the face most of them show.
    let mut cell = vec![[0f32; 4]; GRID * GRID];
    let mut cface = vec![usize::MAX; GRID * GRID];
    for y in 0..GRID {
        for x in 0..GRID {
            let mut acc = [0f32; 4];
            let mut seen: Vec<(usize, usize)> = Vec::new();
            for sy in 0..SS {
                for sx in 0..SS {
                    let i = (y * SS + sy) * n + x * SS + sx;
                    let c = big[i];
                    if c[3] > 0.0 {
                        for j in 0..3 {
                            acc[j] += c[j];
                        }
                        acc[3] += 1.0;
                        match seen.iter_mut().find(|(f, _)| *f == faces[i]) {
                            Some(e) => e.1 += 1,
                            None => seen.push((faces[i], 1)),
                        }
                    }
                }
            }
            // A cell is filled when half its samples are.
            if acc[3] * 2.0 >= (SS * SS) as f32 {
                cell[y * GRID + x] = [acc[0] / acc[3], acc[1] / acc[3], acc[2] / acc[3], 255.0];
                cface[y * GRID + x] = seen.iter().max_by_key(|e| e.1).unwrap().0;
            }
        }
    }
    // A thin dark line where one face meets another, above and left of it, so every box and
    // plate keeps its edges at this size (pixel.py outlines the silhouette).
    let mut lined = cell.clone();
    for y in 0..GRID {
        for x in 0..GRID {
            let i = y * GRID + x;
            if cell[i][3] == 0.0 {
                continue;
            }
            let other = |j: usize| cell[j][3] > 0.0 && cface[j] != cface[i];
            if (x > 0 && other(i - 1)) || (y > 0 && other(i - GRID)) {
                for j in 0..3 {
                    lined[i][j] *= 0.72;
                }
            }
        }
    }
    let k = OUT / GRID;
    for y in 0..OUT {
        for x in 0..OUT {
            let c = lined[(y / k) * GRID + x / k];
            let o = (y * OUT + x) * 4;
            for j in 0..4 {
                out[o + j] = c[j].round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    out
}

/// The sets that have items: every armour material but the dye-vat colours (those recolour an
/// existing piece, they have no icons of their own).
fn sets() -> impl Iterator<Item = &'static (&'static str, [u8; 3])> {
    crate::entity::ARMOR_MATERIALS.iter().filter(|(n, _)| !n.starts_with("dye_"))
}

#[test]
#[ignore]
fn write_armor_icons() {
    let reg = crate::mobs::registry_for_tests();
    let def = &reg.defs[reg.id("player").expect("player model")];
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/tex64/armor3d");
    std::fs::create_dir_all(&dir).unwrap();
    let items = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/vanilla/textures/item");
    let only = std::env::var("MC_ARMOR_ICONS").ok();
    for (name, color) in sets() {
        if only.as_deref().is_some_and(|o| !o.split(',').any(|w| w == *name)) {
            continue;
        }
        for (piece, kind) in PIECES.iter().enumerate() {
            // Only the pieces that are items (some outfits have no boots, `golden` is `gold_` too).
            if !items.join(format!("{name}_{kind}.png")).exists() {
                continue;
            }
            let px = render(def, name, *color, piece);
            image::save_buffer(dir.join(format!("{name}_{kind}.png")), &px, OUT as u32, OUT as u32, image::ColorType::Rgba8).unwrap();
        }
    }
}

/// Every piece of every set draws something that fills a good part of the slot.
#[test]
fn every_armour_icon_shows_its_piece() {
    let reg = crate::mobs::registry_for_tests();
    let def = &reg.defs[reg.id("player").expect("player model")];
    for (name, color) in sets() {
        for piece in 0..4 {
            let px = render(def, name, *color, piece);
            let opaque = px.chunks(4).filter(|p| p[3] > 127).count();
            assert!(opaque > OUT * OUT / 8, "{name} {}: only {opaque} pixels", PIECES[piece]);
        }
    }
}
