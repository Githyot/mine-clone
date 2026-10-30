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
    // piv.w 4: a strip of cloth (a banner): the picture spans the box's width, and from
    // bmin.w to bmax.w of its height (a strip of a taller picture), on every face of layers.x.
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
    @location(5) @interpolate(flat) glint: vec4<f32>,
    // Position relative to the camera's block.
    @location(6) rel: vec3<f32>,
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
    let cloth = it.piv.w > 3.5;
    if (cloth) {
        // Front (+z) as drawn, the back mirrored like the other side of a cloth; the thin edges
        // take the picture's edge pixels.
        let u = clamp(select(c.x, 1.0 - c.x, face == 5u), 0.002, 0.998);
        o.uv = vec2<f32>(u, mix(it.bmin.w, it.bmax.w, clamp(1.0 - c.y, 0.002, 0.998)));
        o.layer = i32(it.layers.x);
    } else if (flat) {
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
    if (part && !cloth && it.rot.w > 0.05) {
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

// The lit colour of a texel `c` of an item: light, materials (glints and relief) and the
// enchantment shimmer, then fog. `fr` is taken before any discard (it needs derivatives).
// `wall`: an extruded item's side wall, whose picture is one pixel stretched (no relief or
// glints there: they would sparkle).
fn ishade(c: vec4<f32>, fr: MatFrame, in: IOut, wall: bool) -> vec4<f32> {
    let sky = ilight(in.light.x) * g.fog_color.w;
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

// The inverse of iturn: a vector in the world back into the item's own frame.
fn iturn_inv(it: Item, v: vec3<f32>) -> vec3<f32> {
    let cy = cos(it.info.x);
    let sy = sin(it.info.x);
    var l = vec3<f32>(v.x * cy - v.z * sy, v.y, v.x * sy + v.z * cy);
    let cp = cos(it.rot.x);
    let sp = sin(it.rot.x);
    l = vec3<f32>(l.x, l.y * cp + l.z * sp, -l.y * sp + l.z * cp);
    let cr = cos(it.rot.y);
    let sr = sin(it.rot.y);
    return vec3<f32>(l.x * cr + l.y * sr, -l.x * sr + l.y * cr, l.z);
}

// A picture with real 3D volume (HP 2026-10-29): handles round, heads thick, blades thin at
// their edges, instead of a picture extruded into a two-pixel slab. item_render::volumes gives
// every cell of a 32x32 grid over the picture a half-thickness; here a box around the picture
// is drawn and each pixel walks its ray through that grid (a 2D DDA) to the first cell whose
// slab it enters, writing that point's depth.

@group(2) @binding(2) var<storage, read> vols: array<u32>;

const VOL_G: f32 = 32.0;
// Half the box's depth in item units: a bit more than the thickest cell (item_render::VOL_MAX).
const VOL_BOX: f32 = 1.85 / 32.0;

struct VOut {
    @builtin(position) clip: vec4<f32>,
    // The box's surface in the item's own 0..1 frame (z: 0.5 is the picture's middle plane).
    @location(0) loc: vec3<f32>,
    @location(1) @interpolate(flat) ii: u32,
};

struct VFrag {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

// How thick this item is drawn: rot.w, 0 meaning a tool's full volume.
fn vthick(it: Item) -> f32 {
    return select(it.rot.w, 1.0, it.rot.w <= 0.0);
}

// Half-thickness of cell (i, j) of a layer, in item units (0 = empty).
fn vh(layer: i32, i: i32, j: i32) -> f32 {
    if (i < 0 || j < 0 || i >= 32 || j >= 32 || layer < 0) {
        return 0.0;
    }
    let k = u32(layer) * 1024u + u32(j * 32 + i);
    if (k / 4u >= arrayLength(&vols)) {
        return 0.0;
    }
    return f32((vols[k / 4u] >> ((k % 4u) * 8u)) & 255u) / (64.0 * VOL_G);
}

@vertex
fn vs_vol(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> VOut {
    let it = items[ii];
    let k = icorner(vi / 6u, vi % 6u);
    let b = VOL_BOX * vthick(it) + 0.002;
    let c = vec3<f32>(k.x, k.y, 0.5 + (k.z * 2.0 - 1.0) * b);
    let world = it.pos.xyz + iturn(it, (c - vec3<f32>(0.5)) * it.pos.w);
    var o: VOut;
    o.clip = g.view_proj * vec4<f32>(world, 1.0);
    o.loc = c;
    o.ii = ii;
    return o;
}

@fragment
fn fs_vol(in: VOut) -> VFrag {
    let it = items[in.ii];
    let s = it.pos.w;
    let th = vthick(it);
    let layer = i32(it.layers.x);
    let cam = vec3<f32>(0.5) + iturn_inv(it, g.cam_pos.xyz - it.pos.xyz) / s;
    let rd = in.loc - cam;
    let tf = length(rd);
    let d = rd / tf;
    // The ray through the box: only the fragment where it leaves the box works (one per pixel,
    // and it still works with the eye inside the box).
    let b = VOL_BOX * th + 0.002;
    let inv = vec3<f32>(select(1e30, 1.0 / d.x, abs(d.x) > 1e-7), select(1e30, 1.0 / d.y, abs(d.y) > 1e-7), select(1e30, 1.0 / d.z, abs(d.z) > 1e-7));
    let ta = (vec3<f32>(0.0, 0.0, 0.5 - b) - cam) * inv;
    let tb = (vec3<f32>(1.0, 1.0, 0.5 + b) - cam) * inv;
    let tn = min(ta, tb);
    let tx = max(ta, tb);
    let tmin = max(max(tn.x, tn.y), tn.z);
    let tmax = min(min(tx.x, tx.y), tx.z);
    if (tf < tmax - 1e-4 || tmax <= tmin) {
        discard;
    }
    let t0 = max(tmin, 0.0);
    let p = cam + d * t0;
    // Grid coordinates: columns along x, rows down the picture (-y).
    let gx = p.x * VOL_G;
    let gy = (1.0 - p.y) * VOL_G;
    let ddx = d.x * VOL_G;
    let ddy = -d.y * VOL_G;
    var i = clamp(i32(floor(gx)), 0, 31);
    var j = clamp(i32(floor(gy)), 0, 31);
    let sx = select(-1, 1, ddx > 0.0);
    let sy = select(-1, 1, ddy > 0.0);
    let tdx = select(1e30, 1.0 / abs(ddx), abs(ddx) > 1e-7);
    let tdy = select(1e30, 1.0 / abs(ddy), abs(ddy) > 1e-7);
    var tmx = t0 + select(1e30, select((gx - f32(i)) / -ddx, (f32(i + 1) - gx) / ddx, ddx > 0.0), abs(ddx) > 1e-7);
    var tmy = t0 + select(1e30, select((gy - f32(j)) / -ddy, (f32(j + 1) - gy) / ddy, ddy > 0.0), abs(ddy) > 1e-7);
    // Which side of the cell the ray came in by: 0 across x, 1 across y, 2 the box's face.
    var axis = 2;
    if (tmin > 0.0) {
        if (tn.x >= tn.y && tn.x >= tn.z) {
            axis = 0;
        } else if (tn.y >= tn.z) {
            axis = 1;
        }
    }
    var t_in = t0;
    var hit = false;
    var wall = false;
    var th_hit = 0.0;
    var nz = 0.0;
    for (var k = 0; k < 72; k++) {
        let t_out = min(min(tmx, tmy), tmax);
        let h = vh(layer, i, j) * th;
        if (h > 0.0) {
            let z0 = cam.z + d.z * t_in - 0.5;
            if (abs(z0) <= h && axis != 2) {
                hit = true;
                wall = true;
                th_hit = t_in;
                break;
            }
            if (abs(d.z) > 1e-7) {
                let side = select(-1.0, 1.0, z0 > 0.0);
                let tz = (0.5 + side * h - cam.z) / d.z;
                if (tz >= t_in - 1e-6 && tz <= t_out) {
                    hit = true;
                    th_hit = max(tz, t_in);
                    nz = side;
                    break;
                }
            }
        }
        if (t_out >= tmax) {
            break;
        }
        if (tmx < tmy) {
            i += sx;
            t_in = tmx;
            tmx += tdx;
            axis = 0;
        } else {
            j += sy;
            t_in = tmy;
            tmy += tdy;
            axis = 1;
        }
        if (i < 0 || j < 0 || i > 31 || j > 31) {
            break;
        }
    }
    if (!hit) {
        discard;
    }
    let q = cam + d * th_hit;
    let world = it.pos.xyz + iturn(it, (q - vec3<f32>(0.5)) * s);
    let clip = g.view_proj * vec4<f32>(world, 1.0);
    let cell = vec2<f32>(f32(i), f32(j));
    let centre = (cell + vec2<f32>(0.5)) / VOL_G;
    var uv = centre;
    var n: vec3<f32>;
    // Tangents along the picture's u and v (for the materials' glints), tilted with the surface.
    var tu = vec3<f32>(1.0, 0.0, 0.0);
    var tv = vec3<f32>(0.0, -1.0, 0.0);
    if (wall) {
        if (axis == 0) {
            n = vec3<f32>(f32(-sx), 0.0, 0.0);
        } else {
            n = vec3<f32>(0.0, f32(sy), 0.0);
        }
    } else {
        uv = clamp(vec2<f32>(q.x, 1.0 - q.y), (cell + vec2<f32>(0.03)) / VOL_G, (cell + vec2<f32>(0.97)) / VOL_G);
        // The surface's slope from the cells around: rounded handles, ridged blades.
        let hx = (vh(layer, i + 1, j) - vh(layer, i - 1, j)) * th * VOL_G * 0.5 * 0.8;
        let hy = (vh(layer, i, j - 1) - vh(layer, i, j + 1)) * th * VOL_G * 0.5 * 0.8;
        n = normalize(vec3<f32>(-hx, -hy, nz));
        tu = vec3<f32>(1.0, 0.0, hx * nz);
        tv = vec3<f32>(0.0, -1.0, -hy * nz);
    }
    var c = isample(uv, layer);
    if (c.a < 0.3) {
        c = isample(centre, layer);
    }
    if (wall) {
        // A picture's rim cells are its dark outline: seen from the side the item would be all
        // outline. The wall shows the colour one cell further in (the handle's wood, the
        // head's metal) unless the item is only one cell thick there.
        var ii = i;
        var jj = j;
        if (axis == 0) {
            ii = i + sx;
        } else {
            jj = j + sy;
        }
        if (vh(layer, ii, jj) > 0.0) {
            let inner = isample((vec2<f32>(f32(ii), f32(jj)) + vec2<f32>(0.5)) / VOL_G, layer);
            if (inner.a >= 0.3) {
                c = vec4<f32>(mix(inner.rgb, c.rgb, 0.25), inner.a);
            }
        }
    }
    var fr: MatFrame;
    fr.dp1 = iturn(it, tu) * (s / VOL_G);
    fr.dp2 = iturn(it, tv) * (s / VOL_G);
    fr.du1 = vec2<f32>(1.0 / VOL_G, 0.0);
    fr.du2 = vec2<f32>(0.0, 1.0 / VOL_G);
    let wn = normalize(iturn(it, n));
    var io: IOut;
    io.clip = clip;
    io.uv = uv;
    io.layer = layer;
    io.shade = dir_shade(wn) + it.layers.w;
    io.light = vec3<f32>(it.info.z, it.info.w, it.layers.w);
    io.dist = length(world - g.cam_pos.xyz);
    io.glint = it.bmin;
    io.rel = world;
    var o: VFrag;
    o.color = ishade(c, fr, io, wall);
    o.depth = clip.z / clip.w;
    return o;
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
