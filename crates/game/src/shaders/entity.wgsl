// Mobs and other box-model entities. Each instance has one matrix per body part in a storage buffer.

struct Bones {
    m: array<mat4x4<f32>, 16>,
    // x: sky light 0..1, y: block light 0..1, z: hurt flash 0..1, w: swell (exploding mobs)
    light: vec4<f32>,
    // A dye: rgb, and w = 0 none, 1 dyed, 2 dyed and glowing, 3 glowing only.
    tint: vec4<f32>,
    // Knees and elbows: joint height, joint depth, bend angle (model pixels, radians).
    bend: array<vec4<f32>, 16>,
};

@group(1) @binding(0) var skins: texture_2d_array<f32>;
@group(1) @binding(1) var skin_samp: sampler;
@group(2) @binding(0) var<storage, read> inst: array<Bones>;
// The material class of each skin layer (material.rs), for the glints and relief.
@group(2) @binding(1) var<storage, read> layer_mat: array<u32>;
// The sun's shadow map (scenefx.rs); a 1×1 dummy when shadows are off.
@group(3) @binding(0) var shadow_map: texture_depth_2d;
@group(3) @binding(1) var shadow_cmp: sampler_comparison;

struct EOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) layer: u32,
    @location(2) shade: f32,
    @location(3) light: vec4<f32>,
    @location(4) dist: f32,
    @location(5) tint: vec4<f32>,
    // Position relative to the camera's block.
    @location(6) rel: vec3<f32>,
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
    o.tint = b.tint;
    o.dist = length(p.xyz - g.cam_pos.xyz);
    o.rel = p.xyz;
    return o;
}

// 1 where the sun (or moon) reaches this point of the model, 0 in shadow (trees, roofs, the
// model's own arms and head).
fn entity_sun(rel: vec3<f32>, sky_light: f32) -> f32 {
    if (g.shadow.x <= 0.0 || sky_light <= 0.0) {
        return 1.0;
    }
    let s = g.sun_dir.xyz;
    let l = select(-s, s, s.y > -0.05);
    let d = length(rel - g.cam_pos.xyz);
    let far_fade = smoothstep(g.shadow.w * 0.75, g.shadow.w, d);
    // Lift the lookup toward the light so the model doesn't shadow its own surface.
    let c = g.shadow_mat * vec4<f32>(rel + l * 0.12, 1.0);
    let uv = vec2<f32>(c.x * 0.5 + 0.5, 0.5 - c.y * 0.5);
    if (far_fade >= 1.0 || any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || c.z > 1.0) {
        return 1.0;
    }
    let v = textureSampleCompareLevel(shadow_map, shadow_cmp, uv, c.z - 0.0005);
    let strength = smoothstep(0.02, 0.2, abs(l.y)) * select(0.25, 0.55, g.sun_dir.y > -0.05);
    return 1.0 - (1.0 - mix(v, 1.0, far_fade)) * strength;
}

@fragment
fn fs_entity(in: EOut) -> @location(0) vec4<f32> {
    let c = textureSample(skins, skin_samp, in.uv, in.layer);
    let fr = mat_frame(in.rel, in.uv);
    if (c.a < 0.5) {
        discard;
    }
    let daylight = g.fog_color.w;
    let sun_seen = entity_sun(in.rel, in.light.x);
    let sky = light_curve(in.light.x) * daylight * sun_seen;
    // The torch in the player's hand lights mobs (and the player) near it as well.
    let hand = g.hand_light.w - length(in.rel - g.hand_light.xyz) / 15.0;
    let blk = light_curve(max(in.light.y, hand));
    var l = max(vec3<f32>(sky), vec3<f32>(blk, blk * 0.93, blk * 0.82));
    l = max(mix(l, sqrt(l), 0.15), vec3<f32>(0.05));
    l = max(l, dim_ambient());
    var base = c.rgb;
    let mode = in.tint.w;
    if (mode > 0.5 && mode < 2.5) {
        // A dye keeps the picture's light and dark and takes on its own colour.
        let lum = dot(base, vec3<f32>(0.299, 0.587, 0.114));
        base = in.tint.rgb * (lum * 1.6 + 0.06);
    }
    if (mode > 1.5) {
        l = max(l, vec3<f32>(0.85));
    }
    var rgb = base * in.shade * l;
    let mat = select(0u, layer_mat[in.layer], g.mat.x > 0.5 && mode < 1.5);
    if (mat != 0u) {
        // Relief from the skin's own light and dark texels, fading out past a dozen blocks.
        var k = 0.0;
        var hx = 0.0;
        var hy = 0.0;
        if (g.mat.x > 1.5 && in.dist < 14.0) {
            let ts = 1.0 / vec2<f32>(textureDimensions(skins, 0));
            let w = vec3<f32>(0.3, 0.55, 0.15);
            let h0 = dot(c.rgb, w);
            let cx = textureSampleGrad(skins, skin_samp, in.uv + vec2<f32>(ts.x, 0.0), in.layer, fr.du1, fr.du2);
            let cy = textureSampleGrad(skins, skin_samp, in.uv + vec2<f32>(0.0, ts.y), in.layer, fr.du1, fr.du2);
            // A see-through neighbour (a plate's edge) counts as a step down.
            hx = select(-0.3, dot(cx.rgb, w), cx.a > 0.5) - h0;
            hy = select(-0.3, dot(cy.rgb, w), cy.a > 0.5) - h0;
            k = mat_relief(mat) * 1.6 * (1.0 - smoothstep(8.0, 14.0, in.dist));
        }
        let v = normalize(g.cam_pos.xyz - in.rel);
        let n = mat_normal(fr, v, hx, hy, k);
        // A gentle shading of the relief itself, for matte materials too.
        let tilt = clamp(dot(n - mat_normal(fr, v, 0.0, 0.0, 0.0), normalize(vec3<f32>(0.3, 1.0, 0.4) + v * 0.5)), -0.5, 0.5);
        rgb = rgb * (1.0 + tilt * 0.6);
        let texel = floor(in.uv * vec2<f32>(textureDimensions(skins, 0)));
        rgb = mat_shade(mat, base, rgb, in.rel, n, in.light.x, max(in.light.y, 0.0), sun_seen, max(hand, 0.0), texel);
    }
    rgb = mix(rgb, vec3<f32>(1.0, 0.25, 0.2), max(in.light.z, 0.0) * 0.6);
    // Negative z: soaked (a little darker and glossier blue). Negative w: frozen in ice.
    rgb = rgb * (1.0 - max(-in.light.z, 0.0) * vec3<f32>(0.28, 0.22, 0.12));
    rgb = mix(rgb, vec3<f32>(1.0), max(in.light.w, 0.0));
    let ice = max(-in.light.w, 0.0);
    let lum = dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
    rgb = mix(rgb, vec3<f32>(0.55, 0.8, 1.0) * (lum * 0.8 + 0.25 * l.b), ice * 0.75);
    let fog = smoothstep(g.fog.x, g.fog.y, in.dist);
    return vec4<f32>(mix(rgb, g.fog_color.rgb, fog), 1.0);
}

// ---- Shadow map pass: the model seen from the sun, depth only ----
struct EShadowOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) layer: u32,
};

@vertex
fn vs_entity_shadow(
    @location(0) pos: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) packed: u32,
    @location(3) layer: u32,
    @builtin(instance_index) ii: u32,
) -> EShadowOut {
    let p = inst[ii].m[packed & 15u] * vec4<f32>(pos, 1.0);
    var o: EShadowOut;
    o.clip = g.shadow_mat * p;
    o.uv = uv;
    o.layer = layer;
    return o;
}

@fragment
fn fs_entity_shadow(in: EShadowOut) {
    // See-through parts of a skin (hat layers, gaps) cast no shadow.
    if (textureSampleLevel(skins, skin_samp, in.uv, in.layer, 0.0).a < 0.5) {
        discard;
    }
}
