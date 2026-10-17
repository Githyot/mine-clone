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
    // The End: a dark violet void with faint fixed specks.
    if (g.weather.z > 1.5 && g.fog.z < 0.5) {
        var c = g.fog_color.rgb * (1.0 + 0.6 * max(dir.y, 0.0));
        let h = hash3(floor(dir * 160.0));
        if (h > 0.996) {
            c += vec3<f32>(0.35, 0.3, 0.45) * (h - 0.996) * 250.0;
        }
        return vec4<f32>(c, 1.0);
    }
    // Underwater, or under the Nether's roof: only fog.
    if (g.fog.z > 0.5 || g.weather.z > 0.5) {
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
    // Stars (hidden by rain clouds).
    let night = clamp(1.0 - daylight * 1.6, 0.0, 1.0);
    if (night > 0.0 && dir.y > -0.05 && g.weather.x < 0.5) {
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
        if (m < 0.12) {
            col = vec3<f32>(1.0, 0.98, 0.82);
        } else if (m < 0.3) {
            col += vec3<f32>(1.0, 0.85, 0.5) * (0.3 - m) * 1.0 * (0.4 + low);
        }
    } else {
        let a = abs(dot(dir, t1) / ds);
        let b = abs(dot(dir, t2) / ds);
        let m = max(a, b);
        if (m < 0.09) {
            let spot = hash3(floor(vec3<f32>(dot(dir, t1) / ds, dot(dir, t2) / ds, 0.0) * 40.0));
            col = vec3<f32>(0.82, 0.84, 0.9) - vec3<f32>(spot * 0.12);
        }
    }
    return vec4<f32>(col, 1.0);
}

// ---- Clouds: 3D blocky clouds, ray-marched per pixel through a grid of cells ----
// Each cell is CELL×THICK×CELL blocks. The ray walks the grid (DDA) until it enters a filled cell,
// then the hit is shaded by which face it entered and written with its true depth, so terrain and
// clouds hide each other correctly and clouds reach far past the render distance.

const CELL: f32 = 12.0;
const THICK: f32 = 4.0;
const CLOUD_MAX: f32 = 1400.0;

fn hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash2(i), hash2(i + vec2<f32>(1.0, 0.0)), u.x), mix(hash2(i + vec2<f32>(0.0, 1.0)), hash2(i + vec2<f32>(1.0, 1.0)), u.x), u.y);
}

fn cloud_at(c: vec2<f32>) -> bool {
    let n = vnoise(c * 0.11) * 0.55 + vnoise(c * 0.31 + vec2<f32>(17.0, 3.0)) * 0.3 + hash2(c) * 0.15;
    // Rain fills the sky with cloud.
    return n > 0.55 - g.weather.x * 0.22;
}

struct CloudFrag {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

@fragment
fn fs_clouds(in: SkyOut) -> CloudFrag {
    let wp = g.inv_view_rot * vec4<f32>(in.ndc, 1.0, 1.0);
    let dir = normalize(wp.xyz / wp.w);
    let drift = vec3<f32>(g.cam_pos.w * 0.8, 0.0, 0.0);
    let ro = g.cam_world.xyz + drift;
    let y0 = g.cam_world.w;
    let y1 = y0 + THICK;
    var t0 = 0.0;
    var t1 = CLOUD_MAX;
    var normal = vec3<f32>(0.0, 0.0, 0.0);
    if (abs(dir.y) < 1e-5) {
        if (ro.y < y0 || ro.y > y1) {
            discard;
        }
    } else {
        let ta = (y0 - ro.y) / dir.y;
        let tb = (y1 - ro.y) / dir.y;
        t0 = max(min(ta, tb), 0.0);
        t1 = min(max(ta, tb), CLOUD_MAX);
        if (t0 > 0.0) {
            normal = vec3<f32>(0.0, -sign(dir.y), 0.0);
        }
    }
    if (t1 <= t0) {
        discard;
    }
    let p = ro + dir * t0;
    var cell = floor(p.xz / CELL);
    let stp = vec2<f32>(select(-1.0, 1.0, dir.x >= 0.0), select(-1.0, 1.0, dir.z >= 0.0));
    let big = 1e9;
    let delta = vec2<f32>(select(big, CELL / abs(dir.x), abs(dir.x) > 1e-6), select(big, CELL / abs(dir.z), abs(dir.z) > 1e-6));
    let bx = (cell.x + select(0.0, 1.0, stp.x > 0.0)) * CELL;
    let bz = (cell.y + select(0.0, 1.0, stp.y > 0.0)) * CELL;
    var tmax = vec2<f32>(select(big, t0 + (bx - p.x) / dir.x, abs(dir.x) > 1e-6), select(big, t0 + (bz - p.z) / dir.z, abs(dir.z) > 1e-6));
    var t = t0;
    var hit = false;
    for (var i = 0; i < 160; i = i + 1) {
        if (cloud_at(cell)) {
            hit = true;
            break;
        }
        if (tmax.x < tmax.y) {
            t = tmax.x;
            tmax.x = tmax.x + delta.x;
            cell.x = cell.x + stp.x;
            normal = vec3<f32>(-stp.x, 0.0, 0.0);
        } else {
            t = tmax.y;
            tmax.y = tmax.y + delta.y;
            cell.y = cell.y + stp.y;
            normal = vec3<f32>(0.0, 0.0, -stp.y);
        }
        if (t > t1) {
            break;
        }
    }
    if (!hit) {
        discard;
    }
    // Face shading: bright tops, darker undersides, like lit blocks.
    var shade = 0.82;
    if (normal.y > 0.5) {
        shade = 1.0;
    } else if (normal.y < -0.5) {
        shade = 0.68;
    } else if (abs(normal.x) > 0.5) {
        shade = 0.88;
    }
    let day = g.fog_color.w;
    let s = g.sun_dir.xyz;
    let low = clamp(1.0 - abs(s.y) * 3.0, 0.0, 1.0) * step(-0.15, s.y);
    var c = mix(vec3<f32>(0.10, 0.11, 0.16), vec3<f32>(1.0, 1.0, 1.0), day) * shade;
    c = mix(c, c * vec3<f32>(1.15, 0.72, 0.55), low * 0.7);
    // Rain clouds are dark grey; lightning lights them up.
    c = c * (1.0 - g.weather.x * 0.35) + vec3<f32>(g.weather.y * 0.6);
    // Fade into the sky with distance.
    let fade = 1.0 - smoothstep(CLOUD_MAX * 0.45, CLOUD_MAX, t);
    c = mix(g.sky_horizon.rgb, c, clamp(fade * 1.2, 0.0, 1.0));
    let rel = g.cam_pos.xyz + dir * t;
    let clip = g.view_proj * vec4<f32>(rel, 1.0);
    var out: CloudFrag;
    out.color = vec4<f32>(c, 0.88 * fade);
    out.depth = clamp(clip.z / clip.w, 0.0, 0.999999);
    return out;
}
