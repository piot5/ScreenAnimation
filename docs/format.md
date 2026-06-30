# .flow Package Format Specification

## Overview

The `.flow` format is a ZIP-based container for ScreenAnimation content. It bundles configuration, shaders, audio, and textures into a single distributable file.

## File Structure

```
package.flow
├── config.toml       (required)   Configuration and parameters
├── shader.wgsl       (required)   WGSL shader source code
├── background.png    (optional)   Wallpaper background image
├── *.wav             (optional)   Audio files (uncompressed PCM WAV)
└── *.png / *.jpg     (optional)   Additional textures
```

## Compression

- ZIP format with **no compression** (store method) for binary assets
- Text files (config.toml, shader.wgsl) may use default deflate
- Maximum uncompressed package size: 100 MB recommended

## config.toml Reference

### Complete Schema

```toml
# === Mode Configuration ===
mode = "animation"        # "animation", "wallpaper", or "sequence"
shader = "fs_default"     # Fragment shader entry point (V1 only)
direction = "forward"     # Shader direction hint (V1 only)
z_order = "top"           # Z-position: "top", "bottom", "middle"

# === Sequence Configuration (V2/Sequence Mode) ===
# Omit for V1 mode

[[sequence]]
name = "step_identifier"  # Unique step name
duration_ms = 3000         # Duration in milliseconds (0 = infinite)
shader_entry = "fs_intro"  # Fragment shader function
sound = "intro.wav"        # Optional: sound to play at step start
texture = "overlay.png"    # Optional: texture overlay
easing = "easeInOut"       # Optional: easing function hint

[[sequence]]
name = "main_loop"
duration_ms = 0            # 0 = loop forever
shader_entry = "fs_main"
sound = "ambient.wav"

# === Logic Parameters (passed as vec4f to shaders) ===
# Use descriptive names - they are mapped by position
# p1=params[0], p2=params[1], p3=params[2], p4=params[3]

speed = 1.0
amplitude = 0.0
frequency = 0.5
brightness = 2.0

# === Feature Flags (boolean-like, passed as 0.0/1.0 to shaders) ===
# f1=flags[0], f2=flags[1], f3=flags[2], f4=flags[3]

enable_effect = true
secondary_feature = false

# === Audio Settings ===
volume = 0.5              # Master volume (0.0 to 1.0)

# === Advanced (Optional) ===
screenshot_capture = false  # Allow Windows capture API
```

### Field Descriptions

#### Mode Fields

| Field | Type | V1 | V2 | Description |
|-------|------|:--:|:--:|-------------|
| `mode` | string | ✓ | ✓ | Operation mode: "animation", "wallpaper" |
| `shader` | string | ✓ | ✗ | Fragment shader entry point (V1 only) |
| `direction` | string | ✓ | ✗ | Rendering direction hint |
| `z_order` | string | ✓ | ✓ | Layer ordering for multiple packages |
| `sequence` | array | ✗ | ✓ | Array of sequence steps (V2 mode) |

#### Sequence Step Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Unique identifier for the step |
| `duration_ms` | u64 | Yes | Step duration (milliseconds, 0 = infinite) |
| `shader_entry` | string | Yes | Fragment shader function name |
| `sound` | string | No | Audio file to play at step start |
| `texture` | string | No | Texture image to load |
| `easing` | string | No | Easing function: "linear", "easeIn", "easeOut", "easeInOut" |

#### Logic and Feature Fields

| Field Prefix | Type | Max Count | Description |
|-------------|------|-----------|-------------|
| `p1`-`p4` | f32 | 4 | Logic parameters |
| `f1`-`f4` | bool | 4 | Feature flags |

**Access in shader:**
```wgsl
// Logic parameters (from [logic] section, by position)
logic_params[0]  // first parameter (e.g., speed)
logic_params[1]  // second parameter (e.g., amplitude)
logic_params[2]  // third parameter (e.g., frequency)
logic_params[3]  // fourth parameter (e.g., brightness)

// Feature flags (from [features] section, as 0.0 or 1.0)
feature_flags[0]  // first feature (e.g., enable_effect)
feature_flags[1]  // second feature
feature_flags[2]  // third feature
feature_flags[3]  // fourth feature
```

## WGSL Shader Specification

### Required Components

Every `.flow` package must provide a complete WGSL shader with:

1. **Uniform Buffer** (Bind Group 1, Binding 0)
2. **Texture Sampler** (Bind Group 0, Binding 1)
3. **Optional Custom Texture** (Bind Group 0, Binding 2-3)
4. **Vertex Shader** (`vs_main`)
5. **Fragment Shader(s)** (one or more entry points)

### Standard Uniforms

```wgsl
struct Uniforms {
    mouse: vec2f,         // Cursor position (0-1, relative to window)
    offset: vec2f,        // Translation offset
    scale: f32,           // Uniform scale
    time: f32,            // Elapsed time in seconds
    logic_params: vec4f,  // [p1, p2, p3, p4] from config.toml
    feature_flags: vec4f  // [f1, f2, f3, f4] as 0.0/1.0
};

@group(1) @binding(0) var<uniform> u: Uniforms;
```

### Standard Bindings

```wgsl
// Bind Group 0: Textures and Samplers
@group(0) @binding(0) var tex0: texture_2d<f32>;      // Desktop background
@group(0) @binding(1) var samp0: sampler;              // Linear sampler
@group(0) @binding(2) var tex1: texture_2d<f32>;      // Custom overlay (optional)
@group(0) @binding(3) var samp1: sampler;              // Custom sampler (optional)
```

### Vertex Shader Template

The vertex shader generates a fullscreen quad without vertex buffers:

```wgsl
@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4f {
    // Generate 6 vertices (2 triangles) from index
    var pos = array<vec2f, 6>(
        vec2f(-1.0, -1.0),  // Triangle 1
        vec2f( 1.0, -1.0),
        vec2f(-1.0,  1.0),
        vec2f(-1.0,  1.0),  // Triangle 2
        vec2f( 1.0, -1.0),
        vec2f( 1.0,  1.0)
    );
    return vec4f(pos[vid], 0.0, 1.0);
}
```

### Fragment Shader Template

```wgsl
@fragment
fn fs_default(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    // Normalized UV coordinates
    let uv = coord.xy / vec2f(textureDimensions(tex0));
    
    // Sample background texture
    let bg = textureSample(tex0, samp0, uv);
    
    // Mouse interaction (0-1 range)
    let mouse = u.mouse;
    
    // Time-based animation
    let t = u.time;
    
    // Example: color shift based on mouse and time
    var color = bg.rgb;
    color.r += sin(t * u.logic_params[0] + mouse.x * 3.14159) * 0.1;
    color.g += cos(t * u.logic_params[1] + mouse.y * 3.14159) * 0.1;
    
    // Feature flag check
    if (u.feature_flags[0] > 0.0) {
        // Feature 1 active
        color = mix(color, vec3f(1.0, 0.0, 0.0), 0.3);
    }
    
    return vec4f(color, bg.a);
}
```

### Multiple Fragment Shaders

For V2 sequences, define multiple fragment functions:

```wgsl
@fragment
fn fs_intro(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    // Intro animation
    let intensity = 1.0 - u.time * 0.5;
    return vec4f(0.0, 0.0, intensity, intensity);
}

@fragment
fn fs_main(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    // Main loop
    let uv = coord.xy / vec2f(textureDimensions(tex0));
    return textureSample(tex0, samp0, uv);
}
```

### Shader Best Practices

1. **Always output alpha**: Required for transparent overlays
   ```wgsl
   return vec4f(color.rgb, 0.5);  // Semi-transparent
   ```

2. **Use `textureDimensions()`**: Don't hardcode resolution
   ```wgsl
   let dims = vec2f(textureDimensions(tex0));
   ```

3. **Clamp UV coordinates**: Prevent edge artifacts
   ```wgsl
   let uv = clamp(coord.xy / dims, vec2f(0.0), vec2f(1.0));
   ```

4. **Avoid branching**: Use `mix()` and `step()` for performance
   ```wgsl
   let flag = step(0.5, u.feature_flags[0]);  // 0.0 or 1.0
   let color = mix(base_color, effect_color, flag);
   ```

5. **Float precision**: Use `f32` explicitly for compatibility
   ```wgsl
   let x: f32 = 1.0;  // Not f16 or f64
   ```

## Audio Specifications

### WAV Requirements

- **Format**: PCM (uncompressed)
- **Channels**: Mono or stereo
- **Sample Rate**: 44100 Hz (recommended) or 48000 Hz
- **Bit Depth**: 16-bit
- **Maximum Size**: 10 MB per file

### Audio in Sequences

```toml
[[sequence]]
name = "explosion"
duration_ms = 2000
shader_entry = "fs_explosion"
sound = "explosion.wav"  # Must match filename in ZIP
```

**Playback Behavior**:
- Sound plays once at step start
- Overlapping steps play sounds simultaneously
- Volume controlled globally by `volume` parameter
- Audio stream shared across all monitors

## Image/Texture Specifications

### Supported Formats

- **PNG** (recommended): RGBA, 8-bit per channel
- **JPEG**: RGB only (no alpha)
- **Maximum Resolution**: 7680×4320 (8K)
- **Color Space**: sRGB (auto-converted)

### Usage in Sequences

```toml
[[sequence]]
name = "video_overlay"
duration_ms = 5000
shader_entry = "fs_video"
texture = "frame_001.png"  # Loaded to tex1 bind group
```

**Texture Binding**:
When a texture is specified in a sequence step:
- Loaded into `tex1` (Bind Group 0, Binding 2)
- Sampled with `samp1` (Bind Group 0, Binding 3)
- Available in fragment shader via `textureSample(tex1, samp1, uv)`

### Wallpaper Background

The file `background.png` (if present) is used as the desktop background:

```toml
# In config.toml
mode = "wallpaper"
```

- Loaded once at initialization
- Resized to monitor resolution (bilinear filtering)
- Uploaded as `desktop_tex` and bound to `tex0`
- Used in shader via `textureSample(tex0, samp0, uv)`

If `background.png` is missing:
- Current desktop is captured via BitBlt (wallpaper mode only)
- Captured image resized and uploaded

## Packaging Tools

### Creating .flow Files

**Windows (PowerShell)**:
```powershell
Compress-Archive -Path config.toml, shader.wgsl, *.wav, *.png -DestinationPath package.flow -CompressionLevel None
```

**Python**:
```python
import zipfile
import os

with zipfile.ZipFile('package.flow', 'w', zipfile.ZIP_STORED) as zf:
    for root, dirs, files in os.walk('build/'):
        for file in files:
            zf.write(os.path.join(root, file), file)
```

**Manual (7-Zip)**:
1. Select all files (config.toml, shader.wgsl, assets)
2. Right-click → 7-Zip → "Add to archive"
3. Archive format: ZIP
4. Compression level: Store (no compression)

### Validation Checklist

Before distributing, verify:

```bash
# List contents
unzip -l package.flow

# Verify required files
unzip -p package.flow config.toml
unzip -p package.flow shader.wgsl

# Check config syntax
python -c "import toml; toml.loads(open('config.toml').read())"

# Test shader (requires WGSL validator)
# Use wgpu-validator or naga
```

## Version Compatibility

### Package Version vs. Engine Version

The `.flow` format version is implicit. Compatibility is determined by feature use:

- **V1 Packages**: `mode = "animation"` or `mode = "wallpaper"`
- **V2 Packages**: `mode = "sequence"` with `[[sequence]]` array

**Backward Compatibility**:
- V1 packages work on all engine versions
- V2 packages require engine 0.2.0+

### Migration from V1 to V2

**Old V1 config**:
```toml
mode = "animation"
shader = "fs_default"
p1 = 1.0
```

**New V2 config**:
```toml
mode = "sequence"

[[sequence]]
name = "default"
duration_ms = 0
shader_entry = "fs_default"

p1 = 1.0
```

## Debugging Packages

### Common Issues

**Black screen**:
- Check WGSL syntax: `naga package.flow/shader.wgsl`
- Verify entry point names match config
- Ensure textures are power-of-2 (optional but recommended)

**No audio**:
- Verify WAV is PCM format: `ffprobe sound.wav`
- Check filename matches config (case-sensitive)
- Ensure volume > 0.0

**Wrong resolution**:
- Provide `background.png` matching monitor aspect ratio
- Use `textureDimensions()` in shader for UV calculation

**Package won't open**:
- Ensure ZIP structure (not nested folder)
- Check file permissions (readable)
- Verify no encryption or password protection

### Diagnostic Tools

**Extract and inspect**:
```bash
mkdir temp_extract
cd temp_extract
tar -xf ../package.flow
ls -la  # Should show flat structure
```

**Test shader standalone**:
```bash
# Use wgpu-validator
cargo install wgpu-validator
wgpu-validator shader.wgsl
```

**Audio conversion** (if needed):
```bash
ffmpeg -i input.mp3 -ar 44100 -ac 2 -sample_fmt s16 output.wav
```

## Security Considerations

- **No Code Execution**: Packages are data-only (ZIP + WGSL)
- **GPU Shader Validation**: WGPU validates WGSL before execution
- **File Size Limits**: Recommend rejecting packages > 100 MB
- **Path Traversal**: ZIP entries with `../` should be rejected by implementation
- **Memory Limits**: Large textures may cause OOM; implement size checks

### Safe Loading Checklist

- ✓ Validate ZIP structure (no absolute paths)
- ✓ Limit texture dimensions (max 8192×8192)
- ✓ Limit audio file count (max 32)
- ✓ Timeout shader compilation (5 seconds)
- ✓ Fallback on invalid config (use defaults)
- ✓ Isolate GPU errors (don't crash engine)