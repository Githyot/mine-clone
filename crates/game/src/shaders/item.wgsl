// Items lying on the ground (and experience orbs): little block cubes, or flat sprites with
// a thin edge. One instance per item; the 36 vertices come from the vertex index.

struct Item {
    // xyz: centre relative to the camera's block; w: size in blocks.
    pos: vec4<f32>,
    // x: yaw, y: 1 for a flat sprite, z: sky light 0..1, w: block light 0..1.
    info: vec4<f32>,
    // Texture layers: side, top, bottom; w: brightness boost (orbs glow).
    layers: vec4<f32>,
    // x: pitch, y: roll (a held item tilts with the camera), zw unused.
    rot: vec4<f32>,
};

@group(1) @binding(0) var itex: texture_2d_array<f32>;
@group(1) @binding(1) var isamp: sampler;
@group(2) @binding(0) var<storage, read> items: array<Item>;

struct IOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) layer: i32,
    @location(2) shade: f32,
    @location(3) light: vec3<f32>,
    @location(4) dist: f32,
};

fn icorner(face: u32, k: u32) -> vec3<f32> {
    var q = array<vec2<f32>, 6>(vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 1.0));
    let s = q[k];
    switch face {
        case 0u: { return vec3(1.0, s.x, s.y); }
        case 1u: { return vec3(0.0, s.y, s.x); }
        case 2u: { return vec3(s.y, 1.0, s.x); }
        case 3u: { return vec3(s.x, 0.0, s.y); }
        case 4u: { return vec3(s.y, s.x, 1.0); }
        default: { return vec3(s.x, s.y, 0.0); }
    }
}

@vertex
fn vs_item(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> IOut {
    let it = items[ii];
    let face = vi / 6u;
    let c = icorner(face, vi % 6u);
    let flat = it.info.y > 0.5;
    var local = (c - vec3<f32>(0.5)) * it.pos.w;
    if (flat) {
        local.z = local.z / 16.0;
    }
    // Roll around the item's facing, then pitch, then yaw.
    let cr = cos(it.rot.y);
    let sr = sin(it.rot.y);
    local = vec3<f32>(local.x * cr - local.y * sr, local.x * sr + local.y * cr, local.z);
    let cp = cos(it.rot.x);
    let sp = sin(it.rot.x);
    local = vec3<f32>(local.x, local.y * cp - local.z * sp, local.y * sp + local.z * cp);
    let cy = cos(it.info.x);
    let sy = sin(it.info.x);
    let rotated = vec3<f32>(local.x * cy + local.z * sy, local.y, -local.x * sy + local.z * cy);
    let world = it.pos.xyz + rotated;
    var o: IOut;
    o.clip = g.view_proj * vec4<f32>(world, 1.0);
    if (flat) {
        // Every face samples the sprite at its own position: front and back show the picture,
        // the edges show its outline pixels, like an extruded item.
        o.uv = vec2<f32>(c.x, 1.0 - c.y);
        o.layer = i32(it.layers.x);
    } else {
        switch face {
            case 0u, 1u: { o.uv = vec2<f32>(c.z, 1.0 - c.y); }
            case 4u, 5u: { o.uv = vec2<f32>(c.x, 1.0 - c.y); }
            default: { o.uv = c.xz; }
        }
        if (face == 2u) {
            o.layer = i32(it.layers.y);
        } else if (face == 3u) {
            o.layer = i32(it.layers.z);
        } else {
            o.layer = i32(it.layers.x);
        }
    }
    var shade = array<f32, 6>(0.6, 0.6, 1.0, 0.5, 0.8, 0.8);
    o.shade = shade[face] + it.layers.w;
    o.light = vec3<f32>(it.info.z, it.info.w, it.layers.w);
    o.dist = length(world - g.cam_pos.xyz);
    return o;
}

fn ilight(l: f32) -> f32 {
    let x = 1.0 - l;
    return (1.0 - x) / (x * 3.0 + 1.0);
}

@fragment
fn fs_item(in: IOut) -> @location(0) vec4<f32> {
    let c = textureSampleLevel(itex, isamp, in.uv, in.layer, 0.0);
    if (c.a < 0.3) {
        discard;
    }
    let sky = ilight(in.light.x) * g.fog_color.w;
    let blk = ilight(in.light.y);
    let l = max(max(vec3<f32>(sky), vec3<f32>(blk, blk * 0.93, blk * 0.82)), vec3<f32>(0.08 + in.light.z));
    let rgb = c.rgb * in.shade * l;
    let fog = smoothstep(g.fog.x, g.fog.y, in.dist);
    return vec4<f32>(mix(rgb, g.fog_color.rgb, fog), 1.0);
}
