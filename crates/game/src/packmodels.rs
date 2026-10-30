//! Block models from resource packs (Lote 9): a pack's `assets/minecraft/blockstates/*.json`
//! and `models/block/*.json` in Minecraft's format, for the blocks we share a name with. Only
//! models that bring their own geometry (`elements`, here or in a parent inside the same pack)
//! change anything; the usual `cube_all`-style models only name textures, which the texture
//! loader already takes from the pack.
//!
//! Supported: boxes (`from`, `to`), each face's `texture` (`#name` looked up through the
//! model and its parents), `uv`, `cullface` and `tintindex`. Not supported, so such a model is
//! left out: element rotation, and different models per orientation (the block's first
//! variant is used for the block as placed by default).

use std::collections::HashMap;
use std::path::Path;

use voxel::block::{BlockId, BlockRegistry, Shape, Transparency};
use voxel::mesh::{model_quads, ModelElement, ModelFace, ModelQuad};

use crate::mods::paintings_pp::{parse_json, Json};
use crate::textures::{minecraft_names, Pack};

/// At most this many textures that only models use (they share the block texture array).
const MAX_EXTRA_TEXTURES: usize = 512;

#[derive(Default)]
pub(crate) struct PackModels {
    /// Pack pictures the models use that aren't ours ("packtex/<ns>/<path>"): their layers
    /// come right after the block textures.
    pub textures: Vec<String>,
    pub models: Vec<(BlockId, Vec<ModelQuad>)>,
    /// Plants with a body (their pictures are at the end of `textures`).
    pub flora: Vec<voxel::flora::FloraModel>,
}

/// Face names in mesh order (+x east, -x west, +y up, -y down, +z south, -z north).
const FACE_NAMES: [&str; 6] = ["east", "west", "up", "down", "south", "north"];

fn face_index(name: &str) -> Option<u8> {
    FACE_NAMES.iter().position(|f| *f == name).map(|i| i as u8)
}

/// "minecraft:block/x", "block/x" or "x" as (namespace, path).
fn split_ref(r: &str) -> (String, String) {
    let (ns, path) = r.split_once(':').unwrap_or(("minecraft", r));
    let path = if path.contains('/') { path.to_string() } else { format!("block/{path}") };
    (ns.to_string(), path)
}

fn num(j: &Json) -> Option<f32> {
    match j {
        Json::Num(n) => Some(*n as f32),
        _ => None,
    }
}

fn vec3(j: Option<&Json>) -> Option<[f32; 3]> {
    let Some(Json::Arr(a)) = j else { return None };
    (a.len() == 3).then(|| [num(&a[0]), num(&a[1]), num(&a[2])]).and_then(|[x, y, z]| Some([x?, y?, z?]))
}

fn text(j: Option<&Json>) -> Option<&str> {
    match j {
        Some(Json::Str(s)) => Some(s),
        _ => None,
    }
}

/// A model's elements and texture names, following its parents inside the pack.
fn resolve(pack: &mut Pack, model: &str) -> Option<(Vec<Json>, HashMap<String, String>)> {
    let mut textures: HashMap<String, String> = HashMap::new();
    let mut elements = None;
    let mut next = Some(model.to_string());
    for _ in 0..8 {
        let Some(m) = next.take() else { break };
        let (ns, path) = split_ref(&m);
        let Some(j) = pack.read(&format!("assets/{ns}/models/{path}.json")).and_then(|b| String::from_utf8(b).ok()).and_then(|t| parse_json(&t)) else { break };
        if let Some(Json::Obj(t)) = j.get("textures") {
            for (k, v) in t {
                // Newer packs write a texture as {"sprite": "...", "force_translucent": true}.
                let v = match v {
                    Json::Str(v) => Some(v.as_str()),
                    o => text(o.get("sprite")),
                };
                if let Some(v) = v {
                    textures.entry(k.clone()).or_insert_with(|| v.to_string());
                }
            }
        }
        if elements.is_none() {
            if let Some(Json::Arr(e)) = j.get("elements") {
                elements = Some(e.clone());
            }
        }
        next = text(j.get("parent")).map(str::to_string);
    }
    Some((elements?, textures))
}

/// The model a blockstate file names first.
fn state_model(pack: &mut Pack, mc: &str) -> Option<String> {
    let j = pack.read(&format!("assets/minecraft/blockstates/{mc}.json")).and_then(|b| String::from_utf8(b).ok()).and_then(|t| parse_json(&t))?;
    let Some(Json::Obj(v)) = j.get("variants") else { return None };
    let pick = v.iter().find(|(k, _)| k.is_empty()).or_else(|| v.first()).map(|(_, m)| m)?;
    let m = match pick {
        Json::Arr(a) => a.first()?,
        m => m,
    };
    text(m.get("model")).map(str::to_string)
}

/// Read every enabled pack's models for our blocks. `first_layer` is where the textures only
/// models use will start.
pub(crate) fn load(reg: &BlockRegistry, base: &Path, enabled: &[String]) -> PackModels {
    let mut out = load_packs(reg, base, enabled);
    let (names, flora) = crate::flora_tex::plan(reg, reg.textures.len() + out.textures.len());
    out.textures.extend(names);
    out.flora = flora;
    out
}

fn load_packs(reg: &BlockRegistry, base: &Path, enabled: &[String]) -> PackModels {
    let mut out = PackModels::default();
    let mut packs: Vec<Pack> = enabled.iter().filter_map(|n| Pack::open(&base.join("resourcepacks").join(n))).collect();
    if packs.is_empty() {
        return out;
    }
    // Our textures by every name Minecraft gives them.
    let mut ours: HashMap<String, u16> = HashMap::new();
    for (i, t) in reg.textures.iter().enumerate() {
        for mc in minecraft_names(t) {
            ours.entry(mc).or_insert(i as u16);
        }
    }
    let first_layer = reg.textures.len();
    let mut extra: HashMap<String, u16> = HashMap::new();
    let mut done: std::collections::HashSet<BlockId> = std::collections::HashSet::new();
    for pack in packs.iter_mut() {
        let files = pack.list();
        let has = |p: &str| files.iter().any(|f| f == p);
        for (id, b) in reg.blocks.iter().enumerate() {
            let id = id as BlockId;
            // Doors, trapdoors, gates and stairs turn and open through their variants: one
            // model on the base id would draw only that one (a closed north door as the pack's
            // first state, open and facing east) differently from all the others.
            // Cubes that turn when placed (furnaces, chests, logs) likewise.
            let turns = matches!(b.shape, Shape::Door | Shape::Trapdoor | Shape::FenceGate | Shape::Stairs) || b.turn != voxel::block::Turn::None;
            // See-through blocks (stained glass, ice) stay ours: pack models only reach the
            // solid pass, where their tinted fill would be cut away. The pack's texture is still used.
            if b.variant != 0 || turns || done.contains(&id) || matches!(b.shape, Shape::Liquid | Shape::Invisible) || b.transparency == Transparency::Translucent || b.name == "air" {
                continue;
            }
            let mut model = None;
            for mc in minecraft_names(&b.name) {
                if has(&format!("assets/minecraft/blockstates/{mc}.json")) {
                    model = state_model(pack, &mc);
                } else if has(&format!("assets/minecraft/models/block/{mc}.json")) {
                    model = Some(format!("minecraft:block/{mc}"));
                }
                if model.is_some() {
                    break;
                }
            }
            let Some(model) = model else { continue };
            let Some((elements, tex)) = resolve(pack, &model) else { continue };
            let mut parsed = Vec::new();
            let mut ok = !elements.is_empty();
            for e in &elements {
                if e.get("rotation").and_then(|r| r.get("angle")).and_then(num).is_some_and(|a| a != 0.0) {
                    ok = false;
                    break;
                }
                let (Some(from), Some(to)) = (vec3(e.get("from")), vec3(e.get("to"))) else {
                    ok = false;
                    break;
                };
                let mut faces = [None; 6];
                if let Some(Json::Obj(fs)) = e.get("faces") {
                    for (name, f) in fs {
                        let Some(fi) = face_index(name) else { continue };
                        // "#side" -> the model's texture of that name, possibly another "#...".
                        let mut t = text(f.get("texture")).unwrap_or("").to_string();
                        for _ in 0..8 {
                            match t.strip_prefix('#').and_then(|k| tex.get(k)) {
                                Some(v) => t = v.clone(),
                                None => break,
                            }
                        }
                        if t.starts_with('#') || t.is_empty() {
                            continue;
                        }
                        let (ns, path) = split_ref(&t);
                        let ours_layer = path.strip_prefix("block/").filter(|_| ns == "minecraft").and_then(|n| ours.get(n).copied());
                        let layer = match ours_layer {
                            Some(l) => l,
                            None => {
                                let key = format!("packtex/{ns}/{path}");
                                if let Some(&l) = extra.get(&key) {
                                    l
                                } else if out.textures.len() < MAX_EXTRA_TEXTURES {
                                    let l = (first_layer + out.textures.len()) as u16;
                                    out.textures.push(key.clone());
                                    extra.insert(key, l);
                                    l
                                } else {
                                    continue;
                                }
                            }
                        };
                        let uv = match f.get("uv") {
                            Some(Json::Arr(a)) if a.len() == 4 => {
                                let v: Vec<f32> = a.iter().filter_map(num).collect();
                                (v.len() == 4).then(|| [v[0], v[1], v[2], v[3]])
                            }
                            _ => None,
                        };
                        let cull = text(f.get("cullface")).and_then(face_index);
                        faces[fi as usize] = Some(ModelFace { uv, layer, cull, tint: f.get("tintindex").is_some() });
                    }
                }
                parsed.push(ModelElement { from, to, faces });
            }
            // A model whose faces all failed to resolve would draw nothing: keep ours.
            ok &= parsed.iter().any(|e| e.faces.iter().any(Option::is_some));
            if !ok {
                log::info!("pack model for {} skipped (rotated or broken elements)", b.name);
                continue;
            }
            done.insert(id);
            out.models.push((id, model_quads(&parsed)));
        }
    }
    if !out.models.is_empty() {
        log::info!("{} block models from resource packs ({} extra textures)", out.models.len(), out.textures.len());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pack_model_becomes_quads() {
        let dir = std::env::temp_dir().join(format!("mc-packmodel-{}", std::process::id()));
        let pack = dir.join("resourcepacks/Lanternas");
        let models = pack.join("assets/minecraft/models/block");
        std::fs::create_dir_all(&models).unwrap();
        std::fs::create_dir_all(pack.join("assets/minecraft/blockstates")).unwrap();
        std::fs::write(pack.join("assets/minecraft/blockstates/torch.json"), r#"{"variants": {"": {"model": "minecraft:block/tocha_grossa"}}}"#).unwrap();
        // A thick torch: a parent with the box, a child naming the texture.
        std::fs::write(
            models.join("tocha_base.json"),
            r##"{"elements": [{"from": [6, 0, 6], "to": [10, 12, 10], "faces": {
                "up": {"texture": "#chama", "uv": [6, 6, 10, 10]}, "down": {"texture": "#lado", "cullface": "down"},
                "north": {"texture": "#lado"}, "south": {"texture": "#lado"}, "east": {"texture": "#lado"}, "west": {"texture": "#lado", "tintindex": 0}}}]}"##,
        )
        .unwrap();
        std::fs::write(models.join("tocha_grossa.json"), r##"{"parent": "block/tocha_base", "textures": {"lado": "block/oak_planks", "chama": "mypack:block/flame"}}"##).unwrap();
        let (reg, _, _) = crate::game_data(crate::embedded::BLOCKS_RON, crate::embedded::ITEMS_RON, crate::embedded::RECIPES_RON).unwrap();
        let m = load(&reg, &dir, &["Lanternas".to_string()]);
        let torch = reg.id("torch").unwrap();
        let (_, quads) = m.models.iter().find(|(id, _)| *id == torch).expect("torch model");
        assert_eq!(quads.len(), 6);
        // The pack's own pictures come first, the painted plant parts after them.
        assert_eq!(m.textures[0], "packtex/mypack/block/flame");
        assert!(m.textures[1..].iter().all(|t| t.starts_with("flora/")));
        let planks = reg.textures.iter().position(|t| t == "oak_planks").unwrap() as u16;
        let up = quads.iter().find(|q| q.face == 2).unwrap();
        assert_eq!(up.layer as usize, reg.textures.len());
        assert!(up.pos.iter().all(|p| p[1] == 12));
        assert!(up.uv.iter().all(|uv| (6..=10).contains(&uv[0]) && (6..=10).contains(&uv[1])));
        let down = quads.iter().find(|q| q.face == 3).unwrap();
        assert_eq!((down.layer, down.cull, down.edge), (planks, 3, true));
        assert!(quads.iter().find(|q| q.face == 1).unwrap().tint);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
