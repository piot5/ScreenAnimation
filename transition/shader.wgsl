struct Uniforms {
    mouse: vec2<f32>,
    offset: vec2<f32>,
    scale: f32,
    time: f32,
    logic_params: vec4<f32>,
    feature_flags: vec4<f32>,
};

@group(0) @binding(0) var tex0: texture_2d<f32>;
@group(0) @binding(1) var samp0: sampler;
@group(1) @binding(0) var<uniform> u: Uniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(vi == 1u || vi == 2u || vi == 5u)) * 2.0 - 1.0;
    let y = f32(i32(vi <= 2u)) * 2.0 - 1.0;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@fragment
fn fs_pulse(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let col = textureSample(tex0, samp0, uv);
    let frames = 32.0;
    let progress = u.time * 10.0; // 10ms per frame
    let scale = 1.0 - (0.05 * sin((progress / frames) * 3.14159));
    let warped_uv = (uv - 0.5) / scale + 0.5;
    return vec4<f32>(col.rgb, 1.0);
}

@fragment
fn fs_woosh(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let col = textureSample(tex0, samp0, uv);
    let frames = 90.0;
    let progress = clamp(u.time * 10.0 / frames, 0.0, 1.0);
    let p = pow(progress, 3.0);
    let offset = -vec2<f32>(p, 0.0);
    let warped_uv = uv + offset;
    return vec4<f32>(col.rgb, 1.0);
}

@fragment
fn fs_black(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}

@fragment
fn fs_default(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let col = textureSample(tex0, samp0, uv);
    return vec4<f32>(col.rgb, 1.0);
}