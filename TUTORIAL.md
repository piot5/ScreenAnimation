# Creating Your First .flow Animation

A complete step-by-step guide to building custom GPU-accelerated animations with ScreenAnimation.

## What is a .flow File?

A `.flow` file is a **portable animation package** — essentially a ZIP archive containing:
- `config.toml` — Animation metadata and GPU parameters
- `shader.wgsl` — GPU shader code (WGSL language)
- Optional assets — Textures, audio, images

Think of it like a **plugin system for GPU shaders**. Once packaged, a `.flow` file runs on any ScreenAnimation installation without recompilation.

---

## Prerequisites

- ScreenAnimation compiled: `cargo build --release`
- Text editor (VS Code, Sublime, etc.)
- Basic WGSL understanding (similar to GLSL/HLSL)

---

## Step 1: Create Your Animation Directory

```bash
mkdir my_first_animation
cd my_first_animation
```

Create three files:
- `config.toml`
- `shader.wgsl`
- `background.png` (optional)

---

## Step 2: Write config.toml

Configuration defines animation parameters and timing.

### Example: Simple Wave Effect

```toml
[animation]
name = "My Wave Animation"
version = "1.0.0"
description = "A mouse-reactive wave distortion effect"

[parameters]
speed = 1.5
amplitude = 0.05
frequency = 3.0
brightness = 1.0
mouse_influence = true

[rendering]
target_fps = 60
background_image = "background.png"

[audio]
enabled = false
# Uncomment to sync with audio:
# audio_file = "soundtrack.wav"
# beat_detection = true
```

**Key Fields:**
| Field | Type | Purpose |
|-------|------|---------|
| `name` | string | Display name |
| `version` | string | Semantic versioning |
| `speed` | float | Animation speed multiplier |
| `amplitude` | float | Wave distortion strength (0.0-1.0) |
| `frequency` | float | Wave density |
| `brightness` | float | Output brightness multiplier |
| `mouse_influence` | bool | Enable mouse tracking |
| `target_fps` | int | Target frame rate |
| `background_image` | string | Path to background texture |

---

## Step 3: Write shader.wgsl

WGSL is WebGPU's shading language. Here's a simple wave shader:

```wgsl
// Uniforms (updated each frame)
@group(0) @binding(0) var<uniform> time: f32;
@group(0) @binding(1) var<uniform> params: vec4<f32>;
@group(0) @binding(2) var screen_texture: texture_2d<f32>;
@group(0) @binding(3) var screen_sampler: sampler;

// Vertex shader (full-screen quad)
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    );
    
    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 0.0),
    );
    
    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    output.uv = uvs[vertex_index];
    return output;
}

// Fragment shader (the actual effect)
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let speed = params.x;          // From config.toml
    let amplitude = params.y;
    let frequency = params.z;
    let brightness = params.w;
    
    // Wave calculation
    let wave_offset = sin(input.uv.x * frequency + time * speed) * amplitude;
    let distorted_uv = vec2<f32>(
        input.uv.x + wave_offset,
        input.uv.y
    );
    
    // Sample distorted texture
    let color = textureSample(screen_texture, screen_sampler, distorted_uv);
    
    // Apply brightness
    return vec4<f32>(color.rgb * brightness, color.a);
}
```

**Breaking it down:**
1. **Uniforms** — Parameters updated each frame (time, config values, textures)
2. **Vertex Shader** — Generates a full-screen quad
3. **Fragment Shader** — Per-pixel effect (wave distortion + brightness)

---

## Step 4: Add Optional Background Image

Place any PNG or JPG in your directory:

```bash
cp ~/Pictures/my_background.png ./background.png
```

Update `config.toml`:
```toml
[rendering]
background_image = "background.png"
```

---

## Step 5: Package into .flow

Use the builder binary:

```bash
# Linux/Mac
target/release/builder --input ./my_first_animation --output my_wave.flow

# Windows
target\release\builder.exe --input .\my_first_animation --output my_wave.flow
```

This creates `my_wave.flow` — a self-contained ZIP package.

---

## Step 6: Run Your Animation

### As a Wallpaper:
```bash
target/release/animationengine Wallpaper my_wave.flow
```

### As a Transition:
```bash
target/release/animationengine Animation my_wave.flow
```

---

## Example Animations

### A. Plasma Effect

```toml
[animation]
name = "Plasma"
description = "Procedural noise plasma"

[parameters]
speed = 2.0
scale = 5.0
complexity = 3.0
```

```wgsl
fn noise(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let speed = params.x;
    let scale = params.y;
    
    var col = vec3<f32>(0.0);
    for (var i: i32 = 0; i < 3; i = i + 1) {
        let angle = atan2(input.uv.y - 0.5, input.uv.x - 0.5) + time * speed;
        let dist = distance(input.uv, vec2<f32>(0.5));
        col += vec3<f32>(
            sin(dist * scale + time * speed) * 0.5 + 0.5,
            sin(dist * scale + time * speed + 2.0) * 0.5 + 0.5,
            sin(dist * scale + time * speed + 4.0) * 0.5 + 0.5
        ) * 0.3;
    }
    return vec4<f32>(col, 1.0);
}
```

### B. Kaleidoscope

```wgsl
@fragment
fn fs_kaleidoscope(input: VertexOutput) -> @location(0) vec4<f32> {
    let uv = input.uv * 2.0 - 1.0;
    let angle = atan2(uv.y, uv.x);
    let radius = length(uv);
    
    let segments = 6.0;
    let adjusted_angle = fract(angle / (3.14159 / segments)) * (3.14159 / segments);
    
    let reflected_uv = vec2<f32>(
        cos(adjusted_angle) * radius,
        sin(adjusted_angle) * radius
    ) * 0.5 + 0.5;
    
    return textureSample(screen_texture, screen_sampler, reflected_uv);
}
```

---

## Step 7: Advanced Features

### Audio Sync

Enable in `config.toml`:
```toml
[audio]
enabled = true
audio_file = "soundtrack.wav"
beat_detection = true
```

Access frequency data in shader:
```wgsl
@group(0) @binding(4) var<storage, read> audio_spectrum: array<f32, 256>;

@fragment
fn fs_audio_reactive(input: VertexOutput) -> @location(0) vec4<f32> {
    let bass = audio_spectrum[4];  // Low frequencies
    let treble = audio_spectrum[200];  // High frequencies
    
    let color = mix(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, 0.0, 1.0),
        bass
    );
    
    return vec4<f32>(color, 1.0);
}
```

### Multiple Shaders (Transitions)

For animation sequences:

```toml
[[shaders]]
name = "capture"
duration = 0.5
file = "capture.wgsl"

[[shaders]]
name = "detach"
duration = 1.5
file = "detach.wgsl"

[[shaders]]
name = "move"
duration = 2.0
file = "move.wgsl"

[[shaders]]
name = "land"
duration = 1.0
file = "land.wgsl"
```

---

## Shader Reference: Common Patterns

### Sine Wave
```wgsl
let wave = sin(input.uv.x * frequency + time * speed);
```

### Circular Gradient
```wgsl
let dist = distance(input.uv, vec2<f32>(0.5, 0.5));
let gradient = 1.0 - dist;
```

### Color Shift
```wgsl
let hue = time * speed;
let color = vec3<f32>(
    sin(hue) * 0.5 + 0.5,
    sin(hue + 2.09) * 0.5 + 0.5,
    sin(hue + 4.18) * 0.5 + 0.5
);
```

### Texture Distortion
```wgsl
let distorted = input.uv + vec2<f32>(
    sin(input.uv.y * frequency + time * speed),
    cos(input.uv.x * frequency + time * speed)
) * amplitude;
let color = textureSample(screen_texture, screen_sampler, distorted);
```

---

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Shader won't compile | Check WGSL syntax in VS Code with WGSL extension |
| Animation too fast/slow | Adjust `speed` in config.toml |
| Colors look wrong | Check `brightness` and ensure textures are sRGB |
| Memory spike | Reduce texture resolution or shader complexity |
| Crash on launch | Ensure all files referenced in config exist |

---

## Performance Tips

1. **Use lower texture resolutions** for faster sampling
2. **Minimize branching** in fragment shaders (if/else affects performance)
3. **Pre-compute constants** outside loops
4. **Target 60 FPS** — high refresh rates burn GPU power

---

## Share Your Animation

Once you're happy with your creation:

```bash
# Upload my_wave.flow to GitHub/itch.io/etc
# Other users can run it immediately without building
```

**Example:** Someone downloads your animation:
```bash
animationengine Wallpaper my_wave.flow
# It just works! No compilation, no dependencies.
```

---

## Next Steps

- Explore `/examples` for more complex shaders
- Read WGSL docs: https://www.w3.org/TR/WGSL/
- Experiment with Shadertoy code (port to WGSL)
- Join the community — share your creations!

Happy shading! 🎨
