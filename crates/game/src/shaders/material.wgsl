// Material feel for items and armour (material.rs, plan/materiais.md): a glint of the sun, the
// moon and torches, the sky mirrored in metals, sparkles in diamond and gems, and a relief taken
// from the texture itself. g.mat.x: 0 off, 1 glints only, 2 also relief and sparkles.

// Per class: specular strength, polish (Blinn-Phong exponent), metal 0..1, sparkle.
fn mat_look(id: u32) -> vec4<f32> {
    var t = array<vec4<f32>, 16>(
        vec4<f32>(0.0, 1.0, 0.0, 0.0),    // none
        vec4<f32>(0.9, 80.0, 0.15, 0.0),  // diamond (its fire is below, no glitter)
        vec4<f32>(0.7, 72.0, 0.1, 0.3),   // gem
        vec4<f32>(1.0, 30.0, 1.0, 0.12),  // gold
        vec4<f32>(0.95, 36.0, 1.0, 0.0),  // iron, silver
        vec4<f32>(0.6, 24.0, 1.0, 0.0),   // copper
        vec4<f32>(0.45, 20.0, 1.0, 0.0),  // netherite
        vec4<f32>(0.55, 22.0, 1.0, 0.0),  // chainmail
        vec4<f32>(0.1, 8.0, 0.0, 0.0),    // leather
        vec4<f32>(0.04, 4.0, 0.0, 0.0),   // cloth
        vec4<f32>(0.35, 28.0, 0.0, 0.0),  // lacquer
        vec4<f32>(0.06, 8.0, 0.0, 0.0),   // wood
        vec4<f32>(0.05, 6.0, 0.0, 0.0),   // stone
        vec4<f32>(0.15, 12.0, 0.0, 0.0),  // bone
        vec4<f32>(0.4, 26.0, 0.0, 0.0),   // shell
        vec4<f32>(0.6, 40.0, 0.3, 0.45)   // celestial
    );
    return t[min(id, 15u)];
}

// How strongly the texture's light and dark parts stand out as relief, per class.
fn mat_relief(id: u32) -> f32 {
    var r = array<f32, 16>(0.0, 0.9, 0.8, 1.4, 1.4, 1.2, 1.3, 1.4, 1.4, 1.2, 0.7, 1.4, 1.5, 1.0, 0.9, 0.8);
    return r[min(id, 15u)];
}

fn mat_curve(l: f32) -> f32 {
    let x = 1.0 - l;
    return (1.0 - x) / (x * 3.0 + 1.0);
}

fn mat_hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

// The surface derivatives a fragment needs, taken before any branching.
struct MatFrame {
    dp1: vec3<f32>,
    dp2: vec3<f32>,
    du1: vec2<f32>,
    du2: vec2<f32>,
};

fn mat_frame(rel: vec3<f32>, uv: vec2<f32>) -> MatFrame {
    var f: MatFrame;
    f.dp1 = dpdx(rel);
    f.dp2 = dpdy(rel);
    f.du1 = dpdx(uv);
    f.du2 = dpdy(uv);
    return f;
}

// The face normal toward the eye, tilted by the texture's relief. `hx`, `hy` are the heights
// (brightness) one texel along u and v minus the texel's own.
fn mat_normal(f: MatFrame, v: vec3<f32>, hx: f32, hy: f32, k: f32) -> vec3<f32> {
    var n = cross(f.dp1, f.dp2);
    if (dot(n, n) < 1e-20) {
        return v;
    }
    n = normalize(n);
    let facing = select(1.0, -1.0, dot(n, v) < 0.0);
    if (k <= 0.0) {
        return n * facing;
    }
    // Surface gradients of u and v (Mikkelsen's cotangent frame): their directions on the face.
    let p2 = cross(f.dp2, n);
    let p1 = cross(n, f.dp1);
    let det = dot(f.dp1, p2);
    if (abs(det) < 1e-20) {
        return n * facing;
    }
    var gu = (p2 * f.du1.x + p1 * f.du2.x) / det;
    var gv = (p2 * f.du1.y + p1 * f.du2.y) / det;
    gu = gu * inverseSqrt(max(dot(gu, gu), 1e-20));
    gv = gv * inverseSqrt(max(dot(gv, gv), 1e-20));
    // Bright texels stand out toward the eye, dark ones sink.
    return normalize(n * facing - (gu * hx + gv * hy) * k);
}

// `base`: the surface colour; `lit`: what the plain lighting gave; `sky`, `blk`: raw light
// levels 0..1; `sun_seen`: 1 where the sun reaches; `hand`: the held torch's light here;
// `texel`: the texel coordinate (sparkles stay on their texel).
fn mat_shade(id: u32, base: vec3<f32>, lit: vec3<f32>, rel: vec3<f32>, n: vec3<f32>, sky: f32, blk: f32, sun_seen: f32, hand: f32, texel: vec2<f32>) -> vec3<f32> {
    let look = mat_look(id);
    let v = normalize(g.cam_pos.xyz - rel);
    let daylight = g.fog_color.w;
    let nv = max(dot(n, v), 0.0);
    // Metals reflect in their own colour; gems and varnish in white, faintly tinted.
    let bl = max(base.r, max(base.g, base.b));
    let tint = base / max(bl, 0.05);
    let spec_col = mix(vec3<f32>(1.0), tint, look.z);
    let fres = mix(0.6 + 0.4 * pow(1.0 - nv, 4.0), 1.0, look.z);

    // The sun by day, the moon (fainter, bluer) by night, both only where the sky reaches.
    let s = g.sun_dir.xyz;
    let up = s.y > -0.05;
    let ld = select(-s, s, up);
    let sky_i = mat_curve(sky) * sun_seen * smoothstep(0.0, 0.2, ld.y);
    let sun_col = select(vec3<f32>(0.55, 0.62, 0.8) * 0.5, vec3<f32>(1.0, 0.96, 0.88), up) * mix(0.3, 1.0, daylight);
    var light = vec3<f32>(0.0);
    let h_sun = normalize(ld + v);
    light += sun_col * sky_i * pow(max(dot(n, h_sun), 0.0), look.y) * max(dot(n, ld), 0.0);
    // Torch light: from the torch in the hand and from moving lights (dropped torches, burning
    // mobs) at their places, and from above the eye for placed torches, lanterns, lava and fire
    // (block light has no direction), so metal and ore catch a glint from any light. A flame is
    // a broad light: its glint is wider than the sun's.
    let warm = vec3<f32>(1.0, 0.82, 0.56);
    let wide = max(look.y * 0.7, 6.0);
    if (hand > 0.0) {
        let lh = normalize(g.hand_light.xyz - rel);
        let hh = normalize(lh + v);
        light += warm * mat_curve(hand) * 1.3 * pow(max(dot(n, hh), 0.0), wide) * max(dot(n, lh), 0.0);
    }
    for (var i = 0; i < 8; i++) {
        let d = g.dyn_lights[i];
        let lv = d.w - length(rel - d.xyz) / 15.0;
        if (lv > 0.0) {
            let ld2 = normalize(d.xyz - rel);
            let hd = normalize(ld2 + v);
            light += warm * mat_curve(lv) * pow(max(dot(n, hd), 0.0), wide) * max(dot(n, ld2), 0.0);
        }
    }
    // Studio lights for polished surfaces, placed around the camera rather than in the world
    // (like a matcap): one up and to the left facing the eye, one up and beyond. They stand for
    // the open sky by day and for placed torches, lanterns, lava and fire at night (block light
    // has no direction). The mirrored ray changes across a face, so the highlight slides as you
    // move: that is what makes gold and silver read as shiny from any side.
    let f4 = g.inv_view_rot * vec4<f32>(0.0, 0.0, 1.0, 1.0);
    let fwd = normalize(f4.xyz / f4.w);
    let side = normalize(cross(fwd, vec3<f32>(0.0, 1.0, 0.0)) + vec3<f32>(1e-4, 0.0, 0.0));
    let upv = cross(side, fwd);
    let rr = reflect(-v, n);
    let rv = vec3<f32>(dot(rr, side), dot(rr, upv), -dot(rr, fwd));
    let k1 = pow(max(dot(rv, normalize(vec3<f32>(-0.35, 0.8, 0.5))), 0.0), look.y * 0.9) * 1.4;
    let k2 = pow(max(dot(rv, normalize(vec3<f32>(0.3, 0.7, -0.65))), 0.0), look.y * 0.9) * 0.8;
    let key_col = vec3<f32>(1.0, 0.98, 0.94) * mat_curve(sky) * daylight * 0.7 + warm * mat_curve(blk) * 0.9;
    light += key_col * (k1 + k2) * max(look.z, min(look.x, 0.9));
    var add = spec_col * light * look.x * fres * 1.6;

    // Metals and gems mirror the sky above them (or the dark ground below), and warm torch
    // light underground. A polished metal shows a bright band where the horizon is mirrored
    // and darker ground under it: that contrast across a face is what reads as shiny.
    let r = reflect(-v, n);
    let horizon_band = exp(-abs(r.y - 0.04) * 9.0);
    let sky_mirror = mix(g.sky_horizon.rgb * 0.16, mix(g.sky_horizon.rgb, g.sky_top.rgb, sqrt(clamp(r.y, 0.0, 1.0))), smoothstep(-0.12, 0.06, r.y))
        + g.sky_horizon.rgb * horizon_band * 0.55 * look.z;
    let env = sky_mirror * mat_curve(sky) * daylight + warm * mat_curve(max(blk, hand)) * (0.35 - 0.15 * look.z);
    let mirror = max(look.z, look.w * 0.6) * look.x;
    // Metal hardly scatters light: its colour comes from what it mirrors.
    let diffuse = lit * (1.0 - 0.6 * look.z);
    var shine = spec_col * env * mirror * mix(0.45, 0.95, look.z) * fres;
    // Keep the mirrored sky under the bloom threshold, so a whole face never glows.
    let env_lim = mix(0.5, 0.9, daylight);
    let dm = max(diffuse.r, max(diffuse.g, diffuse.b));
    let sm = max(shine.r, max(shine.g, shine.b));
    if (dm + sm > env_lim && sm > 0.0) {
        shine *= clamp((max(env_lim, dm) - dm) / sm, 0.0, 1.0);
    }
    var rgb = diffuse + shine;

    // Diamond fire: light split into rainbow bands across the facets (the texture's light and
    // dark areas), sliding through the colours as the view turns. A few broad bands, not dots.
    if (id == 1u && g.mat.x > 1.5) {
        let lum = dot(base, vec3<f32>(0.3, 0.55, 0.15));
        let facet = floor(lum * 3.0);
        let fd = normalize(vec3<f32>(mat_hash(vec2<f32>(facet, 1.3)) - 0.5, mat_hash(vec2<f32>(facet, 4.7)) - 0.5, mat_hash(vec2<f32>(facet, 8.9)) - 0.5));
        let phase = dot(r, fd) * 2.4 + facet * 0.21 + dot(rel, vec3<f32>(0.37, 0.29, 0.41));
        let hue = fract(phase);
        let spectrum = clamp(abs(fract(hue + vec3<f32>(0.0, 0.667, 0.333)) * 6.0 - 3.0) - 1.0, vec3<f32>(0.0), vec3<f32>(1.0));
        let band = pow(max(sin(phase * 6.2832), 0.0), 16.0);
        let around = max(max(sky_i * mix(0.3, 1.0, daylight), mat_curve(max(blk, hand)) * 0.9), 0.1);
        add += spectrum * band * around * 0.5;
    }

    // Sparkles: a few bright texels each light up as the view passes their angle.
    if (g.mat.x > 1.5 && look.w > 0.0) {
        let hsh = mat_hash(texel + vec2<f32>(f32(id) * 7.0, 3.0));
        let lum = dot(base, vec3<f32>(0.3, 0.55, 0.15));
        if (hsh > 1.0 - 0.1 * look.w && lum > 0.35) {
            let dir = normalize(vec3<f32>(mat_hash(texel + 1.7) - 0.5, mat_hash(texel + 5.3) - 0.5, mat_hash(texel + 9.1) - 0.5));
            let phase = dot(v, dir) * 22.0 + g.cam_pos.w * (0.8 + hsh);
            let tw = pow(max(sin(phase), 0.0), 28.0);
            let around = max(max(sky_i * mix(0.25, 1.0, daylight), mat_curve(max(blk, hand))), 0.12);
            add += mix(vec3<f32>(1.0), tint, 0.25) * tw * look.w * around * 0.8;
        }
    }

    // Keep glints from blowing up into white blobs under bloom: a highlight may rise only a
    // little above the bloom threshold (0.42 at night, 0.86 by day).
    let lim = max(max(rgb.r, max(rgb.g, rgb.b)) + 0.08, mix(0.62, 0.98, daylight));
    let top = max(rgb + add, vec3<f32>(0.0));
    let peak = max(top.r, max(top.g, top.b));
    if (peak > lim) {
        let room = max(lim - max(rgb.r, max(rgb.g, rgb.b)), 0.0);
        let am = max(add.r, max(add.g, add.b));
        add = add * clamp(room / max(am, 1e-4), 0.0, 1.0);
    }
    return rgb + add;
}
