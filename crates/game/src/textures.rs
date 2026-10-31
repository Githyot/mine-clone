//! Block textures: the base game's art plus resource packs stacked on top, like Minecraft.
//!
//! A resource pack is a folder or a `.zip` inside `resourcepacks/`. Two layouts are understood:
//! - ours: `textures/block/<name>.png`
//! - Minecraft's: `assets/minecraft/textures/block/<name>.png` (and the older `.../blocks/` names),
//!   so any Minecraft texture pack you own works. Grey textures that Minecraft colours at runtime
//!   (grass, leaves, water) are coloured here when they are loaded.

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use crate::embedded;

pub struct TextureSet {
    /// Width and height of every layer.
    pub size: u32,
    /// RGBA8 pixels per layer, each `size * size * 4` bytes.
    pub layers: Vec<Vec<u8>>,
    /// Names of the packs that were used, for the debug screen.
    pub packs: Vec<String>,
    /// Interface art from the packs (empty when no pack has any: our own drawing is used).
    pub gui: GuiAtlas,
}

/// Interface pictures from packs, packed into one image. Each sprite is keyed by its path under
/// `assets/minecraft/textures/gui/` without `.png`, and stores its place in the atlas (0..1).
#[derive(Default)]
pub struct GuiAtlas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub sprites: std::collections::HashMap<String, [f32; 4]>,
}

/// The interface pictures a pack may replace.
fn gui_names() -> Vec<String> {
    let mut v: Vec<String> = ["tab_items", "tab_inventory", "tab_item_search"]
        .iter()
        .map(|n| format!("container/creative_inventory/{n}"))
        .collect();
    for row in ["top", "bottom"] {
        for state in ["selected", "unselected"] {
            for i in 1..=7 {
                v.push(format!("sprites/container/creative_inventory/tab_{row}_{state}_{i}"));
            }
        }
    }
    for n in ["scroller", "scroller_disabled"] {
        v.push(format!("sprites/container/creative_inventory/{n}"));
    }
    for n in ["helmet", "chestplate", "leggings", "boots", "shield"] {
        v.push(format!("sprites/container/slot/{n}"));
    }
    for n in [
        "hotbar", "hotbar_selection", "crosshair", "food_empty", "food_half", "food_full", "armor_empty", "armor_half", "armor_full",
        "experience_bar_background", "experience_bar_progress", "heart/container", "heart/full", "heart/half", "air", "air_bursting",
    ] {
        v.push(format!("sprites/hud/{n}"));
    }
    v
}

/// Gather the interface pictures of the enabled packs (first pack wins) into one atlas. What no
/// pack has comes from the base game's own interface art (`packs/vanilla/textures/gui/`).
fn load_gui(packs: &mut [Pack], base: &Path) -> GuiAtlas {
    let mut imgs: Vec<(String, image::RgbaImage)> = Vec::new();
    for name in gui_names() {
        let rel = format!("assets/minecraft/textures/gui/{name}.png");
        let found = packs.iter_mut().find_map(|p| p.read(&rel).and_then(|b| decode(&b))).or_else(|| {
            std::fs::read(base.join("packs/vanilla/textures/gui").join(format!("{name}.png")))
                .ok()
                .or_else(|| embedded::GUI_TEXTURES.iter().find(|(n, _)| *n == name).map(|(_, b)| b.to_vec()))
                .and_then(|b| decode(&b))
        });
        if let Some(img) = found {
            imgs.push((name, first_frame(img)));
        }
    }
    if imgs.is_empty() {
        return GuiAtlas::default();
    }
    // Shelf packing, tallest first.
    imgs.sort_by_key(|(_, i)| std::cmp::Reverse(i.height()));
    let width = imgs.iter().map(|(_, i)| i.width()).max().unwrap_or(1).max(1024).next_power_of_two();
    let mut places = Vec::with_capacity(imgs.len());
    let (mut x, mut y, mut shelf) = (0u32, 0u32, 0u32);
    for (_, img) in &imgs {
        if x + img.width() > width {
            x = 0;
            y += shelf;
            shelf = 0;
        }
        places.push((x, y));
        x += img.width();
        shelf = shelf.max(img.height());
    }
    let height = (y + shelf).max(1).next_power_of_two();
    if height > 8192 {
        log::warn!("interface art too big for one atlas; using our own");
        return GuiAtlas::default();
    }
    let mut atlas = image::RgbaImage::new(width, height);
    let mut sprites = std::collections::HashMap::new();
    for ((name, img), (px, py)) in imgs.into_iter().zip(places) {
        image::imageops::replace(&mut atlas, &img, px as i64, py as i64);
        let (w, h) = (width as f32, height as f32);
        sprites.insert(name, [px as f32 / w, py as f32 / h, (px + img.width()) as f32 / w, (py + img.height()) as f32 / h]);
    }
    GuiAtlas { width, height, pixels: atlas.into_raw(), sprites }
}

enum PackKind {
    Dir(PathBuf),
    Zip(zip::ZipArchive<Cursor<Vec<u8>>>),
}

pub(crate) struct Pack {
    name: String,
    kind: PackKind,
}

impl Pack {
    pub(crate) fn open(path: &Path) -> Option<Pack> {
        let name = path.file_name()?.to_string_lossy().to_string();
        if path.is_dir() {
            Some(Pack { name, kind: PackKind::Dir(path.to_path_buf()) })
        } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip")) {
            let bytes = std::fs::read(path).ok()?;
            let zip = zip::ZipArchive::new(Cursor::new(bytes)).ok()?;
            Some(Pack { name, kind: PackKind::Zip(zip) })
        } else {
            None
        }
    }

    /// Every file under `assets/`, as paths from `assets/` on (a zip's wrapping folder left out).
    pub(crate) fn list(&mut self) -> Vec<String> {
        match &mut self.kind {
            PackKind::Dir(d) => {
                let mut out = Vec::new();
                let mut stack = vec![d.join("assets")];
                while let Some(dir) = stack.pop() {
                    for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                        let p = e.path();
                        if p.is_dir() {
                            stack.push(p);
                        } else if let Ok(rel) = p.strip_prefix(&*d) {
                            out.push(rel.to_string_lossy().replace('\\', "/"));
                        }
                    }
                }
                out
            }
            PackKind::Zip(z) => z.file_names().filter_map(|n| n.find("assets/").map(|i| n[i..].to_string())).collect(),
        }
    }

    pub(crate) fn read(&mut self, rel: &str) -> Option<Vec<u8>> {
        match &mut self.kind {
            PackKind::Dir(d) => std::fs::read(d.join(rel)).ok(),
            PackKind::Zip(z) => {
                // Some zips wrap everything in one top-level folder; try both.
                let direct = z.by_name(rel).ok().map(|mut f| {
                    let mut v = Vec::new();
                    let _ = f.read_to_end(&mut v);
                    v
                });
                if direct.is_some() {
                    return direct;
                }
                let names: Vec<String> = z.file_names().map(|s| s.to_string()).collect();
                // Only a wrapping folder: our own layout must not pick up a Minecraft-layout file
                // ("assets/minecraft/textures/block/..."), which still needs its colour.
                let hit = names
                    .into_iter()
                    .find(|n| n.ends_with(&format!("/{rel}")) && (rel.starts_with("assets/") || !n.contains("assets/minecraft/")))?;
                let mut f = z.by_name(&hit).ok()?;
                let mut v = Vec::new();
                f.read_to_end(&mut v).ok()?;
                Some(v)
            }
        }
    }
}

/// Shader packs that come inside resource packs (Lote 9): a `shaders/post.wgsl` (and
/// optionally `shaders/pack.ron`) in our own shader pack format, at the pack's top. They are
/// copied to `shaderpacks/.pacotes/<pack>/` so the shader menu lists them while the resource
/// pack is on. Minecraft's own core shaders (GLSL) can't run here and are ignored. Returns
/// whether the list changed.
pub fn extract_pack_shaders(base: &Path, enabled: &[String]) -> bool {
    let out = base.join("shaderpacks").join(".pacotes");
    let had = std::fs::read_dir(&out).map(|r| r.count() > 0).unwrap_or(false);
    let _ = std::fs::remove_dir_all(&out);
    let mut n = 0;
    for name in enabled {
        let Some(mut pack) = Pack::open(&base.join("resourcepacks").join(name)) else { continue };
        let Some(wgsl) = pack.read("shaders/post.wgsl") else { continue };
        let id: String = name.trim_end_matches(".zip").chars().filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')).collect();
        let dir = out.join(id.trim());
        if std::fs::create_dir_all(&dir).is_err() {
            continue;
        }
        let _ = std::fs::write(dir.join("post.wgsl"), wgsl);
        if let Some(ron) = pack.read("shaders/pack.ron") {
            let _ = std::fs::write(dir.join("pack.ron"), ron);
        }
        n += 1;
    }
    if n > 0 {
        log::info!("{n} shader packs from resource packs");
    }
    had || n > 0
}

/// Packs available in `dir`, sorted by name.
pub fn list_resource_packs(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .map(|r| {
            r.filter_map(|e| e.ok())
                .filter(|e| {
                    let p = e.path();
                    p.is_dir() || p.extension().is_some_and(|x| x.eq_ignore_ascii_case("zip"))
                })
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// How Minecraft names our textures (newest names first, then pre-1.13 names).
pub(crate) fn minecraft_names(name: &str) -> Vec<String> {
    let mut v = vec![name.to_string()];
    let extra: &[&str] = match name {
        "grass_block_top" => &["grass_top"],
        "grass_block_side" => &["grass_side"],
        "snow_grass_side" => &["grass_block_snow", "grass_side_snowed"],
        "short_grass" => &["grass", "tallgrass"],
        "water" => &["water_still"],
        "lava" => &["lava_still"],
        "poppy" => &["flower_rose"],
        "dandelion" => &["flower_dandelion"],
        "bricks" => &["brick"],
        "stone_bricks" => &["stonebrick"],
        "mossy_cobblestone" => &["cobblestone_mossy"],
        "sandstone_side" => &["sandstone", "sandstone_normal"],
        "furnace_front" => &["furnace_front_off"],
        "torch" => &["torch_on"],
        "dead_bush" => &["deadbush"],
        "melon_side" => &["melon_side"],
        "dark_oak_log" => &["log_big_oak"],
        "dark_oak_log_top" => &["log_big_oak_top"],
        "dark_oak_planks" => &["planks_big_oak"],
        "dark_oak_leaves" => &["leaves_big_oak"],
        "terracotta" => &["hardened_clay"],
        "red_mushroom" => &["mushroom_red"],
        "brown_mushroom" => &["mushroom_brown"],
        "lily_pad" => &["waterlily"],
        _ => &[],
    };
    v.extend(extra.iter().map(|s| s.to_string()));
    for wood in ["oak", "birch", "spruce", "jungle", "acacia"] {
        if name == format!("{wood}_log") {
            v.push(format!("log_{wood}"));
        } else if name == format!("{wood}_log_top") {
            v.push(format!("log_{wood}_top"));
        } else if name == format!("{wood}_planks") {
            v.push(format!("planks_{wood}"));
        } else if name == format!("{wood}_leaves") {
            v.push(format!("leaves_{wood}"));
        }
    }
    if let Some(color) = name.strip_suffix("_wool") {
        let old = if color == "light_gray" { "silver" } else { color };
        v.push(format!("wool_colored_{old}"));
    }
    v
}

/// Colour Minecraft applies to its grey textures.
fn minecraft_tint(name: &str) -> Option<[u8; 3]> {
    match name {
        "grass_block_top" | "short_grass" => Some([0x79, 0xC0, 0x5A]),
        "oak_leaves" => Some([0x59, 0xAE, 0x30]),
        "spruce_leaves" => Some([0x61, 0x99, 0x61]),
        "birch_leaves" => Some([0x80, 0xA7, 0x55]),
        "jungle_leaves" | "vine" => Some([0x4A, 0xB0, 0x2A]),
        "acacia_leaves" | "dark_oak_leaves" => Some([0x5C, 0xA8, 0x34]),
        "mangrove_leaves" => Some([0x8D, 0xB1, 0x27]),
        "lily_pad" => Some([0x2E, 0x80, 0x30]),
        "water" => Some([0x3F, 0x76, 0xE4]),
        _ => None,
    }
}

fn decode(bytes: &[u8]) -> Option<image::RgbaImage> {
    image::load_from_memory(bytes).ok().map(|i| i.to_rgba8())
}

/// Keep only the first square frame (animated textures are vertical strips).
fn first_frame(img: image::RgbaImage) -> image::RgbaImage {
    let w = img.width();
    if img.height() > w {
        image::imageops::crop_imm(&img, 0, 0, w, w).to_image()
    } else {
        img
    }
}

fn tint(img: &mut image::RgbaImage, c: [u8; 3]) {
    for p in img.pixels_mut() {
        for k in 0..3 {
            p.0[k] = (p.0[k] as u32 * c[k] as u32 / 255) as u8;
        }
    }
}

/// Fit a texture to the array's layer size: smaller pictures grow with crisp pixels, bigger
/// ones (a 64x texture with the resolution capped at 16x or 32x) shrink by averaging, like mipmaps.
fn resize_nearest(img: &image::RgbaImage, size: u32) -> Vec<u8> {
    if img.width() == size && img.height() == size {
        return img.as_raw().clone();
    }
    if img.width() > size && img.width() == img.height() && img.width().is_power_of_two() {
        let levels = img.width().trailing_zeros() - size.trailing_zeros() + 1;
        return mip_chain(img.as_raw(), img.width(), levels).pop().unwrap();
    }
    let mut out = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let sx = x * img.width() / size;
            let sy = y * img.height() / size;
            let p = img.get_pixel(sx, sy).0;
            let i = ((y * size + x) * 4) as usize;
            out[i..i + 4].copy_from_slice(&p);
        }
    }
    out
}

fn missing(size: u32) -> Vec<u8> {
    let mut out = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let magenta = ((x * 2 / size) + (y * 2 / size)) % 2 == 0;
            let i = ((y * size + x) * 4) as usize;
            out[i..i + 4].copy_from_slice(if magenta { &[248, 0, 248, 255] } else { &[0, 0, 0, 255] });
        }
    }
    out
}

/// Sound effects from the enabled packs: for each of our sound keys ("dig/stone",
/// "random/explode"...) the variants the first pack that has them gives, as `.ogg` bytes.
/// Read from `assets/minecraft/sounds/<key><n>.ogg` (Minecraft's layout) or `sounds/<key><n>.ogg`.
pub fn pack_sounds(base: &Path, enabled_packs: &[String], keys: &[&'static str]) -> std::collections::HashMap<&'static str, Vec<std::sync::Arc<[u8]>>> {
    let mut packs: Vec<Pack> = enabled_packs.iter().filter_map(|n| Pack::open(&base.join("resourcepacks").join(n))).collect();
    let mut out = std::collections::HashMap::new();
    for &key in keys {
        'packs: for pack in packs.iter_mut() {
            for root in ["assets/minecraft/sounds", "sounds"] {
                let mut found: Vec<std::sync::Arc<[u8]>> = Vec::new();
                if let Some(b) = pack.read(&format!("{root}/{key}.ogg")) {
                    found.push(b.into());
                }
                for n in 1..=8 {
                    match pack.read(&format!("{root}/{key}{n}.ogg")) {
                        Some(b) => found.push(b.into()),
                        None if n > 1 => break,
                        None => {}
                    }
                }
                if !found.is_empty() {
                    out.insert(key, found);
                    break 'packs;
                }
            }
        }
    }
    out
}

/// Folder holding the base game's textures when running from the source tree, if present.
pub fn vanilla_texture_dir(base: &Path) -> Option<PathBuf> {
    let d = base.join("packs/vanilla/textures/block");
    d.is_dir().then_some(d)
}

/// Resolution of the base game's own textures (pixels per block face).
pub const BASE_RESOLUTION: u32 = 64;

/// Load every texture into one array. `max_res` caps the layer size (16, 32, 64...): the setting
/// for computers with little video memory; packs above it are shrunk smoothly.
/// Pictures plugins drew for their items ("plugin_<item>"), used before any pack.
pub static PLUGIN_TEXTURES: std::sync::Mutex<Vec<(String, image::RgbaImage)>> = std::sync::Mutex::new(Vec::new());

pub fn load(names: &[String], base: &Path, enabled_packs: &[String], max_res: u32) -> TextureSet {
    let mut packs: Vec<Pack> = enabled_packs.iter().filter_map(|n| Pack::open(&base.join("resourcepacks").join(n))).collect();
    let used: Vec<String> = packs.iter().map(|p| p.name.clone()).collect();
    let vanilla_dir = vanilla_texture_dir(base);

    let mut images: Vec<Option<image::RgbaImage>> = Vec::with_capacity(names.len());
    for full in names {
        // Item sprites are named "item/<name>" and live in the item folders.
        let (kind, name) = match full.strip_prefix("item/") {
            Some(n) => ("item", n),
            None => ("block", full.as_str()),
        };
        let mc_folders: [&str; 2] = if kind == "item" { ["item", "items"] } else { ["block", "blocks"] };
        let mut found: Option<image::RgbaImage> = None;
        // Plant bodies are painted below, once their plant's own picture is loaded.
        if crate::flora_tex::parse(full).is_some() || crate::bowdraw::parse(full).is_some() {
            images.push(None);
            continue;
        }
        // A picture only a pack's block model uses, by its path in the pack.
        if let Some(rest) = full.strip_prefix("packtex/") {
            let (ns, path) = rest.split_once('/').unwrap_or(("minecraft", rest));
            found = packs.iter_mut().find_map(|p| p.read(&format!("assets/{ns}/textures/{path}.png")).and_then(|b| decode(&b))).map(first_frame);
            if found.is_none() {
                log::warn!("missing texture '{full}'");
            }
            images.push(found);
            continue;
        }
        if name.starts_with("plugin_") {
            found = PLUGIN_TEXTURES.lock().unwrap().iter().find(|(n, _)| n == name).map(|(_, i)| i.clone());
        }
        'packs: for pack in packs.iter_mut() {
            if found.is_some() {
                break;
            }
            if let Some(img) = pack.read(&format!("textures/{kind}/{name}.png")).and_then(|b| decode(&b)) {
                found = Some(first_frame(img));
                break;
            }
            for mc in minecraft_names(name) {
                for folder in mc_folders {
                    let path = format!("assets/minecraft/textures/{folder}/{mc}.png");
                    if let Some(img) = pack.read(&path).and_then(|b| decode(&b)) {
                        let mut img = first_frame(img);
                        if let Some(c) = minecraft_tint(name) {
                            tint(&mut img, c);
                        }
                        // Minecraft draws grass sides as dirt plus a grey overlay coloured by biome.
                        if name == "grass_block_side" {
                            let ov = format!("assets/minecraft/textures/{folder}/{mc}_overlay.png");
                            if let Some(mut o) = pack.read(&ov).and_then(|b| decode(&b)).map(first_frame) {
                                tint(&mut o, [0x79, 0xC0, 0x5A]);
                                if o.dimensions() == img.dimensions() {
                                    for (d, s) in img.pixels_mut().zip(o.pixels()) {
                                        if s.0[3] > 127 {
                                            *d = *s;
                                        }
                                    }
                                }
                            }
                        }
                        found = Some(img);
                        break 'packs;
                    }
                }
            }
        }
        if found.is_none() {
            if let Some(d) = &vanilla_dir {
                let dir = if kind == "item" { d.with_file_name("item") } else { d.clone() };
                found = std::fs::read(dir.join(format!("{name}.png"))).ok().and_then(|b| decode(&b));
            }
        }
        if found.is_none() {
            let list = if kind == "item" { embedded::ITEM_TEXTURES } else { embedded::TEXTURES };
            found = list.iter().find(|(n, _)| *n == name).and_then(|(_, b)| decode(b));
        }
        if found.is_none() {
            log::warn!("missing texture '{full}'");
        }
        images.push(found.map(first_frame));
    }

    for (i, full) in names.iter().enumerate() {
        if let Some((tex, part)) = crate::flora_tex::parse(full) {
            let sprite = names.iter().position(|n| n == tex).and_then(|j| images[j].as_ref());
            images[i] = Some(crate::flora_tex::generate(part, sprite, tex));
        }
        // Bows drawn back, painted from the bow's own picture.
        if let Some((tex, stage)) = crate::bowdraw::parse(full) {
            let sprite = names.iter().position(|n| n == tex).and_then(|j| images[j].as_ref());
            images[i] = Some(crate::bowdraw::generate(sprite, stage));
        }
    }

    // All layers of a texture array share one size: use the largest (packs can be 16x, 32x,
    // 128x...; a 16x Minecraft pack over our 64x art grows 4x, pixel for pixel).
    let cap = max_res.clamp(16, 256).next_power_of_two();
    let size = images.iter().flatten().map(|i| i.width()).max().unwrap_or(16).clamp(16, 256).next_power_of_two().min(cap);
    let layers = images.iter().map(|i| i.as_ref().map(|i| resize_nearest(i, size)).unwrap_or_else(|| missing(size))).collect();
    let gui = load_gui(&mut packs, base);
    TextureSet { size, layers, packs: used, gui }
}

/// Mip chain for one layer, averaging only visible pixels so cut-out edges don't darken.
pub fn mip_chain(base: &[u8], size: u32, levels: u32) -> Vec<Vec<u8>> {
    mip_chain_rect(base, size, size, levels)
}

/// Mip chain of a w × h picture (both halve at every level).
pub fn mip_chain_rect(base: &[u8], w: u32, h: u32, levels: u32) -> Vec<Vec<u8>> {
    let mut out = vec![base.to_vec()];
    let (mut s, mut sh) = (w, h);
    for _ in 1..levels {
        let prev = out.last().unwrap();
        let ns = (s / 2).max(1);
        let nh = (sh / 2).max(1);
        let mut next = vec![0u8; (ns * nh * 4) as usize];
        for y in 0..nh {
            for x in 0..ns {
                let mut c = [0u32; 4];
                let mut wsum = 0u32;
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let i = (((y * 2 + dy) * s + x * 2 + dx) * 4) as usize;
                    let a = prev[i + 3] as u32;
                    for k in 0..3 {
                        c[k] += prev[i + k] as u32 * a;
                    }
                    c[3] += a;
                    wsum += a;
                }
                let o = ((y * ns + x) * 4) as usize;
                if wsum > 0 {
                    for k in 0..3 {
                        next[o + k] = (c[k] / wsum) as u8;
                    }
                }
                // Keep cut-out textures from fading away at a distance.
                let a = c[3] / 4;
                next[o + 3] = if a > 0 && a < 255 { (a * 5 / 4).min(255) as u8 } else { a as u8 };
            }
        }
        out.push(next);
        s = ns;
        sh = nh;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_sounds_are_found_in_minecraft_layout() {
        let base = std::env::temp_dir().join(format!("mc-packsnd-{}", std::process::id()));
        let snd = base.join("resourcepacks/P/assets/minecraft/sounds/dig");
        std::fs::create_dir_all(&snd).unwrap();
        for n in 1..=3 {
            std::fs::write(snd.join(format!("stone{n}.ogg")), [n as u8]).unwrap();
        }
        std::fs::write(snd.join("wood.ogg"), [9u8]).unwrap();
        let got = pack_sounds(&base, &["P".to_string()], &["dig/stone", "dig/wood", "dig/sand"]);
        assert_eq!(got.get("dig/stone").map(|v| v.len()), Some(3));
        assert_eq!(got.get("dig/wood").map(|v| v.len()), Some(1));
        assert!(!got.contains_key("dig/sand"));
        let _ = std::fs::remove_dir_all(&base);
    }
}
