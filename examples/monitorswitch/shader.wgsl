//! Monitor switch animation shader
//!
//! Sequence:
//! 1. fs_pulse    (0.32s) - Pulse/Scale effect with screenshot
//! 2. fs_woosh    (0.9s)  - Horizontal slide/woosh effect

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

// Step 1: Pulse effect - scale in/out
@fragment
fn fs_pulse(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let progress = clamp(u.time * 3.0, 0.0, 1.0); // ~0.33s duration
    
    // Pulse: scale from 1.0 → 0.95 → 1.0
    let pulse_intensity = u.logic_params[1]; // p2 = 0.05
    let scale = 1.0 - pulse_intensity * sin(progress * 3.14159);
    
    // Center-weighted scaling
    let centered = (uv - vec2<f32>(0.5, 0.5)) / scale + vec2<f32>(0.5, 0.5);
    
    let col = textureSample(tex0, samp0, clamp(centered, vec2<f32>(0.0), vec2<f32>(1.0)));
    
    // Subtle flash at peak
    let flash = sin(progress * 3.14159) * 0.1;
    
    return vec4<f32>(col.rgb + flash, 1.0);
}

// Step 2: Woosh effect - horizontal slide
@fragment
fn fs_woosh(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let progress = clamp(u.time * 1.2, 0.0, 1.0); // ~0.83s duration
    
    // Ease-out cubic: fast start, smooth end
    let eased = 1.0 - pow(1.0 - progress, 3.0);
    
    // Horizontal offset from config p1 (0.15 = 15% screen width)
    let woosh_offset = u.logic_params[0];
    
    // Slide from right to left (or left to right depending on sign)
    var shifted_uv = uv;
    shifted_uv.x = shifted_uv.x - eased * woosh_offset;
    
    // Add slight motion blur trail
    let blur_amount = eased * 0.02;
    var blurred = vec3<f32>(0.0);
    let samples = 3;
    for (var i = 0; i < samples; i++) {
        let offset_x = (f32(i) / f32(samples - 1) - 0.5) * blur_amount;
        let sample_uv = shifted_uv + vec2<f32>(offset_x, 0.0);
        blurred += textureSample(tex0, samp0, clamp(sample_uv, vec2<f32>(0.0), vec2<f32>(1.0))).rgb;
    }
    blurred = blurred / f32(samples);
    
    return vec4<f32>(blurred, 1.0);
}

// Fallback: static screenshot
@fragment
fn fs_default(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let col = textureSample(tex0, samp0, uv);
    return vec4<f32>(col.rgb, 1.0);
}