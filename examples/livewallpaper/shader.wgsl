struct Uniforms {
    mouse: vec2<f32>,
    offset: vec2<f32>,
    scale: f32,
    time: f32,
    logic_params: vec4<f32>,
    feature_flags: vec4<f32>,
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
fn fs_live_wallpaper(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = in.uv;
    let mouse = u.mouse;
    let time = u.time;
    let speed = u.logic_params[0];
    let amplitude = u.logic_params[1];
    let frequency = u.logic_params[2];
    let brightness = u.logic_params[3];
    let mouse_influence = u.feature_flags[0];
    
    let dist = distance(uv, mouse);
    let wave = sin(dist * frequency - time * speed) * amplitude;
    
    var warped_uv = uv;
    if (mouse_influence > 0.5) {
        let direction = normalize(uv - mouse);
        warped_uv = uv + direction * wave * 0.1;
    }
    
    let color = textureSample(t, s, clamp(warped_uv, vec2<f32>(0.0), vec2<f32>(1.0)));
    return vec4<f32>(color.rgb * brightness, 1.0);
}