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
    // w: the top of open storage's dark inside (0 for none; bmin.w is its floor).
    rot: vec4<f32>,
    // A part of a block (a swinging door, a chest lid) when piv.w is 1 (2: its side faces'
    // pictures mirrored, a left-hinged door; 3: a box inside a box, a drawer): corners in the
    // block's 0..1 frame, and the pivot inside the block that pos stands for. On a whole item,
    // bmin is the enchantment shimmer instead: rgb its colour, w its strength (0 = none).
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
// The sun's shadow map (scenefx), so a swinging door is shaded like the still block it replaces.
@group(3) @binding(0) var shadow_map: texture_depth_2d;
@group(3) @binding(1) var shadow_cmp: sampler_comparison;

struct IOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) layer: i32,
    @location(2) shade: f32,
    @location(3) light: vec3<f32>,
    @location(4) dist: f32,
    @location(5) @interpolate(flat) glint: vec4<f32>,
    // Position relative to the camera's block.
    @location(6) rel: vec3<f32>,
    // A block part's face turned the way it faces now (zero for anything else: no sun shadow).
    @location(7) @interpolate(flat) n: vec3<f32>,
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
            if (abs(it.piv.w - 2.0) < 0.5 && wf != 2u && wf != 3u) {
                o.uv.x = 1.0 - o.uv.x;
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
    o.n = vec3<f32>(0.0);
    if (part) {
        // The face's direction turned with the part, and the terrain's shade for that
        // direction, so a door swinging round darkens and brightens like still faces do.
        var nl = array<vec3<f32>, 6>(vec3(1.0, 0.0, 0.0), vec3(-1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0), vec3(0.0, -1.0, 0.0), vec3(0.0, 0.0, 1.0), vec3(0.0, 0.0, -1.0))[face];
        nl = vec3<f32>(nl.x * cr - nl.y * sr, nl.x * sr + nl.y * cr, nl.z);
        nl = vec3<f32>(nl.x, nl.y * cp - nl.z * sp, nl.y * sp + nl.z * cp);
        let n = vec3<f32>(nl.x * cy + nl.z * sy, nl.y, -nl.x * sy + nl.z * cy);
        o.n = n;
        let n2 = n * n;
        o.shade = n2.x * 0.6 + n2.z * 0.8 + n2.y * select(0.5, 1.0, n.y > 0.0) + it.layers.w;
    }
    if (part && it.rot.w > 0.05) {
        // Inside open storage: faces inside the box (not on its outside or rim) are in
        // shadow, deepest at the floor (bmin.w) and lighter towards the top (rot.w); the
        // underside of a lid is dim. A box inside a box (piv.w 3, a drawer) keeps its own
        // sides out of it.
        var inner = false;
        let edge = select(0.01, 0.07, it.piv.w > 2.5);
        switch face {
            case 0u, 1u: { inner = inb.x > edge && inb.x < 1.0 - edge; }
            case 4u, 5u: { inner = inb.z > 0.01 && inb.z < 0.99; }
            case 2u: { inner = inb.y < it.rot.w - 0.02; }
            default: { inner = inb.y > 0.05; }
        }
        if (inner) {
            if (face == 3u) {
                o.shade = 0.3;
            } else {
                let h = clamp((inb.y - it.bmin.w) / max(it.rot.w - it.bmin.w, 0.05), 0.0, 1.0);
                o.shade = mix(0.13, 0.5, h * h) * select(1.0, 0.85, face == 0u || face == 1u);
            }
        }
    }
    o.light = vec3<f32>(it.info.z, it.info.w, it.layers.w);
    o.dist = length(world - g.cam_pos.xyz);
    o.rel = world;
    o.glint = select(it.bmin, vec4<f32>(0.0), part);
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

// How much of the sky light reaches a block part's face: the terrain's sun shadows
// (terrain.wgsl sun_visibility and lit), faces turned away from the sun in their own shadow.
// 1 for everything that is not a block part.
fn part_sun(in: IOut) -> f32 {
    if (g.shadow.x <= 0.0 || in.light.x <= 0.0 || dot(in.n, in.n) < 0.5) {
        return 1.0;
    }
    let s = g.sun_dir.xyz;
    let l = select(-s, s, s.y > -0.05);
    let ndl = dot(in.n, l);
    var v = smoothstep(0.0, 0.2, ndl);
    let d = length(in.rel - g.cam_pos.xyz);
    let far_fade = smoothstep(g.shadow.w * 0.75, g.shadow.w, d);
    if (v > 0.0 && far_fade < 1.0) {
        let p = in.rel + in.n * (0.03 + 0.06 * (1.0 - ndl)) * (1.0 + d * 0.02);
        let c = g.shadow_mat * vec4<f32>(p, 1.0);
        let uv = vec2<f32>(c.x * 0.5 + 0.5, 0.5 - c.y * 0.5);
        if (all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0)) && c.z <= 1.0) {
            let m = textureSampleCompareLevel(shadow_map, shadow_cmp, uv, c.z - 0.0005);
            v = min(v, mix(m, 1.0, far_fade));
        }
    }
    let strength = g.shadow.x * smoothstep(0.02, 0.2, abs(l.y)) * select(0.25, 0.55, g.sun_dir.y > -0.05);
    return 1.0 - (1.0 - v) * strength;
}

// The lit colour of a texel `c` of an item: light, materials (glints and relief) and the
// enchantment shimmer, then fog. `fr` is taken before any discard (it needs derivatives).
// `wall`: an extruded item's side wall, whose picture is one pixel stretched (no relief or
// glints there: they would sparkle).
fn ishade(c: vec4<f32>, fr: MatFrame, in: IOut, wall: bool) -> vec4<f32> {
    let sky = ilight(in.light.x) * g.fog_color.w * part_sun(in);
    let blk = ilight(in.light.y);
    let l = max(max(vec3<f32>(sky), vec3<f32>(blk, blk * 0.93, blk * 0.82)), vec3<f32>(0.08 + in.light.z));
    var rgb = c.rgb * in.shade * l;
    let mat = select(0u, layer_mat[u32(max(in.layer, 0))], g.mat.x > 0.5 && in.light.z < 0.05 && !wall);
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
    if (in.glint.w > 0.0) {
        // Enchanted: a slow band of colour sweeping across, over a faint steady tint. Kept
        // well under white so bloom in shader packs only softens it.
        let band = pow(0.5 + 0.5 * sin((in.uv.x + in.uv.y) * 5.0 - g.cam_pos.w * 2.2), 6.0);
        rgb += in.glint.rgb * in.glint.w * (0.18 + 0.55 * band);
    }
    let fog = smoothstep(g.fog.x, g.fog.y, in.dist);
    return vec4<f32>(mix(rgb, g.fog_color.rgb, fog), 1.0);
}

@fragment
fn fs_item(in: IOut) -> @location(0) vec4<f32> {
    let c = isample(in.uv, in.layer);
    let fr = mat_frame(in.rel, in.uv);
    if (c.a < 0.3) {
        discard;
    }
    return ishade(c, fr, in, false);
}

// A flat picture held in a hand, extruded like the classic item models: the picture on its
// front and back, 1/16 of its width apart, and a side wall along every pixel edge where an
// opaque pixel meets a clear one. Quads 0 and 1 are the faces; then one plane between every
// two columns, then one between every two rows. Each plane is drawn whole and the fragment
// keeps only the stretches where the two pixels beside it differ.

struct EOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) layer: i32,
    // x: 0 face, 1 plane between columns, 2 plane between rows; y: which column or row.
    @location(2) @interpolate(flat) edge: vec2<f32>,
    // Shading facing +axis and -axis (a face uses x).
    @location(3) @interpolate(flat) shade: vec2<f32>,
    @location(4) light: vec3<f32>,
    @location(5) dist: f32,
    @location(6) @interpolate(flat) glint: vec4<f32>,
    @location(7) rel: vec3<f32>,
};

fn iturn(it: Item, v: vec3<f32>) -> vec3<f32> {
    let cr = cos(it.rot.y);
    let sr = sin(it.rot.y);
    var l = vec3<f32>(v.x * cr - v.y * sr, v.x * sr + v.y * cr, v.z);
    let cp = cos(it.rot.x);
    let sp = sin(it.rot.x);
    l = vec3<f32>(l.x, l.y * cp - l.z * sp, l.y * sp + l.z * cp);
    let cy = cos(it.info.x);
    let sy = sin(it.info.x);
    return vec3<f32>(l.x * cy + l.z * sy, l.y, -l.x * sy + l.z * cy);
}

// The terrain's face shading (top 1, north/south 0.8, east/west 0.6, bottom 0.5), for any
// direction, so a turning item is lit like the blocks around it.
fn dir_shade(n: vec3<f32>) -> f32 {
    return n.x * n.x * 0.6 + n.z * n.z * 0.8 + n.y * n.y * select(0.5, 1.0, n.y > 0.0);
}

@vertex
fn vs_extrude(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> EOut {
    let it = items[ii];
    // The item texture's width (item_render.rs puts it here for extruded items).
    let res = u32(it.rot.w);
    let q = vi / 6u;
    var sq = array<vec2<f32>, 6>(vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 1.0));
    let s = sq[vi % 6u];
    let h = 1.0 / 32.0;
    let r = f32(res);
    var c: vec3<f32>;
    var n: vec3<f32>;
    var o: EOut;
    if (q < 2u) {
        let z = select(-h, h, q == 0u);
        c = vec3<f32>(s.x, s.y, 0.5 + z);
        n = vec3<f32>(0.0, 0.0, select(-1.0, 1.0, q == 0u));
        o.edge = vec2<f32>(0.0, 0.0);
    } else if (q < 3u + res) {
        let i = f32(q - 2u);
        c = vec3<f32>(i / r, s.x, 0.5 + mix(-h, h, s.y));
        n = vec3<f32>(1.0, 0.0, 0.0);
        o.edge = vec2<f32>(1.0, i);
    } else {
        let j = f32(q - 3u - res);
        c = vec3<f32>(s.x, j / r, 0.5 + mix(-h, h, s.y));
        n = vec3<f32>(0.0, 1.0, 0.0);
        o.edge = vec2<f32>(2.0, j);
    }
    let world = it.pos.xyz + iturn(it, (c - vec3<f32>(0.5)) * it.pos.w);
    let wn = normalize(iturn(it, n));
    o.clip = g.view_proj * vec4<f32>(world, 1.0);
    o.uv = vec2<f32>(c.x, 1.0 - c.y);
    o.layer = i32(it.layers.x);
    o.shade = vec2<f32>(dir_shade(wn), dir_shade(-wn)) + vec2<f32>(it.layers.w);
    o.light = vec3<f32>(it.info.z, it.info.w, it.layers.w);
    o.dist = length(world - g.cam_pos.xyz);
    o.glint = it.bmin;
    o.rel = world;
    return o;
}

// As an item seen whole (fs_item's lighting and materials) with this fragment's shading.
fn eshade(c: vec4<f32>, fr: MatFrame, e: EOut, shade: f32) -> vec4<f32> {
    var i: IOut;
    i.clip = e.clip;
    i.uv = e.uv;
    i.layer = e.layer;
    i.shade = shade;
    i.light = e.light;
    i.dist = e.dist;
    i.glint = e.glint;
    i.rel = e.rel;
    i.n = vec3<f32>(0.0);
    return ishade(c, fr, i, e.edge.x > 0.5);
}

@fragment
fn fs_extrude(in: EOut) -> @location(0) vec4<f32> {
    let r = f32(textureDimensions(itex).x);
    let fr = mat_frame(in.rel, in.uv);
    if (in.edge.x < 0.5) {
        let c = isample(in.uv, in.layer);
        if (c.a < 0.3) {
            discard;
        }
        return eshade(c, fr, in, in.shade.x);
    }
    // The pixels on the low side (left / below) and the high side of this plane.
    let k = in.edge.y;
    var lo_uv: vec2<f32>;
    var hi_uv: vec2<f32>;
    if (in.edge.x < 1.5) {
        let v = (floor(in.uv.y * r) + 0.5) / r;
        lo_uv = vec2<f32>((k - 0.5) / r, v);
        hi_uv = vec2<f32>((k + 0.5) / r, v);
    } else {
        let u = (floor(in.uv.x * r) + 0.5) / r;
        lo_uv = vec2<f32>(u, 1.0 - (k - 0.5) / r);
        hi_uv = vec2<f32>(u, 1.0 - (k + 0.5) / r);
    }
    var lo = vec4<f32>(0.0);
    var hi = vec4<f32>(0.0);
    if (k > 0.5) {
        lo = isample(lo_uv, in.layer);
    }
    if (k < r - 0.5) {
        hi = isample(hi_uv, in.layer);
    }
    let lo_on = lo.a >= 0.3;
    let hi_on = hi.a >= 0.3;
    if (lo_on == hi_on) {
        discard;
    }
    // The wall belongs to the opaque pixel and faces away from it.
    if (lo_on) {
        return eshade(lo, fr, in, in.shade.x);
    }
    return eshade(hi, fr, in, in.shade.y);
}

// The soft glow around items on the ground: a round sprite facing the camera, added on top
// of the scene. pos: centre and radius; layers: rgb colour, w strength; info.z: sky light
// there (the glow shows less in daylight).
struct HOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) q: vec2<f32>,
    @location(1) @interpolate(flat) col: vec4<f32>,
    @location(2) dist: f32,
};

@vertex
fn vs_halo(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> HOut {
    let it = items[ii];
    var q = array<vec2<f32>, 6>(vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(1.0, 1.0), vec2(-1.0, -1.0), vec2(1.0, 1.0), vec2(-1.0, 1.0));
    let k = q[vi];
    let to_cam = g.cam_pos.xyz - it.pos.xyz;
    let d = normalize(to_cam);
    var right = cross(vec3<f32>(0.0, 1.0, 0.0), d);
    if (dot(right, right) < 0.0001) {
        right = vec3<f32>(1.0, 0.0, 0.0);
    }
    right = normalize(right);
    let up = cross(d, right);
    let r = it.pos.w;
    let world = it.pos.xyz + (right * k.x + up * k.y) * r;
    let clip = g.view_proj * vec4<f32>(world, 1.0);
    // Depth-tested as if a radius nearer the camera, so the ground the item lies on doesn't
    // cut the glow in a hard line.
    let near = g.view_proj * vec4<f32>(world + d * min(r, length(to_cam) * 0.5), 1.0);
    var o: HOut;
    o.clip = vec4<f32>(clip.xy / clip.w * near.w, near.z, near.w);
    o.q = k;
    let day = it.info.z * g.fog_color.w;
    o.col = vec4<f32>(it.layers.rgb, it.layers.w * mix(1.0, 0.4, day));
    o.dist = length(world - g.cam_pos.xyz);
    return o;
}

@fragment
fn fs_halo(in: HOut) -> @location(0) vec4<f32> {
    let r2 = dot(in.q, in.q);
    if (r2 >= 1.0) {
        discard;
    }
    let f = (1.0 - r2) * (1.0 - r2);
    let fog = 1.0 - smoothstep(g.fog.x, g.fog.y, in.dist);
    return vec4<f32>(in.col.rgb, in.col.w * f * fog);
}
