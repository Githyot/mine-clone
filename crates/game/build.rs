//! Embeds the base game's textures and music into the executable, so the game runs even if it is
//! copied somewhere without the `packs/` and `assets/` folders. Files on disk still win when present.
use std::fs;
use std::path::Path;

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("embedded.rs");
    let mut code = String::new();

    let tex_dir = root.join("packs/vanilla/textures/block");
    println!("cargo:rerun-if-changed={}", tex_dir.display());
    let mut tex: Vec<_> = fs::read_dir(&tex_dir)
        .map(|r| r.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "png")).collect())
        .unwrap_or_default();
    tex.sort();
    code.push_str("pub static TEXTURES: &[(&str, &[u8])] = &[\n");
    for p in &tex {
        let name = p.file_stem().unwrap().to_string_lossy();
        if name == "preview" {
            continue;
        }
        code.push_str(&format!("    ({:?}, include_bytes!({:?})),\n", name, p.display().to_string()));
    }
    code.push_str("];\n");

    let blocks = root.join("packs/vanilla/blocks.ron");
    println!("cargo:rerun-if-changed={}", blocks.display());
    code.push_str(&format!("pub static BLOCKS_RON: &str = include_str!({:?});\n", blocks.display().to_string()));

    for (var, sub) in [("ENTITY_TEXTURES", "entity"), ("PAINTING_TEXTURES", "painting"), ("ITEM_TEXTURES", "item")] {
        let dir = root.join("packs/vanilla/textures").join(sub);
        println!("cargo:rerun-if-changed={}", dir.display());
        let mut files: Vec<_> = fs::read_dir(&dir)
            .map(|r| r.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "png")).collect())
            .unwrap_or_default();
        files.sort();
        code.push_str(&format!("pub static {var}: &[(&str, &[u8])] = &[\n"));
        for p in &files {
            let name = p.file_stem().unwrap().to_string_lossy();
            if name == "preview" {
                continue;
            }
            code.push_str(&format!("    ({:?}, include_bytes!({:?})),\n", name, p.display().to_string()));
        }
        code.push_str("];\n");
    }
    for (var, file) in [("MOBS_RON", "packs/vanilla/mobs.ron"), ("PAINTINGS_RON", "packs/vanilla/paintings.ron"), ("ITEMS_RON", "packs/vanilla/items.ron"), ("RECIPES_RON", "packs/vanilla/recipes.ron")] {
        let p = root.join(file);
        println!("cargo:rerun-if-changed={}", p.display());
        if p.exists() {
            code.push_str(&format!("pub static {var}: &str = include_str!({:?});\n", p.display().to_string()));
        } else {
            code.push_str(&format!("pub static {var}: &str = \"\";\n"));
        }
    }

    let music_dir = root.join("assets/music");
    println!("cargo:rerun-if-changed={}", music_dir.display());
    let mut music: Vec<_> = fs::read_dir(&music_dir)
        .map(|r| r.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "ogg")).collect())
        .unwrap_or_default();
    music.sort();
    code.push_str("pub static MUSIC: &[(&str, &[u8])] = &[\n");
    for p in &music {
        let name = p.file_stem().unwrap().to_string_lossy();
        code.push_str(&format!("    ({:?}, include_bytes!({:?})),\n", name, p.display().to_string()));
    }
    code.push_str("];\n");
    let tracks = music_dir.join("tracks.json");
    let tracks_src = if tracks.exists() { format!("include_str!({:?})", tracks.display().to_string()) } else { "\"[]\"".into() };
    code.push_str(&format!("pub static TRACKS_JSON: &str = {tracks_src};\n"));
    fs::write(out, code).unwrap();
}
