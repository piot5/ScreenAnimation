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

fn kaleidoscope_uv(uv: vec2<f32>, segments: f32, angle: f32) -> vec2<f32> {
    let centered = uv - vec2<f32>(0.5, 0.5);
    let dist = length(centered);
    let current_angle = atan2(centered.y, centered.x) + angle;
    let sector_angle = 2.0 * 3.14159 / segments;
    let half_sector = sector_angle * 0.5;
    let a = abs(((current_angle + half_sector) % sector_angle) - half_sector);
    return vec2<f32>(
        cos(a) * dist + 0.5,
        sin(a) * dist + 0.5
    );
}

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
fn fs_intro(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let brightness = u.logic_params[3];
    
    let fade_in = clamp(u.time * 1.5, 0.0, 1.0);
    let col = textureSample(t, s, kaleidoscope_uv(uv, u.logic_params[0], 0.0));
    
    return vec4<f32>(col.rgb * brightness * fade_in, 1.0);
}

@fragment
fn fs_spin(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let segments = u.logic_params[0];
    let rotation_speed = u.logic_params[1];
    let effect_blend = u.logic_params[2];
    let brightness = u.logic_params[3];
    let enable_kaleidoscope = u.feature_flags[0];
    
    let base = textureSample(t, s, uv);
    
    if (enable_kaleidoscope < 0.5) {
        return vec4<f32>(base.rgb * brightness, 1.0);
    }
    
    let angle = u.time * rotation_speed;
    let k_uv = kaleidoscope_uv(uv, segments, angle);
    let k_color = textureSample(t, s, clamp(k_uv, vec2<f32>(0.0), vec2<f32>(1.0)));
    
    let final_color = mix(base.rgb, k_color.rgb, effect_blend);
    return vec4<f32>(final_color * brightness, 1.0);
}

@fragment
fn fs_default(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let segments = u.logic_params[0];
    let effect_blend = u.logic_params[2];
    let brightness = u.logic_params[3];
    let enable_kaleidoscope = u.feature_flags[0];
    
    let base = textureSample(t, s, uv);
    
    if (enable_kaleidoscope < 0.5) {
        return vec4<f32>(base.rgb * brightness, 1.0);
    }
    
    let k_uv = kaleidoscope_uv(uv, segments, 0.0);
    let k_color = textureSample(t, s, clamp(k_uv, vec2<f32>(0.0), vec2<f32>(1.0)));
    
    let final_color = mix(base.rgb, k_color.rgb, effect_blend);
    return vec4<f32>(final_color * brightness, 1.0);
}