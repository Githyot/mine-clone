// Distant terrain tiles (lod.rs). Lit like fully sky-lit ground, fogged like chunks, and thrown
// away inside `g.effects.w` blocks of the eye, where real chunks are drawn.

struct LodOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) rel: vec3<f32>,
};

@vertex
fn vs_lod(@location(0) pos: vec3<f32>, @location(1) color: vec4<f32>, @location(2) origin: vec3<f32>) -> LodOut {
    var p = pos + origin;
    let d = length(p.xz - g.cam_pos.xz);
    // Near the real terrain the tiles sink and lose their tree tops, so the band where both are
    // drawn shows the real blocks.
    let far = smoothstep(g.effects.w + 16.0, g.effects.w + 64.0, d);
    p.y += color.a * 255.0 * far - 1.0 - 6.0 * (1.0 - far);
    var o: LodOut;
    o.clip = g.view_proj * vec4<f32>(p, 1.0);
    o.color = color.rgb;
    o.rel = p - g.cam_pos.xyz;
    return o;
}

@fragment
fn fs_lod(in: LodOut) -> @location(0) vec4<f32> {
    if (length(in.rel.xz) < g.effects.w) {
        discard;
    }
    let daylight = g.fog_color.w;
    let b = g.fog.w;
    var l = vec3<f32>(daylight);
    l = mix(l, sqrt(l), b * 0.5);
    l = max(l, vec3<f32>(0.03 + b * 0.04));
    let fog = smoothstep(g.fog.x, g.fog.y, length(in.rel));
    return vec4<f32>(mix(in.color * l, g.fog_color.rgb, fog), 1.0);
}
