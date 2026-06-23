---
title: "Building a GPU-Accelerated Animation Engine for Windows with Rust and WGPU"
tags: rust, wgpu, graphics, windows, shaders
cover_image: /cover.png
published: true
---

# Building a GPU-Accelerated Animation Engine for Windows with Rust and WGPU

I recently built **ScreenAnimation**, a real-time animation engine that renders WGSL shaders directly to your Windows desktop. In this article, I'll share the architecture, implementation details, and lessons learned.

## What Does It Do?

ScreenAnimation can:
- Render transparent overlay animations (think Rainmeter)
- Embed animations as desktop wallpapers behind your icons
- Play synchronized audio
- Run across multiple monitors simultaneously

All powered by GPU-accelerated WGSL shaders.

## The Tech Stack

```toml
[dependencies]
wgpu = "0.19"           # Cross-platform GPU API
windows = "0.54"        # Win32 bindings
rodio = "0.17"          # Audio playback
clap = { version = "4.4", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"            # Config parsing
zip = "0.6"             # Package loading
image = "0.24"          # Texture handling
bytemuck = "1.14"       # Safe byte casting
```

**Why these choices?**
- **WGPU**: Abstraction over Vulkan/DX12/Metal. Excellent Rust API.
- **windows-rs**: First-class Rust bindings for Win32. Much better than the old `winapi` crate.
- **Rodio**: Simple cross-platform audio. Good enough for WAV playback.

## Architecture Overview

The project is split into four main modules:

```
src/
├── engine.rs      # WGPU core (device, pipelines, bind groups)
├── loader.rs      # .flow package parser (ZIP + TOML)
├── logic.rs       # Uniform buffer calculation
└── windows.rs     # HWND management + Win32 integration
```

Plus the binary entry point in `src/bin/animationengine.rs`.

### The Engine Module

The heart of the system. `GpuCore` owns all GPU resources:

```rust
pub struct GpuCore {
    pub device: Device,
    pub queue: Queue,
    pub bind_group_layout: BindGroupLayout,  // 4 entries: tex+samp+tex+samp
    pub uniform_layout: BindGroupLayout,     // 1 entry: uniform buffer
    pub sampler: Sampler,
    pub pipelines: HashMap<String, RenderPipeline>,
}
```

Key design decisions:
- **Shared device/queue**: One GPU context for all windows
- **4-entry bind group**: Background texture + sampler, plus optional custom texture + sampler
- **Pipeline caching**: One pipeline per unique shader entry point

### The Loader Module

Reads `.flow` packages (ZIP archives):

```rust
pub struct FlowPackage {
    pub config: Config,
    pub sounds: HashMap<String, Arc<Vec<u8>>>,
    pub image_data: Option<Vec<u8>>,  // Wallpaper background
    pub textures: HashMap<String, (u32, u32, Vec<u8>)>,  // RGBA
    pub shader_src: String,
}
```

Features:
- Lazy texture decoding (only when needed)
- Shared audio via `Arc` (zero-copy between decoder and playback)
- Fallback config values with `unwrap_or_default()`

### The Logic Module

Calculates uniform buffers each frame:

```rust
pub struct LogicEngine {
    pub start_time: Instant,
}

impl LogicEngine {
    pub fn update(&self, flow: &FlowPackage, mouse_rel: [f32; 2]) -> Uniforms {
        Uniforms {
            mouse: mouse_rel,
            time: self.start_time.elapsed().as_secs_f32(),
            logic_params: [
                flow.val("p1", 0.0),
                flow.val("p2", 0.0),
                flow.val("p3", 0.0),
                flow.val("p4", 0.0),
            ],
            feature_flags: [
                if flow.feature("f1") { 1.0 } else { 0.0 },
                // ...
            ],
            ..Default::default()
        }
    }
}
```

### The Windows Module

Handles HWND creation and desktop integration:

```rust
pub unsafe fn init_windows(
    gpu: &GpuCore,
    inst: &wgpu::Instance,
    class: PCWSTR,
    hi: HINSTANCE,
    is_wp: bool,
    flow: &FlowPackage,
) -> Vec<MonitorWindow>
```

The wallpaper mode uses the famous **WorkerW trick**:
```rust
// Find WorkerW behind desktop icons
SendMessageTimeoutW(Progman, 0x052C, ...);
let workerw = FindWorkerW();
// Create child window in WorkerW
CreateWindowExW(0, class, "", WS_CHILD | WS_VISIBLE, 0, 0, w, h, workerw, ...);
```

## The Render Loop

The main loop is refreshingly simple:

```rust
loop {
    // 1. Poll messages
    while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    // 2. Update mouse position (atomic, lock-free)
    let mx = MOUSE_X.load(Ordering::Relaxed);
    let my = MOUSE_Y.load(Ordering::Relaxed);

    // 3. For each monitor window:
    for window in &mut windows {
        // Calculate relative mouse position (0-1)
        let rel_x = (mx - rect.left) as f32 / width as f32;
        
        // Write uniform buffer (208 bytes)
        queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
        
        // Render
        let frame = surface.get_current_texture()?;
        let mut rpass = encoder.begin_render_pass(...);
        rpass.set_pipeline(pipeline);
        rpass.set_bind_group(0, &texture_bg);
        rpass.set_bind_group(1, &uniform_bg);
        rpass.draw(0..6);  // Fullscreen quad
        queue.submit(encoder.finish());
    }
}
```

**That's it**: poll messages, update uniforms, draw 6 vertices per monitor.

## WGSL Shader Structure

The vertex shader generates a fullscreen quad without vertex buffers:

```wgsl
@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4f {
    var pos = array<vec2f, 6>(
        vec2f(-1, -1), vec2f(1, -1), vec2f(-1, 1),
        vec2f(-1, 1), vec2f(1, -1), vec2f(1, 1)
    );
    return vec4f(pos[vid], 0.0, 1.0);
}
```

Fragment shaders get these uniforms automatically:

```wgsl
struct Uniforms {
    mouse: vec2f,         // 0-1 normalized
    offset: vec2f,
    scale: f32,
    time: f32,            // Seconds since start
    logic_params: vec4f,  // User-defined p1-p4
    feature_flags: vec4f  // Boolean f1-f4
}
```

## The .flow Package Format

ZIP-based with this structure:
```
animation.flow
├── config.toml      # Parameters, sequences
├── shader.wgsl      # WGSL source
├── sound.wav        # Audio (optional)
└── background.png   # Wallpaper (optional)
```

**Example config.toml (V2 sequence mode):**

```toml
mode = "sequence"

[[sequence]]
name = "intro"
duration_ms = 3000
shader_entry = "fs_intro"
sound = "intro.wav"

[p1] = 2.0
[f1] = true
volume = 0.5
```

## Performance

Achieving 60 FPS was straightforward:

- **CPU**: ~1% single core (mostly message pump)
- **GPU**: Single draw call per monitor (6 vertices)
- **Bandwidth**: 208 bytes uniform update × 60 FPS ≈ 12 KB/s per monitor
- **Memory**: ~50 MB base, +10 MB per monitor

The lock-free atomic for mouse tracking is elegant:

```rust
// In WndProc:
MOUSE_X.store((l.0 & 0xffff) as i16 as i32, Ordering::Relaxed);
MOUSE_Y.store(((l.0 >> 16) & 0xffff) as i16 as i32, Ordering::Relaxed);
```

No mutex, no lock contention. Just lock-free `AtomicI32`.

## Lessons Learned

### ✅ WGPU is Production-Ready

After 6 months of development, WGPU feels stable. The validation layers caught most mistakes early. Only issue: occasional driver bugs on older Intel GPUs.

### ✅ Rust + Win32 Works Great

The `windows-rs` crate is excellent. `SendMessageTimeoutW`, `EnumDisplayMonitors`, `CreateWindowExW` all work as expected. The `PWSTR`/`PCWSTR` types prevent buffer overflows at compile time.

### ✅ Modular Architecture Paid Off

Having separate modules for loading, rendering, and system integration meant I could test each independently. The `logic` module even works without Windows (great for CI).

### ⚠️ Shader Compilation is Slow

WGPU shader compilation takes 100-200ms on first run. Not a problem for release, but annoying during development. Solution: Cache compiled pipelines to disk (planned feature).

### ⚠️ Multi-Monitor Edge Cases

G-SYNC/FreeSync monitors can have different refresh rates. Currently vsync-locked to each monitor's rate. Could lead to desynced animations. Future: frame pacing with `WaitForVerticalBlankBegin`.

### ⚠️ Audio Sync is Approximate

Rodio plays audio asynchronously. Good enough for music synchronization, but not sample-accurate. Acceptable for this project.

## What's Next?

1. **Hot shader reload**: File watcher + pipeline recompilation
2. **Video textures**: FFmpeg integration for MP4/WebM
3. **GUI editor**: Tauri app for creating .flow packages visually
4. **Linux port**: Wayland + wgpu (architecturally feasible)
5. **Plugin system**: Lua/WGSL scripting for complex logic

## Building It Yourself

```bash
git clone <repository-url>
cd Build_ScreenAnimation
cargo build --release

# Run example
animationengine.exe Animation assets/animation1.flow
```

**Requirements:**
- Windows 10/11
- Rust 1.70+
- Visual Studio Build Tools
- Vulkan/DX12 compatible GPU

## Conclusion

WGPU + Rust is a fantastic combination for graphics programming. The safety guarantees let you focus on algorithms instead of memory management, and the performance is indistinguishable from C++.

If you're interested in GPU programming, desktop customization, or just want to see some WGSL shaders, check out the project. Feedback and contributions welcome!

---

**Resources:**
- **Source**: https://github.com/<user>/Build_ScreenAnimation
- **Docs**: README.md, docs/architecture.md, docs/format.md
- **Examples**: assets/animation1.flow, assets/wallpaper1.flow

**Questions?** Leave a comment or open a GitHub Discussion!