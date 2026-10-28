// Items lying on the ground (and experience orbs): little block cubes, or flat sprites with
// a thin edge. One instance per item; the 36 vertices come from the vertex index.

struct Item {
    // xyz: centre relative to the camera's block; w: size in blocks.
    pos: vec4<f32>,
    // x: yaw, y: 1 for a flat sprite, z: sky light 0..1, w: block light 0..1.
    info: vec4<f32>,
    // Texture layers: side, top, bottom; w: brightness boost (orbs glow).
    layers: vec4<f32>,
    // x: pitch, y: roll (a held item tilts with the camera), z: a block part's quarter turns,
    // w: 1 for a chest's parts (its inner faces are dark).
    rot: vec4<f32>,
    // A part of a block (a swinging door, a chest lid) when piv.w is 1: corners in the block's
    // 0..1 frame, and the pivot inside the block that pos stands for.
    bmin: vec4<f32>,
    bmax: vec4<f32>,
    piv: vec4<f32>,
};

@group(1) @binding(0) var itex: texture_2d_array<f32>;
@group(1) @binding(1) var isamp: sampler;
// Layers from 2048 on live in a second array (renderer::LAYERS_PER_ARRAY).
@group(1) @binding(2) var itex2: texture_2d_array<f32>;
@group(2) @binding(0) var<storage, read> items: array<Item>;
// The material class of each texture layer (material.rs), for the glints and relief.
@group(2) @binding(1) var<storage, read> layer_mat: array<u32>;

struct IOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) layer: i32,
    @location(2) shade: f32,
    @location(3) light: vec3<f32>,
    @location(4) dist: f32,
    // Position relative to the camera's block.
    @location(5) rel: vec3<f32>,
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
    let part = it.piv.w > 0.5;
    var local = (c - vec3<f32>(0.5)) * it.pos.w;
    // Where this corner sits in the block, for a block part's texture.
    let inb = mix(it.bmin.xyz, it.bmax.xyz, c);
    if (part) {
        local = inb - it.piv.xyz;
    }
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
        var wf = face;
        if (part) {
            // The piece of the texture under it, turned with the block like the terrain does
            // (rot.z quarter turns), so it lines up with the still block it replaces.
            var t = inb;
            var turn = array<u32, 6>(4u, 5u, 2u, 3u, 1u, 0u);
            for (var k = 0; k < i32(it.rot.z); k++) {
                t = vec3<f32>(1.0 - t.z, t.y, t.x);
                wf = turn[wf];
            }
            switch wf {
                case 0u: { o.uv = vec2<f32>(1.0 - t.z, 1.0 - t.y); }
                case 1u: { o.uv = vec2<f32>(t.z, 1.0 - t.y); }
                case 2u: { o.uv = t.xz; }
                case 3u: { o.uv = vec2<f32>(t.x, 1.0 - t.z); }
                case 4u: { o.uv = vec2<f32>(t.x, 1.0 - t.y); }
                default: { o.uv = vec2<f32>(1.0 - t.x, 1.0 - t.y); }
            }
        } else {
            switch face {
                case 0u, 1u: { o.uv = vec2<f32>(c.z, 1.0 - c.y); }
                case 4u, 5u: { o.uv = vec2<f32>(c.x, 1.0 - c.y); }
                default: { o.uv = c.xz; }
            }
        }
        if (wf == 2u) {
            o.layer = i32(it.layers.y);
        } else if (wf == 3u) {
            o.layer = i32(it.layers.z);
        } else if (part && wf == 4u && it.bmax.w > 0.0 && inb.z > 0.99) {
            // A chest's front (bmax.w is its layer).
            o.layer = i32(it.bmax.w);
        } else {
            o.layer = i32(it.layers.x);
        }
    }
    var shade = array<f32, 6>(0.6, 0.6, 1.0, 0.5, 0.8, 0.8);
    o.shade = shade[face] + it.layers.w;
    if (part && it.rot.w > 0.5) {
        // Inside an open chest: faces inside the box (not on its outside or rim) are in
        // shadow, deepest at the floor; the lid's underside is dim.
        var inner = false;
        switch face {
            case 0u, 1u: { inner = inb.x > 0.01 && inb.x < 0.99; }
            case 4u, 5u: { inner = inb.z > 0.01 && inb.z < 0.99; }
            case 2u: { inner = inb.y < 0.6; }
            default: { inner = inb.y > 0.05; }
        }
        if (inner) {
            if (face == 3u) {
                o.shade = 0.3;
            } else {
                let h = clamp((inb.y - 0.0625) / 0.5625, 0.0, 1.0);
                o.shade = mix(0.13, 0.5, h * h) * select(1.0, 0.85, face == 0u || face == 1u);
            }
        }
    }
    o.light = vec3<f32>(it.info.z, it.info.w, it.layers.w);
    o.dist = length(world - g.cam_pos.xyz);
    o.rel = world;
    return o;
}

fn ilight(l: f32) -> f32 {
    let x = 1.0 - l;
    return (1.0 - x) / (x * 3.0 + 1.0);
}

fn isample(uv: vec2<f32>, layer: i32) -> vec4<f32> {
    if (layer < 2048) {
        return textureSampleLevel(itex, isamp, uv, layer, 0.0);
    }
    return textureSampleLevel(itex2, isamp, uv, layer - 2048, 0.0);
}

@fragment
fn fs_item(in: IOut) -> @location(0) vec4<f32> {
    var c: vec4<f32>;
    if (in.layer < 2048) {
        c = textureSampleLevel(itex, isamp, in.uv, in.layer, 0.0);
    } else {
        c = textureSampleLevel(itex2, isamp, in.uv, in.layer - 2048, 0.0);
    }
    let fr = mat_frame(in.rel, in.uv);
    if (c.a < 0.3) {
        discard;
    }
    let sky = ilight(in.light.x) * g.fog_color.w;
    let blk = ilight(in.light.y);
    let l = max(max(vec3<f32>(sky), vec3<f32>(blk, blk * 0.93, blk * 0.82)), vec3<f32>(0.08 + in.light.z));
    var rgb = c.rgb * in.shade * l;
    let mat = select(0u, layer_mat[u32(max(in.layer, 0))], g.mat.x > 0.5 && in.light.z < 0.05);
    if (mat != 0u) {
        var k = 0.0;
        var hx = 0.0;
        var hy = 0.0;
        if (g.mat.x > 1.5 && in.dist < 14.0) {
            let ts = 1.0 / vec2<f32>(textureDimensions(itex, 0));
            let w = vec3<f32>(0.3, 0.55, 0.15);
            let h0 = dot(c.rgb, w);
            let cx = isample(in.uv + vec2<f32>(ts.x, 0.0), in.layer);
            let cy = isample(in.uv + vec2<f32>(0.0, ts.y), in.layer);
            hx = select(-0.3, dot(cx.rgb, w), cx.a > 0.3) - h0;
            hy = select(-0.3, dot(cy.rgb, w), cy.a > 0.3) - h0;
            k = mat_relief(mat) * 1.6 * (1.0 - smoothstep(8.0, 14.0, in.dist));
        }
        let v = normalize(g.cam_pos.xyz - in.rel);
        let n = mat_normal(fr, v, hx, hy, k);
        let tilt = clamp(dot(n - mat_normal(fr, v, 0.0, 0.0, 0.0), normalize(vec3<f32>(0.3, 1.0, 0.4) + v * 0.5)), -0.5, 0.5);
        rgb = rgb * (1.0 + tilt * 0.6);
        let hand = g.hand_light.w - length(in.rel - g.hand_light.xyz) / 15.0;
        let texel = floor(in.uv * vec2<f32>(textureDimensions(itex, 0)));
        rgb = mat_shade(mat, c.rgb, rgb, in.rel, n, in.light.x, in.light.y, 1.0, max(hand, 0.0), texel);
    }
    let fog = smoothstep(g.fog.x, g.fog.y, in.dist);
    return vec4<f32>(mix(rgb, g.fog_color.rgb, fog), 1.0);
}
