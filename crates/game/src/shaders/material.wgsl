// Material feel for items and armour (material.rs, plan/materiais.md): a glint of the sun, the
// moon and torches, the sky mirrored in metals, sparkles in diamond and gems, and a relief taken
// from the texture itself. g.mat.x: 0 off, 1 glints only, 2 also relief and sparkles.

// Per class: specular strength, polish (Blinn-Phong exponent), metal 0..1, sparkle.
fn mat_look(id: u32) -> vec4<f32> {
    var t = array<vec4<f32>, 16>(
        vec4<f32>(0.0, 1.0, 0.0, 0.0),    // none
        vec4<f32>(0.9, 96.0, 0.15, 1.0),  // diamond
        vec4<f32>(0.7, 72.0, 0.1, 0.55),  // gem
        vec4<f32>(0.85, 44.0, 1.0, 0.12), // gold
        vec4<f32>(0.7, 32.0, 1.0, 0.0),   // iron
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
    var r = array<f32, 16>(0.0, 0.9, 0.8, 0.9, 0.9, 0.9, 1.3, 1.4, 1.4, 1.2, 0.7, 1.4, 1.5, 1.0, 0.9, 0.8);
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
    // Torch light: from the torch in the hand, else from above the eye, so a lit item still
    // catches a glint in a cave or at night.
    let warm = vec3<f32>(1.0, 0.82, 0.56);
    if (hand > 0.0) {
        let lh = normalize(g.hand_light.xyz - rel);
        let hh = normalize(lh + v);
        light += warm * mat_curve(hand) * pow(max(dot(n, hh), 0.0), look.y) * max(dot(n, lh), 0.0);
    }
    let lb = normalize(v + vec3<f32>(0.0, 0.9, 0.0));
    let hb = normalize(lb + v);
    light += warm * mat_curve(blk) * 0.7 * pow(max(dot(n, hb), 0.0), look.y) * max(dot(n, lb), 0.0);
    var add = spec_col * light * look.x * fres * 1.6;

    // Metals and gems mirror the sky above them (or the dark ground below), and warm torch
    // light underground.
    let r = reflect(-v, n);
    let sky_mirror = select(g.sky_horizon.rgb * 0.25, mix(g.sky_horizon.rgb, g.sky_top.rgb, clamp(r.y, 0.0, 1.0)), r.y > 0.0);
    let env = sky_mirror * mat_curve(sky) * daylight + warm * mat_curve(max(blk, hand)) * 0.35;
    let mirror = max(look.z, look.w * 0.6) * look.x;
    var rgb = lit * (1.0 - 0.35 * look.z) + spec_col * env * mirror * 0.45 * fres;

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
