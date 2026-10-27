@group(1) @binding(0) var tex: texture_2d_array<f32>;
@group(1) @binding(1) var samp: sampler;
// Sun shadows and water reflections (Sildur's idea). Dummies when they are off; the scene copy
// and its depth are only real while the translucent geometry is drawn.
@group(2) @binding(0) var shadow_map: texture_depth_2d;
@group(2) @binding(1) var shadow_cmp: sampler_comparison;
@group(2) @binding(2) var scene_copy: texture_2d<f32>;
@group(2) @binding(3) var scene_depth: texture_depth_2d;

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
    let layer = (w.y >> 10u) & 2047u;
    let emissive = (w.y >> 21u) & 1u;
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
    if (((w.z >> 8u) & 1u) == 1u) {
        // Foliage (odd red tint): it turns with the seasons.
        out.color *= season_tint(tint);
    }
    out.light = vec2<f32>(f32(w.z & 15u), f32((w.z >> 4u) & 15u)) / 15.0;
    out.dist = length(p - g.cam_pos.xyz);
    out.kind = flags | (face << 8u) | (emissive << 12u);
    out.rel = p;
    return out;
}

// Outward normal of a block face (mesh face order +x, -x, +y, -y, +z, -z).
fn face_normal(face: u32) -> vec3<f32> {
    var n = array<vec3<f32>, 6>(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(-1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(0.0, -1.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(0.0, 0.0, -1.0));
    return n[min(face, 5u)];
}

// Direction toward the light that casts shadows: the sun by day, the moon by night.
fn shadow_light() -> vec3<f32> {
    let s = g.sun_dir.xyz;
    return select(-s, s, s.y > -0.05);
}

// 1 where the sun (or moon) reaches this point, 0 in shadow. Faces turned away from the light
// are in their own shadow; plants and leaves are lit through.
fn sun_visibility(in: VOut) -> f32 {
    if (g.shadow.x <= 0.0 || in.light.x <= 0.0) {
        return 1.0;
    }
    let l = shadow_light();
    let plant = (in.kind & 1u) != 0u;
    var n = face_normal((in.kind >> 8u) & 7u);
    if (plant) {
        n = l;
    }
    let ndl = dot(n, l);
    if (ndl <= 0.0) {
        return 0.0;
    }
    let d = length(in.rel - g.cam_pos.xyz);
    let far_fade = smoothstep(g.shadow.w * 0.75, g.shadow.w, d);
    if (far_fade >= 1.0) {
        return 1.0;
    }
    // Push the lookup off the surface a little (more at grazing light) to avoid acne.
    let p = in.rel + n * (0.03 + 0.06 * (1.0 - ndl)) * (1.0 + d * 0.02);
    let c = g.shadow_mat * vec4<f32>(p, 1.0);
    let uv = vec2<f32>(c.x * 0.5 + 0.5, 0.5 - c.y * 0.5);
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || c.z > 1.0) {
        return 1.0;
    }
    let z = c.z - 0.0005;
    var v = textureSampleCompareLevel(shadow_map, shadow_cmp, uv, z);
    if (g.shadow.z > 0.5) {
        // Soft: four more filtered taps around the point.
        let o = g.shadow.y * 1.2;
        v += textureSampleCompareLevel(shadow_map, shadow_cmp, uv + vec2<f32>(o, o * 0.4), z);
        v += textureSampleCompareLevel(shadow_map, shadow_cmp, uv + vec2<f32>(-o * 0.4, o), z);
        v += textureSampleCompareLevel(shadow_map, shadow_cmp, uv + vec2<f32>(-o, -o * 0.4), z);
        v += textureSampleCompareLevel(shadow_map, shadow_cmp, uv + vec2<f32>(o * 0.4, -o), z);
        v *= 0.2;
    }
    return mix(v, 1.0, far_fade);
}

fn lit(c: vec3<f32>, in: VOut) -> vec3<f32> {
    let daylight = g.fog_color.w;
    var sky = light_curve(in.light.x) * daylight;
    if (g.shadow.x > 0.0) {
        // Shadows take away the direct light only: low light (dawn, dusk) and the moon cast
        // weaker ones, and the open sky still lights shadowed ground.
        let l = shadow_light();
        let strength = g.shadow.x * smoothstep(0.02, 0.2, abs(l.y)) * select(0.25, 0.55, g.sun_dir.y > -0.05);
        sky *= 1.0 - (1.0 - sun_visibility(in)) * strength;
    }
    // A light in the player's hand fades one level per block, like a placed torch.
    var hand = g.hand_light.w - length(in.rel - g.hand_light.xyz) / 15.0;
    for (var i = 0; i < 8; i++) {
        let d = g.dyn_lights[i];
        hand = max(hand, d.w - length(in.rel - d.xyz) / 15.0);
    }
    let blk = light_curve(max(in.light.y, hand));
    // Torches are a little warm, sky light is neutral.
    var l = max(vec3<f32>(sky), vec3<f32>(blk, blk * 0.93, blk * 0.82));
    let b = g.fog.w;
    l = mix(l, sqrt(l), b * 0.5);
    l = max(l, vec3<f32>(0.03 + b * 0.04));
    // The Nether is never pitch dark: a dim warm glow everywhere.
    l = max(l, dim_ambient());
    var rgb = c * in.color * l;
    if (g.effects.z > 0.0) {
        // Shader packs draw in HDR: bright texels right next to a light (torch flames,
        // glowstone, lava) go past 1.0 so the bloom pass makes them glow.
        let hot = smoothstep(0.8, 0.95, in.light.y) * smoothstep(0.5, 0.85, max(c.r, max(c.g, c.b)));
        rgb *= 1.0 + hot * g.effects.z;
    }
    return rgb;
}

// Emissive textures (Continuity idea): the bright, coloured pixels of an ore stay lit in the dark.
fn glow_pixels(c: vec3<f32>, rgb: vec3<f32>, in: VOut) -> vec3<f32> {
    if (((in.kind >> 12u) & 1u) == 0u || g.extra.y < 0.5) {
        return rgb;
    }
    let hi = max(c.r, max(c.g, c.b));
    let lo = min(c.r, min(c.g, c.b));
    let glow = smoothstep(0.2, 0.32, hi - lo) * smoothstep(0.3, 0.5, hi);
    let full = c * in.color * (0.9 + g.effects.z * 0.35);
    return mix(rgb, max(rgb, full), glow);
}

fn fog_amount(in: VOut) -> f32 {
    return smoothstep(g.fog.x, g.fog.y, in.dist);
}

fn shade(c: vec3<f32>, in: VOut) -> vec3<f32> {
    return mix(glow_pixels(c, lit(c, in), in), g.fog_color.rgb, fog_amount(in));
}

// Pixel relief: each texel is treated as a little bump as high as it is bright, lit from
// the top-left, so stone, bark and bricks look carved instead of painted. Two extra texture
// reads, only near the camera. `texels` is how many texels one screen pixel covers: further
// away the bumps are read from a smaller mip level, so 64x textures don't shimmer.
fn relief(c: vec4<f32>, uv: vec2<f32>, in: VOut, texels: f32) -> f32 {
    let strength = g.sun_dir.w * (1.0 - smoothstep(16.0, 32.0, in.dist));
    if (strength <= 0.0) {
        return 1.0;
    }
    let full = textureDimensions(tex).x;
    // The columns are at most 32 to a block: finer art keeps its colours but shares heights in
    // 2x2 groups, which halves the walk on 64x textures (packs are mostly 16x anyway).
    let coarsest = u32(max(log2(f32(full) / 16.0), 0.0));
    let level = max(u32(clamp(floor(log2(max(texels, 1.0))), 0.0, 3.0)), coarsest);
    let size = f32(max(full >> level, 1u));
    let lv = f32(level);
    let px = fract(uv) * size;
    let cell = (floor(px) + 0.5) / size;
    let here = textureSampleLevel(tex, samp, cell, in.layer, lv);
    let up = textureSampleLevel(tex, samp, cell - vec2<f32>(0.0, 1.0 / size), in.layer, lv);
    let left = textureSampleLevel(tex, samp, cell - vec2<f32>(1.0 / size, 0.0), in.layer, lv);
    let lum = vec3<f32>(0.3, 0.59, 0.11);
    let h = select(dot(here.rgb, lum), dot(c.rgb, lum), level == 0u);
    // Texels at the picture's edge have no neighbour there: leave them flat.
    let hu = select(dot(up.rgb, lum), h, floor(px.y) < 0.5 || up.a < 0.5);
    let hl = select(dot(left.rgb, lum), h, floor(px.x) < 0.5 || left.a < 0.5);
    // Finer textures have smaller steps between texels: scale the slope so the relief keeps
    // the same depth at 16x and 64x.
    let k = 1.1 * clamp(size / 16.0, 1.0, 4.0) * 0.5 + 0.55;
    let slope = (h - hu) + (h - hl);
    return clamp(1.0 + slope * k * strength, 0.72, 1.22);
}

// Connected textures: on a side that continues into the same block (glass), the outer
// texel row is swapped for the one just inside, so the frame line disappears.
fn ctm_sample(in: VOut) -> vec4<f32> {
    let m = (in.kind >> 3u) & 15u;
    let dx = dpdx(in.uv);
    let dy = dpdy(in.uv);
    var uv = in.uv;
    if (m != 0u) {
        let e = 1.0 / 16.0;
        let f = fract(in.uv);
        if ((m & 1u) != 0u && f.x < e) { uv.x += e; }
        if ((m & 2u) != 0u && f.x >= 1.0 - e) { uv.x -= e; }
        if ((m & 4u) != 0u && f.y < e) { uv.y += e; }
        if ((m & 8u) != 0u && f.y >= 1.0 - e) { uv.y -= e; }
    }
    return textureSampleGrad(tex, samp, uv, in.layer, dx, dy);
}

const LUM = vec3<f32>(0.3, 0.59, 0.11);

// Block depth (parallax occlusion mapping, as in shader packs). The height of each texel comes
// from the texture itself: texels darker than the texture's average sink into the face
// (mortar, cracks, the gaps between cobbles), brighter ones stay on top. Textures are pixel art,
// so the face is treated as a field of little square columns, one per texel: the view ray is
// walked texel by texel into the face (no layers, no smearing) until it meets a column's top or
// side. Sides are darker, and a short walk toward the sun darkens the bottoms of crevices.
// Only on plain block faces near the camera (`g.pom`).
struct Parallax {
    uv: vec2<f32>,
    // Multiplies the colour: 1 lit, lower on a column's side or in a crevice the sun can't reach.
    light: f32,
    // How much of the full depth is used here (0 = none: too far, or not a plain face).
    weight: f32,
};

// Brightness and colourfulness of a texel: what its height is read from.
fn pom_tone(c: vec3<f32>) -> vec2<f32> {
    let hi = max(c.r, max(c.g, c.b));
    let lo = min(c.r, min(c.g, c.b));
    return vec2<f32>(dot(c, LUM), (hi - lo) / max(hi, 0.2));
}

// How far below the face a texel lies, 0 (top) to 1 (full depth). Darker than the texture's
// average sinks (cracks, the gaps between cobbles); so does greyer than average, which puts
// pale mortar below red bricks and leaves moss and ore bits standing out.
fn pom_depth(cell: vec2<f32>, size: f32, layer: u32, lv: f32, avg: vec2<f32>) -> f32 {
    let k = avg - pom_tone(textureSampleLevel(tex, samp, (cell + 0.5) / size, layer, lv).rgb);
    return clamp(k.x * 2.4 + k.y * 2.2 + 0.12, 0.0, 1.0);
}

// `t` and `b`: how far the point moves (blocks, relative to the camera) per unit of u and v.
fn parallax(in: VOut, t: vec3<f32>, b: vec3<f32>, texels: f32) -> Parallax {
    var out = Parallax(in.uv, 1.0, 0.0);
    let weight = 1.0 - smoothstep(g.pom.z * 0.6, g.pom.z, in.dist);
    let depth = g.pom.x * weight;
    // Not on plants, liquids or connected glass.
    if (depth <= 0.002 || (in.kind & 0x7bu) != 0u) {
        return out;
    }
    let n = face_normal((in.kind >> 8u) & 7u);
    let tt = t / max(dot(t, t), 1e-8);
    let bb = b / max(dot(b, b), 1e-8);
    let e = g.cam_pos.xyz - in.rel;
    let ez = dot(e, n);
    if (ez <= 1e-4 || abs(dot(t, n)) + abs(dot(b, n)) > 0.05) {
        return out;
    }
    // Texture shift per block of depth going into the face (kept sane at grazing angles).
    var ray = vec2<f32>(dot(e, tt), dot(e, bb)) / ez;
    ray *= min(1.0, 4.0 / max(length(ray), 1e-4));
    let full = textureDimensions(tex).x;
    // The columns are at most 32 to a block: finer art keeps its colours but shares heights in
    // 2x2 groups, which halves the walk on 64x textures (packs are mostly 16x anyway).
    let coarsest = u32(max(log2(f32(full) / 16.0), 0.0));
    let level = max(u32(clamp(floor(log2(max(texels, 1.0))), 0.0, 3.0)), coarsest);
    let lv = f32(level);
    let size = f32(max(full >> level, 1u));
    let avg = pom_tone(textureSampleLevel(tex, samp, in.uv, in.layer, f32(textureNumLevels(tex) - 1u)).rgb);
    // In texels: start point and movement over the whole depth (d = 0 → 1).
    let p0 = in.uv * size;
    let dir = -ray * depth * size;
    var cell = floor(p0);
    let stp = sign(dir);
    let inv = 1.0 / max(abs(dir), vec2<f32>(1e-6));
    // Depth at which the ray leaves the current column through an x or a y side.
    var next = (select(p0 - cell, cell + 1.0 - p0, dir > vec2<f32>(0.0))) * inv;
    var d_in = 0.0;
    var side = 0;
    var hit_d = 1.0;
    var h = pom_depth(cell, size, in.layer, lv, avg);
    let most = i32(g.pom.y);
    for (var i = 0; i < 48; i = i + 1) {
        if (h <= d_in) {
            // The ray met this column's side as it came in.
            hit_d = d_in;
            break;
        }
        let d_out = min(min(next.x, next.y), 1.0);
        if (h <= d_out || i >= most || d_out >= 1.0) {
            // It comes down on this column's top.
            hit_d = min(h, d_out);
            side = 0;
            break;
        }
        if (next.x < next.y) {
            cell.x += stp.x;
            next.x += inv.x;
            side = 1;
        } else {
            cell.y += stp.y;
            next.y += inv.y;
            side = 2;
        }
        d_in = d_out;
        h = pom_depth(cell, size, in.layer, lv, avg);
    }
    // Stay inside the column that was hit, so the colour comes from it and not a neighbour.
    let p = clamp(p0 + dir * hit_d, cell + 0.002, cell + 0.998);
    out.uv = p / size;
    out.weight = weight;
    // Deeper is darker: less of the sky reaches the bottom of a crack.
    out.light = 1.0 - hit_d * 0.3 * weight;
    let l = shadow_light();
    let sun = g.fog_color.w * in.light.x * smoothstep(0.02, 0.2, abs(l.y));
    if (side != 0 && hit_d > 0.0) {
        // A column's side: it faces back toward where the ray came from.
        let sn = select(-b * stp.y, -t * stp.x, side == 1);
        let facing = max(dot(normalize(sn), l), 0.0);
        out.light *= 0.8 + 0.2 * facing * sun;
    }
    if (g.pom.w > 0.5 && hit_d > 0.03 && sun > 0.0) {
        let lz = dot(l, n);
        if (lz > 0.05) {
            var lray = vec2<f32>(dot(l, tt), dot(l, bb)) / lz;
            lray *= min(1.0, 4.0 / max(length(lray), 1e-4));
            // Two looks toward the light: a third and two thirds of the way up.
            let up = lray * depth * size * hit_d;
            let o1 = hit_d * 0.67 - pom_depth(floor(p + up * 0.33), size, in.layer, lv, avg);
            let o2 = hit_d * 0.33 - pom_depth(floor(p + up * 0.67), size, in.layer, lv, avg);
            let occ = max(o1, o2) * 8.0;
            out.light *= 1.0 - clamp(occ, 0.0, 1.0) * 0.4 * sun;
        }
    }
    return out;
}

@fragment
fn fs_solid(in: VOut) -> @location(0) vec4<f32> {
    let fw = fwidth(in.uv) * f32(textureDimensions(tex).x);
    let texels = max(fw.x, fw.y);
    // The face's texture directions in the world, from how position and uv change across the
    // screen (works for every face and model without storing tangents in the mesh).
    let dx = dpdx(in.uv);
    let dy = dpdy(in.uv);
    let px = dpdx(in.rel);
    let py = dpdy(in.rel);
    let det = dx.x * dy.y - dx.y * dy.x;
    var c = ctm_sample(in);
    if (c.a < 0.5) {
        discard;
    }
    var p = Parallax(in.uv, 1.0, 0.0);
    if (g.pom.x > 0.0 && abs(det) > 1e-12) {
        let t = (px * dy.y - py * dx.y) / det;
        let b = (py * dx.x - px * dy.x) / det;
        p = parallax(in, t, b, texels);
        if (p.weight > 0.0) {
            let moved = textureSampleGrad(tex, samp, p.uv, in.layer, dx, dy);
            // A cut-out texture (torch, ladder) keeps its outline: never shift onto a hole.
            if (moved.a >= 0.5) {
                c = moved;
            } else {
                p = Parallax(in.uv, 1.0, 0.0);
            }
        }
    }
    // Close up the depth already shows the bumps: the flat relief only takes over as it fades.
    var r = 1.0;
    if (p.weight < 1.0) {
        r = mix(relief(c, p.uv, in, texels), 1.0, p.weight);
    }
    return vec4<f32>(shade(c.rgb * r * p.light, in), 1.0);
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

// Sky seen in a direction (for reflections that leave the screen).
fn sky_toward(r: vec3<f32>) -> vec3<f32> {
    return mix(g.sky_horizon.rgb, g.sky_top.rgb, smoothstep(-0.02, 0.45, r.y));
}

fn linear_depth(d: f32) -> f32 {
    let n = g.screen.z;
    let f = g.screen.w;
    return n * f / max(f - d * (f - n), 1e-6);
}

// Screen-space reflection: walk the reflected ray across the drawn scene until it passes behind
// something. rgb + how much was found (0 = use the sky).
fn screen_reflection(start: vec3<f32>, r: vec3<f32>) -> vec4<f32> {
    let steps = i32(g.extra.z);
    let size = g.screen.xy;
    var p = start;
    var dt = 0.35;
    var prev = start;
    for (var i = 0; i < steps; i = i + 1) {
        prev = p;
        p += r * dt;
        dt *= 1.28;
        let c = g.view_proj * vec4<f32>(p, 1.0);
        if (c.w <= 0.05) {
            break;
        }
        let ndc = c.xy / c.w;
        if (abs(ndc.x) > 1.0 || abs(ndc.y) > 1.0) {
            break;
        }
        let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
        let d = textureLoad(scene_depth, vec2<i32>(clamp(uv * size, vec2<f32>(0.0), size - 1.0)), 0);
        if (d >= 1.0) {
            continue;
        }
        let scene_z = linear_depth(d);
        let ray_z = c.w;
        if (ray_z > scene_z + 0.02 && ray_z < scene_z + dt * 2.5 + 0.6) {
            // Refine between the last two points.
            var a = prev;
            var b = p;
            for (var k = 0; k < 4; k = k + 1) {
                let m = (a + b) * 0.5;
                let cm = g.view_proj * vec4<f32>(m, 1.0);
                let um = vec2<f32>(cm.x / cm.w * 0.5 + 0.5, 0.5 - cm.y / cm.w * 0.5);
                let dm = textureLoad(scene_depth, vec2<i32>(clamp(um * size, vec2<f32>(0.0), size - 1.0)), 0);
                if (cm.w > linear_depth(dm)) {
                    b = m;
                } else {
                    a = m;
                }
            }
            let cb = g.view_proj * vec4<f32>(b, 1.0);
            let ub = clamp(vec2<f32>(cb.x / cb.w * 0.5 + 0.5, 0.5 - cb.y / cb.w * 0.5), vec2<f32>(0.0), vec2<f32>(1.0));
            let col = textureLoad(scene_copy, vec2<i32>(clamp(ub * size, vec2<f32>(0.0), size - 1.0)), 0).rgb;
            // Fade toward the screen's edges, where the picture runs out.
            let e = min(min(ub.x, 1.0 - ub.x), min(ub.y, 1.0 - ub.y));
            return vec4<f32>(col, smoothstep(0.0, 0.08, e));
        }
    }
    return vec4<f32>(0.0);
}

@fragment
fn fs_translucent(in: VOut) -> @location(0) vec4<f32> {
    let c = ctm_sample(in);
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
        let r = reflect(v, n);
        if (g.extra.z > 0.5) {
            // Reflections: the world mirrored where the screen shows it, the sky elsewhere.
            let k = 0.1 + fres * 0.75;
            var refl = sky_toward(r) * max(sky, 0.15);
            let ss = screen_reflection(in.rel, normalize(vec3<f32>(r.x, abs(r.y), r.z)));
            refl = mix(refl, ss.rgb, ss.a);
            rgb = mix(rgb, refl, k);
            a = mix(a, 1.0, k * 0.7);
        } else {
            rgb = mix(rgb, g.sky_horizon.rgb * sky, fres * 0.45);
        }
        let spec = pow(max(dot(r, g.sun_dir.xyz), 0.0), 120.0) * g.fog_color.w * sky;
        rgb += vec3<f32>(1.0, 0.92, 0.75) * spec * 1.6;
        a = mix(a, 1.0, fres * 0.5 + min(spec, 1.0) * 0.5);
    }
    return vec4<f32>(mix(rgb, g.fog_color.rgb, fog), mix(a, 1.0, fog));
}

// ---- Shadow map pass: the terrain seen from the sun, depth only ----
struct ShadowOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) layer: u32,
};

@vertex
fn vs_shadow(@location(0) w: vec3<u32>, @location(1) origin: vec3<f32>) -> ShadowOut {
    let p = vec3<f32>(f32(w.x & 1023u), f32((w.x >> 10u) & 1023u), f32((w.x >> 20u) & 1023u)) / 16.0 + origin;
    var out: ShadowOut;
    out.clip = g.shadow_mat * vec4<f32>(p, 1.0);
    out.uv = vec2<f32>(f32(w.y & 31u), f32((w.y >> 5u) & 31u)) / 16.0;
    if (((w.y >> 25u) & 4u) != 0u) {
        // Merged quads are always full blocks of opaque texture: any uv will do.
        out.uv = vec2<f32>(0.5, 0.5);
    }
    out.layer = (w.y >> 10u) & 2047u;
    return out;
}

@fragment
fn fs_shadow(in: ShadowOut) {
    // Leaves and plants cast the shape of their texture.
    if (textureSampleLevel(tex, samp, in.uv, in.layer, 0.0).a < 0.5) {
        discard;
    }
}
