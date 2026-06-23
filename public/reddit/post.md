# Showcase: ScreenAnimation - GPU-accelerated screen animations for Windows

Hey r/rust and r/programming,

I've been working on a GPU-accelerated screen animation and wallpaper engine for Windows, built with Rust and WGPU. Wanted to share the project and get feedback from the community.

## What is it?

ScreenAnimation is a real-time animation engine that renders WGSL shaders directly to your screen, either as transparent overlays or embedded as desktop wallpapers behind your icons.

**Key features:**
- 🎨 Hardware-accelerated rendering via WGPU
- 🖥️ Multi-monitor support
- 🎵 Built-in audio playback
- 🎬 Two modes: Simple (V1) and Sequence-based (V2)
- 📦 Custom `.flow` package format (ZIP-based)

## Tech Stack

- **Language**: Rust (2021 edition)
- **Graphics**: WGPU 0.19 + WGSL shaders
- **Audio**: Rodio
- **Windows Integration**: windows-rs crate with Win32 APIs
- **CLI**: clap

## Architecture Highlights

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   loader.rs │    │   engine.rs │    │   logic.rs  │
│  .flow ZIP  │ →  │  WGPU Core  │ ←  │  Uniforms   │
└─────────────┘    └─────────────┘    └─────────────┘
                          ↓
                   ┌─────────────┐
                   │ windows.rs  │
                   │  HWND + API │
                   └─────────────┘
```

- **Modular design**: Clear separation between loading, rendering, and system integration
- **Zero-copy where possible**: Shared `Arc<Vec<u8>>` for audio data
- **Performance**: Single draw call per frame, 208-byte uniform updates

## The `.flow` Package Format

Packages are ZIP archives containing:
- `config.toml` - Parameters and sequence definitions
- `shader.wgsl` - WGSL fragment shaders
- `*.wav` - Audio files
- `background.png` - Wallpaper background (optional)

**Example fragment shader:**
```wgsl
struct Uniforms {
    mouse: vec2f,
    time: f32,
    logic_params: vec4f,
    feature_flags: vec4f
};

@group(1) @binding(0) var<uniform> u: Uniforms;

@fragment
fn fs_default(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    let uv = coord.xy / vec2f(textureDimensions(tex0));
    let bg = textureSample(tex0, samp0, uv);
    
    var color = bg.rgb;
    color.r += sin(u.time * u.logic_params[0] + u.mouse.x * 3.14) * 0.1;
    
    return vec4f(color, bg.a);
}
```

## Performance

- 60 FPS on integrated graphics
- ~50 MB base memory footprint
- ~12 KB/s uniform buffer bandwidth per monitor
- Lock-free mouse tracking with atomics

## Build & Run

```bash
git clone <repo>
cd Build_ScreenAnimation
cargo build --release

# Animation mode
animationengine.exe Animation assets\animation1.flow

# Wallpaper mode
animationengine.exe Wallpaper assets\wallpaper1.flow
```

**Requirements:**
- Windows 10/11
- Rust 1.70+
- Visual Studio Build Tools
- GPU with Vulkan/DX12/Metal support

## Lessons Learned

1. **WGPU is production-ready**: Used it for real-time rendering with no issues
2. **Rust + Win32 is viable**: The `windows-rs` crate is excellent
3. **CLI-first design**: clap made argument parsing trivial
4. **WGSL is powerful**: Shader hot-reloading is easy when shaders are just strings

## Open to Feedback

- Architecture improvements
- WGSL best practices
- Performance optimizations
- Cross-platform ideas (would love to port to Linux with Wayland)

## Links

- **Source**: [GitHub](<repository-url>)
- **Documentation**: See `docs/` folder in repo
- **Examples**: `assets/` folder with sample .flow packages

## What's Next?

- [ ] Video texture support (MP4/WebM)
- [ ] Hot-shader reload during runtime
- [ ] Lua/WGSL scripting for complex logic
- [ ] Linux port (Wayland + wgpu)
- [ ] Plugin system for effects

Would love to hear your thoughts, especially around:
- The uniform buffer layout (208 bytes, is this optimal?)
- Multi-monitor synchronization
- Shader debugging workflows
- Potential security considerations for loading external .flow files

Thanks for reading! 🦀

---

**TL;DR**: Windows animation engine in Rust + WGPU. Renders WGSL shaders as overlays/wallpapers. Custom .flow package format. Feedback welcome.

Edit: Formatting fixes
Edit 2: Thanks for the awards! Will answer questions in the morning.