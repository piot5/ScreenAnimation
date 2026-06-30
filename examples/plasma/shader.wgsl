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
fn fs_plasma(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let time = u.time;
    
    // Parameters from config.toml
    let speed = u.logic_params[0];
    let frequency = u.logic_params[1];
    let color_shift = u.logic_params[2];
    let brightness = u.logic_params[3];
    
    let enable_plasma = u.feature_flags[0];
    
    let base = textureSample(t, s, uv);
    
    if (enable_plasma < 0.5) {
        return vec4<f32>(base.rgb * brightness, 1.0);
    }
    
    let t_scaled = time * speed;
    let v1 = sin(uv.x * frequency + t_scaled);
    let v2 = sin(uv.y * frequency + t_scaled * 0.7);
    let v3 = sin((uv.x + uv.y) * frequency + t_scaled * 0.5);
    let v4 = sin(length(uv - vec2<f32>(0.5, 0.5)) * frequency * 2.0 - t_scaled);
    
    let plasma = (v1 + v2 + v3 + v4) * 0.25;
    
    let r = sin(plasma * 3.14159 + 0.0) * 0.5 + 0.5;
    let g = sin(plasma * 3.14159 + 2.094) * 0.5 + 0.5;
    let b = sin(plasma * 3.14159 + 4.188) * 0.5 + 0.5;
    
    var plasma_color = vec3<f32>(r, g, b) * color_shift;
    var final_color = mix(base.rgb, plasma_color, 0.6);
    final_color = final_color * brightness;
    
    return vec4<f32>(final_color, 1.0);
}