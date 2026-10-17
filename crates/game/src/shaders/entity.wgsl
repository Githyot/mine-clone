// Mobs and other box-model entities. Each instance has one matrix per body part in a storage buffer.

struct Bones {
    m: array<mat4x4<f32>, 16>,
    // x: sky light 0..1, y: block light 0..1, z: hurt flash 0..1, w: swell (exploding mobs)
    light: vec4<f32>,
};

@group(1) @binding(0) var skins: texture_2d_array<f32>;
@group(1) @binding(1) var skin_samp: sampler;
@group(2) @binding(0) var<storage, read> inst: array<Bones>;

struct EOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) layer: u32,
    @location(2) shade: f32,
    @location(3) light: vec4<f32>,
    @location(4) dist: f32,
};

fn light_curve(l: f32) -> f32 {
    let x = 1.0 - l;
    return (1.0 - x) / (x * 3.0 + 1.0);
}

@vertex
fn vs_entity(
    @location(0) pos: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) packed: u32,
    @location(3) layer: u32,
    @builtin(instance_index) ii: u32,
) -> EOut {
    let part = packed & 15u;
    let shade = f32((packed >> 4u) & 255u) / 255.0;
    let b = inst[ii];
    let p = b.m[part] * vec4<f32>(pos, 1.0);
    var o: EOut;
    o.clip = g.view_proj * p;
    o.uv = uv;
    o.layer = layer;
    o.shade = shade;
    o.light = b.light;
    o.dist = length(p.xyz - g.cam_pos.xyz);
    return o;
}

@fragment
fn fs_entity(in: EOut) -> @location(0) vec4<f32> {
    let c = textureSampleLevel(skins, skin_samp, in.uv, in.layer, 0.0);
    if (c.a < 0.5) {
        discard;
    }
    let daylight = g.fog_color.w;
    let sky = light_curve(in.light.x) * daylight;
    let blk = light_curve(in.light.y);
    var l = max(vec3<f32>(sky), vec3<f32>(blk, blk * 0.93, blk * 0.82));
    l = max(mix(l, sqrt(l), 0.15), vec3<f32>(0.05));
    l = max(l, dim_ambient());
    var rgb = c.rgb * in.shade * l;
    rgb = mix(rgb, vec3<f32>(1.0, 0.25, 0.2), in.light.z * 0.6);
    rgb = mix(rgb, vec3<f32>(1.0), in.light.w);
    let fog = smoothstep(g.fog.x, g.fog.y, in.dist);
    return vec4<f32>(mix(rgb, g.fog_color.rgb, fog), 1.0);
}
