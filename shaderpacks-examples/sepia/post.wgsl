// Exemplo de shader de pós-processamento: tons de sépia, vinheta e grão de filme.
// O jogo coloca um cabeçalho antes deste arquivo com: color_at(uv), depth_at(uv),
// distance_at(uv), is_sky(uv), luma(c), opt(i) e a struct `post` (veja LEIA-ME.txt).

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

@fragment
fn fs_post(in: PostIn) -> @location(0) vec4<f32> {
    var c = color_at(in.uv);
    // Sépia: o brilho da cor vira um marrom quente.
    let l = luma(c);
    let sepia = vec3<f32>(l * 1.07, l * 0.86, l * 0.62);
    c = mix(c, sepia, opt(0));
    // Vinheta: escurece os cantos.
    if (opt(1) > 0.5) {
        let d = length(in.uv - vec2<f32>(0.5));
        c *= 1.0 - smoothstep(0.35, 0.85, d) * 0.6;
    }
    // Grão: muda a cada quadro.
    let n = hash(in.pos.xy + vec2<f32>(fract(post.params.x * 7.0) * 100.0)) - 0.5;
    c += vec3<f32>(n * opt(2));
    return vec4<f32>(clamp(c, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
