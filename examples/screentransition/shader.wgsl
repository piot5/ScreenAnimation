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
fn fs_capture(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let col = textureSample(tex0, samp0, uv);
    let flash = 1.0 - u.time * 2.0;
    return vec4<f32>(col.rgb + vec3<f32>(flash), 1.0);
}

@fragment
fn fs_detach(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let progress = clamp(u.time * u.logic_params[0], 0.0, 1.0);
    let angle = progress * 0.3;
    let lift = progress * 0.1;
    
    var offset = vec2<f32>(0.0, lift);
    let rotated_uv = uv + offset;
    
    let col = textureSample(tex0, samp0, clamp(rotated_uv, vec2<f32>(0.0), vec2<f32>(1.0)));
    let alpha = 1.0 - progress * 0.3;
    return vec4<f32>(col.rgb, alpha);
}

@fragment
fn fs_move(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let progress = clamp(u.time * 0.5, 0.0, 1.0);
    let move_x = u.logic_params[1] * progress;
    let perspective = u.logic_params[3];
    
    var warped_uv = uv;
    warped_uv.x = warped_uv.x - move_x;
    let scale = 1.0 - progress * perspective * 0.3;
    warped_uv = (warped_uv - 0.5) * scale + 0.5;
    
    let col = textureSample(tex0, samp0, clamp(warped_uv, vec2<f32>(0.0), vec2<f32>(1.0)));
    let shadow = 1.0 - progress * 0.2;
    return vec4<f32>(col.rgb * shadow, 1.0);
}

@fragment
fn fs_land(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let progress = clamp(u.time * u.logic_params[0], 0.0, 1.0);
    let settle = 1.0 - progress;
    
    var offset = vec2<f32>(0.0, settle * 0.05);
    let settled_uv = uv + offset;
    
    let col = textureSample(tex0, samp0, clamp(settled_uv, vec2<f32>(0.0), vec2<f32>(1.0)));
    return vec4<f32>(col.rgb, 1.0);
}

@fragment
fn fs_stable(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let col = textureSample(tex0, samp0, uv);
    return vec4<f32>(col.rgb, 1.0);
}

@fragment
fn fs_default(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let col = textureSample(tex0, samp0, uv);
    return vec4<f32>(col.rgb, 1.0);
}
