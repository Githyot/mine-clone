struct LOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_lines(@location(0) pos: vec3<f32>, @location(1) color: vec4<f32>) -> LOut {
    var o: LOut;
    o.clip = g.view_proj * vec4<f32>(pos, 1.0);
    o.color = color;
    return o;
}

@fragment
fn fs_lines(in: LOut) -> @location(0) vec4<f32> {
    return in.color;
}
