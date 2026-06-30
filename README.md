# ScreenAnimation

GPU-accelerated screen animations and wallpaper engine for Windows, built with Rust/WGPU.

## Features

- **GPU-accelerated** rendering
- **Multi-monitor** support
- **Live Wallpaper**: Mouse-reactive desktop background with wave distortion
- **Screen Transitions**: Capture, detach, move, and land screen content with 3D effects
- **Audio Sync**: WAV sound effects triggered by animation events
- **Hot Reload**: Shaders reloaded automatically on file changes

## Quick Start

### Build

```bash
cargo build --release
```

### Run Examples

```bash
# Live wallpaper: mouse-reactive wave distortion on desktop background
target\release\animationengine.exe Wallpaper examples\livewallpaper.flow

# Screen transition: capture, detach, move to other screen, land
target\release\animationengine.exe Animation examples\screentransition.flow
```

## Creating Custom Animations

### 1. Directory Structure

```
my_animation/
├── config.toml       # Configuration and parameters
├── shader.wgsl       # WGSL shader code
├── background.png    # (Optional) Background image
├── *.wav             # (Optional) Audio files
└── *.png/*.jpg       # (Optional) Textures
```

### 2. Build Package

```bash
target\release\builder.exe --input my_animation --output my_animation.flow
```

### 3. Run

```bash
target\release\animationengine.exe Wallpaper my_animation.flow
target\release\animationengine.exe Animation my_animation.flow
```

## Example Animations

### Live Wallpaper

Mouse-reactive wave distortion on live desktop screenshot.

**Parameters:**
- `speed`: Wave animation speed
- `amplitude`: Wave distortion strength
- `frequency`: Wave density
- `brightness`: Overall brightness multiplier
- `mouse_influence`: Enable mouse-reactive warping

**Shader:** `fs_live_wallpaper`

### Screen Transition

Multi-step sequence: capture screen with flash, detach with lift, move to target position with perspective, land and stabilize.

**Sequence Steps:**
1. `capture` (0.5s): Flash effect on screenshot
2. `detach` (1.5s): Lift off screen
3. `move` (2.0s): Move sideways with perspective
4. `land` (1.0s): Settle down
5. `stable` (infinite): Hold final state

**Shaders:** `fs_capture`, `fs_detach`, `fs_move`, `fs_land`, `fs_stable`

## Configuration Reference

### config.toml

```toml
# Logic parameters (passed to shader as vec4)
[logic]
param1 = 1.0
param2 = 0.0
param3 = 0.0
param4 = 0.0

# Feature flags (passed as 0.0 or 1.0)
[features]
feature1 = true
feature2 = false

# Mode: "wallpaper" or "animation"
mode = "wallpaper"

# Fragment shader entry point
shader = "fs_default"

# Sequence mode (optional)
[[sequence]]
name = "intro"
duration_ms = 1000
shader_entry = "fs_intro"
sound = "intro.wav"
```

### WGSL Shader

```wgsl
struct Uniforms {
    mouse: vec2f,         // Cursor position (0-1)
    offset: vec2f,        // Translation offset
    scale: f32,           // Scale factor
    time: f32,            // Elapsed seconds
    logic_params: vec4f,  // [p1, p2, p3, p4]
    feature_flags: vec4f  // [f1, f2, f3, f4]
};

@group(0) @binding(0) var tex0: texture_2d<f32>;
@group(0) @binding(1) var samp0: sampler;
@group(1) @binding(0) var<uniform> u: Uniforms;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4f {
    // Fullscreen quad (6 vertices)
    ...
}

@fragment
fn fs_default(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    let uv = coord.xy / vec2f(textureDimensions(tex0));
    let color = textureSample(tex0, samp0, uv);
    return vec4f(color.rgb, 1.0);
}
```

## Architecture

### Engine Components

- `engine`: WGPU core (device, pipelines, bind groups)
- `loader`: .flow package parsing (ZIP archives)
- `logic`: Uniform buffer calculations per frame
- `windows`: Windows API integration for window management

### Data Flow

```
config.toml + shader.wgsl + assets
    ↓ builder.exe
package.flow (ZIP)
    ↓ animationengine.exe
FlowPackage → GpuCore → MonitorWindows → Render Loop
```

## Performance

- Startup: ~1s (GPU init + shader compilation)
- Memory: ~50MB base
- Frametime: <16ms (60 FPS)

## Documentation

- [.flow Package Format](docs/format.md)
- [Architecture Overview](docs/architecture.md)
- [Building Guide](docs/building.md)

## License

MIT