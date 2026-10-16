// Rain and snow: one camera-facing quad per block column around the player, from the first
// roof (or the ground) up to above the head. The streaks and flakes are drawn procedurally
// and scroll down with time; each column has its own pattern from its world position.

struct RainIn {
    @location(0) col: vec4<f32>,
    @location(1) info: vec4<f32>,
};

struct RainOut {
    @builtin(position) clip: vec4<f32>,
    // x: across the column 0..1, y: world height.
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) info: vec4<f32>,
    @location(2) dist: f32,
};

fn rhash(n: f32) -> f32 {
    return fract(sin(n * 12.9898) * 43758.5453);
}

@vertex
fn vs_rain(@builtin(vertex_index) vi: u32, r: RainIn) -> RainOut {
    var corner = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0),
    );
    let c = corner[vi];
    let center = vec2<f32>(r.col.x + 0.5, r.col.y + 0.5);
    // Turn the quad to face the camera, around the vertical axis.
    let to_cam = g.cam_pos.xz - center;
    let len = max(length(to_cam), 0.001);
    let side = vec2<f32>(-to_cam.y, to_cam.x) / len;
    let xz = center + side * (c.x - 0.5);
    let y = mix(r.col.z, r.col.w, c.y);
    let p = vec3<f32>(xz.x, y, xz.y);
    var out: RainOut;
    out.clip = g.view_proj * vec4<f32>(p, 1.0);
    // World height: the camera block's origin is cam_world - cam_pos.
    out.uv = vec2<f32>(c.x, y + g.cam_world.y - g.cam_pos.y);
    out.info = r.info;
    out.dist = len;
    return out;
}

@fragment
fn fs_rain(in: RainOut) -> @location(0) vec4<f32> {
    let t = g.cam_pos.w;
    let seed = in.info.z * 7.31 + in.info.w * 13.17;
    let snow = in.info.x > 0.5;
    var a = 0.0;
    for (var k = 0; k < 4; k = k + 1) {
        let h = seed + f32(k) * 3.7;
        let lane = 0.12 + rhash(h) * 0.76;
        if (snow) {
            // Flakes drift slowly and sway side to side.
            let sway = sin(t * 1.3 + h * 5.0 + in.uv.y * 0.6) * 0.08;
            let fall = in.uv.y * 0.5 + t * 0.55 * (0.8 + rhash(h + 1.0) * 0.4) + rhash(h + 2.0);
            let f = fract(fall);
            if (abs(in.uv.x - lane - sway) < 0.035 && f < 0.07) {
                a = 0.9;
            }
        } else {
            // Streaks fall fast; each lane at its own speed and phase.
            let fall = in.uv.y * 0.32 + t * 3.6 * (0.85 + rhash(h + 1.0) * 0.3) + rhash(h + 2.0);
            let f = fract(fall);
            if (abs(in.uv.x - lane) < 0.014 && f < 0.22) {
                a = max(a, 0.42 * (0.4 + f / 0.22 * 0.6));
            }
        }
    }
    // Fades toward the edge of the rain area, and with how hard it rains.
    a = a * (1.0 - smoothstep(6.0, 10.0, in.dist)) * g.weather.x;
    if (a < 0.02) {
        discard;
    }
    let day = g.fog_color.w;
    let light = max(in.info.y * day, 0.18) + g.weather.y * 0.6;
    var c = vec3<f32>(0.55, 0.62, 0.78);
    if (snow) {
        c = vec3<f32>(0.95, 0.96, 1.0);
    }
    return vec4<f32>(c * min(light, 1.2), a);
}
