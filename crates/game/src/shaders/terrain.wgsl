@group(1) @binding(0) var tex: texture_2d_array<f32>;
@group(1) @binding(1) var samp: sampler;

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) layer: u32,
    @location(2) color: vec3<f32>,
    @location(3) light: vec2<f32>,
    @location(4) dist: f32,
};

fn light_curve(l: f32) -> f32 {
    // Same shape as the classic brightness table: dark levels fall off quickly.
    let x = 1.0 - l;
    return (1.0 - x) / (x * 3.0 + 1.0);
}

@vertex
fn vs_main(@location(0) w: vec3<u32>, @location(1) origin: vec3<f32>) -> VOut {
    var shades = array<f32, 6>(0.6, 0.6, 1.0, 0.5, 0.8, 0.8);
    var ao_curve = array<f32, 4>(0.45, 0.64, 0.82, 1.0);
    var p = vec3<f32>(f32(w.x & 1023u), f32((w.x >> 10u) & 1023u), f32((w.x >> 20u) & 1023u)) / 16.0 + origin;
    let ao = (w.x >> 30u) & 3u;
    let uv = vec2<f32>(f32(w.y & 31u), f32((w.y >> 5u) & 31u)) / 16.0;
    let layer = (w.y >> 10u) & 4095u;
    let face = (w.y >> 22u) & 7u;
    let flags = w.y >> 25u;
    if ((flags & 1u) != 0u) {
        // Leaves and plants sway gently in the wind.
        let wp = p + g.cam_world.xyz - g.cam_pos.xyz;
        let t = g.cam_pos.w;
        p.x += sin(t * 1.6 + wp.x * 0.7 + wp.z * 0.4 + wp.y * 0.3) * 0.025;
        p.z += cos(t * 1.2 + wp.z * 0.6 + wp.x * 0.3) * 0.025;
    }
    var out: VOut;
    out.clip = g.view_proj * vec4<f32>(p, 1.0);
    out.uv = uv;
    out.layer = layer;
    let tint = vec3<f32>(f32((w.z >> 8u) & 255u), f32((w.z >> 16u) & 255u), f32((w.z >> 24u) & 255u)) / 200.0;
    out.color = tint * shades[face] * ao_curve[ao];
    out.light = vec2<f32>(f32(w.z & 15u), f32((w.z >> 4u) & 15u)) / 15.0;
    out.dist = length(p - g.cam_pos.xyz);
    return out;
}

fn shade(c: vec3<f32>, in: VOut) -> vec3<f32> {
    let daylight = g.fog_color.w;
    let sky = light_curve(in.light.x) * daylight;
    let blk = light_curve(in.light.y);
    // Torches are a little warm, sky light is neutral.
    var l = max(vec3<f32>(sky), vec3<f32>(blk, blk * 0.93, blk * 0.82));
    let b = g.fog.w;
    l = mix(l, sqrt(l), b * 0.5);
    l = max(l, vec3<f32>(0.03 + b * 0.04));
    let rgb = c * in.color * l;
    let fog = smoothstep(g.fog.x, g.fog.y, in.dist);
    return mix(rgb, g.fog_color.rgb, fog);
}

@fragment
fn fs_solid(in: VOut) -> @location(0) vec4<f32> {
    let c = textureSample(tex, samp, in.uv, in.layer);
    if (c.a < 0.5) {
        discard;
    }
    return vec4<f32>(shade(c.rgb, in), 1.0);
}

@fragment
fn fs_translucent(in: VOut) -> @location(0) vec4<f32> {
    let c = textureSample(tex, samp, in.uv, in.layer);
    if (c.a < 0.02) {
        discard;
    }
    let fog = smoothstep(g.fog.x, g.fog.y, in.dist);
    return vec4<f32>(shade(c.rgb, in), mix(c.a, 1.0, fog));
}
