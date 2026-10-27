// Mobs and other box-model entities. Each instance has one matrix per body part in a storage buffer.

struct Bones {
    m: array<mat4x4<f32>, 16>,
    // x: sky light 0..1, y: block light 0..1, z: hurt flash 0..1, w: swell (exploding mobs)
    light: vec4<f32>,
    // Knees and elbows: joint height, joint depth, bend angle (model pixels, radians).
    bend: array<vec4<f32>, 16>,
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
    var q = pos;
    let j = b.bend[part];
    if (j.z != 0.0) {
        // Below the joint the limb turns with the lower half; a pixel either side blends.
        let a = j.z * clamp(0.5 - (q.y - j.x) * 0.5, 0.0, 1.0);
        let c = cos(a);
        let s = sin(a);
        let dy = q.y - j.x;
        let dz = q.z - j.y;
        q = vec3<f32>(q.x, j.x + dy * c - dz * s, j.y + dy * s + dz * c);
    }
    let p = b.m[part] * vec4<f32>(q, 1.0);
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
    let c = textureSample(skins, skin_samp, in.uv, in.layer);
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
