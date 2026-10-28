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
    // Interface art, keyed by its path under textures/gui/ without ".png" ("sprites/hud/hotbar").
    let gui_dir = root.join("packs/vanilla/textures/gui");
    println!("cargo:rerun-if-changed={}", gui_dir.display());
    let mut gui = Vec::new();
    walk_png(&gui_dir, &mut gui);
    gui.sort();
    code.push_str("pub static GUI_TEXTURES: &[(&str, &[u8])] = &[\n");
    for p in &gui {
        let key = p.strip_prefix(&gui_dir).unwrap().with_extension("").to_string_lossy().replace('\\', "/");
        code.push_str(&format!("    ({:?}, include_bytes!({:?})),\n", key, p.display().to_string()));
    }
    code.push_str("];\n");

    for (var, file) in [("MOBS_RON", "packs/vanilla/mobs.ron"), ("PAINTINGS_RON", "packs/vanilla/paintings.ron"), ("ITEMS_RON", "packs/vanilla/items.ron"), ("RECIPES_RON", "packs/vanilla/recipes.ron"), ("STRUCTURES_RON", "packs/vanilla/structures.ron"), ("T5_BLOCKS_RON", "packs/vanilla/etapa5/blocks.ron"), ("T5_ITEMS_RON", "packs/vanilla/etapa5/items.ron"), ("T5_MOBS_RON", "packs/vanilla/etapa5/mobs.ron"), ("T5_RECIPES_RON", "packs/vanilla/etapa5/recipes.ron"), ("RPG_ITEMS_RON", "packs/vanilla/rpg/items.ron"), ("RPG_RECIPES_RON", "packs/vanilla/rpg/recipes.ron"), ("DEEP_BLOCKS_RON", "packs/vanilla/deep/blocks.ron"), ("DEEP_ITEMS_RON", "packs/vanilla/deep/items.ron"), ("DEEP_RECIPES_RON", "packs/vanilla/deep/recipes.ron"), ("TR_BLOCKS_RON", "packs/vanilla/tr/blocks.ron"), ("TR_ITEMS_RON", "packs/vanilla/tr/items.ron"), ("TR_MOBS_RON", "packs/vanilla/tr/mobs.ron"), ("AGUAS_MOBS_RON", "packs/vanilla/aguas/mobs.ron"), ("AGUAS_BLOCKS_RON", "packs/vanilla/aguas/blocks.ron"), ("VULCAO_BLOCKS_RON", "packs/vanilla/vulcao/blocks.ron"), ("VULCAO_ITEMS_RON", "packs/vanilla/vulcao/items.ron"), ("VULCAO_RECIPES_RON", "packs/vanilla/vulcao/recipes.ron"), ("VULCAO_MOBS_RON", "packs/vanilla/vulcao/mobs.ron"), ("TR_RECIPES_RON", "packs/vanilla/tr/recipes.ron"), ("TR_STRUCTURES_RON", "packs/vanilla/tr/structures.ron"), ("LOTE11_ITEMS_RON", "packs/vanilla/rpg/lote11_items.ron"), ("LOTE11_RECIPES_RON", "packs/vanilla/rpg/lote11_recipes.ron"), ("FASE6_BLOCKS_RON", "packs/vanilla/fase6/blocks.ron"), ("FASE6_ITEMS_RON", "packs/vanilla/fase6/items.ron"), ("FASE6_RECIPES_RON", "packs/vanilla/fase6/recipes.ron"), ("FASE6_STRUCTURES_RON", "packs/vanilla/fase6/structures.ron"), ("FASE6_MOBS_RON", "packs/vanilla/fase6/mobs.ron"), ("MODS_BLOCKS_RON", "packs/vanilla/mods/blocks.ron"), ("MODS_ITEMS_RON", "packs/vanilla/mods/items.ron"), ("MODS_RECIPES_RON", "packs/vanilla/mods/recipes.ron"), ("MODS_MOBS_RON", "packs/vanilla/mods/mobs.ron"), ("MODS_STRUCTURES_RON", "packs/vanilla/mods/structures.ron"), ("MODS_DIMS_MOBS_RON", "packs/vanilla/mods/dims_mobs.ron"), ("MODS_DIMS_STRUCTURES_RON", "packs/vanilla/mods/dims_structures.ron")] {
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

fn walk_png(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            println!("cargo:rerun-if-changed={}", p.display());
            walk_png(&p, out);
        } else if p.extension().is_some_and(|x| x == "png") {
            out.push(p);
        }
    }
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
