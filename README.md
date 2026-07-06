# ScreenAnimation

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust 1.70+](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org)
[![WGPU](https://img.shields.io/badge/graphics-WGPU-orange.svg)](https://wgpu.rs)

GPU-accelerated screen animations and wallpaper engine for Windows, built with Rust/WGPU. Features a novel **.flow plugin system** for portable, self-contained animations.

> **What makes this different:** Unlike closed-source wallpaper engines, ScreenAnimation lets you **write WGSL shaders** and package them as portable `.flow` files that run anywhere without recompilation.

## Features

- 🎨 **GPU-accelerated** rendering (WGPU + DXGI)
- 🖥️ **Multi-monitor** support with per-monitor animations
- 🌊 **Live Wallpaper**: Mouse-reactive desktop background with wave distortion
- 🎬 **Screen Transitions**: Capture, detach, move, and land screen content with 3D effects
- 🔊 **Audio Sync**: WAV sound effects triggered by animation events
- ♻️ **Hot Reload**: Shaders reloaded automatically on file changes (live development)
- 📦 **Plugin System**: `.flow` format packages animations (config + shader + assets) as portable ZIP files
- ⚡ **Performance**: <16ms per frame (60+ FPS), ~50MB memory footprint

## Quick Start

### Build

```bash
cargo build --release
```

Binaries in `target/release/`:
- `animationengine.exe` — Runtime engine
- `builder.exe` — Package builder

### Run Examples

```bash
# Live wallpaper: mouse-reactive wave distortion
./target/release/animationengine Wallpaper examples/livewallpaper.flow

# Screen transition: capture → detach → move → land
./target/release/animationengine Animation examples/screentransition.flow

# Plasma effect
./target/release/animationengine Wallpaper examples/plasma.flow

# Kaleidoscope
./target/release/animationengine Wallpaper examples/kaleidoscope.flow
```

## The .flow Plugin System

The **core innovation** of ScreenAnimation is the `.flow` format — a portable animation package.

### What is .flow?

A `.flow` file is a ZIP archive containing:
```
animation.flow (ZIP)
├── config.toml       # Parameters, metadata, timing
├── shader.wgsl       # GPU shader code
├── background.png    # Optional: background image
├── *.wav             # Optional: audio files
└── *.png/*.jpg       # Optional: textures
```

### How it Works

```
┌─────────────────────────────────────────┐
│  Step 1: Create your animation          │
│  - Write config.toml (parameters)       │
│  - Write shader.wgsl (GPU code)         │
│  - Add assets (images, audio)           │
└────────────────┬────────────────────────┘
                 │
                 ↓
        ┌──────────────────┐
        │  builder.exe     │
        │  (Packager)      │
        └────────┬─────────┘
                 │
                 ↓
        ┌──────────────────┐
        │  animation.flow  │ ← Self-contained package
        │  (ZIP Archive)   │
        └────────┬─────────┘
                 │
                 ↓
┌─────────────────────────────────────────┐
│  Step 2: Run anywhere                   │
│  animationengine Wallpaper animation    │
│  → Works on any ScreenAnimation install │
└─────────────────────────────────────────┘
```

### Example: Create Your First Animation

```bash
# 1. Create directory
mkdir my_wave_effect
cd my_wave_effect

# 2. Write config.toml
cat > config.toml << 'EOF'
[animation]
name = "Wave Effect"
description = "Mouse-reactive wave distortion"

[parameters]
speed = 1.5
amplitude = 0.05
frequency = 3.0
brightness = 1.0
mouse_influence = true
EOF

# 3. Write shader.wgsl (see TUTORIAL.md for full example)
cat > shader.wgsl << 'EOF'
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let wave_offset = sin(input.uv.x * params.z + time * params.x) * params.y;
    let distorted_uv = vec2<f32>(input.uv.x + wave_offset, input.uv.y);
    return textureSample(screen_texture, screen_sampler, distorted_uv);
}
EOF

# 4. Package it
../target/release/builder --input . --output ../my_wave.flow

# 5. Run it!
../target/release/animationengine Wallpaper ../my_wave.flow
```

👉 **Full tutorial:** See [TUTORIAL.md](TUTORIAL.md) for step-by-step guide with shader examples.

## Creating Custom Animations

### Directory Structure

```
my_animation/
├── config.toml           # Configuration and parameters
├── shader.wgsl          # WGSL shader code
├── background.png       # (Optional) Background image
├── *.wav                # (Optional) Audio files
└── *.png/*.jpg          # (Optional) Textures
```

### Step 1: config.toml

```toml
[animation]
name = "My Animation"
version = "1.0.0"
description = "Description here"

[parameters]
speed = 1.0
amplitude = 0.05
frequency = 2.0
brightness = 1.0
mouse_influence = true

[rendering]
target_fps = 60
background_image = "background.png"

[audio]
enabled = false
# audio_file = "soundtrack.wav"
# beat_detection = true
```

### Step 2: shader.wgsl

WGSL shader code. See examples in `examples/` directory.

```wgsl
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Your shader code here
    return textureSample(screen_texture, screen_sampler, input.uv);
}
```

### Step 3: Package & Run

```bash
# Package
./target/release/builder --input my_animation --output my_animation.flow

# Run as wallpaper
./target/release/animationengine Wallpaper my_animation.flow

# Run as transition
./target/release/animationengine Animation my_animation.flow
```

## Example Animations Included

| Animation | Type | Size | Purpose |
|-----------|------|------|---------|
| `livewallpaper.flow` | Wallpaper | 261 MB | Mouse-reactive wave on live desktop |
| `screentransition.flow` | Animation | 290 MB | Multi-step screen transition (capture→detach→move→land) |
| `animation1.flow` | Animation | 134 MB | Complex 3D transition effect |
| `plasma.flow` | Wallpaper | 1.1 KB | Pure procedural plasma (no assets) |
| `kaleidoscope.flow` | Wallpaper | 1.3 KB | Symmetric kaleidoscope effect |
| `wave.flow` | Wallpaper | 1.2 KB | Simple sine wave distortion |
| `monitorswitch.flow` | Animation | 135 MB | Display switching animation |
| `wallpaper1.flow` | Wallpaper | 25 GB | High-resolution wallpaper with complex shaders |

## Architecture

### Engine Components

| Component | Purpose |
|-----------|---------|
| `engine/` | WGPU core (device, pipelines, bind groups) |
| `loader/` | `.flow` package parsing (ZIP deserialization) |
| `logic/` | Uniform buffer calculations per frame |
| `windows/` | Windows API integration (DXGI, window management) |

### Data Flow

```
config.toml + shader.wgsl + assets
    ↓ (builder.exe)
package.flow (ZIP)
    ↓ (animationengine.exe)
FlowPackage → GpuCore → MonitorWindows → Render Loop
```

## Performance

- **Startup:** ~1s (GPU init + shader compilation)
- **Memory:** ~50MB base
- **Frametime:** <16ms (60 FPS on modern GPUs)
- **Multi-monitor:** Linear scaling per monitor

## Platform Support

| OS | Status | Requirements |
|----|--------|--------------|
| **Windows 10/11** | ✅ Full support | DXGI + WGPU |
| **Linux (Wayland)** | ✅ Full support | WGPU + Wayland |
| **Linux (X11)** | ⏳ Partial | WGPU (no window management) |
| **macOS** | ⏳ Planned | WGPU + Metal |

## Building

### Requirements

- Rust 1.70+ ([Install](https://rustup.rs/))
- Windows: MSVC build tools + Windows SDK
- Linux: `libwayland-dev`, `libdrm-dev`

### Compile

```bash
# Full release build
cargo build --release

# Build specific binary
cargo build -p screen_animation --release

# Run tests
cargo test --all
```

## Customization

### Shader Development

Edit `shader.wgsl` while `animationengine` is running — changes reload automatically (hot reload).

### Parameter Tuning

Modify `config.toml` values without rebuilding:
- `speed` — Animation playback speed
- `amplitude` — Effect distortion strength
- `frequency` — Effect pattern density
- `brightness` — Output brightness

### Audio Integration

Enable in config:
```toml
[audio]
enabled = true
audio_file = "soundtrack.wav"
beat_detection = true
```

Access frequency spectrum in shader (0-256 bands):
```wgsl
@group(0) @binding(4) var<storage, read> audio_spectrum: array<f32, 256>;
```

## Documentation

- **[TUTORIAL.md](TUTORIAL.md)** — Step-by-step guide to creating animations
- **[examples/](examples/)** — 10+ pre-made animations with source configs
- **WGSL Reference:** https://www.w3.org/TR/WGSL/

## Comparison: ScreenAnimation vs Alternatives

| Feature | ScreenAnimation | Wallpaper Engine | OBS | Blender |
|---------|-----------------|------------------|-----|---------|
| Open Source | ✅ Yes | ❌ No | ✅ Yes | ✅ Yes |
| User Shaders | ✅ WGSL | ❌ No | ⚠️ Limited | ✅ Yes |
| Portable Format | ✅ .flow | ❌ Proprietary | ❌ No | ⚠️ .blend files |
| Desktop Wallpaper | ✅ Yes | ✅ Yes | ❌ No | ❌ No |
| Screen Transitions | ✅ Yes | ⚠️ Limited | ✅ Yes | ✅ Yes |
| Performance | ⚡ 60+ FPS | ⚡ 60+ FPS | ⚠️ CPU-heavy | ⚠️ Variable |
| Learning Curve | 📚 Medium | ❌ N/A | 📚 Medium | 📚 Steep |

## License

MIT License — see [LICENSE](LICENSE)

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Ideas for Enhancement

- [ ] macOS/Linux native window management
- [ ] Animation marketplace (publish/share `.flow` files)
- [ ] Web-based shader editor + preview
- [ ] Performance profiler for shaders
- [ ] Multi-shader sequence editor (UI)
- [ ] Blender export plugin

## Author

**piot5** — https://github.com/piot5

---

## Quick Links

- 📖 [Full Tutorial](TUTORIAL.md)
- 🎨 [Example Animations](examples/)
- 🐛 [Issue Tracker](https://github.com/piot5/ScreenAnimation/issues)
- 💬 [Discussions](https://github.com/piot5/ScreenAnimation/discussions)

---

<p align="center">
  <strong>ScreenAnimation</strong> — GPU-accelerated wallpapers with shader plugins
  <br/>
  Made with ❤️ in Rust/WGPU
</p>
