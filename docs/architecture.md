# Architecture Overview

## System Design

ScreenAnimation is a GPU-accelerated animation engine for Windows that renders real-time animations using WGPU and WGSL shaders. The system is designed around a multi-monitor, multi-layer architecture.

## Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                     Main Process                             │
├─────────────────────────────────────────────────────────────┤
│  animationengine.exe                                        │
│  ├── CLI Parser (clap)                                      │
│  ├── Package Loader (ZIP extraction)                        │
│  ├── Audio Engine (rodio)                                   │
│  ├── GPU Core (WGPU device + pipelines)                     │
│  ├── Windows Integration (HWND + message loop)              │
│  └── Render Loop (V1 or V2)                                │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    screen_animation Crate                   │
├─────────────────────────────────────────────────────────────┤
│  engine      - WGPU core: device, queue, pipelines          │
│  loader      - .flow package parsing (ZIP → assets)         │
│  logic       - Uniform buffer calculation per frame         │
│  windows     - Window creation + monitor enumeration        │
│  background  - Image loading, resizing, GPU upload          │
│  screenshot  - Desktop capture via BitBlt                   │
└─────────────────────────────────────────────────────────────┘
```

## Data Flow

```
.flow File (ZIP)
    ↓
FlowPackage (config + shader + assets)
    ↓
GpuCore (compile shaders, create pipelines)
    ↓
MonitorWindows (one per display)
    ↓ per frame: LogicEngine → Uniforms → GPU
Render Loop
    ↓
Swapchain → Screen
```

## Rendering Architecture

### Uniform Buffer Layout

The GPU receives per-frame data through a 64-byte uniform buffer:

```
Offset 0:  mouse (vec2<f32>)           - Normalized cursor position
Offset 8:  offset (vec2<f32>)          - Translation offset (reserved)
Offset 16: scale (f32)                  - Uniform scale factor
Offset 20: time (f32)                   - Elapsed seconds since start
Offset 24: padding (2 × f32)            - Alignment for vec4
Offset 32: logic_params (vec4<f32>)     - p1-p4 from config.toml
Offset 48: feature_flags (vec4<f32>)    - f1-f4 as 0.0/1.0
Total: 64 bytes
```

### Bind Group Layout

Two bind groups enable flexible shader composition:

**Bind Group 0: Textures & Samplers**
- Binding 0: Background texture (desktop capture or background.png)
- Binding 1: Linear sampler for background texture
- Binding 2: Optional custom texture (from sequence steps)
- Binding 3: Sampler for custom texture

**Bind Group 1: Uniform Buffer**
- Binding 0: Per-frame uniform data (mouse, time, params)

## Module Responsibilities

### `engine` (GpuCore)
- Manages WGPU device and command queue
- Compiles WGSL shader modules
- Creates and caches render pipelines
- Provides bind group layouts
- One instance shared across all monitor windows

### `loader` (FlowPackage)
- Opens and validates .flow ZIP archives
- Parses config.toml into structured Config
- Extracts and decodes audio files (WAV → Arc<Vec<u8>>)
- Decodes images (PNG/JPG → RGBA8 + dimensions)
- Stores WGSL shader source code
- Security: validates paths, enforces size limits

### `logic` (LogicEngine)
- Tracks animation start time
- Calculates elapsed time per frame
- Reads logic parameters (p1-p4) from config
- Reads feature flags (f1-f4) from config
- Produces Uniforms structure for GPU upload

### `windows` (MonitorWindow)
- Enumerates monitors via EnumDisplayMonitors
- Implements WorkerW trick for wallpaper embedding
- Creates native Win32 windows per monitor
- Configures WGPU surfaces from HWND handles
- Manages GPU resources per window (textures, buffers, bind groups)

### `background`
- Decodes background.png from ZIP
- Resizes to monitor resolution (bilinear filtering)
- Converts RGBA → BGRA for Windows DIB compatibility
- Creates and manages GPU textures

### `screenshot`
- Captures desktop via DXGI Output Duplication (GPU-accelerated, ~0.5-1ms)
- Falls back to BitBlt (GDI, CPU-bound, ~5-7ms) when DXGI is unavailable
- Creates D3D11 device + staging texture for GPU → CPU readback
- Handles frame acquisition with timeout, access lost recovery
- Supports multi-output enumeration for diagnostics
- DXGI pipeline: AcquireNextFrame → CopyResource (GPU→staging) → Map → row-pitch-aware readback → Unmap

## Operation Modes

### V1: Simple Mode (Animation/Wallpaper)
- Loads single shader entry point
- Continuous rendering loop
- Mouse position tracked via WndProc → atomics
- 60 FPS target
- Transparent overlay (Animation) or embedded (Wallpaper)

### V2: Sequence Mode (Multi-step)
- Loads multiple shader entry points from sequence array
- Steps run for configured durations
- Media events triggered at timestamps
- Sound playback synchronized to step start
- Fallback to V1 if sequence array is empty

## Threading Model

```
Main Thread:
├── Windows Message Pump (PeekMessage/DispatchMessage)
├── Audio Playback (rodio, internal thread)
├── GPU Submission (queue.submit)
└── Render Loop (frame rate throttled)

No worker threads - everything runs on main thread
- GPU operations are async but polled via wgpu
- Audio uses internal ring buffer
- No locks needed (single-threaded)
```

## Memory Ownership

### GPU Resources (WGPU-managed)
- Device, Queue: owned by GpuCore
- Pipelines: cached in HashMap by GpuCore
- Textures, Buffers, BindGroups: owned by MonitorWindow
- Samplers: shared via GpuCore (single sampler for all windows)

### CPU Resources
- FlowPackage: loaded once at startup, owns all assets
- Sounds: Arc<Vec<u8>> shared between loader, audio decoder
- Textures: HashMap<String, (u32, u32, Vec<u8>)> in FlowPackage
- MonitorWindows: Vec<MonitorWindow>, one per monitor

## Extension Points

### Custom Shaders
- Entry points: `fs_default`, `fs_intro`, etc.
- Access uniforms via `@group(1) @binding(0) var<uniform> u: Uniforms;`
- Sample textures via `@group(0) @binding(0..3)`

### Sequence Steps
- Define in config.toml: `[[sequence]]`
- Each step specifies: duration, shader_entry, media events
- Steps can loop (duration_ms = 0) or run once

### Media Events
- Sound playback: `sound = "fileName.wav"`
- Texture overlay: `texture = "image.png"` (loaded to tex1)

## Performance Characteristics

- **Startup**: ~1s (package load + GPU init + shader compilation)
- **Memory**: ~50MB base + 10-50MB per .flow package
- **GPU**: ~100 draw calls per frame (one per monitor)
- **CPU**: <1ms per frame for logic + uniform upload
- **Frame Time**: <16ms target (60 FPS)
- **Audio Latency**: <50ms (rodio internal buffering)

## Security Architecture

### Input Validation
- ZIP paths: reject `..` and absolute paths
- Package size: hard limit 100MB uncompressed
- Texture size: max 8192×8192 pixels
- Audio count: max 32 files

### Resource Limits
- Maximum texture files: 16
- Total uncompressed size: 100MB
- Individual texture: 8192×8192 @ 4 bytes = 256MB

### Isolation
- Each .flow package runs in same process (no sandbox)
- WGSL validated by WGPU before execution
- GDI resources (DCs, bitmaps) cleaned up immediately