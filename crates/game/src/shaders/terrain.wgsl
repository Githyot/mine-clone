@group(1) @binding(0) var tex: texture_2d_array<f32>;
@group(1) @binding(1) var samp: sampler;

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) layer: u32,
    @location(2) color: vec3<f32>,
    @location(3) light: vec2<f32>,
    @location(4) dist: f32,
    // Mesh flags in the low bits, face in bits 8..10.
    @location(5) @interpolate(flat) kind: u32,
    // Position relative to the camera's block.
    @location(6) rel: vec3<f32>,
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
    let wp = p + g.cam_world.xyz - g.cam_pos.xyz;
    let t = g.cam_pos.w;
    if ((flags & 1u) != 0u) {
        // Leaves and plants sway gently in the wind (shader packs can make it stronger).
        let a = 0.025 * g.effects.x;
        let gust = 1.0 + (g.effects.x - 1.0) * 0.5 * (sin(t * 0.37 + wp.x * 0.05) * 0.5 + 0.5);
        p.x += sin(t * 1.6 + wp.x * 0.7 + wp.z * 0.4 + wp.y * 0.3) * a * gust;
        p.z += cos(t * 1.2 + wp.z * 0.6 + wp.x * 0.3) * a * gust;
    }
    if ((flags & 2u) != 0u && face == 2u && g.effects.y > 0.5) {
        // Water surface bobs a little; it stays at or below its normal height.
        p.y += (sin(t * 1.7 + wp.x * 0.9 + wp.z * 0.5) + sin(t * 1.1 - wp.z * 0.8 + wp.x * 0.3)) * 0.018 - 0.04;
    }
    var out: VOut;
    out.clip = g.view_proj * vec4<f32>(p, 1.0);
    out.uv = uv;
    if ((flags & 4u) != 0u) {
        // Merged quad: repeat the texture once per block, from the position in the section.
        let q = vec3<f32>(f32(w.x & 1023u), f32((w.x >> 10u) & 1023u), f32((w.x >> 20u) & 1023u)) / 16.0;
        switch face {
            case 0u: { out.uv = vec2<f32>(1.0 - q.z, 1.0 - q.y); }
            case 1u: { out.uv = vec2<f32>(q.z, 1.0 - q.y); }
            case 2u: { out.uv = vec2<f32>(q.x, q.z); }
            case 3u: { out.uv = vec2<f32>(q.x, 1.0 - q.z); }
            case 4u: { out.uv = vec2<f32>(q.x, 1.0 - q.y); }
            default: { out.uv = vec2<f32>(1.0 - q.x, 1.0 - q.y); }
        }
    }
    out.layer = layer;
    let tint = vec3<f32>(f32((w.z >> 8u) & 255u), f32((w.z >> 16u) & 255u), f32((w.z >> 24u) & 255u)) / 200.0;
    out.color = tint * shades[face] * ao_curve[ao];
    out.light = vec2<f32>(f32(w.z & 15u), f32((w.z >> 4u) & 15u)) / 15.0;
    out.dist = length(p - g.cam_pos.xyz);
    out.kind = flags | (face << 8u);
    out.rel = p;
    return out;
}

fn lit(c: vec3<f32>, in: VOut) -> vec3<f32> {
    let daylight = g.fog_color.w;
    let sky = light_curve(in.light.x) * daylight;
    // A light in the player's hand fades one level per block, like a placed torch.
    let hand = g.hand_light.w - length(in.rel - g.hand_light.xyz) / 15.0;
    let blk = light_curve(max(in.light.y, hand));
    // Torches are a little warm, sky light is neutral.
    var l = max(vec3<f32>(sky), vec3<f32>(blk, blk * 0.93, blk * 0.82));
    let b = g.fog.w;
    l = mix(l, sqrt(l), b * 0.5);
    l = max(l, vec3<f32>(0.03 + b * 0.04));
    var rgb = c * in.color * l;
    if (g.effects.z > 0.0) {
        // Shader packs draw in HDR: bright texels right next to a light (torch flames,
        // glowstone, lava) go past 1.0 so the bloom pass makes them glow.
        let hot = smoothstep(0.8, 0.95, in.light.y) * smoothstep(0.5, 0.85, max(c.r, max(c.g, c.b)));
        rgb *= 1.0 + hot * g.effects.z;
    }
    return rgb;
}

fn fog_amount(in: VOut) -> f32 {
    return smoothstep(g.fog.x, g.fog.y, in.dist);
}

fn shade(c: vec3<f32>, in: VOut) -> vec3<f32> {
    return mix(lit(c, in), g.fog_color.rgb, fog_amount(in));
}

@fragment
fn fs_solid(in: VOut) -> @location(0) vec4<f32> {
    let c = textureSample(tex, samp, in.uv, in.layer);
    if (c.a < 0.5) {
        discard;
    }
    return vec4<f32>(shade(c.rgb, in), 1.0);
}

// Slope of a few travelling waves at a world position (for rippling water).
fn wave_slope(xz: vec2<f32>, t: f32) -> vec2<f32> {
    let d1 = vec2<f32>(0.8, 0.6);
    let d2 = vec2<f32>(-0.55, 0.83);
    let d3 = vec2<f32>(0.24, -0.97);
    var s = d1 * cos(dot(xz, d1) * 1.7 + t * 1.9) * 0.6;
    s += d2 * cos(dot(xz, d2) * 2.9 + t * 2.6) * 0.35;
    s += d3 * cos(dot(xz, d3) * 5.3 + t * 3.4) * 0.2;
    return s;
}

@fragment
fn fs_translucent(in: VOut) -> @location(0) vec4<f32> {
    let c = textureSample(tex, samp, in.uv, in.layer);
    if (c.a < 0.02) {
        discard;
    }
    let fog = fog_amount(in);
    var rgb = lit(c.rgb, in);
    var a = c.a;
    let water_top = (in.kind & 2u) != 0u && ((in.kind >> 8u) & 7u) == 2u;
    if (water_top && g.effects.y > 0.5 && g.fog.z < 0.5) {
        // Rippling water: a wavy normal shifts the brightness, reflects the sky at grazing
        // angles and catches the sun.
        let wp = in.rel + g.cam_world.xyz - g.cam_pos.xyz;
        let s = wave_slope(wp.xz, g.cam_pos.w);
        let n = normalize(vec3<f32>(-s.x * 0.22, 1.0, -s.y * 0.22));
        let v = normalize(in.rel - g.cam_pos.xyz);
        let sky = light_curve(in.light.x);
        let fres = pow(1.0 - max(-v.y, 0.0), 4.0);
        rgb *= 0.9 + (n.x + n.z) * 1.2;
        rgb = mix(rgb, g.sky_horizon.rgb * sky, fres * 0.45);
        let r = reflect(v, n);
        let spec = pow(max(dot(r, g.sun_dir.xyz), 0.0), 120.0) * g.fog_color.w * sky;
        rgb += vec3<f32>(1.0, 0.92, 0.75) * spec * 1.6;
        a = mix(a, 1.0, fres * 0.5 + min(spec, 1.0) * 0.5);
    }
    return vec4<f32>(mix(rgb, g.fog_color.rgb, fog), mix(a, 1.0, fog));
}
