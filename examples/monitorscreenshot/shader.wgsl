//! Per-monitor screenshot fly-in animation
//!
//! Sequence:
//! 1. Start with black background
//! 2. Screenshot flies in from specified direction
//! 3. Motion blur trail during flight
//! 4. Optional glow effect on arrival
//!
//! Animation stages:
//! - 0.0-0.3: Black screen (preparation)
//! - 0.3-1.0: Fly-in with motion blur
//! - 1.0+: Static screenshot with optional glow

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

// Get fly-in offset based on direction and progress
fn get_fly_offset(direction: i32, progress: f32) -> vec2<f32> {
    // Direction: 0=right, 1=left, 2=top, 3=bottom
    // Progress 0.0 = off-screen, 1.0 = settled
    let p = 1.0 - progress; // Start at 1.0 (fully off), end at 0.0
    
    if direction == 0 {
        // From right: start with positive x offset
        return vec2<f32>(p * 1.5, 0.0);
    } else if direction == 1 {
        // From left: start with negative x offset
        return vec2<f32>(-p * 1.5, 0.0);
    } else if direction == 2 {
        // From top: start with positive y offset
        return vec2<f32>(0.0, -p * 1.5);
    } else {
        // From bottom: start with negative y offset
        return vec2<f32>(0.0, p * 1.5);
    }
}

// Main fly-in animation fragment shader
@fragment
fn fs_flyin(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let direction = i32(u.logic_params[0]);
    let speed_mult = u.logic_params[1];
    let blur_intensity = u.logic_params[2];
    let enable_motion_blur = u.feature_flags[0] > 0.5;
    let enable_glow = u.feature_flags[1] > 0.5;
    
    // Animation timeline:
    // 0.0 - 0.25s: Black screen
    // 0.25s - 1.25s: Fly-in (adjustable by speed)
    // 1.25s+: Static with glow
    
    let black_duration = 0.25;
    let fly_duration = 1.0 / speed_mult;
    let total_anim = black_duration + fly_duration;
    
    var final_color = vec3<f32>(0.0, 0.0, 0.0);
    
    if u.time < black_duration {
        // Stage 1: Black screen
        final_color = vec3<f32>(0.0, 0.0, 0.0);
    } else if u.time < total_anim {
        // Stage 2: Fly-in animation
        let fly_time = u.time - black_duration;
        let progress = clamp(fly_time / fly_duration, 0.0, 1.0);
        
        // Ease-out for smooth deceleration
        let eased = 1.0 - pow(1.0 - progress, 3.0);
        
        // Get directional offset
        let offset = get_fly_offset(direction, eased);
        
        // Motion blur trail during flight
        if enable_motion_blur && blur_intensity > 0.0 {
            var blurred = vec3<f32>(0.0);
            let samples = 5;
            var total_weight = 0.0;
            
            for (var i = 0; i < samples; i++) {
                let t = f32(i) / f32(samples - 1);
                // Sample along the motion path (trailing behind)
                let trail_progress = clamp(eased - t * blur_intensity * 0.5, 0.0, 1.0);
                let trail_offset = get_fly_offset(direction, trail_progress);
                let sample_uv = uv - trail_offset;
                
                if sample_uv.x >= 0.0 && sample_uv.x <= 1.0 && sample_uv.y >= 0.0 && sample_uv.y <= 1.0 {
                    let weight = 1.0 - abs(t - 0.5) * 0.5;
                    blurred += textureSample(tex0, samp0, sample_uv).rgb * weight;
                    total_weight += weight;
                }
            }
            
            if total_weight > 0.0 {
                final_color = blurred / total_weight;
            }
        } else {
            // No motion blur, just shifted UV
            let sample_uv = uv - offset;
            if sample_uv.x >= 0.0 && sample_uv.x <= 1.0 && sample_uv.y >= 0.0 && sample_uv.y <= 1.0 {
                final_color = textureSample(tex0, samp0, sample_uv).rgb;
            }
        }
        
        // Add edge glow during flight
        if enable_glow && progress < 0.8 {
            let glow_strength = (1.0 - progress) * 0.15;
            final_color += vec3<f32>(glow_strength, glow_strength, glow_strength);
        }
    } else {
        // Stage 3: Static screenshot with optional glow pulse
        final_color = textureSample(tex0, samp0, uv).rgb;
        
        // Subtle glow pulse on arrival
        if enable_glow && u.time < total_anim + 0.5 {
            let settle_time = u.time - total_anim;
            let pulse = exp(-settle_time * 8.0) * 0.1;
            final_color += vec3<f32>(pulse, pulse, pulse);
        }
    }
    
    return vec4<f32>(final_color, 1.0);
}

// Fallback: simple static display
@fragment
fn fs_default(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let col = textureSample(tex0, samp0, uv);
    return vec4<f32>(col.rgb, 1.0);
}