// Raindrops on the screen (screen_drops.rs). One quad per drop over the bead and its trail.
// The bead is a small lens: it shows the picture behind it turned over and shrunk, a little
// blurred (it is out of focus), with a darker rim and a highlight from the sky above. The
// trail above a running drop only bends the picture sideways a touch.

struct Drops {
    // Width, height in pixels; z = 1 when the picture behind can be read.
    screen: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: Drops;
@group(0) @binding(1) var scene: texture_2d<f32>;
@group(0) @binding(2) var lin: sampler;

struct DOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) px: vec2<f32>,
    // Centre (px), radius (px), trail length (px).
    @location(1) @interpolate(flat) a: vec4<f32>,
    // Alpha, seed, how fast it runs (0..1).
    @location(2) @interpolate(flat) b: vec4<f32>,
};

@vertex
fn vs_drop(@builtin(vertex_index) vi: u32, @location(0) a: vec4<f32>, @location(1) b: vec4<f32>) -> DOut {
    let corner = vec2<f32>(f32(vi & 1u), f32(vi >> 1u));
    let r = a.z;
    let lo = vec2<f32>(a.x - r * 1.5, a.y - a.w - r * 1.3);
    let hi = vec2<f32>(a.x + r * 1.5, a.y + r * 1.6);
    let p = mix(lo, hi, corner);
    var o: DOut;
    o.clip = vec4<f32>(p.x / u.screen.x * 2.0 - 1.0, 1.0 - p.y / u.screen.y * 2.0, 0.0, 1.0);
    o.px = p;
    o.a = a;
    o.b = b;
    return o;
}

fn behind(p: vec2<f32>) -> vec3<f32> {
    let uv = clamp(p / u.screen.xy, vec2<f32>(0.001), vec2<f32>(0.999));
    return textureSampleLevel(scene, lin, uv, 0.0).rgb;
}

// Four taps around p: drops on a lens are never sharp.
fn soft(p: vec2<f32>, s: f32) -> vec3<f32> {
    return (behind(p + vec2<f32>(s, 0.3 * s)) + behind(p - vec2<f32>(s, 0.3 * s)) + behind(p + vec2<f32>(-0.3 * s, s)) + behind(p - vec2<f32>(-0.3 * s, s))) * 0.25;
}

@fragment
fn fs_drop(in: DOut) -> @location(0) vec4<f32> {
    let c = in.a.xy;
    let r = max(in.a.z, 0.5);
    let trail = in.a.w;
    let seed = in.b.y;
    let run = in.b.z;
    let read = u.screen.z > 0.5;

    // Bead: round with a heavier bottom, a little longer while it runs, a wobbly outline.
    var q = (in.px - c) / r;
    q.y = q.y / (1.0 + 0.22 * run);
    q.y = q.y / (1.0 + 0.14 * max(q.y, 0.0)) + 0.06;
    let ang = atan2(q.y, q.x);
    let wob = 1.0 + 0.03 * sin(ang * 3.0 + seed * 23.0) + 0.02 * sin(ang * 5.0 + seed * 61.0);
    let d = length(q) / wob;
    let aa = 1.3 / r;
    let bead = 1.0 - smoothstep(1.0 - aa, 1.0, d);

    // Trail: a thin wet streak above, narrowing and drying toward its top.
    let up = c.y - in.px.y;
    let t = clamp(up / max(trail, 1.0), 0.0, 1.0);
    let wiggle = sin(up * 0.09 + seed * 31.0) * r * 0.12 + sin(up * 0.23 + seed * 7.0) * r * 0.05;
    let tw = r * (0.42 * (1.0 - t) + 0.12);
    let dx = in.px.x - c.x - wiggle;
    var streak = (1.0 - smoothstep(tw - 1.0, tw, abs(dx))) * (1.0 - t) * step(0.0, up) * step(1.0, trail);
    // Broken into short beads near its top.
    streak *= mix(1.0, smoothstep(0.2, 0.6, sin(up * 0.35 + seed * 13.0) * 0.5 + 0.5), t);
    streak *= 0.55;

    let cover = max(bead, streak);
    if (cover <= 0.002) {
        discard;
    }
    var col: vec3<f32>;
    var alpha: f32;
    if (bead >= streak) {
        // The lens: the picture around the drop, turned over and shrunk.
        let k = clamp(d, 0.0, 1.0);
        let sample_at = c - q * r * (1.6 + 1.4 * k * k);
        let rim = smoothstep(0.62, 1.0, d);
        let hl = smoothstep(0.34, 0.0, length(q - vec2<f32>(-0.3, -0.42)));
        let glint = smoothstep(0.22, 0.0, length(q - vec2<f32>(0.18, 0.62))) * 0.35;
        if (read) {
            // Water gathers light: the inside is a little brighter than what it shows.
            col = soft(sample_at, r * 0.14) * 0.9 + vec3<f32>(0.05, 0.055, 0.065);
            col = col * (1.0 - 0.24 * rim) + vec3<f32>(hl * 0.5 + glint * 0.3);
            alpha = 0.93;
        } else {
            col = vec3<f32>(0.16, 0.18, 0.2) * rim + vec3<f32>(hl * 0.9 + glint);
            alpha = 0.12 + 0.3 * rim + hl * 0.5;
        }
    } else {
        let bend = dx / max(tw, 0.5);
        if (read) {
            col = soft(in.px + vec2<f32>(-bend * r * 0.35, 0.0), 1.0) * 0.92 + vec3<f32>(0.035);
            col += vec3<f32>(smoothstep(0.2, -0.6, bend) * 0.12);
            alpha = 0.9;
        } else {
            col = vec3<f32>(0.85, 0.9, 0.95);
            alpha = 0.14 * (1.0 - abs(bend));
        }
    }
    return vec4<f32>(col, alpha * cover * in.b.x);
}
