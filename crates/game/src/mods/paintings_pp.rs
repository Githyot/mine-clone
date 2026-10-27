//! Paintings++ idea: painting packs. Drop a pack (a folder or a .zip) into `paintings/packs/`
//! and its paintings join the picker. Two layouts are read:
//!
//! * Paintings++ style: a JSON file listing `{"paintings": [{"name": "x", "x": 2, "y": 1}, ...]}`
//!   (or `"square": 2`), with `x.png` anywhere in the pack;
//! * data-pack style: `painting_variant/*.json` files with `asset_id`, `width` and `height`,
//!   the picture at `assets/<namespace>/textures/painting/<path>.png`.
//!
//! A pack with no JSON at all still works: every PNG in it becomes a painting sized from its
//! shape. Pictures with clear pixels hang as see-through paintings. Packs are only read from the
//! player's own folder; nothing is bundled with the game.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use crate::paintings::{fit_blocks, has_alpha, load_png, PaintingDef, MAX_SIZE};

/// A JSON value (enough of JSON for pack files).
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(v) => v.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    fn num(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            Json::Str(s) => s.parse().ok(),
            _ => None,
        }
    }
    fn str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }
}

/// Parse a JSON document (None when it is not valid JSON).
pub fn parse_json(text: &str) -> Option<Json> {
    let mut p = Parser { s: text.as_bytes(), i: 0 };
    let v = p.value()?;
    p.ws();
    (p.i == p.s.len()).then_some(v)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }
    fn eat(&mut self, c: u8) -> bool {
        self.ws();
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn value(&mut self) -> Option<Json> {
        self.ws();
        match *self.s.get(self.i)? {
            b'{' => {
                self.i += 1;
                let mut v = Vec::new();
                if self.eat(b'}') {
                    return Some(Json::Obj(v));
                }
                loop {
                    self.ws();
                    let k = self.string()?;
                    if !self.eat(b':') {
                        return None;
                    }
                    v.push((k, self.value()?));
                    if self.eat(b',') {
                        continue;
                    }
                    return self.eat(b'}').then_some(Json::Obj(v));
                }
            }
            b'[' => {
                self.i += 1;
                let mut v = Vec::new();
                if self.eat(b']') {
                    return Some(Json::Arr(v));
                }
                loop {
                    v.push(self.value()?);
                    if self.eat(b',') {
                        continue;
                    }
                    return self.eat(b']').then_some(Json::Arr(v));
                }
            }
            b'"' => self.string().map(Json::Str),
            b't' if self.s[self.i..].starts_with(b"true") => {
                self.i += 4;
                Some(Json::Bool(true))
            }
            b'f' if self.s[self.i..].starts_with(b"false") => {
                self.i += 5;
                Some(Json::Bool(false))
            }
            b'n' if self.s[self.i..].starts_with(b"null") => {
                self.i += 4;
                Some(Json::Null)
            }
            _ => {
                let start = self.i;
                while self.i < self.s.len() && matches!(self.s[self.i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
                    self.i += 1;
                }
                std::str::from_utf8(&self.s[start..self.i]).ok()?.parse().ok().map(Json::Num)
            }
        }
    }
    fn string(&mut self) -> Option<String> {
        if self.s.get(self.i) != Some(&b'"') {
            return None;
        }
        self.i += 1;
        let mut out = Vec::new();
        while let Some(&c) = self.s.get(self.i) {
            self.i += 1;
            match c {
                b'"' => return String::from_utf8(out).ok(),
                b'\\' => {
                    let e = *self.s.get(self.i)?;
                    self.i += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let hex = std::str::from_utf8(self.s.get(self.i..self.i + 4)?).ok()?;
                            self.i += 4;
                            let ch = char::from_u32(u32::from_str_radix(hex, 16).ok()?).unwrap_or('?');
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        other => out.push(other),
                    }
                }
                _ => out.push(c),
            }
        }
        None
    }
}

/// The files of one pack: path inside the pack (with `/`) -> bytes. Only JSON and PNG files.
fn pack_files(path: &Path) -> HashMap<String, Vec<u8>> {
    let mut files = HashMap::new();
    let wanted = |n: &str| {
        let l = n.to_ascii_lowercase();
        l.ends_with(".json") || l.ends_with(".png")
    };
    if path.is_dir() {
        fn walk(dir: &Path, root: &Path, out: &mut HashMap<String, Vec<u8>>, depth: u32, wanted: &dyn Fn(&str) -> bool) {
            let Ok(rd) = std::fs::read_dir(dir) else { return };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() && depth < 8 {
                    walk(&p, root, out, depth + 1, wanted);
                } else if let Ok(rel) = p.strip_prefix(root) {
                    let rel = rel.to_string_lossy().replace('\\', "/");
                    if wanted(&rel) {
                        if let Ok(b) = std::fs::read(&p) {
                            out.insert(rel, b);
                        }
                    }
                }
            }
        }
        walk(path, path, &mut files, 0, &wanted);
    } else if path.extension().is_some_and(|x| x.eq_ignore_ascii_case("zip")) {
        let Ok(f) = std::fs::File::open(path) else { return files };
        let Ok(mut z) = zip::ZipArchive::new(f) else { return files };
        for i in 0..z.len() {
            let Ok(mut e) = z.by_index(i) else { continue };
            let name = e.name().to_string();
            if e.is_dir() || !wanted(&name) || e.size() > 16 << 20 {
                continue;
            }
            let mut b = Vec::new();
            if e.read_to_end(&mut b).is_ok() {
                files.insert(name, b);
            }
        }
    }
    files
}

fn file_stem(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name)
}

fn title_of(stem: &str) -> String {
    let t = stem.replace(['_', '-'], " ");
    let mut c = t.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => t,
    }
}

/// The paintings of one pack's files.
pub fn read_pack(pack: &str, files: &HashMap<String, Vec<u8>>) -> Vec<PaintingDef> {
    let mut out = Vec::new();
    let png_named = |stem: &str| -> Option<(&String, &Vec<u8>)> {
        let mut best: Option<(&String, &Vec<u8>)> = None;
        for (k, v) in files {
            if k.to_ascii_lowercase().ends_with(".png") && file_stem(k) == stem {
                // Prefer pictures in a painting folder.
                if best.is_none() || k.contains("painting") {
                    best = Some((k, v));
                }
            }
        }
        best
    };
    let add = |_key: &str, stem: &str, title: String, w: u32, h: u32, bytes: &[u8], out: &mut Vec<PaintingDef>| {
        let (w, h) = (w.clamp(1, MAX_SIZE), h.clamp(1, MAX_SIZE));
        if let Some(pixels) = load_png(bytes, w, h) {
            out.push(PaintingDef { name: format!("pack:{pack}:{stem}"), title, width: w, height: h, alpha: has_alpha(&pixels), pixels });
        }
    };
    let mut jsons: Vec<(&String, &Vec<u8>)> = files.iter().filter(|(k, _)| k.to_ascii_lowercase().ends_with(".json")).collect();
    jsons.sort_by(|a, b| a.0.cmp(b.0));
    for (path, bytes) in jsons {
        let Some(j) = std::str::from_utf8(bytes).ok().and_then(parse_json) else { continue };
        if let Some(Json::Arr(list)) = j.get("paintings") {
            // Paintings++: name plus x/y or square, sizes in blocks.
            for e in list {
                let Some(name) = e.get("name").and_then(Json::str) else { continue };
                let sq = e.get("square").and_then(Json::num);
                let w = e.get("x").and_then(Json::num).or(sq).unwrap_or(1.0) as u32;
                let h = e.get("y").and_then(Json::num).or(sq).unwrap_or(1.0) as u32;
                let stem = file_stem(name);
                if let Some((k, b)) = png_named(stem) {
                    let title = e.get("title").and_then(Json::str).map(String::from).unwrap_or_else(|| title_of(stem));
                    let k = k.clone();
                    add(&k, stem, title, w, h, b, &mut out);
                }
            }
        } else if let (Some(asset), Some(w), Some(h)) = (j.get("asset_id").and_then(Json::str), j.get("width").and_then(Json::num), j.get("height").and_then(Json::num)) {
            // Data-pack painting variant.
            let (ns, p) = asset.split_once(':').unwrap_or(("minecraft", asset));
            let want = format!("assets/{ns}/textures/painting/{p}.png");
            let found = files.iter().find(|(k, _)| k.ends_with(&want)).or_else(|| png_named(file_stem(p)));
            if let Some((k, b)) = found {
                let title = match j.get("title") {
                    Some(Json::Str(s)) => s.clone(),
                    Some(t) => t.get("text").and_then(Json::str).map(String::from).unwrap_or_else(|| title_of(file_stem(p))),
                    None => title_of(file_stem(path)),
                };
                let k = k.clone();
                add(&k, file_stem(p), title, w as u32, h as u32, b, &mut out);
            }
        }
    }
    // No list for these pictures: size from their shape.
    if out.is_empty() {
        let mut pngs: Vec<(&String, &Vec<u8>)> = files.iter().filter(|(k, _)| k.to_ascii_lowercase().ends_with(".png") && !k.ends_with("pack.png")).collect();
        pngs.sort_by(|a, b| a.0.cmp(b.0));
        for (k, b) in pngs {
            let Ok(img) = image::load_from_memory(b) else { continue };
            let (w, h) = fit_blocks(img.width(), img.height());
            let stem = file_stem(k).to_string();
            let k = k.clone();
            add(&k, &stem, title_of(&stem), w, h, b, &mut out);
        }
    }
    out
}

/// Every pack in the folder, in name order.
pub fn import_packs(dir: &Path) -> Vec<PaintingDef> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut packs: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    packs.sort();
    let mut out = Vec::new();
    for p in packs {
        let name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let files = pack_files(&p);
        if files.is_empty() {
            continue;
        }
        let got = read_pack(&name, &files);
        log::info!("painting pack '{name}': {} paintings", got.len());
        out.extend(got);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32, clear: bool) -> Vec<u8> {
        let img = image::RgbaImage::from_fn(w, h, |x, _| image::Rgba([200, 100, 50, if clear && x % 2 == 0 { 0 } else { 255 }]));
        let mut out = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
        out
    }

    #[test]
    fn json_parses() {
        let j = parse_json(r#"{"paintings": [{"name": "a", "x": 2, "y": 1}, {"name": "bé", "square": 3}], "ok": true, "n": null}"#).unwrap();
        let Some(Json::Arr(l)) = j.get("paintings") else { panic!() };
        assert_eq!(l.len(), 2);
        assert_eq!(l[1].get("name"), Some(&Json::Str("bé".into())));
        assert!(parse_json("{\"a\": }").is_none());
    }

    #[test]
    fn reads_both_pack_layouts() {
        let mut files = HashMap::new();
        files.insert("assets/mypack/paintings.json".to_string(), br#"{"paintings": [{"name": "wide", "x": 2, "y": 1}, {"name": "ghost", "square": 2}]}"#.to_vec());
        files.insert("assets/mypack/textures/painting/wide.png".to_string(), png(32, 16, false));
        files.insert("assets/mypack/textures/painting/ghost.png".to_string(), png(32, 32, true));
        let got = read_pack("p", &files);
        assert_eq!(got.len(), 2);
        let wide = got.iter().find(|d| d.name == "pack:p:wide").unwrap();
        assert_eq!((wide.width, wide.height, wide.alpha), (2, 1, false));
        assert!(got.iter().find(|d| d.name == "pack:p:ghost").unwrap().alpha);

        let mut dp = HashMap::new();
        dp.insert("data/art/painting_variant/sea.json".to_string(), br#"{"asset_id": "art:sea", "width": 3, "height": 2}"#.to_vec());
        dp.insert("assets/art/textures/painting/sea.png".to_string(), png(48, 32, false));
        let got = read_pack("d", &dp);
        assert_eq!(got.len(), 1);
        assert_eq!((got[0].width, got[0].height), (3, 2));
    }
}
