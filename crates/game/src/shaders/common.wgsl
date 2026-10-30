struct Globals {
    view_proj: mat4x4<f32>,
    // Rotation + projection only, inverted: turns a screen position into a view direction (sky).
    inv_view_rot: mat4x4<f32>,
    // xyz: eye position relative to the block the camera is in; w: time in seconds.
    cam_pos: vec4<f32>,
    // xyz: absolute eye position; w: cloud height.
    cam_world: vec4<f32>,
    // rgb: fog colour; w: daylight 0..1.
    fog_color: vec4<f32>,
    // x: fog start, y: fog end, z: 1 when under water, w: brightness setting.
    fog: vec4<f32>,
    sun_dir: vec4<f32>,
    sky_top: vec4<f32>,
    sky_horizon: vec4<f32>,
    // x: plant waving strength (1 = classic), y: rippling water 0/1, z: light source glow
    // (shader packs only; 0 = off).
    effects: vec4<f32>,
    // A torch in the hand: xyz relative to the camera's block, w: light level 0..1.
    hand_light: vec4<f32>,
    // x: rain 0..1, y: lightning flash 0..1, z: dimension (0 Overworld, 1 Nether, 2 End,
    // -1 Aether, 1.25 Otherside: the mod pack's dimensions), w: night vision 0..1.
    weather: vec4<f32>,
    // Moving lights: dropped torches, burning mobs. xyz like hand_light, w: level (0 = unused).
    dyn_lights: array<vec4<f32>, 8>,
    // Camera-block-relative position → shadow map clip space.
    shadow_mat: mat4x4<f32>,
    // x: shadow strength (0 = off), y: one shadow texel (clip units), z: soft 0/1, w: distance.
    shadow: vec4<f32>,
    // x: cloud style (0 one layer, 1 layered), y: emissive textures, z: reflection steps,
    // w: the year for the seasons (mods/seasons.rs), 0 when off.
    extra: vec4<f32>,
    // x, y: screen size; z: near plane; w: far plane.
    screen: vec4<f32>,
    // Block depth (parallax occlusion mapping): x depth in blocks (0 = off), y most steps,
    // z distance where it has faded out, w self-shadows 0/1.
    pom: vec4<f32>,
    // Hand light shadows: xyz corner of the solid-block grid relative to the camera's block,
    // w: 1 when on.
    occl: vec4<f32>,
    // xy: how far the wind has carried the clouds (speed 1), z: classic clouds' drift,
    // w: cloud clock (seconds, stops while paused).
    clouds: vec4<f32>,
    // Material feel of items and armour (material.rs): x 0 off, 1 glints, 2 also relief and
    // sparkles.
    mat: vec4<f32>,
    // Animated textures: layer + 1 of x fire, y soul fire, z lava (0 = none).
    anim: vec4<f32>,
    // Red glow of lit redstone: xyz like hand_light, w: glow 0..1 (0 = unused).
    red_lights: array<vec4<f32>, 8>,
};

@group(0) @binding(0) var<uniform> g: Globals;

// The Nether and the End are never pitch dark: a dim warm glow, or a pale violet one.
// Night vision (weather.w) lifts every dark corner the same way.
fn dim_ambient() -> vec3<f32> {
    // The Aether's undersides stay bright with the sky's light; the Otherside glows faintly teal.
    if (g.weather.z < -0.5) {
        return vec3<f32>(0.5, 0.56, 0.64);
    }
    if (g.weather.z > 1.1 && g.weather.z < 1.5) {
        return vec3<f32>(0.34, 0.42, 0.46);
    }
    if (g.weather.z > 1.5) {
        return max(vec3<f32>(0.62, 0.6, 0.68), vec3<f32>(g.weather.w));
    }
    return max(vec3<f32>(0.34, 0.24, 0.2) * g.weather.z, vec3<f32>(g.weather.w));
}

// Seasons (mods/seasons.rs): g.extra.w is 0 with seasons off, else 1 + the year (0..1 from
// the first day of spring). Foliage colour is multiplied by a key per part of the year,
// blended smoothly between parts; dry, yellow biomes (savanna, desert) turn less.
fn season_tint(biome: vec3<f32>) -> vec3<f32> {
    if (g.extra.w < 0.5) {
        return vec3<f32>(1.0);
    }
    var keys = array<vec3<f32>, 12>(
        vec3<f32>(0.92, 1.04, 0.9),  // early spring: fresh, light
        vec3<f32>(0.9, 1.08, 0.86),
        vec3<f32>(0.95, 1.04, 0.88),
        vec3<f32>(0.97, 1.0, 0.86),  // summer: deep green
        vec3<f32>(1.0, 0.98, 0.84),
        vec3<f32>(1.06, 0.97, 0.78),
        vec3<f32>(1.2, 0.95, 0.62),  // autumn: yellow, orange, red
        vec3<f32>(1.42, 0.84, 0.45),
        vec3<f32>(1.3, 0.72, 0.42),
        vec3<f32>(1.08, 0.82, 0.62), // winter: dull and brownish
        vec3<f32>(1.0, 0.84, 0.7),
        vec3<f32>(0.94, 0.9, 0.8)
    );
    let y = fract(g.extra.w - 1.0) * 12.0 - 0.5;
    let i = u32(floor(y + 12.0)) % 12u;
    let j = (i + 1u) % 12u;
    let f = smoothstep(0.0, 1.0, fract(y + 12.0));
    let k = mix(keys[i], keys[j], f);
    let dry = smoothstep(0.75, 0.6, biome.b / max(biome.r, 0.01));
    return mix(k, vec3<f32>(1.0), dry * 0.7);
}
