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



### WGSL Shader



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



## License

MIT