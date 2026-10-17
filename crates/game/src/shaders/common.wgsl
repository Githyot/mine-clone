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
    // x: rain 0..1, y: lightning flash 0..1, z: dimension (0 Overworld, 1 Nether, 2 End),
    // w: night vision 0..1.
    weather: vec4<f32>,
    // Moving lights: dropped torches, burning mobs. xyz like hand_light, w: level (0 = unused).
    dyn_lights: array<vec4<f32>, 8>,
};

@group(0) @binding(0) var<uniform> g: Globals;

// The Nether and the End are never pitch dark: a dim warm glow, or a pale violet one.
// Night vision (weather.w) lifts every dark corner the same way.
fn dim_ambient() -> vec3<f32> {
    if (g.weather.z > 1.5) {
        return max(vec3<f32>(0.62, 0.6, 0.68), vec3<f32>(g.weather.w));
    }
    return max(vec3<f32>(0.34, 0.24, 0.2) * g.weather.z, vec3<f32>(g.weather.w));
}
