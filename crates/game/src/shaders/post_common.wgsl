// ---- Cabeçalho dos shaders de pós-processamento (vem antes de todo post.wgsl) ----
struct Post {
    // x, y: tamanho da tela em pixels; z, w: 1/x, 1/y.
    resolution: vec4<f32>,
    // xy: posição do sol na tela (0..1, y para baixo); z: 0..1, quanto o sol está visível na
    // tela e acima do horizonte; w: luz do dia (0 noite .. 1 dia).
    sun: vec4<f32>,
    // xyz: direção do sol no mundo (y > 0 = acima do horizonte); w: pôr/nascer do sol 0..1.
    sun_dir: vec4<f32>,
    // rgb: cor da névoa/horizonte; w: 1 debaixo d'água.
    fog_color: vec4<f32>,
    // x: tempo em segundos; y: plano próximo; z: distância máxima (blocos);
    // w: qualidade (0 = Rápido, 1 = Bonito).
    params: vec4<f32>,
    // As opções do pack.ron, na ordem: use opt(0) .. opt(7).
    options: array<vec4<f32>, 2>,
    // Uso interno (suavização TAA): posição na tela deste quadro → do quadro anterior.
    reproj: mat4x4<f32>,
    // x: TAA ligado; y: o quadro anterior pode ser usado.
    taa: vec4<f32>,
};

@group(0) @binding(0) var<uniform> post: Post;
// O mundo já desenhado (cores lineares 0..1, pode passar de 1 no sol).
@group(0) @binding(1) var scene: texture_2d<f32>;
// Profundidade: 1.0 onde só há céu.
@group(0) @binding(2) var scene_depth: texture_depth_2d;
// Amostrador linear que não repete nas bordas.
@group(0) @binding(3) var lin: sampler;

struct PostIn {
    @builtin(position) pos: vec4<f32>,
    // 0..1 na tela, (0, 0) no canto de cima à esquerda.
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_post(@builtin(vertex_index) i: u32) -> PostIn {
    var pts = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    var o: PostIn;
    o.pos = vec4<f32>(pts[i], 0.0, 1.0);
    o.uv = vec2<f32>(pts[i].x * 0.5 + 0.5, 0.5 - pts[i].y * 0.5);
    return o;
}

// Valor da opção i (0..7).
fn opt(i: i32) -> f32 {
    return post.options[i / 4][i % 4];
}

// Cor do mundo em uv.
fn color_at(uv: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(scene, lin, uv, 0.0).rgb;
}

// Profundidade bruta em uv (0..1; 1 = céu).
fn depth_at(uv: vec2<f32>) -> f32 {
    let size = vec2<i32>(post.resolution.xy);
    let p = clamp(vec2<i32>(uv * post.resolution.xy), vec2<i32>(0), size - vec2<i32>(1));
    return textureLoad(scene_depth, p, 0);
}

fn is_sky(uv: vec2<f32>) -> bool {
    return depth_at(uv) >= 1.0;
}

// Distância em blocos do olho até o que está em uv (muito grande no céu).
fn distance_at(uv: vec2<f32>) -> f32 {
    let d = depth_at(uv);
    let n = post.params.y;
    let f = post.params.z;
    return n * f / max(f - d * (f - n), 1e-6);
}

// Brilho percebido de uma cor.
fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}
// ---- Fim do cabeçalho ----
