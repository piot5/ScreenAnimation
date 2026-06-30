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

    // Parameters from config.toml logic section (p1-p4)
    let wave_speed = u.logic_params[0];      // p1: wave animation speed
    let wave_amplitude = u.logic_params[1];  // p2: wave distortion strength
    let wave_frequency = u.logic_params[2];  // p3: wave density
    let brightness = u.logic_params[3];      // p4: brightness multiplier

    // Feature flags from config.toml features section (f1-f4)
    let enable_wave = u.feature_flags[0];        // f1: enable wave effect
    let enable_mouse_warp = u.feature_flags[1];  // f2: enable mouse-reactive warping
    let show_vignette = u.feature_flags[2];      // f3: show vignette effect

    // Calculate distance from mouse for radial distortion
    let dist = distance(uv, mouse);

    // Create wave effect based on distance and time
    var wave = 0.0;
    if (enable_wave > 0.5) {
        wave = sin(dist * wave_frequency - time * wave_speed) * wave_amplitude;
    }

    // Apply mouse-reactive deformation
    var warped_uv = uv;
    if (enable_mouse_warp > 0.5) {
        let direction = normalize(uv - mouse);
        let strength = smoothstep(0.5, 0.0, dist) * 0.15;
        warped_uv = uv + direction * wave * strength;
    } else {
        warped_uv = uv + vec2<f32>(wave * 0.05);
    }

    // Sample the deformed coordinates
    let color = textureSample(t, s, clamp(warped_uv, vec2<f32>(0.0), vec2<f32>(1.0)));

    // Apply vignette effect
    var vignette: f32;
    if (show_vignette > 0.5) {
        vignette = 1.0 - smoothstep(0.3, 1.2, dist) * 0.3;
    } else {
        vignette = 1.0;
    }

    return vec4<f32>(color.rgb * brightness * vignette, 1.0);
}