//! Embeds the base game's textures and music into the executable, so the game runs even if it is
//! copied somewhere without the `packs/` and `assets/` folders. Files on disk still win when present.
use std::fs;
use std::path::Path;

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    windows_icon(&root);
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
    for (var, file) in [("MOBS_RON", "packs/vanilla/mobs.ron"), ("PAINTINGS_RON", "packs/vanilla/paintings.ron"), ("ITEMS_RON", "packs/vanilla/items.ron"), ("RECIPES_RON", "packs/vanilla/recipes.ron"), ("STRUCTURES_RON", "packs/vanilla/structures.ron"), ("MODS_BLOCKS_RON", "packs/vanilla/mods/blocks.ron"), ("MODS_ITEMS_RON", "packs/vanilla/mods/items.ron"), ("MODS_RECIPES_RON", "packs/vanilla/mods/recipes.ron"), ("MODS_MOBS_RON", "packs/vanilla/mods/mobs.ron"), ("MODS_STRUCTURES_RON", "packs/vanilla/mods/structures.ron"), ("MODS_DIMS_MOBS_RON", "packs/vanilla/mods/dims_mobs.ron"), ("MODS_DIMS_STRUCTURES_RON", "packs/vanilla/mods/dims_structures.ron")] {
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

/// The program icon of the Windows .exe (the chameleon on a dirt block), compiled with windres
/// when it is available (the MinGW cross build); other builds simply go without it.
fn windows_icon(root: &Path) {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let ico = root.join("assets/icon/icon.ico");
    println!("cargo:rerun-if-changed={}", ico.display());
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).to_path_buf();
    let rc = out.join("icon.rc");
    let obj = out.join("icon.o");
    let ico_path = ico.display().to_string().replace('\\', "/");
    if fs::write(&rc, format!("1 ICON \"{ico_path}\"\n")).is_err() {
        return;
    }
    for tool in ["x86_64-w64-mingw32-windres", "windres"] {
        let ok = std::process::Command::new(tool).arg(&rc).args(["-O", "coff", "-o"]).arg(&obj).status().is_ok_and(|s| s.success());
        if ok {
            println!("cargo:rustc-link-arg-bins={}", obj.display());
            return;
        }
    }
}
