// Built-in post-processing (appended to post_common.wgsl).
// Bloom: fs_bright keeps the bright parts of the scene at half size, fs_down halves it again a
// few times, fs_up adds each level back onto the one above with a tent blur, so the half-size
// level ends up holding a wide, soft glow. fs_rays blurs sky pixels toward the sun (god rays).
// The fs_realcado / fs_suave composites put it together on the screen.

@group(1) @binding(0) var tex_a: texture_2d<f32>;
@group(1) @binding(1) var tex_b: texture_2d<f32>;

fn sample_a(uv: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(tex_a, lin, uv, 0.0).rgb;
}

// Darker scenes glow from dimmer lights, so torches stand out at night.
fn bloom_threshold() -> f32 {
    return mix(0.42, 0.86, post.sun.w);
}

fn bright_part(c: vec3<f32>) -> vec3<f32> {
    let t = bloom_threshold();
    let knee = 0.12;
    let l = max(c.r, max(c.g, c.b));
    // Soft knee: a smooth ramp instead of a hard cut, then everything above the threshold.
    var s = clamp(l - t + knee, 0.0, 2.0 * knee);
    s = s * s / (4.0 * knee + 1e-4);
    let w = max(s, l - t) / max(l, 1e-4);
    return c * w;
}

@fragment
fn fs_bright(in: PostIn) -> @location(0) vec4<f32> {
    // Four bilinear taps cover a 4×4 block of full-size pixels. Weighting by 1/(1+luma) keeps
    // single very bright pixels from flickering.
    let t = post.resolution.zw;
    var acc = vec3<f32>(0.0);
    var wsum = 0.0;
    for (var i = 0; i < 4; i = i + 1) {
        let o = vec2<f32>(f32(i & 1) * 2.0 - 1.0, f32(i >> 1u) * 2.0 - 1.0);
        let c = bright_part(color_at(in.uv + o * t));
        let w = 1.0 / (1.0 + luma(c));
        acc += c * w;
        wsum += w;
    }
    return vec4<f32>(acc / wsum, 1.0);
}

@fragment
fn fs_down(in: PostIn) -> @location(0) vec4<f32> {
    let t = 1.0 / vec2<f32>(textureDimensions(tex_a));
    var c = sample_a(in.uv) * 0.5;
    c += sample_a(in.uv + vec2<f32>(-t.x, -t.y)) * 0.125;
    c += sample_a(in.uv + vec2<f32>(t.x, -t.y)) * 0.125;
    c += sample_a(in.uv + vec2<f32>(-t.x, t.y)) * 0.125;
    c += sample_a(in.uv + vec2<f32>(t.x, t.y)) * 0.125;
    return vec4<f32>(c, 1.0);
}

@fragment
fn fs_up(in: PostIn) -> @location(0) vec4<f32> {
    // 3×3 tent filter on the smaller level; the pipeline adds the result onto the bigger one.
    let t = 1.0 / vec2<f32>(textureDimensions(tex_a));
    var c = sample_a(in.uv) * 4.0;
    c += (sample_a(in.uv + vec2<f32>(-t.x, 0.0)) + sample_a(in.uv + vec2<f32>(t.x, 0.0))) * 2.0;
    c += (sample_a(in.uv + vec2<f32>(0.0, -t.y)) + sample_a(in.uv + vec2<f32>(0.0, t.y))) * 2.0;
    c += sample_a(in.uv + vec2<f32>(-t.x, -t.y)) + sample_a(in.uv + vec2<f32>(t.x, -t.y));
    c += sample_a(in.uv + vec2<f32>(-t.x, t.y)) + sample_a(in.uv + vec2<f32>(t.x, t.y));
    return vec4<f32>(c / 16.0, 1.0);
}

fn hash12(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

// Light colour of the sun: white at noon, orange when low.
fn sun_tint() -> vec3<f32> {
    return mix(vec3<f32>(1.0, 0.94, 0.82), vec3<f32>(1.0, 0.55, 0.25), post.sun_dir.w);
}

@fragment
fn fs_rays(in: PostIn) -> @location(0) vec4<f32> {
    let vis = post.sun.z;
    if (vis <= 0.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    let fancy = post.params.w > 0.5;
    let n = select(16, 40, fancy);
    let sun = post.sun.xy;
    let aspect = post.resolution.x * post.resolution.w;
    // March from this pixel toward the sun; every sky sample near the sun lets light through.
    let delta = (sun - in.uv) * (0.85 / f32(n));
    // A per-pixel offset turns banding into fine noise that the upscale smooths out.
    var p = in.uv + delta * hash12(in.pos.xy);
    var acc = 0.0;
    var decay = 1.0;
    for (var i = 0; i < n; i = i + 1) {
        p += delta;
        let sky = select(0.0, 1.0, depth_at(p) >= 1.0);
        let d = length((p - sun) * vec2<f32>(aspect, 1.0));
        acc += sky * (1.0 - smoothstep(0.0, 0.45, d)) * decay;
        decay *= 0.965;
    }
    acc = acc / f32(n) * vis;
    return vec4<f32>(sun_tint() * acc, 1.0);
}

// Gentle tonemap: the identity up to 0.72, then a smooth shoulder toward 1. Works on the
// brightest channel so hues don't shift.
fn tonemap(c: vec3<f32>) -> vec3<f32> {
    let k = 0.72;
    let m = max(c.r, max(c.g, c.b));
    if (m <= k) {
        return c;
    }
    let mapped = k + (1.0 - k) * (1.0 - exp(-(m - k) / (1.0 - k)));
    return c * (mapped / m);
}

fn saturate_color(c: vec3<f32>, s: f32) -> vec3<f32> {
    return max(mix(vec3<f32>(luma(c)), c, s), vec3<f32>(0.0));
}

fn vignette(uv: vec2<f32>, amount: f32) -> f32 {
    let d = length((uv - vec2<f32>(0.5)) * vec2<f32>(1.0, 0.8));
    return 1.0 - amount * smoothstep(0.25, 0.8, d);
}

fn dither(pos: vec2<f32>) -> f32 {
    return (hash12(pos) - 0.5) / 255.0;
}

// Options: 0 bloom, 1 sun rays, 2 saturation, 3 vignette, 4 haze, 5 waving plants (terrain),
// 6 rippling water (terrain), 7 exposure.
@fragment
fn fs_realcado(in: PostIn) -> @location(0) vec4<f32> {
    var c = color_at(in.uv);
    let day = post.sun.w;
    let night = 1.0 - day;
    let sunset = post.sun_dir.w;
    let under = post.fog_color.w > 0.5;

    // Distance haze: far land fades into a bluish (or golden, at sunset) air before the fog.
    if (opt(4) > 0.5 && !under) {
        let d = distance_at(in.uv);
        if (d < post.params.z * 0.999) {
            let h = smoothstep(post.params.z * 0.3, post.params.z, d) * 0.22;
            var air = mix(post.fog_color.rgb, vec3<f32>(0.62, 0.74, 0.95) * (0.3 + 0.7 * day), 0.4);
            air = mix(air, vec3<f32>(1.0, 0.7, 0.45) * day, sunset * 0.35);
            c = mix(c, air, h);
        }
    }

    // Bloom: the up-sampled chain is the sum of all levels, so divide by how many there are.
    let levels = select(3.0, 5.0, post.params.w > 0.5);
    let glow = sample_a(in.uv) / levels;
    c += glow * opt(0) * 1.6;

    // Sun rays, a little stronger at sunrise and sunset.
    let rays = textureSampleLevel(tex_b, lin, in.uv, 0.0).rgb;
    c += rays * opt(1) * (0.55 + sunset * 0.45);

    c *= opt(7);

    // Colour grading: lively days, warm sunsets, cool blue nights.
    c = saturate_color(c, opt(2) * mix(0.8, 1.0, day));
    var tint = mix(vec3<f32>(1.0), vec3<f32>(1.1, 0.95, 0.82), sunset * 0.75);
    tint = mix(tint, vec3<f32>(0.78, 0.9, 1.22), night * 0.8);
    c *= tint;
    // A touch of contrast around the mid tones.
    let cc = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
    c += (cc * cc * (3.0 - 2.0 * cc) - cc) * 0.12 * day;

    c = tonemap(c) * vignette(in.uv, opt(3));
    return vec4<f32>(c + vec3<f32>(dither(in.pos.xy)), 1.0);
}

// Options: 0 bloom, 1 saturation, 2 warmth, 3 vignette.
@fragment
fn fs_suave(in: PostIn) -> @location(0) vec4<f32> {
    var c = color_at(in.uv);
    c += sample_a(in.uv) / 3.0 * opt(0) * 1.4;
    c = saturate_color(c, opt(1));
    let warm = opt(2) * (0.4 + 0.6 * post.sun.w);
    c *= mix(vec3<f32>(1.0), vec3<f32>(1.08, 1.0, 0.88), warm);
    c = tonemap(c) * vignette(in.uv, opt(3));
    return vec4<f32>(c + vec3<f32>(dither(in.pos.xy)), 1.0);
}

// Temporal anti-aliasing: every frame is drawn shifted by a fraction of a pixel; blending each
// one with the last (moved to where it was on screen) smooths edges. The last frame is clamped
// to the colours around this pixel, so moving things don't leave trails.
@fragment
fn fs_taa(in: PostIn) -> @location(0) vec4<f32> {
    let size = vec2<i32>(post.resolution.xy);
    let px = clamp(vec2<i32>(in.pos.xy), vec2<i32>(0), size - vec2<i32>(1));
    let cur = textureLoad(scene, px, 0).rgb;
    if (post.taa.y < 0.5) {
        return vec4<f32>(cur, 1.0);
    }
    var lo = cur;
    var hi = cur;
    for (var j = -1; j <= 1; j = j + 1) {
        for (var i = -1; i <= 1; i = i + 1) {
            let c = textureLoad(scene, clamp(px + vec2<i32>(i, j), vec2<i32>(0), size - vec2<i32>(1)), 0).rgb;
            lo = min(lo, c);
            hi = max(hi, c);
        }
    }
    // Where was this point last frame?
    let d = textureLoad(scene_depth, px, 0);
    let ndc = vec4<f32>(in.uv.x * 2.0 - 1.0, 1.0 - in.uv.y * 2.0, d, 1.0);
    let prev = post.reproj * ndc;
    let puv = vec2<f32>(prev.x / prev.w * 0.5 + 0.5, 0.5 - prev.y / prev.w * 0.5);
    if (any(puv < vec2<f32>(0.0)) || any(puv > vec2<f32>(1.0))) {
        return vec4<f32>(cur, 1.0);
    }
    let hist = clamp(textureSampleLevel(tex_a, lin, puv, 0.0).rgb, lo, hi);
    // A little sharpening keeps the pixel-art textures crisp.
    let sharp = cur + (cur - (lo + hi) * 0.5) * 0.12;
    return vec4<f32>(mix(max(sharp, vec3<f32>(0.0)), hist, 0.86), 1.0);
}
