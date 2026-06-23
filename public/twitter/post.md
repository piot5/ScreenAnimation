🦀 GPU-accelerated screen animations for Windows – built with Rust + WGPU

New open source project:

• Renders WGSL shaders as transparent overlays or wallpapers
• Multi-monitor support
• Built-in audio playback
• Custom .flow package format (ZIP)

Performance: 60 FPS, ~50 MB base

Tech: Rust, WGPU 0.19, windows-rs, rodio

Example shader:
```wgsl
@fragment
fn fs_default(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    let uv = coord.xy / vec2f(textureDimensions(tex0));
    let bg = textureSample(tex0, samp0, uv);
    return vec4f(bg.rgb + sin(u.time) * 0.1, bg.a);
}
```

Build:
```bash
cargo build --release
animationengine.exe Animation assets/animation1.flow
```

GitHub: <repository-url>

#RustLang #WGPU #GraphicsProgramming #OpenSource #Windows