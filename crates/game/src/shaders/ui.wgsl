struct Screen {
    size: vec4<f32>,
};
@group(0) @binding(0) var<uniform> screen: Screen;
@group(1) @binding(0) var font: texture_2d<f32>;
@group(1) @binding(1) var blocks: texture_2d_array<f32>;
@group(1) @binding(2) var samp: sampler;
@group(1) @binding(3) var gui: texture_2d<f32>;
// Layers from 2048 on live in a second array (renderer::LAYERS_PER_ARRAY).
@group(1) @binding(4) var blocks2: texture_2d_array<f32>;

struct UOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) layer: i32,
};

@vertex
fn vs_ui(@location(0) pos: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) color: vec4<f32>, @location(3) layer: i32) -> UOut {
    var o: UOut;
    o.clip = vec4<f32>(pos.x / screen.size.x * 2.0 - 1.0, 1.0 - pos.y / screen.size.y * 2.0, 0.0, 1.0);
    o.uv = uv;
    o.color = color;
    o.layer = layer;
    return o;
}

@fragment
fn fs_ui(in: UOut) -> @location(0) vec4<f32> {
    if (in.layer == -2) {
        return in.color;
    }
    if (in.layer == -1) {
        let a = textureSampleLevel(font, samp, in.uv, 0.0).r;
        if (a < 0.5) {
            discard;
        }
        return in.color;
    }
    if (in.layer == -3) {
        let g = textureSampleLevel(gui, samp, in.uv, 0.0) * in.color;
        if (g.a < 0.05) {
            discard;
        }
        return g;
    }
    var c: vec4<f32>;
    if (in.layer < 2048) {
        c = textureSampleLevel(blocks, samp, in.uv, in.layer, 0.0) * in.color;
    } else {
        c = textureSampleLevel(blocks2, samp, in.uv, in.layer - 2048, 0.0) * in.color;
    }
    if (c.a < 0.1) {
        discard;
    }
    return c;
}
