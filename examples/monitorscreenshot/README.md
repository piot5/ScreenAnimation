# MonitorScreenshot Fly-In Animation

## Overview

This example creates a per-monitor screenshot animation that starts with a black background and flies in from a specified direction. Each monitor displays its own screenshot that animates into view.

**Animation Sequence:**
1. **Black screen** (0.25s) - Fade from black
2. **Fly-in effect** (0.5-2.0s) - Screenshot slides in with motion blur trail
3. **Settle glow** (0.5s) - Subtle pulse on arrival

**Key Features:**
- Per-monitor DXGI screenshot capture (GPU-accelerated, ~1ms per monitor)
- Directional fly-in animation (right, left, top, or bottom)
- Motion blur trail during flight
- Edge glow effect during and after animation
- Fully configurable speed, blur intensity, and direction

## How It Works

### Screenshot Capture

On initialization, each monitor captures its current desktop state using DXGI Output Duplication:

```
1. EnumDisplayMonitors → get all monitor rectangles
2. For each monitor:
   a. DXGI capture → BGRA pixel buffer (~1ms)
   b. Upload to GPU texture
   c. Create overlay window
   d. Start fly-in animation
```

### Animation Pipeline

The shader (`shader.wgsl`) implements a 3-stage animation:

**Stage 1: Black (0.0s - 0.25s)**
- Full black screen
- Prepares viewer for animation

**Stage 2: Fly-in (0.25s - varies)**
- Screenshot slides in from specified direction
- Motion blur trail follows movement
- Edge glow brightens leading edge
- Ease-out deceleration for smooth landing

**Stage 3: Settle (varies - varies + 0.5s)**
- Static screenshot display
- Subtle glow pulse fades out
- Animation complete

### Configuration

#### config.toml

```toml
mode = "animation"
shader = "fs_flyin"

[logic]
p1 = 0      # Direction: 0=right, 1=left, 2=top, 3=bottom
p2 = 1.0    # Speed: 0.5=slow, 1.0=normal, 2.0=fast
p3 = 0.03   # Motion blur: 0.0=off, 0.03=low, 0.05=high
p4 = 0.0    # Reserved

[features]
f1 = true   # Enable motion blur trail
f2 = true   # Enable edge glow effect

[[sequence]]
name = "flyin"
duration_ms = 1000
shader_entry = "fs_flyin"
```

**Parameter Details:**

| Parameter | Type | Range | Description |
|-----------|------|-------|-------------|
| `p1` | int | 0-3 | Fly-in direction |
| `p2` | float | 0.1-3.0 | Animation speed multiplier |
| `p3` | float | 0.0-0.1 | Motion blur intensity |
| `p4` | float | - | Reserved for future use |
| `f1` | bool | - | Enable motion blur trail |
| `f2` | bool | - | Enable glow effect |

#### Direction Options

```wgsl
0 = From Right   → flies leftward from right edge
1 = From Left    → flies rightward from left edge
2 = From Top     → flies downward from top edge
3 = From Bottom  → flies upward from bottom edge
```

## Building a .flow Package

To create a distributable `.flow` package:

### Quick Build Script (Windows)

```powershell
# Create output directory
mkdir -p build

# Package as ZIP (monitorscreenshot.flow)
Compress-Archive -Path config.toml, shader.wgsl -DestinationPath build/monitorscreenshot.flow

# Verify package
Expand-Archive -Path build/monitorscreenshot.flow -DestinationPath build/test -Force
```

### Manual Build

```bash
# Navigate to example directory
cd examples/monitorscreenshot

# Create ZIP archive (must be named .flow)
zip monitorscreenshot.flow config.toml shader.wgsl

# Verify structure
unzip -l monitorscreenshot.flow
# Should show:
#   config.toml
#   shader.wgsl
```

## Usage

### With ScreenAnimation Application

```bash
# Run the example (if built into the application)
screen_animation --flow examples/monitorscreenshot/monitorscreenshot.flow

# Or specify custom config
screen_animation --flow my_custom_animation.flow
```

### Integration Code Example

```rust
use screen_animation::{loader::FlowPackage, windows::init_windows, engine::GpuCore};

// Load the flow package
let flow = FlowPackage::load("monitorscreenshot.flow")?;

// Initialize GPU
let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
let gpu = pollster::block_on(GpuCore::new(&instance, &flow.shader_src, &["fs_flyin"]))?;

// Create windows for all monitors (wallpaper mode)
let windows = unsafe {
    init_windows(&gpu, &instance, class_name, hinstance, true, &flow)
};

// Render loop (60 FPS)
loop {
    let uniforms = logic.update(&flow, mouse_pos);
    // Upload uniforms and render...
}
```

## Dependencies

- **wgpu 0.19** - GPU rendering
- **windows crate** - Win32 API access (DXGI, GDI, window management)
- **image crate** - Image decoding (for future textures)
- **zip crate** - Package loading

## Shader Details

### Vertex Shader

Creates a fullscreen quad with two triangles:
```wgsl
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput
```

### Fragment Shader Functions

**`get_fly_offset(direction, progress)`**
- Calculates UV offset based on direction and progress (0.0-1.0)
- Returns offset large enough to go fully off-screen (1.5x)

**`fs_flyin(in)`**
- Main animation fragment shader
- Handles all 3 animation stages
- Implements motion blur via multi-sample trail
- Adds glow effect if enabled

### Motion Blur Implementation

```wgsl
// Sample 5 points along motion path
for (var i = 0; i < 5; i++) {
    let trail_progress = clamp(eased - i * blur_intensity * 0.5, 0.0, 1.0);
    let trail_offset = get_fly_offset(direction, trail_progress);
    // Accumulate samples with distance-based weights
}
```

### Glow Effect

During flight: Brightens image based on speed (faster = brighter)
On settle: Exponential fade-out pulse (0.1 intensity over 0.5s)

## Performance

| Stage | Time | Description |
|-------|------|-------------|
| DXGI capture | ~1ms per monitor | GPU screenshot capture |
| GPU upload | ~2ms per monitor | Texture data upload |
| Window creation | ~5ms per monitor | HWND + WGPU surface |
| **Total init** | **~50-100ms** | **2-4 monitors** |
| Per-frame render | ~1ms | Shader execution + present |

## Troubleshooting

**Black screen after animation:**
- Verify config.toml `shader = "fs_flyin"` matches actual function name
- Check that all feature flags are properly formatted as booleans

**Animation doesn't play:**
- Ensure `duration_ms > 0` in sequence
- Verify shader entry point exists: `fn fs_flyin(...)`

**No motion blur:**
- Set `f1 = true` in `[features]`
- Set `p3 > 0.0` (e.g., 0.03)

**Wrong fly direction:**
- Check `p1` value (0-3)
- Remember: 0=right, 1=left, 2=top, 3=bottom

## Future Enhancements

- [ ] Custom background images (currently uses live screenshot)
- [ ] Multiple sequences with transitions
- [ ] Audio synchronization (woosh sound on fly-in)
- [ ] Per-monitor directional control
- [ ] Rotation/tilt during flight
- [ ] Scale effects (zoom in/out)

## Related Files

- `shader.wgsl` - WGSL shader code
- `config.toml` - Animation configuration
- `../monitorswitch/` - Similar monitor-based example with pulse+woosh

## License

MIT - See repository root for details.