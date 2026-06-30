//! Screenshot-to-other-monitor transition shader
//!
//! Sequence:
//! 1. fs_capture    (0.5s) - Screenshot mit Kamerablitz-Effekt
//! 2. fs_detach     (1.0s) - Screeninhalt löst sich vom Hintergrund
//! 3. fs_move       (2.0s) - Inhalt gleitet zum anderen Monitor
//! 4. fs_land       (1.0s) - Inhalt setzt sich ab/an

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

// Step 1: Screenshot mit Kamera-Blitz-Effekt (0.5s)
// Der Blitz blendet von weiß auf transparent
@fragment
fn fs_capture(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let col = textureSample(tex0, samp0, uv);

    // Kamera-Blitz: startet weiß und blendet schnell aus
    // time = 0 → voller Blitz, time = 0.5 → Blitz weg
    let flash_intensity = 1.0 - clamp(u.time * 3.0, 0.0, 1.0);
    let flashed = mix(col.rgb, vec3<f32>(1.0), flash_intensity * 0.8);

    // Leichte Vignette für Kamera-Feeling
    let dist = distance(uv, vec2<f32>(0.5, 0.5));
    let vignette = 1.0 - smoothstep(0.3, 0.9, dist) * 0.3;

    return vec4<f32>(flashed * vignette, 1.0);
}

// Step 2: Screen löst sich ab (1.0s) und hebt nach oben
// Mit Schattenwurf und Schräglage
@fragment
fn fs_detach(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let progress = clamp(u.time * 1.2, 0.0, 1.0); // 0→1 über ~0.83s

    // Ease-out cubic: schneller Start, sanftes Ende
    let eased = 1.0 - pow(1.0 - progress, 3.0);

    // Anheben: Inhalt wandert nach oben
    let lift_offset = eased * 0.15;
    var shifted_uv = uv;
    shifted_uv.y = shifted_uv.y - lift_offset;

    // Perspektive: oberer Rand schmaler (3D-Effekt)
    let y_center = shifted_uv.y - 0.5;
    let perspective_scale = 1.0 - eased * y_center * 0.15;

    // Horizontal leicht eindrehen
    let rotate_angle = eased * 0.08;
    let cos_a = cos(rotate_angle);
    let sin_a = sin(rotate_angle);
    let centered = shifted_uv - vec2<f32>(0.5, 0.5);
    let rotated = vec2<f32>(
        centered.x * cos_a - centered.y * sin_a,
        centered.x * sin_a + centered.y * cos_a
    ) * perspective_scale + vec2<f32>(0.5, 0.5);

    // Textur sample mit Clamp
    let col = textureSample(tex0, samp0, clamp(rotated, vec2<f32>(0.0), vec2<f32>(1.0)));

    // Schatten unter dem sich ablösenden Screen
    let shadow_y = 0.5 + lift_offset * 1.2;
    let shadow_dist = abs(uv.y - shadow_y);
    let shadow = 0.3 * exp(-shadow_dist * 20.0) * eased;

    // Alpha: Screen wird minimal transparent beim Anheben
    let alpha = 1.0 - eased * 0.1;

    return vec4<f32>(col.rgb - vec3<f32>(shadow * 0.3), alpha);
}

// Step 3: Inhalt gleitet zum anderen Monitor (2.0s)
// Horizontale Verschiebung mit gleichzeitiger Skalierung
@fragment
fn fs_move(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let progress = clamp(u.time * 0.6, 0.0, 1.0); // 0→1 über ~1.67s

    // Ease-in-out: sanft starten, sanft enden
    // WGSL select(a, b, cond) → cond ? b : a
    let ease_in = 2.0 * progress * progress;
    let ease_out = 1.0 - pow(-2.0 * progress + 2.0, 2.0) * 0.5;
    let eased = select(ease_out, ease_in, progress < 0.5);

    // Move-Parameter aus Config: p1 = Verschiebung X
    let move_distance = u.logic_params[0];

    // Horizontale Verschiebung: Inhalt gleitet nach rechts
    var shifted_uv = uv;
    shifted_uv.x = shifted_uv.x - eased * move_distance;

    // Leichtes Schrumpfen während der Bewegung
    let shrink = 1.0 - eased * 0.08;
    // Leichte vertikale Schwingung (als ob es in der Luft schwebt)
    let float_bob = sin(progress * 3.14159 * 4.0) * 0.01 * eased;
    shifted_uv.y = shifted_uv.y + float_bob;

    // Zentrieren für Skalierung
    let centered = (shifted_uv - vec2<f32>(0.5, 0.5)) / shrink + vec2<f32>(0.5, 0.5);

    // Motion Blur: horizontal verschmieren während der Bewegung
    let speed = 0.6; // Geschwindigkeit
    var blur_color = vec3<f32>(0.0);
    let samples = 5;
    for (var i = 0; i < samples; i++) {
        let offset_x = f32(i) / f32(samples - 1) - 0.5;
        let blur_uv = centered + vec2<f32>(offset_x * eased * 0.02, 0.0);
        blur_color += textureSample(tex0, samp0, clamp(blur_uv, vec2<f32>(0.0), vec2<f32>(1.0))).rgb;
    }
    blur_color = blur_color / f32(samples);

    return vec4<f32>(blur_color, 1.0);
}

// Step 4: Inhalt setzt sich ab (1.0s)
// Sanftes Einfedern an der Zielposition
@fragment
fn fs_land(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let progress = clamp(u.time * 1.2, 0.0, 1.0); // 0→1 über ~0.83s

    // Overshoot-Bounce: kurz drüber hinausfedern
    let bounce = 1.0 + sin(progress * 3.14159 * 3.0) * (1.0 - progress) * 0.03;

    // Finale Move-Position
    let move_distance = u.logic_params[0];

    // Inhalt kommt von der Seite und federt ein
    var shifted_uv = uv;
    shifted_uv.x = shifted_uv.x - move_distance * (1.0 - progress * bounce);

    // Zurückskalieren auf Normalgröße
    let scale_back = 1.0 - (1.0 - progress) * 0.06;
    let centered = (shifted_uv - vec2<f32>(0.5, 0.5)) / scale_back + vec2<f32>(0.5, 0.5);

    let col = textureSample(tex0, samp0, clamp(centered, vec2<f32>(0.0), vec2<f32>(1.0)));

    // Leichter Schattenwurf beim Landen (kurz verstärkt)
    let shadow_intensity = (1.0 - progress) * 0.2;
    let col_with_shadow = col.rgb - vec3<f32>(shadow_intensity * 0.2);

    return vec4<f32>(col_with_shadow, 1.0);
}

// Fallback: stabiler Screen
@fragment
fn fs_default(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let col = textureSample(tex0, samp0, uv);
    return vec4<f32>(col.rgb, 1.0);
}