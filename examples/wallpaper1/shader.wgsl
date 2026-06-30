struct Uniforms {
    offset: vec2<f32>,
    scale: f32,
    time: f32,
    params: vec4<f32>,
    flags: vec4<f32>,
};

@group(0) @binding(0) var t: texture_2d<f32>;
@group(0) @binding(1) var s: sampler;
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
fn fs_default(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = in.uv;
    if (u.flags[0] > 0.5) {
        uv += u.offset * 0.05;
    }
    let wave = sin(uv.y * u.params[2] + u.time * u.params[0]) * u.params[1];
    let color = textureSample(t, s, vec2<f32>(uv.x + wave, uv.y));
    return color * u.params[3];
}