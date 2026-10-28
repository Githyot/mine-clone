//! The interface in other languages (Lote 5 of plan/o-que-falta.md). The game is written in
//! Portuguese; a language file maps its texts to another language, and every text drawn on
//! screen goes through `tr` on its way to the font. Built in: English (packs/vanilla/lang/en.ron).
//! Players can add their own: `lang/<code>.ron` next to the game, the same map of
//! "Portuguese text": "translation" (a file named en.ron there adds to and corrects ours).
//!
//! Texts with numbers in them are matched by their template ("Campo de visão: {:.0}" matches
//! "Campo de visão: 70"); "Label: value" texts are translated half by half; the names of
//! blocks, items and mobs fall back to their internal names ("pale_oak_log" → "Pale Oak Log").

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock, RwLock};

const BUILT_IN_EN: &str = include_str!("../../../packs/vanilla/lang/en.ron");

#[derive(Default)]
struct Table {
    exact: HashMap<String, String>,
    /// (the Portuguese template cut at its placeholders, the translated template)
    templates: Vec<(Vec<String>, String)>,
}

static TABLE: RwLock<Option<Table>> = RwLock::new(None);
static NAMES: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn names() -> &'static Mutex<HashMap<String, String>> {
    NAMES.get_or_init(Default::default)
}

fn cache() -> &'static Mutex<HashMap<String, String>> {
    CACHE.get_or_init(Default::default)
}

/// Cut a template at its `{…}` placeholders: "a {} b {:.0}" → ["a ", " b ", ""].
fn pieces(t: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut depth = 0;
    for c in t.chars() {
        match c {
            '{' => {
                depth += 1;
                if depth == 1 {
                    out.push(String::new());
                }
            }
            '}' if depth > 0 => depth -= 1,
            _ if depth > 0 => {}
            _ => out.last_mut().unwrap().push(c),
        }
    }
    out
}

/// The values a text has where its template has placeholders, if it fits the template.
fn fit<'a>(s: &'a str, p: &[String]) -> Option<Vec<&'a str>> {
    let mut rest = s.strip_prefix(p[0].as_str())?;
    let mut vals = vec![];
    for (k, piece) in p.iter().enumerate().skip(1) {
        let last = k == p.len() - 1;
        let at = if last {
            if piece.is_empty() {
                rest.len()
            } else {
                rest.strip_suffix(piece.as_str())?.len()
            }
        } else if piece.is_empty() {
            return None;
        } else {
            rest.find(piece.as_str())?
        };
        vals.push(&rest[..at]);
        rest = &rest[at + piece.len()..];
    }
    rest.is_empty().then_some(vals)
}

fn fill(template: &str, vals: &[&str]) -> String {
    let mut out = String::new();
    let mut depth = 0;
    let mut k = 0;
    for c in template.chars() {
        match c {
            '{' => {
                if depth == 0 {
                    out.push_str(vals.get(k).copied().unwrap_or(""));
                    k += 1;
                }
                depth += 1;
            }
            '}' if depth > 0 => depth -= 1,
            _ if depth > 0 => {}
            _ => out.push(c),
        }
    }
    out
}

fn add(t: &mut Table, text: &str) {
    let Ok(map) = ron::from_str::<HashMap<String, String>>(text) else {
        log::warn!("a language file could not be read");
        return;
    };
    for (pt, tr) in map {
        if pt.contains('{') {
            let p = pieces(&pt);
            // Only templates with words in them: "{} {}" would match anything.
            if p.iter().map(|x| x.trim().len()).sum::<usize>() >= 3 {
                t.templates.retain(|(q, _)| *q != p);
                t.templates.push((p, tr));
            }
        } else {
            t.exact.insert(pt, tr);
        }
    }
}

/// Switch the language: "pt" is the game's own; others load the built-in file and `lang/<code>.ron`.
pub fn set_language(code: &str, base: &Path) {
    cache().lock().unwrap().clear();
    let mut table = TABLE.write().unwrap();
    if code == "pt" {
        *table = None;
        return;
    }
    let mut t = Table::default();
    if code == "en" {
        add(&mut t, BUILT_IN_EN);
    }
    if let Ok(text) = std::fs::read_to_string(base.join("lang").join(format!("{code}.ron"))) {
        add(&mut t, &text);
    }
    // Longest templates first, so the most specific one wins.
    t.templates.sort_by_key(|(p, _)| std::cmp::Reverse(p.iter().map(|x| x.len()).sum::<usize>()));
    *table = Some(t);
}

/// Languages to choose from: Portuguese, English and the player's own files in `lang/`.
pub fn languages(base: &Path) -> Vec<String> {
    let mut out = vec!["pt".to_string(), "en".to_string()];
    if let Ok(dir) = std::fs::read_dir(base.join("lang")) {
        let mut more: Vec<String> = dir
            .filter_map(|e| e.ok())
            .filter_map(|e| e.path().file_stem().map(|s| s.to_string_lossy().to_string()).filter(|_| e.path().extension().is_some_and(|x| x == "ron")))
            .filter(|c| !out.contains(c))
            .collect();
        more.sort();
        out.extend(more);
    }
    out
}

pub fn language_name(code: &str) -> String {
    match code {
        "pt" => "Português".into(),
        "en" => "English".into(),
        c => c.to_string(),
    }
}

/// A block, item or mob: its Portuguese name and its internal one, for the fallback.
pub fn add_name(display: &str, internal: &str) {
    if display.is_empty() || internal.is_empty() {
        return;
    }
    let english: Vec<String> = internal
        .split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
        })
        .collect();
    names().lock().unwrap().entry(display.to_string()).or_insert(english.join(" "));
}

fn lookup(t: &Table, s: &str) -> Option<String> {
    if let Some(e) = t.exact.get(s) {
        return Some(e.clone());
    }
    for (p, tr) in &t.templates {
        if let Some(vals) = fit(s, p) {
            let vals: Vec<String> = vals.iter().map(|v| lookup(t, v).unwrap_or_else(|| v.to_string())).collect();
            let refs: Vec<&str> = vals.iter().map(|v| v.as_str()).collect();
            return Some(fill(tr, &refs));
        }
    }
    if let Some(n) = names().lock().unwrap().get(s) {
        return Some(n.clone());
    }
    // "Label: value" halves, and the same with a leading "> " or trailing " <" marker.
    if let Some((a, b)) = s.split_once(": ") {
        let (ta, tb) = (lookup(t, a), lookup(t, b));
        if ta.is_some() || tb.is_some() {
            return Some(format!("{}: {}", ta.unwrap_or_else(|| a.to_string()), tb.unwrap_or_else(|| b.to_string())));
        }
    }
    if let Some(inner) = s.strip_prefix("> ").and_then(|x| x.strip_suffix(" <")) {
        return lookup(t, inner).map(|x| format!("> {x} <"));
    }
    None
}

/// The text in the chosen language (the same text when there is no translation).
pub fn tr(s: &str) -> Cow<'_, str> {
    let table = TABLE.read().unwrap();
    let Some(t) = table.as_ref() else { return Cow::Borrowed(s) };
    if s.len() < 2 || !s.chars().any(|c| c.is_alphabetic()) {
        return Cow::Borrowed(s);
    }
    let mut c = cache().lock().unwrap();
    if let Some(x) = c.get(s) {
        return Cow::Owned(x.clone());
    }
    let out = lookup(t, s).unwrap_or_else(|| s.to_string());
    if c.len() > 8192 {
        c.clear();
    }
    c.insert(s.to_string(), out.clone());
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_and_halves() {
        let p = pieces("Campo de visão: {:.0}");
        assert_eq!(fit("Campo de visão: 70", &p), Some(vec!["70"]));
        assert_eq!(fill("Field of view: {:.0}", &["70"]), "Field of view: 70");
        let p = pieces("{} de {} xp");
        assert_eq!(fit("3 de 10 xp", &p), Some(vec!["3", "10"]));
        assert_eq!(fit("3 de 10 ouro", &p), None);
        let mut t = Table::default();
        add(&mut t, r#"{"Sim": "Yes", "Nuvens": "Clouds", "Volume da música: {:.0}%": "Music: {:.0}%"}"#);
        assert_eq!(lookup(&t, "Nuvens: Sim").as_deref(), Some("Clouds: Yes"));
        assert_eq!(lookup(&t, "Volume da música: 60%").as_deref(), Some("Music: 60%"));
        add_name("Tronco de Carvalho Pálido", "pale_oak_log");
        assert_eq!(lookup(&t, "Tronco de Carvalho Pálido").as_deref(), Some("Pale Oak Log"));
    }

    #[test]
    fn built_in_english_reads() {
        let mut t = Table::default();
        add(&mut t, BUILT_IN_EN);
        assert!(t.exact.len() + t.templates.len() > 300, "{} texts", t.exact.len() + t.templates.len());
        for (p, tr) in &t.templates {
            assert_eq!(p.len(), pieces(tr).len(), "placeholders differ in {tr:?}");
        }
    }
}
