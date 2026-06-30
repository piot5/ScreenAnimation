struct Uniforms {
    offset: vec2<f32>,
    scale: f32,
    time: f32,
};

@group(1) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0,  1.0), vec2<f32>(-1.0, -1.0), vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0), vec2<f32>( 1.0, -1.0), vec2<f32>( 1.0,  1.0)
    );
    var uv = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(1.0, 0.0)
    );
    let p = pos[in_vertex_index];
    out.clip_position = vec4<f32>((p * uniforms.scale) + uniforms.offset, 0.0, 1.0);
    out.tex_coords = uv[in_vertex_index];
    return out;
}

@fragment
fn fs_default(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}

@fragment
fn fs_flow_right(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let edge = smoothstep(0.9, 1.0, in.tex_coords.x + uniforms.offset.x * 0.5);
    return color + vec4<f32>(edge, edge * 0.5, 0.0, 0.0);
}

@fragment
fn fs_flow_left(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}

@fragment
fn fs_flow_up(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}

@fragment
fn fs_flow_down(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}