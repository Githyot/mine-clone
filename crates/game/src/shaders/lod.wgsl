// Distant terrain nodes (lod.rs). Lit like fully sky-lit ground, fogged like chunks. Nodes that reach
// into the area of real chunks (inside `g.effects.w` blocks of the eye) use `fs_lod`, which throws
// those pixels away; every other node uses `fs_lod_far`, which skips the test so the GPU can reject
// hidden pixels early. Lighting is worked out per vertex: the fragments only fog.

struct LodOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) rel: vec3<f32>,
};

@vertex
fn vs_lod(@location(0) packed: u32, @location(1) color: vec4<f32>, @location(2) origin: vec3<f32>) -> LodOut {
    // x, z in cells (6 bits each), level (3 bits), y * 8 + 8192 (14 bits).
    let step = f32(4u << ((packed >> 12u) & 7u));
    let y = f32((packed >> 15u) & 16383u) / 8.0 - 1024.0;
    var p = vec3<f32>(f32(packed & 63u) * step, y, f32((packed >> 6u) & 63u) * step) + origin;
    let d = length(p.xz - g.cam_pos.xz);
    // Near the real terrain the nodes sink and lose their tree tops, so the band where both are
    // drawn shows the real blocks.
    let far = smoothstep(g.effects.w + 16.0, g.effects.w + 64.0, d);
    p.y += color.a * 255.0 * far - 1.0 - 6.0 * (1.0 - far);
    var o: LodOut;
    o.clip = g.view_proj * vec4<f32>(p, 1.0);
    var c = color.rgb;
    if (c.g > c.r * 1.05 && c.g > c.b * 1.05) {
        // Grass and tree tops turn with the seasons, like the real chunks.
        c *= season_tint(vec3<f32>(1.0, 1.0, 0.9));
    }
    let daylight = g.fog_color.w;
    let b = g.fog.w;
    var l = vec3<f32>(daylight);
    l = mix(l, sqrt(l), b * 0.5);
    l = max(l, vec3<f32>(0.03 + b * 0.04));
    o.color = c * l;
    o.rel = p - g.cam_pos.xyz;
    return o;
}

fn lod_fog(in: LodOut) -> vec4<f32> {
    let fog = smoothstep(g.fog.x, g.fog.y, length(in.rel));
    return vec4<f32>(mix(in.color, g.fog_color.rgb, fog), 1.0);
}

@fragment
fn fs_lod(in: LodOut) -> @location(0) vec4<f32> {
    if (length(in.rel.xz) < g.effects.w) {
        discard;
    }
    return lod_fog(in);
}

@fragment
fn fs_lod_far(in: LodOut) -> @location(0) vec4<f32> {
    return lod_fog(in);
}
