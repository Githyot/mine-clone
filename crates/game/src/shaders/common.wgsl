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
};

@group(0) @binding(0) var<uniform> g: Globals;
