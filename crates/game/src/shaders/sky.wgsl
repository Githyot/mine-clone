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
//
// Layered style (Better Clouds idea, g.extra.x = 1): two blocky layers at different heights with
// different cell sizes, drifting with the wind at different speeds, plus a thin high sheet of
// streaky cloud. Cells have a density instead of on/off, so the edges of a cloud are see-through,
// the light passes through thin parts (opacity grows with the length of the ray inside a cell),
// and cells fade in and out slowly as the noise drifts, so the clouds change shape over time.

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

// Layered style: how thick the cloud in cell `c` of a layer is (0 = clear sky).
fn cloud_density(c: vec2<f32>, seed: f32, cover: f32) -> f32 {
    let t = g.clouds.w;
    // Slow change of shape: the broad noise drifts a little against the wind, so clouds grow,
    // shrink and split as they travel.
    let n = vnoise(c * 0.11 + vec2<f32>(seed + t * 0.004, seed * 0.7 - t * 0.003)) * 0.55 + vnoise(c * 0.31 + vec2<f32>(17.0 + seed, 3.0)) * 0.3 + hash2(c + seed) * 0.15;
    let thr = 0.55 - g.weather.x * 0.22 + cover;
    return smoothstep(thr, thr + 0.07, n);
}

// The wind: one direction for every layer (turning very slowly), each layer at its own speed.
// The distance is added up on the CPU frame by frame (Game::advance_clouds): working it out
// here from the clock made the clouds race whenever the clock was large, the rain changed or
// the direction turned.
fn wind(speed: f32) -> vec3<f32> {
    return vec3<f32>(g.clouds.x, 0.0, g.clouds.y) * speed;
}

// Colour of a cloud face lit by the sky (day, night, sunset, rain, lightning).
fn cloud_light(shade: f32) -> vec3<f32> {
    let day = g.fog_color.w;
    let s = g.sun_dir.xyz;
    let low = clamp(1.0 - abs(s.y) * 3.0, 0.0, 1.0) * step(-0.15, s.y);
    var c = mix(vec3<f32>(0.10, 0.11, 0.16), vec3<f32>(1.0, 1.0, 1.0), day) * shade;
    c = mix(c, c * vec3<f32>(1.15, 0.72, 0.55), low * 0.7);
    // Rain clouds are dark grey; lightning lights them up.
    return c * (1.0 - g.weather.x * 0.35) + vec3<f32>(g.weather.y * 0.6);
}

fn face_shade(normal: vec3<f32>) -> f32 {
    // Bright tops, darker undersides, like lit blocks.
    if (normal.y > 0.5) {
        return 1.0;
    } else if (normal.y < -0.5) {
        return 0.68;
    } else if (abs(normal.x) > 0.5) {
        return 0.88;
    }
    return 0.82;
}

// Where the ray is inside the slab y0..y1: (t0, t1, entered through top/bottom).
fn slab(ro: vec3<f32>, dir: vec3<f32>, y0: f32, y1: f32) -> vec3<f32> {
    if (abs(dir.y) < 1e-5) {
        if (ro.y < y0 || ro.y > y1) {
            return vec3<f32>(1.0, 0.0, 0.0);
        }
        return vec3<f32>(0.0, CLOUD_MAX, 0.0);
    }
    let ta = (y0 - ro.y) / dir.y;
    let tb = (y1 - ro.y) / dir.y;
    let t0 = max(min(ta, tb), 0.0);
    return vec3<f32>(t0, min(max(ta, tb), CLOUD_MAX), select(0.0, 1.0, t0 > 0.0));
}

// Result of one layer: premultiplied colour and coverage, and the distance of the first hit.
struct CloudHit {
    rgba: vec4<f32>,
    t: f32,
};

// Walk one blocky layer. `soft` = layered style: density, light through thin parts, several
// cells blended front to back; otherwise the first filled cell is solid (classic).
fn march_layer(ro: vec3<f32>, dir: vec3<f32>, y0: f32, thick: f32, cell_size: f32, seed: f32, cover: f32, soft: bool, max_steps: i32) -> CloudHit {
    var res: CloudHit;
    res.rgba = vec4<f32>(0.0);
    res.t = CLOUD_MAX;
    let sl = slab(ro, dir, y0, y0 + thick);
    let t0 = sl.x;
    let t1 = sl.y;
    if (t1 <= t0) {
        return res;
    }
    var normal = select(vec3<f32>(0.0), vec3<f32>(0.0, -sign(dir.y), 0.0), sl.z > 0.5);
    let p = ro + dir * t0;
    var cell = floor(p.xz / cell_size);
    let stp = vec2<f32>(select(-1.0, 1.0, dir.x >= 0.0), select(-1.0, 1.0, dir.z >= 0.0));
    let big = 1e9;
    let delta = vec2<f32>(select(big, cell_size / abs(dir.x), abs(dir.x) > 1e-6), select(big, cell_size / abs(dir.z), abs(dir.z) > 1e-6));
    let bx = (cell.x + select(0.0, 1.0, stp.x > 0.0)) * cell_size;
    let bz = (cell.y + select(0.0, 1.0, stp.y > 0.0)) * cell_size;
    var tmax = vec2<f32>(select(big, t0 + (bx - p.x) / dir.x, abs(dir.x) > 1e-6), select(big, t0 + (bz - p.z) / dir.z, abs(dir.z) > 1e-6));
    var t = t0;
    // Neighbouring filled cells are one cloud: the face shade is picked when the ray enters the
    // cloud from clear air and kept while it crosses into the next filled cells, so the walls
    // between cells inside a cloud are never drawn (no grid of lines on the underside).
    var in_cloud = false;
    var run_shade = 1.0;
    for (var i = 0; i < max_steps; i = i + 1) {
        let t_exit = min(min(tmax.x, tmax.y), t1);
        var a = 0.0;
        var filled = false;
        if (soft) {
            let d = cloud_density(cell, seed, cover);
            if (d > 0.0) {
                filled = true;
                // Opacity from the length of the ray inside the cloud (Beer-Lambert), so a path
                // split over two cells gives the same result as one cell: no seam at the border.
                // Thin crossings and low-density edges still let light through.
                a = 1.0 - exp(-(t_exit - t) * 0.9 * d * d);
            }
        } else if (cloud_at(cell)) {
            filled = true;
            a = 1.0;
        }
        if (filled && !in_cloud) {
            run_shade = face_shade(normal);
            if (soft) {
                // Darker toward the bottom of the layer.
                let h = clamp((ro.y + dir.y * t - y0) / thick, 0.0, 1.0);
                run_shade *= mix(0.8, 1.04, h);
            }
        }
        in_cloud = filled;
        if (a > 0.0005) {
            let fade = 1.0 - smoothstep(CLOUD_MAX * 0.45, CLOUD_MAX, t);
            let c = mix(g.sky_horizon.rgb, cloud_light(run_shade), clamp(fade * 1.2, 0.0, 1.0));
            let aa = a * fade;
            res.rgba += vec4<f32>(c * aa, aa) * (1.0 - res.rgba.a);
            res.t = min(res.t, t);
            if (!soft || res.rgba.a > 0.85) {
                break;
            }
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
    return res;
}

// A thin high sheet of streaky cloud: one plane, no grid.
fn high_sheet(ro: vec3<f32>, dir: vec3<f32>, y: f32) -> CloudHit {
    var res: CloudHit;
    res.rgba = vec4<f32>(0.0);
    res.t = CLOUD_MAX;
    if (abs(dir.y) < 0.03) {
        return res;
    }
    let t = (y - ro.y) / dir.y;
    if (t <= 0.0 || t > CLOUD_MAX * 1.6) {
        return res;
    }
    let q = (ro.xz + dir.xz * t) * vec2<f32>(0.004, 0.012);
    let n = vnoise(q) * 0.65 + vnoise(q * 2.7 + vec2<f32>(5.0, 1.0)) * 0.35;
    let a = smoothstep(0.52 - g.weather.x * 0.2, 0.8, n) * 0.4 * (1.0 - smoothstep(CLOUD_MAX * 0.6, CLOUD_MAX * 1.6, t));
    let c = cloud_light(0.95);
    res.rgba = vec4<f32>(c * a, a);
    res.t = t;
    return res;
}

struct CloudFrag {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

@fragment
fn fs_clouds(in: SkyOut) -> CloudFrag {
    let wp = g.inv_view_rot * vec4<f32>(in.ndc, 1.0, 1.0);
    let dir = normalize(wp.xyz / wp.w);
    let base = g.cam_world.w;
    var acc = vec4<f32>(0.0);
    var tn = CLOUD_MAX * 2.0;
    if (g.extra.x < 0.5) {
        // Classic: one layer drifting west to east.
        let h = march_layer(g.cam_world.xyz + vec3<f32>(g.clouds.z * 0.8, 0.0, 0.0), dir, base, THICK, CELL, 0.0, 0.0, false, 160);
        acc = h.rgba;
        tn = h.t;
    } else {
        // Layered: nearest layer first, stop once the sky is covered.
        let ys = vec3<f32>(base, base + 42.0, base + 120.0);
        var order = vec3<i32>(0, 1, 2);
        if (g.cam_world.y > base + 49.0) {
            order = vec3<i32>(1, 0, 2);
        }
        if (g.cam_world.y > base + 120.0) {
            order = vec3<i32>(2, 1, 0);
        }
        for (var k = 0; k < 3; k = k + 1) {
            if (acc.a > 0.85) {
                break;
            }
            let li = order[k];
            var h: CloudHit;
            if (li == 0) {
                h = march_layer(g.cam_world.xyz + wind(0.8), dir, ys.x, THICK, CELL, 0.0, 0.0, true, 72);
            } else if (li == 1) {
                h = march_layer(g.cam_world.xyz + wind(1.5), dir, ys.y, 7.0, 22.0, 41.0, 0.07, true, 32);
            } else {
                h = high_sheet(g.cam_world.xyz + wind(3.0), dir, ys.z);
            }
            acc += h.rgba * (1.0 - acc.a);
            tn = min(tn, h.t);
        }
    }
    if (acc.a < 0.004) {
        discard;
    }
    let rel = g.cam_pos.xyz + dir * min(tn, CLOUD_MAX);
    let clip = g.view_proj * vec4<f32>(rel, 1.0);
    var out: CloudFrag;
    out.color = vec4<f32>(acc.rgb / acc.a, min(acc.a / select(1.0, 0.85, g.extra.x > 0.5), 1.0) * 0.88);
    out.depth = clamp(clip.z / clip.w, 0.0, 0.999999);
    return out;
}
