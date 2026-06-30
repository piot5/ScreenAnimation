struct Uniforms {
    mouse: vec2<f32>,
    offset: vec2<f32>,
    scale: f32,
    time: f32,
    logic_params: vec4<f32>,
    feature_flags: vec4<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;
@group(1) @binding(0) var<uniform> u: Uniforms;

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
    
    // Calculate scale and offset from uniforms
    let scale = u.scale;
    let offset = u.offset;
    
    let p = pos[in_vertex_index];
    // Apply scale from center, then offset
    out.clip_position = vec4<f32>((p * scale) + offset, 0.0, 1.0);
    out.tex_coords = uv[in_vertex_index];
    return out;
}

@fragment
fn fs_default(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}

// Step 1: Fullscreen display (scale=1.0, offset=(0,0))
@fragment
fn fs_pulse(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    
    // Sequence phases based on time:
    // 0.0 - 0.5s: Fullscreen
    // 0.5 - 1.5s: Shrink to 0.95
    // 1.5 - 2.0s: Restore to 1.0
    // 2.0+: Static
    
    let shrink_amount = u.logic_params[0] * 0.05; // p1: shrink factor (0.0-0.2 -> actual)
    let mut current_scale = 1.0;
    
    if u.time > 0.5 && u.time < 1.5 {
        // Shrink phase
        let progress = (u.time - 0.5) / 1.0;
        current_scale = 1.0 - shrink_amount * progress;
    } else if u.time >= 1.5 && u.time < 2.0 {
        // Restore phase
        let progress = (u.time - 1.5) / 0.5;
        current_scale = 1.0 - shrink_amount * (1.0 - progress);
    }
    
    // Apply coordinate transformation for shrink (from center)
    let centered_uv = in.tex_coords - vec2<f32>(0.5, 0.5);
    let scaled_uv = centered_uv / current_scale + vec2<f32>(0.5, 0.5);
    
    // Check bounds
    if scaled_uv.x >= 0.0 && scaled_uv.x <= 1.0 && scaled_uv.y >= 0.0 && scaled_uv.y <= 1.0 {
        let col = textureSample(t_diffuse, s_diffuse, scaled_uv);
        
        // Edge glow during shrink
        let edge_dist = min(min(scaled_uv.x, 1.0 - scaled_uv.x), min(scaled_uv.y, 1.0 - scaled_uv.y));
        let glow = smoothstep(0.0, 0.05, edge_dist);
        
        return col * (0.8 + 0.2 * glow);
    }
    
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}

// Step 2: Flow left - slide from right to left
@fragment
fn fs_flow_left(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    
    // Slide left animation (offset passed via uniforms)
    // offset.x goes from 0.0 -> -1.0 (moves left)
    
    // Calculate offset based on time
    let slide_speed = u.logic_params[2]; // p3: slide speed
    let progress = clamp(u.time * slide_speed, 0.0, 1.0);
    
    // Ease-out: fast start, slow end
    let eased = 1.0 - pow(1.0 - progress, 2.0);
    
    // Current offset (moving left = negative x)
    let current_offset_x = -eased * 1.5;
    
    // Sample UV with offset
    let adjusted_uv = in.tex_coords - vec2<f32>(current_offset_x, 0.0);
    
    if adjusted_uv.x >= 0.0 && adjusted_uv.x <= 1.0 {
        let col = textureSample(t_diffuse, s_diffuse, adjusted_uv);
        
        // Motion blur trail (trailing edge on right)
        let blur_samples = 3;
        var blurred = vec3<f32>(0.0);
        for (var i = 0; i < blur_samples; i++) {
            let t = f32(i) / f32(blur_samples - 1);
            let trail_uv = adjusted_uv + vec2<f32>(t * 0.03, 0.0);
            if trail_uv.x >= 0.0 && trail_uv.x <= 1.0 {
                blurred += textureSample(t_diffuse, s_diffuse, trail_uv).rgb;
            }
        }
        blurred = blurred / f32(blur_samples);
        
        return vec4<f32>(blurred, 1.0);
    }
    
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}

// Step 3: Flow right (reserved, not used in this sequence)
@fragment
fn fs_flow_right(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let edge = smoothstep(0.9, 1.0, in.tex_coords.x + u.offset.x * 0.5);
    return color + vec4<f32>(edge, edge * 0.5, 0.0, 0.0);
}

@fragment
fn fs_flow_up(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}

@fragment
fn fs_flow_down(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}