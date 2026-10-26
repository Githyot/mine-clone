//! Shader packs: post-processing effects drawn over the 3D scene (never over the HUD).
//! Two packs are built in; players can add their own as folders in `shaderpacks/`, each with a
//! `pack.ron` (name, description, options) and a `post.wgsl` fragment shader. The GPU side is in
//! `post.rs`; this file only finds, describes and configures packs.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Most options a pack can have: they reach the shader as `opt(0)` .. `opt(7)`.
pub const MAX_OPTIONS: usize = 8;

#[derive(Deserialize, Clone, Debug)]
pub enum PackOption {
    /// On/off; the shader sees 1.0 or 0.0.
    Bool { name: String, label: String, default: bool },
    /// A number the menu cycles from `min` to `max` in `step`s (a tenth of the range when 0).
    Float {
        name: String,
        label: String,
        default: f32,
        min: f32,
        max: f32,
        #[serde(default)]
        step: f32,
    },
}

impl PackOption {
    pub fn name(&self) -> &str {
        match self {
            PackOption::Bool { name, .. } | PackOption::Float { name, .. } => name,
        }
    }
    pub fn label(&self) -> &str {
        match self {
            PackOption::Bool { label, .. } | PackOption::Float { label, .. } => label,
        }
    }
    pub fn default_value(&self) -> f32 {
        match self {
            PackOption::Bool { default, .. } => *default as u8 as f32,
            PackOption::Float { default, .. } => *default,
        }
    }
    /// The value after one click in the menu.
    pub fn next(&self, v: f32) -> f32 {
        match self {
            PackOption::Bool { .. } => if v > 0.5 { 0.0 } else { 1.0 },
            PackOption::Float { min, max, step, .. } => {
                let step = if *step > 0.0 { *step } else { (max - min) / 10.0 };
                let n = v + step;
                if n > max + step * 0.01 {
                    *min
                } else {
                    // Snap to the step grid so repeated clicks don't drift.
                    let k = ((n - min) / step).round();
                    (min + k * step).min(*max)
                }
            }
        }
    }
    pub fn display(&self, v: f32) -> String {
        match self {
            PackOption::Bool { .. } => (if v > 0.5 { "Sim" } else { "Não" }).to_string(),
            PackOption::Float { step, max, min, .. } => {
                let step = if *step > 0.0 { *step } else { (max - min) / 10.0 };
                if step >= 1.0 {
                    format!("{v:.0}")
                } else if step >= 0.1 {
                    format!("{v:.1}")
                } else {
                    format!("{v:.2}")
                }
            }
        }
    }
}

/// Contents of a pack's `pack.ron`.
#[derive(Deserialize, Clone, Debug)]
pub struct PackDef {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub options: Vec<PackOption>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Builtin {
    /// Bloom, sun rays, colour grading, vignette, distance haze, waving plants, rippling water.
    Realcado,
    /// Light bloom and colour grading only.
    Suave,
}

#[derive(Clone, Debug)]
pub enum PackSource {
    Builtin(Builtin),
    /// Folder holding `pack.ron` and `post.wgsl`.
    User(PathBuf),
}

#[derive(Clone, Debug)]
pub struct ShaderPack {
    /// What settings.ron stores: the built-in name, or the folder name for user packs.
    pub id: String,
    pub def: PackDef,
    pub source: PackSource,
}

impl ShaderPack {
    /// Option values for the shader: saved values where present, defaults otherwise.
    pub fn values(&self, saved: Option<&BTreeMap<String, f32>>) -> [f32; MAX_OPTIONS] {
        let mut v = [0.0; MAX_OPTIONS];
        for (i, o) in self.def.options.iter().take(MAX_OPTIONS).enumerate() {
            v[i] = saved.and_then(|m| m.get(o.name())).copied().unwrap_or_else(|| o.default_value());
        }
        v
    }
}

fn float(name: &str, label: &str, default: f32, min: f32, max: f32, step: f32) -> PackOption {
    PackOption::Float { name: name.into(), label: label.into(), default, min, max, step }
}

fn boolean(name: &str, label: &str, default: bool) -> PackOption {
    PackOption::Bool { name: name.into(), label: label.into(), default }
}

/// Option order matters: the built-in shaders read them by index (see `post.wgsl`).
pub fn builtin_packs() -> Vec<ShaderPack> {
    vec![
        ShaderPack {
            id: "Realçado".into(),
            def: PackDef {
                name: "Realçado".into(),
                description: "Brilho, raios de sol, cores vivas, névoa e água ondulante".into(),
                options: vec![
                    float("bloom", "Brilho (bloom)", 1.0, 0.0, 2.0, 0.25),
                    float("raios", "Raios de sol", 1.0, 0.0, 2.0, 0.25),
                    float("saturacao", "Saturação", 1.15, 0.5, 1.5, 0.05),
                    float("vinheta", "Vinheta", 0.35, 0.0, 1.0, 0.05),
                    boolean("nevoa", "Névoa de distância", true),
                    boolean("folhas", "Folhas balançando", true),
                    boolean("agua", "Água ondulante", true),
                    float("exposicao", "Exposição", 1.0, 0.6, 1.6, 0.1),
                ],
            },
            source: PackSource::Builtin(Builtin::Realcado),
        },
        ShaderPack {
            id: "Suave".into(),
            def: PackDef {
                name: "Suave".into(),
                description: "Leve: um pouco de brilho e cores mais quentes".into(),
                options: vec![
                    float("bloom", "Brilho (bloom)", 0.5, 0.0, 1.0, 0.1),
                    float("saturacao", "Saturação", 1.05, 0.5, 1.5, 0.05),
                    float("calor", "Cores quentes", 0.3, 0.0, 1.0, 0.1),
                    float("vinheta", "Vinheta", 0.2, 0.0, 1.0, 0.05),
                ],
            },
            source: PackSource::Builtin(Builtin::Suave),
        },
    ]
}

/// Built-in packs first, then every valid folder in `dir`. Broken folders are reported, not fatal.
pub fn list_packs(dir: &Path) -> (Vec<ShaderPack>, Vec<String>) {
    let mut packs = builtin_packs();
    let mut errors = Vec::new();
    let folders = |d: &Path| -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(d).map(|r| r.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_dir()).collect()).unwrap_or_default();
        v.sort();
        v
    };
    // Then the ones that came inside resource packs (see `textures::extract_pack_shaders`).
    let mut dirs = folders(dir);
    dirs.extend(folders(&dir.join(".pacotes")));
    for d in dirs {
        let id = d.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if !d.join("post.wgsl").is_file() {
            continue;
        }
        if packs.iter().any(|p| p.id.eq_ignore_ascii_case(&id)) {
            errors.push(format!("{id}: nome igual a um pacote embutido"));
            continue;
        }
        let def = match std::fs::read_to_string(d.join("pack.ron")) {
            Ok(text) => match ron::from_str::<PackDef>(&text) {
                Ok(mut def) => {
                    if def.options.len() > MAX_OPTIONS {
                        log::warn!("shader pack {id}: only the first {MAX_OPTIONS} options are used");
                        def.options.truncate(MAX_OPTIONS);
                    }
                    def
                }
                Err(e) => {
                    let msg = format!("{id}/pack.ron: {e}");
                    log::warn!("{msg}");
                    errors.push(msg);
                    continue;
                }
            },
            // pack.ron is optional: a lone post.wgsl is a pack without options.
            Err(_) => PackDef { name: id.clone(), description: String::new(), options: Vec::new() },
        };
        packs.push(ShaderPack { id, def, source: PackSource::User(d) });
    }
    (packs, errors)
}

/// Find a pack by id, forgiving case and missing accents ("realcado" finds "Realçado").
pub fn find<'a>(packs: &'a [ShaderPack], id: &str) -> Option<&'a ShaderPack> {
    let fold = |s: &str| -> String {
        s.chars()
            .map(|c| match c {
                'á' | 'à' | 'â' | 'ã' | 'Á' | 'À' | 'Â' | 'Ã' => 'a',
                'é' | 'ê' | 'É' | 'Ê' => 'e',
                'í' | 'Í' => 'i',
                'ó' | 'ô' | 'õ' | 'Ó' | 'Ô' | 'Õ' => 'o',
                'ú' | 'Ú' => 'u',
                'ç' | 'Ç' => 'c',
                c => c.to_ascii_lowercase(),
            })
            .collect()
    };
    packs.iter().find(|p| p.id == id).or_else(|| packs.iter().find(|p| fold(&p.id) == fold(id)))
}

const EXAMPLE_RON: &str = include_str!("../../../shaderpacks-examples/sepia/pack.ron");
const EXAMPLE_WGSL: &str = include_str!("../../../shaderpacks-examples/sepia/post.wgsl");
/// What every user shader gets in front of its own code.
pub const HEADER_WGSL: &str = include_str!("shaders/post_common.wgsl");

/// Create `shaderpacks/` with the example pack the first time, and refresh the instructions.
pub fn ensure_folder(dir: &Path) {
    if !dir.exists() && std::fs::create_dir_all(dir).is_ok() {
        let ex = dir.join("sepia");
        if std::fs::create_dir_all(&ex).is_ok() {
            let _ = std::fs::write(ex.join("pack.ron"), EXAMPLE_RON);
            let _ = std::fs::write(ex.join("post.wgsl"), EXAMPLE_WGSL);
        }
    }
    if dir.is_dir() {
        let _ = std::fs::write(dir.join("LEIA-ME.txt"), readme());
    }
}

fn readme() -> String {
    format!(
        "PACOTES DE SHADER\n\
         =================\n\n\
         Cada pacote é uma pasta aqui dentro com dois arquivos:\n\n\
         \x20 pack.ron   nome, descrição e opções (opcional)\n\
         \x20 post.wgsl  o shader de pós-processamento, em WGSL\n\n\
         Escolha o pacote no menu de pausa (Esc) > Shaders. Depois de editar os arquivos,\n\
         clique em \"Recarregar pacotes\" para ver o resultado sem fechar o jogo.\n\
         Se o shader tiver um erro, o jogo mostra a mensagem e desliga o shader (não fecha).\n\
         A pasta \"sepia\" é um exemplo: copie-a com outro nome para começar o seu.\n\
         Um resource pack também pode trazer um shader: ponha os mesmos dois arquivos na pasta\n\
         \"shaders/\" do pack. Com o pack ligado, o shader aparece no menu com o nome do pack\n\
         (o jogo copia para \".pacotes/\" aqui dentro; não edite lá).\n\n\
         pack.ron\n\
         --------\n\
         (\n\
         \x20   name: \"Meu shader\",\n\
         \x20   description: \"Uma linha que aparece no menu\",\n\
         \x20   options: [\n\
         \x20       Float(name: \"forca\", label: \"Força\", default: 0.8, min: 0.0, max: 1.0, step: 0.1),\n\
         \x20       Bool(name: \"vinheta\", label: \"Vinheta\", default: true),\n\
         \x20   ],\n\
         )\n\n\
         Até {max} opções. No shader, opt(0) é a primeira, opt(1) a segunda, e assim por diante.\n\
         Opções Bool valem 1.0 (sim) ou 0.0 (não). Os valores escolhidos ficam em settings.ron.\n\n\
         post.wgsl\n\
         ---------\n\
         O jogo desenha o mundo (céu, terreno, mobs, nuvens, água) numa textura, roda o seu\n\
         shader na tela inteira e só depois desenha o HUD por cima. Escreva uma função:\n\n\
         \x20 @fragment\n\
         \x20 fn fs_post(in: PostIn) -> @location(0) vec4<f32> {{\n\
         \x20     let c = color_at(in.uv);\n\
         \x20     return vec4<f32>(c, 1.0);\n\
         \x20 }}\n\n\
         O código abaixo é colocado antes do seu arquivo (não copie, ele já está lá). Por causa\n\
         dele, os números de linha nas mensagens de erro ficam {lines} linhas à frente.\n\n\
         {header}\n",
        max = MAX_OPTIONS,
        lines = format!("{HEADER_WGSL}\n").lines().count(),
        header = HEADER_WGSL
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_pack_parses() {
        let def: PackDef = ron::from_str(EXAMPLE_RON).unwrap();
        assert!(!def.options.is_empty() && def.options.len() <= MAX_OPTIONS);
    }

    #[test]
    fn float_options_cycle_and_wrap() {
        let o = float("x", "X", 0.5, 0.0, 1.0, 0.25);
        assert_eq!(o.next(0.5), 0.75);
        assert_eq!(o.next(1.0), 0.0);
        let b = boolean("b", "B", true);
        assert_eq!(b.next(b.default_value()), 0.0);
    }

    #[test]
    fn find_ignores_accents() {
        let packs = builtin_packs();
        assert_eq!(find(&packs, "realcado").map(|p| p.id.as_str()), Some("Realçado"));
        assert!(find(&packs, "nada").is_none());
    }
}
