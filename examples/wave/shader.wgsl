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
fn fs_wave(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let time = u.time;
    let wave_speed = u.logic_params[0];
    let wave_amplitude = u.logic_params[1];
    let wave_frequency = u.logic_params[2];
    let brightness = u.logic_params[3];
    let enable_waves = u.feature_flags[0];
    let enable_ripple = u.feature_flags[1];
    
    let base = textureSample(t, s, uv);
    
    if (enable_waves < 0.5) {
        return vec4<f32>(base.rgb * brightness, 1.0);
    }
    
    var wave = 0.0;
    let t_scaled = time * wave_speed;
    wave += sin(uv.x * 3.14159 * wave_frequency + t_scaled) * 0.5;
    wave += sin(uv.y * 3.14159 * wave_frequency * 0.8 + t_scaled * 1.3) * 0.5;
    wave += sin((uv.x + uv.y) * 3.14159 * wave_frequency * 0.5 + t_scaled * 0.7) * 0.5;
    wave = wave / 3.0;
    
    var warped_uv = uv;
    warped_uv.x += wave * wave_amplitude;
    warped_uv.y += wave * wave_amplitude * 0.7;
    
    if (enable_ripple > 0.5) {
        let dist = distance(uv, vec2<f32>(0.5, 0.5));
        let ripple = sin(dist * 20.0 - t_scaled * 3.0) * 0.02 * smoothstep(0.5, 0.2, dist);
        warped_uv += ripple;
    }
    
    let col = textureSample(t, s, clamp(warped_uv, vec2<f32>(0.0), vec2<f32>(1.0)));
    return vec4<f32>(col.rgb * brightness, 1.0);
}