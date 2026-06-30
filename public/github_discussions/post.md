# Discussion: ScreenAnimation - GPU-accelerated screen animations in Rust/WGPU

Hey everyone,

I've just published a new project and wanted to start a discussion about the architecture and implementation choices.

## Project Overview

**ScreenAnimation** is a Windows-based animation engine that renders WGSL shaders in real-time. It can operate as:
1. Transparent overlay animations (like Rainmeter)
2. Embedded desktop wallpapers (like Wallpaper Engine)

## Architecture Decisions

### Why WGPU over raw Vulkan/DX12?

I chose WGPU for cross-API portability. Currently targets:
- Vulkan (primary)
- DX12 (fallback)
- Metal (future: macOS port)

The abstraction layer is mature enough for production use. The only pain point was initial adapter selection, but `wgpu::Instance::default()` handles this well.

### Why Rust?

Three main reasons:
1. **Memory safety**: No use-after-free in shader loading code
2. **Performance**: Zero-cost abstractions for the render loop
3. **Concurrency**: Lock-free `AtomicI32` for mouse tracking is trivial in Rust

### The `.flow` Package Format

ZIP-based with this structure:
```
animation.flow
├── config.toml  (mode, shader entry, sequence steps)
├── shader.wgsl  (WGSL fragment shaders)
├── sounds/*.wav (PCM audio)
└── textures/*.png (optional)
```

**Design rationale:**
- ZIP: Universal format, easy to create/manipulate
- No compression for binary assets: Fast loading
- TOML: Human-readable config, good Rust ecosystem support

## Implementation Details

### Multi-Monitor Handling

```rust
EnumDisplayMonitors(...)  // Get all monitor RECTs
for rect in rects {
    CreateWindowExW(...)  // One HWND per monitor
    inst.create_surface(WindowWrapper(hwnd))
}
```

Each monitor gets its own swapchain. No synchronization between monitors (they render independently).

### Audio Sync

Using `rodio` for simplicity. Audio plays once at sequence step start. No precise sync with GPU rendering (acceptable for this use case).

### Shader Hot-Reload (Planned)

Since shaders are strings, runtime reload is easy:
```rust
if metadata("shader.wgsl").modified() != last_modified {
    pipelines = recompile(new_shader_src)?;
}
```

## Open Questions

I'd love feedback on:

1. **Multi-monitor sync**: Should I implement frame pacing across monitors?

2. **Shader debugging**: Currently just WGPU validation layers. Any better approaches?

3. **Security**: .flow packages execute WGSL code. Should I:
   - Sign packages?
   - Sandbox shader resource access?
   - Limit execution time?

4. **Performance**: Is 208 bytes/frame for uniforms reasonable? Could this be smaller?

5. **Cross-platform**: Linux port with Wayland + wgpu seems feasible. Anyone interested in collaborating?

6. **Feature prioritization**:
   - Video textures (MP4/WebM)?
   - Lua scripting for complex logic?
   - GUI editor for .flow packages?

## Code Highlights

The render loop is straightforward:
```rust
loop {
    poll_window_messages();
    update_mouse_position();
    
    for window in &mut windows {
        let uniforms = logic.update(&flow, mouse_pos);
        queue.write_buffer(&uniform_buffer, uniforms);
        
        let frame = surface.get_current_texture()?;
        encoder.begin_render_pass(...)
            .set_pipeline(pipeline)
            .set_bind_group(0, &texture_bg)
            .set_bind_group(1, &uniform_bg)
            .draw(0..6);
        queue.submit(encoder.finish());
    }
}
```

Single draw call per frame. 6 vertices (fullscreen quad).

## Performance Numbers

- **FPS**: 60 (vsync-limited)
- **CPU**: ~1% single core
- **GPU**: Minimal (single triangle list draw)
- **Memory**: ~50 MB base, +10 MB per monitor
- **Bandwidth**: ~12 KB/s uniform updates per monitor

## Build Instructions

```bash
cargo build --release
animationengine.exe Animation assets/animation1.flow
```

Requires: Windows 10/11, Rust 1.70+, VS Build Tools, Vulkan/DX12 GPU.

## Links

- **Source**: https://github.com/<user>/Build_ScreenAnimation
- **Docs**: `README.md`, `docs/architecture.md`, `docs/format.md`
- **Examples**: `assets/animation1.flow`, `assets/wallpaper1.flow`

## What are you working on?

Would love to hear what other Rust graphics projects are out there. Especially interested in:
- Real-time rendering engines
- Shader tooling
- Desktop customization tools

Let me know your thoughts!

---

**Discussion prompts:**
- What would you improve in this architecture?
- Have you used WGPU in production? Any lessons learned?
- Best practices for shader authoring and debugging?
- Ideas for the .flow format evolution? go to binary would be something i guess