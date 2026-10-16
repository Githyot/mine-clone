struct SkyOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) ndc: vec2<f32>,
};

@vertex
fn vs_sky(@builtin(vertex_index) i: u32) -> SkyOut {
    var pts = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    var o: SkyOut;
    o.clip = vec4<f32>(pts[i], 0.0, 1.0);
    o.ndc = pts[i];
    return o;
}

fn hash3(p: vec3<f32>) -> f32 {
    return fract(sin(dot(p, vec3<f32>(12.9898, 78.233, 37.719))) * 43758.5453);
}

@fragment
fn fs_sky(in: SkyOut) -> @location(0) vec4<f32> {
    let wp = g.inv_view_rot * vec4<f32>(in.ndc, 1.0, 1.0);
    let dir = normalize(wp.xyz / wp.w);
    let s = g.sun_dir.xyz;
    let daylight = g.fog_color.w;
    if (g.fog.z > 0.5) {
        return vec4<f32>(g.fog_color.rgb, 1.0);
    }
    var col = mix(g.sky_horizon.rgb, g.sky_top.rgb, smoothstep(-0.02, 0.45, dir.y));
    if (dir.y < 0.0) {
        col = mix(g.sky_horizon.rgb, g.sky_horizon.rgb * 0.55, smoothstep(0.0, -0.3, dir.y));
    }
    // Sunrise and sunset glow.
    let low = clamp(1.0 - abs(s.y) * 3.5, 0.0, 1.0);
    let toward = max(dot(normalize(vec2<f32>(dir.x, dir.z) + vec2<f32>(1e-5, 0.0)), normalize(vec2<f32>(s.x, s.z) + vec2<f32>(1e-5, 0.0))), 0.0);
    let band = clamp(1.0 - abs(dir.y - 0.05) * 4.0, 0.0, 1.0);
    col = mix(col, vec3<f32>(0.95, 0.45, 0.2), low * toward * toward * band * 0.75);
    // Stars.
    let night = clamp(1.0 - daylight * 1.6, 0.0, 1.0);
    if (night > 0.0 && dir.y > -0.05) {
        let q = floor(dir * 220.0);
        let h = hash3(q);
        if (h > 0.9975) {
            col += vec3<f32>(0.8 * night * (h - 0.9975) * 400.0);
        }
    }
    // Square sun and moon, like the classic game.
    let t1 = vec3<f32>(0.0, 0.0, 1.0);
    let t2 = normalize(cross(s, t1));
    let ds = dot(dir, s);
    if (ds > 0.0) {
        let a = abs(dot(dir, t1) / ds);
        let b = abs(dot(dir, t2) / ds);
        let m = max(a, b);
        if (m < 0.085) {
            col = vec3<f32>(1.0, 0.98, 0.82);
        } else if (m < 0.2) {
            col += vec3<f32>(1.0, 0.85, 0.5) * (0.2 - m) * 1.2 * (0.4 + low);
        }
    } else {
        let a = abs(dot(dir, t1) / ds);
        let b = abs(dot(dir, t2) / ds);
        let m = max(a, b);
        if (m < 0.06) {
            let spot = hash3(floor(vec3<f32>(dot(dir, t1) / ds, dot(dir, t2) / ds, 0.0) * 60.0));
            col = vec3<f32>(0.82, 0.84, 0.9) - vec3<f32>(spot * 0.12);
        }
    }
    return vec4<f32>(col, 1.0);
}

// ---- Clouds: a flat layer of blocky clouds that drifts slowly ----

struct CloudOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) rel: vec3<f32>,
};

const CLOUD_R: f32 = 384.0;

@vertex
fn vs_clouds(@builtin(vertex_index) i: u32) -> CloudOut {
    var pts = array<vec2<f32>, 6>(vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0));
    let cam_block = g.cam_world.xyz - g.cam_pos.xyz;
    let p = vec3<f32>(g.cam_pos.x + pts[i].x * CLOUD_R, g.cam_world.w - cam_block.y, g.cam_pos.z + pts[i].y * CLOUD_R);
    var o: CloudOut;
    o.clip = g.view_proj * vec4<f32>(p, 1.0);
    o.rel = p;
    return o;
}

fn hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash2(i), hash2(i + vec2<f32>(1.0, 0.0)), u.x), mix(hash2(i + vec2<f32>(0.0, 1.0)), hash2(i + vec2<f32>(1.0, 1.0)), u.x), u.y);
}

@fragment
fn fs_clouds(in: CloudOut) -> @location(0) vec4<f32> {
    let cam_block = g.cam_world.xyz - g.cam_pos.xyz;
    let world = in.rel.xz + cam_block.xz + vec2<f32>(g.cam_pos.w * 0.6, 0.0);
    let cell = floor(world / 12.0);
    let n = vnoise(cell * 0.18) * 0.7 + hash2(cell) * 0.3;
    if (n < 0.56) {
        discard;
    }
    let d = length(in.rel.xz - g.cam_pos.xz);
    let fade = 1.0 - smoothstep(CLOUD_R * 0.55, CLOUD_R, d);
    let day = g.fog_color.w;
    let c = mix(vec3<f32>(0.12, 0.13, 0.18), vec3<f32>(1.0, 1.0, 1.0), day);
    return vec4<f32>(c, 0.8 * fade);
}
