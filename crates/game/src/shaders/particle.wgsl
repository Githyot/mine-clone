// Particles: tiny cubes cut from block textures (bits of a broken block, drops of water).
// Each instance is one cube; its 36 vertices come from the vertex index, no vertex buffer.

struct Particle {
    // xyz: centre relative to the camera's block; w: edge length in blocks.
    pos: vec4<f32>,
    // xy: corner of the texture piece (0..1); z: texture layer; w: sky light 0..1.
    uv: vec4<f32>,
    // rgb: tint; w: block light 0..1.
    tint: vec4<f32>,
};

@group(1) @binding(0) var ptex: texture_2d_array<f32>;
@group(1) @binding(1) var psamp: sampler;
@group(2) @binding(0) var<storage, read> parts: array<Particle>;

struct POut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) layer: i32,
    @location(2) tint: vec3<f32>,
    @location(3) light: vec2<f32>,
    @location(4) dist: f32,
};

fn corner(face: u32, k: u32) -> vec3<f32> {
    // Two triangles per face: corners 0,1,2 and 0,2,3 of a unit square.
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
fn vs_particle(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> POut {
    let p = parts[ii];
    let face = vi / 6u;
    let k = vi % 6u;
    let c = corner(face, k);
    let world = p.pos.xyz + (c - vec3<f32>(0.5)) * p.pos.w;
    var q = array<vec2<f32>, 6>(vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 1.0));
    var shade = array<f32, 6>(0.6, 0.6, 1.0, 0.5, 0.8, 0.8);
    var o: POut;
    o.clip = g.view_proj * vec4<f32>(world, 1.0);
    // A 4×4 pixel piece of the texture.
    o.uv = p.uv.xy + q[k] * 0.25;
    o.layer = i32(p.uv.z);
    o.tint = p.tint.rgb * shade[face];
    o.light = vec2<f32>(p.uv.w, p.tint.w);
    o.dist = length(world - g.cam_pos.xyz);
    return o;
}

fn plight(l: f32) -> f32 {
    let x = 1.0 - l;
    return (1.0 - x) / (x * 3.0 + 1.0);
}

@fragment
fn fs_particle(in: POut) -> @location(0) vec4<f32> {
    let c = textureSampleLevel(ptex, psamp, in.uv, in.layer, 0.0);
    if (c.a < 0.3) {
        discard;
    }
    let sky = plight(in.light.x) * g.fog_color.w;
    let blk = plight(in.light.y);
    let l = max(max(vec3<f32>(sky), vec3<f32>(blk, blk * 0.93, blk * 0.82)), vec3<f32>(0.06));
    let rgb = c.rgb * in.tint * l;
    let fog = smoothstep(g.fog.x, g.fog.y, in.dist);
    return vec4<f32>(mix(rgb, g.fog_color.rgb, fog), 1.0);
}
