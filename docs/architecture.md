# ScreenAnimation Architecture

## Overview

ScreenAnimation is a GPU-accelerated screen animation and wallpaper engine for Windows. It leverages WGPU for hardware-accelerated rendering with native Windows API integration for window management and desktop embedding.

## Design Philosophy

1. **Performance First**: Direct GPU access via WGPU, minimal overhead
2. **Modular Design**: Clear separation between rendering, logic, and system integration
3. **Cross-Monitor Support**: Automatic multi-monitor window creation
4. **Flexibility**: Two operation modes (V1 simple, V2 sequence-based)
5. **Extensibility**: Plugin-like .flow packages with shaders and assets

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      animationengine.exe                     │
│                     (Binary Crate)                          │
├─────────────────────────────────────────────────────────────┤
│  CLI Parser (clap)                                          │
│  ├── Animation <path>                                       │
│  └── Wallpaper <path>                                       │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                    screen_animation (Lib)                   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐ │
│  │    engine     │  │    loader     │  │     logic     │ │
│  │    (WGPU)     │  │  (.flow ZIP)  │  │   (Uniforms)  │ │
│  ├───────────────┤  ├───────────────┤  ├───────────────┤ │
│  │ GpuCore       │  │ FlowPackage   │  │ LogicEngine   │ │
│  │ Uniforms      │  │  - Config     │  │ - update()    │ │
│  │ WindowWrapper │  │  - Shader     │  │ - val()       │ │
│  │               │  │  - Sounds     │  │ - feature()   │ │
│  │               │  │  - Textures   │  │               │ │
│  └───────────────┘  └───────────────┘  └───────────────┘ │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                    windows                           │   │
│  │  ┌────────────────┐      ┌──────────────────────┐  │   │
│  │  │ MonitorWindow   │      │ init_windows()       │  │   │
│  │  │ - HWND          │      │ - EnumDisplayMonitors │  │   │
│  │  │ - Surface       │      │ - CreateWindowExW     │  │   │
│  │  │ - BindGroups    │      │ - fetch_worker_w()    │  │   │
│  │  │ - UniformBuffer │      │ - capture_or_load()   │  │   │
│  │  └────────────────┘      └──────────────────────┘  │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                    External Dependencies                    │
├─────────────────────────────────────────────────────────────┤
│  wgpu (0.19)          - GPU API abstraction                 │
│  windows (0.54)       - Win32 API bindings                  │
│  rodio (0.17)         - Audio playback                      │
│  clap (4.4)           - CLI parsing                         │
│  serde + toml         - Configuration deserialization       │
│  zip                  - .flow package reading               │
│  image                - Texture loading/resizing            │
└─────────────────────────────────────────────────────────────┘
```

## Core Modules

### engine

The heart of the GPU abstraction layer.

**GpuCore** struct:
- `device`: Logical GPU device (command buffer allocation, resource creation)
- `queue`: Instruction queue for GPU commands
- `bind_group_layout`: 4-entry layout for textures and samplers (desktop + custom)
- `uniform_layout`: Bind group layout for uniform buffers
- `sampler`: Shared linear sampler for texture sampling
- `pipelines`: HashMap of shader entry points to render pipelines

**Initialization Flow**:
1. Request adapter (GPU selection)
2. Request device and queue
3. Create shader module from WGSL source
4. Create bind group layouts (texture+sampler, uniform)
5. Create pipeline layout combining both layouts
6. For each shader entry point:
   - Create render pipeline with vertex/fragment states
   - Store in `pipelines` HashMap

**Uniforms** struct:
- 208 bytes total
- `mouse: [f32; 2]` - Normalized cursor position (0-1)
- `offset: [f32; 2]` - Translation offset
- `scale: f32` - Uniform scale
- `time: f32` - Elapsed seconds
- `logic_params: [f32; 4]` - User-defined parameters (p1-p4)
- `feature_flags: [f32; 4]` - Boolean flags (f1-f4)

**WindowWrapper**:
- Wraps raw HWND
- Implements `HasWindowHandle` and `HasDisplayHandle` traits
- Allows wgpu to create surfaces from native Windows handles

### loader

Responsible for reading .flow packages (ZIP archives).

**FlowPackage** struct:
- `config: Config` - Parsed configuration
- `sounds: HashMap<String, Arc<Vec<u8>>>` - Shared audio data
- `image_data: Option<Vec<u8>>` - Wallpaper background
- `textures: HashMap<String, (u32, u32, Vec<u8>)>` - RGBA textures with dimensions
- `shader_src: String` - Complete WGSL source

**Loading Process**:
1. Open ZIP file
2. Read and parse `config.toml` (with fallback to defaults)
3. Read `shader.wgsl` as string
4. Iterate archive entries:
   - `.wav` → load into `sounds` HashMap (wrapped in Arc)
   - `background.png` → store in `image_data`
   - `.png`/`.jpg` → decode with `image` crate, convert to RGBA, store in `textures`

**Key Methods**:
- `val(key, default)`: Extract float values from config logic section
- `feature(key)`: Extract boolean features from config

### logic

Frame-by-frame uniform calculation.

**LogicEngine**:
- `start_time: Instant` - Reference point for time uniform

**Usage**:
```rust
let logic = LogicEngine::new();
loop {
    let uniforms = logic.update(&flow, mouse_position);
    gpu.queue.write_buffer(..., bytemuck::bytes_of(&uniforms));
}
```

**update() Parameters**:
- `flow: &FlowPackage` - Access to config values
- `mouse_rel: [f32; 2]` - Normalized mouse coordinates

Returns fully populated `Uniforms` struct.

### windows

Native Windows API integration for window management.

**MonitorWindow**:
Contains all per-window GPU resources:
- `hwnd: HWND` - Native window handle
- `surface: wgpu::Surface` - Swapchain surface
- `texture_bind_group` - Background texture + sampler
- `uniform_buffer` - Dynamic uniform data
- `uniform_bind_group` - Uniform buffer binding
- `desktop_tex: wgpu::Texture` - Background texture (V1)

**init_windows() Flow**:
1. Enumerate all display monitors via `EnumDisplayMonitors`
2. For wallpaper mode: Find WorkerW window via `fetch_worker_w()`
3. For each monitor:
   - Create window with appropriate styles (overlay vs child)
   - Capture desktop background or load from package
   - Create GPU texture and upload background
   - Create surface from window handle
   - Configure swapchain
   - Create bind groups for texture and uniforms
4. Return Vec<MonitorWindow>

**Window Styles**:
- Animation: `WS_POPUP | WS_VISIBLE | WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_TRANSPARENT`
- Wallpaper: `WS_CHILD | WS_VISIBLE` (child of WorkerW)

**Desktop Capture**:
- If `background.png` exists in package: Resize and use
- Otherwise: Capture current desktop via BitBlt + GetDIBits

## Data Flow

### Initialization Sequence

```rust
// 1. Load package
let flow = FlowPackage::load("animation.flow")?;

// 2. Initialize audio
let (_stream, sink) = rodio::OutputStream::try_default()?;
sink.set_volume(flow.val("volume", 0.5));

// 3. Setup window class
RegisterClassW(&WNDCLASSW { ... });

// 4. Create GPU instance
let inst = wgpu::Instance::default();

// 5. Compile shaders and create pipelines
let gpu = GpuCore::new(&inst, &flow.shader_src, &entries).await?;

// 6. Create windows on all monitors
let wins = init_windows(&gpu, &inst, class_name, hinstance, is_wallpaper, &flow);
```

### V1 Render Loop

```
Frame Start
    │
    ▼
Poll Window Messages (PeekMessageW)
    │
    ▼
GetCursorPos() ──────────────┐
    │                       │
    ▼                       │
For each window:            │
    │                       │
    ▼                       │
GetWindowRect()             │
    │                       │
    ▼                       │
Calculate relative mouse    │
position (0-1 range)        │
    │                       │
    ▼                       │
LogicEngine::update()       │
    │                       │
    ▼                       │
write_buffer(Uniforms)      │
    │                       │
    ▼                       │
get_current_texture()       │
    │                       │
    ▼                       │
Create render pass          │
    │                       │
    ▼                       │
Set pipeline, bind groups   │
    │                       │
    ▼                       │
draw(6) // Fullscreen quad   │
    │                       │
    ▼                       │
queue.submit()              │
    │                       │
    ▼                       │
fr.present()                 │
    │                       │
    └───────────────────────┘
    │
    ▼
Throttle to 60 FPS
```

### V2 Sequence Flow

```
For each step in sequence:
    │
    ▼
Record start_time
    │
    ▼
Play step sound (if any)
    │
    ▼
While elapsed < duration_ms:
    │
    ▼
    [Same as V1 render loop with fixed uniforms]
    │
    ▼
Next step
```

## Memory Model

**Shared Ownership**:
- Sound data: `Arc<Vec<u8>>` - Shared between loader and decoder
- Config: Cloned via `Arc`-like semantics (cheap clones of small structs)

**GPU Resources**:
- One `GpuCore` instance shared across all windows
- Per-window: Surface, texture bind group, uniform buffer, uniform bind group
- Shared: Device, queue, sampler, pipelines, bind group layouts

**Buffer Updates**:
- Uniform buffer: 208 bytes per frame per window (via write_buffer)
- Texture: Uploaded once at initialization

## Graphics Pipeline

```
Vertex Stage:
  - No vertex buffers (vertex_index generated)
  - 6 vertices forming fullscreen quad
  - Direct position output (-1 to 1)

Fragment Stage:
  - Bind Group 0: Background texture + sampler
  - Bind Group 1: Uniform buffer
  - User shader logic with access to:
    * textureSample(tex0, samp0, uv)
    * Uniforms structure
    
Output:
  - BGRA8UnormSrgb format
  - Alpha blending enabled (for transparency)
```

## Windows Integration Details

### Window Creation (Animation Mode)

```c
CreateWindowExW(
    WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_TRANSPARENT,
    "WgpuAnim",
    "",  // No title
    WS_POPUP | WS_VISIBLE,
    x, y, width, height,
    NULL,  // No parent
    NULL, hInstance, NULL
);
```

**Style Flags**:
- `WS_EX_LAYERED`: Supports per-pixel alpha
- `WS_EX_TRANSPARENT`: Click-through (mouse events pass to windows below)
- `WS_EX_TOPMOST`: Stay above normal windows
- `WS_POPUP`: Borderless window

### Window Creation (Wallpaper Mode)

```c
WorkerW = FindWorkerW();  // Special desktop window

CreateWindowExW(
    0,  // No extended styles
    "WgpuAnim",
    "",
    WS_CHILD | WS_VISIBLE,
    0, 0, width, height,
    WorkerW,  // Parent is WorkerW
    NULL, hInstance, NULL
);
```

**WorkerW Trick**:
Windows desktop has a special window hierarchy:
- `Progman` → `SHELLDLL_DefView` (icons) + `WorkerW` (behind icons)
- Sending `0x052C` message to Progman creates a second WorkerW
- Animation becomes child of this WorkerW → appears behind icons

### DPI Awareness

```rust
SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
```

- Per-monitor DPI awareness (V2 = latest, best scaling)
- Required for correct rendering on mixed-DPI setups
- Windows 10 1703+ feature

## Performance Characteristics

**CPU Usage**:
- Polling loop: ~1% single core
- Minimal allocations in hot path
- Lock-free atomics for mouse tracking

**GPU Usage**:
- Single draw call per frame per monitor (6 vertices)
- Uniform buffer update: 208 bytes × 60 FPS = ~12 KB/s
- Texture upload: One-time at startup

**Memory Footprint**:
- Base: ~50 MB (WGPU + window resources)
- Per monitor: ~10 MB (surface, textures, buffers)
- Audio: Shared across instances (Arc)

**Bottlenecks**:
- Swapchain presentation (vsync-locked)
- Window message pump
- Image decoding at load time

## Concurrency Model

- **Main Thread**: All Windows API, GPU device access, rendering
- **Audio Thread**: Rodio internal thread for sample mixing
- **No Shared Mutability**: All GPU operations sequential on main thread
- **Atomic Mouse Tracking**: Lock-free HWND procedure → main loop communication

## Extension Points

**.flow Packages**:
- Custom WGSL shaders with user-defined entry points
- Audio triggers at sequence steps
- Texture overlays
- Configurable logic parameters

**Shader API**:
- Replaceable fragment shaders
- Shared vertex shader
- Access to time, mouse, logic parameters
- Two texture slots (background + overlay)

**Future Possibilities**:
- Video texture support
- Network asset streaming
- Hot-reload of shaders
- Custom logic functions in WGSL

## Debugging Tips

**Enable WGPU Debugging**:
```rust
let inst = wgpu::Instance::new(wgpu::InstanceDescriptor {
    backends: wgpu::Backends::all(),
    dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
    // ... more options
});
```

**Debug Layers**:
- Windows: Use Graphics Debugging Tools
- WGPU: Set `WGPU_DEBUG=1` environment variable

**Common Issues**:
- Black screen: Check shader compilation errors
- Crashes: Invalid HWND, surface creation failure
- No audio: Missing WAV files, rodio initialization failure
- Wrong DPI: Missing `SetProcessDpiAwarenessContext`
- Behind icons: WorkerW not found (check desktop state)

Build with `cargo build --features debug` for verbose logging.