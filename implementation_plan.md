# Implementation Plan - Addressing Weaknesses

## Executive Summary

This plan addresses the critical and medium-priority weaknesses identified in the 
evaluation_report.md. The focus is on completing the disabled DXGI capture, adding 
essential tests, and implementing core missing features.

**Critical Finding**: The DXGI capture in `screenshot.rs` acquires frames correctly 
but returns black placeholder data (lines 214-238). This is a 10× performance regression.

---

## Phase 1: Critical Fixes (Week 1)

### 1.1 Fix DXGI Capture Implementation
**File**: `src/screenshot.rs`  
**Priority**: P0 (Critical)  
**Current Score**: 58/100 → Target: 90/100

**Problem**: 
- `capture_dxgi()` acquires frames via `AcquireNextFrame` but never reads the GPU texture
- Returns `vec![0u8; (width * height * 4) as usize]` (black frames)
- Comments acknowledge this is a placeholder (lines 230-237)

**Required Changes**:

```rust
// 1. Add staging texture for CPU readback
let staging_desc = wgpu::TextureDescriptor {
    label: Some("DXGI staging texture"),
    size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    mip_level_count: 1,
    sample_count: 1,
    dimension: wgpu::TextureDimension::D2,
    format: wgpu::TextureFormat::Bgra8Unorm,
    usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::MAP_READ,
    view_formats: &[],
};
let staging_tex = gpu.device.create_texture(&staging_desc);

// 2. Copy DXGI resource to staging texture
gpu.queue.copy_external_texture_to_texture(
    &dxgi_texture,
    &staging_tex,
    wgpu::Origin3d::ZERO,
    wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
);

// 3. Map staging texture and read pixels
let buffer = staging_tex.slice(..).map_async(wgpu::MapMode::Read, ...);
gpu.device.poll(wgpu::Maintain::Poll);
let data = buffer.get_mapped_range().to_vec();
```

**Challenges**:
- `IDXGIResource` must be wrapped to implement `wgpu::Texture` interface
- Requires maintaining a `GpuCore` reference in `DxgiCaptureState`
- Need to handle texture format conversion (DXGI → wgpu)

**Alternative Approach** (if wgpu integration is complex):
- Use `dxgi::1_2::IDXGIOutputDuplication::GetFrame` with CPU readback
- Simpler but slightly slower (still 5× faster than BitBlt)

**Testing**:
- Verify random frame content (not all black)
- Measure latency vs BitBlt fallback
- Test multi-monitor capture
- Verify no memory leaks over 24hr run

**Estimated Effort**: 2-3 days

---

### 1.2 Add Integration Tests
**Priority**: P1 (High)  
**Current**: No integration tests exist

**Required Tests**:

1. **Full Render Pipeline Test** (`tests/integration_render_test.rs`)
```rust
#[test]
fn test_full_render_loop() {
    // 1. Initialize GpuCore
    // 2. Create test window
    // 3. Load test .flow package
    // 4. Run 60 frames
    // 5. Verify textures uploaded
    // 6. Verify uniform updates
    // 7. Cleanup
}
```

2. **Multi-Monitor Initialization Test**
```rust
#[test]
fn test_multi_monitor_window_creation() {
    // 1. Enumerate monitors
    // 2. Create MonitorWindow for each
    // 3. Verify surfaces created
    // 4. Verify correct monitor rects
}
```

3. **Desktop Capture Accuracy Test** (`tests/screenshot_tests.rs`)
```rust
#[test]
fn test_capture_not_black() {
    // 1. Capture desktop
    // 2. Verify pixels are not all 0x00
    // 3. Verify some pixels match known desktop color
}
```

4. **End-to-End Mode Test**
```rust
#[test]
fn test_animation_mode_pipeline() {
    // Animation: Overlay window creation + render loop
}

#[test]
fn test_wallpaper_mode_pipeline() {
    // Wallpaper: WorkerW + child window + desktop capture
}
```

**Testing Framework**:
- Use `tempfile` for temporary .flow packages
- Mock windows where possible (hard to test Windows APIs on CI)
- Consider `windows::TestUtils` for unit testing

**Estimated Effort**: 3-4 days

---

### 1.3 Fix Performance Benchmarks
**File**: `benches/performance.rs`  
**Priority**: P1 (High)  
**Current Score**: 65/100

**Current Issues**:
- Missing: package loading, desktop capture, GPU upload, full render pipeline
- No memory tracking
- No thermal detection

**Required Additions**:

```rust
// 1. Package Loading Benchmark
fn bench_package_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("loading");
    group.bench_function("load_1mb_flow", |b| {
        b.iter(|| FlowPackage::load("test.flow"))
    });
    group.bench_function("extract_background", |b| {
        b.iter(|| extract_background(&flow))
    });
    group.finish();
}

// 2. Desktop Capture Benchmark
fn bench_desktop_capture(c: &mut Criterion) {
    let mut group = c.benchmark_group("capture");
    group.bench_function("dxgi_1080p", |b| {
        b.iter(|| unsafe {
            screenshot::capture_or_fallback(1920, 1080, None)
        })
    });
    group.bench_function("bitblt_1080p_fallback", |b| {
        // Direct BitBlt benchmark
    });
    group.finish();
}

// 3. GPU Upload Benchmark
fn bench_gpu_upload(c: &mut Criterion) {
    let mut group = c.benchmark_group("gpu");
    group.bench_function("upload_1920x1080_bgra", |b| {
        b.iter(|| upload_background(&gpu, &tex, &data, 1920, 1080))
    });
    group.finish();
}

// 4. Memory Usage Tracking
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_rss");
    group.measurement_time(std::time::Duration::from_secs(10));
    group.bench_function("steady_state_rss", |b| {
        b.iter(|| {
            // Run 1000 frames, measure RSS
            let rss = get_rss_kb();
            criterion::black_box(rss);
        })
    });
    group.finish();
}
```

**Add diagnostic tools**:
- `MemoryProfiler` struct for tracking allocations
- `ThermalMonitor` for detecting CPU/GPU throttling
- `FramePacer` for measuring frame time variance

**Estimated Effort**: 2 days

---

## Phase 2: Medium Priority Features (Week 2-3)

### 2.1 Animated Parameter Interpolation
**File**: `src/logic.rs`, `src/loader.rs`  
**Priority**: P2 (Medium)

**Current State**: 
- Logic parameters (p1-p4) are static values from config.toml
- No support for keyframe animation

**Proposed Enhancement**:

```toml
[logic.params.p1]
# Animated parameter example
keyframes = [
  { time = 0.0, value = 0.0 },
  { time = 1.0, value = 1.0 },
  { time = 2.0, value = 0.0 }
]
interpolation = "linear"  # or "smooth", "step"

# Static parameter (existing behavior)
value = 1.0
```

**Implementation**:
1. Add `AnimParam` struct to `loader.rs`:
```rust
struct AnimParam {
    keyframes: Vec<(f32, f32)>,  // (time, value)
    interpolation: InterpolationType,
    current_value: f32,
}
```

2. Update `LogicEngine::update()` to interpolate:
```rust
fn update(&self, flow: &FlowPackage, mouse_rel: [f32; 2]) -> Uniforms {
    let elapsed = self.start_time.elapsed().as_secs_f32();
    
    // Interpolate animated parameters
    let p1 = flow.anim_param("p1").interpolate(elapsed);
    
    Uniforms {
        logic_params: [p1, p2, p3, p4],
        // ...
    }
}
```

3. Add interpolation methods:
- Linear: `lerp(a, b, t)`
- Smooth: `smoothstep`
- Step: `floor(t)`

**Benefits**:
- Enables complex animations without code changes
- Reduces need for multiple .flow packages

**Estimated Effort**: 3-4 days

---

### 2.2 Runtime Parameter Updates
**File**: `src/logic.rs`, `src/windows.rs`  
**Priority**: P2 (Medium)

**Current State**: 
- Parameters only read at initialization
- Requires package reload to change values

**Proposed Solution**:
1. Add file watcher using `notify` crate:
```rust
struct ConfigWatcher {
    recommended_rx: Receiver<ConfigEvent>,
}
```

2. Hot-reload logic:
```rust
impl LogicEngine {
    pub fn try_reload(&mut self, flow: &FlowPackage) {
        if self.config_version != flow.version {
            self.update_cache(flow);
            self.config_version = flow.version;
        }
    }
}
```

3. Add IPC channel for GUI updates:
- Named pipes or Windows messages
- GUI editor → running process communication

**Estimated Effort**: 2 days

---

### 2.3 Parameter Range Validation
**File**: `src/logic.rs`  
**Priority**: P2 (Medium)

**Current State**: 
- `validate_param()` exists but not comprehensive
- No warnings for unusual values

**Enhancement**:
```rust
fn validate_param(value: f32) -> f32 {
    // Detect potential issues
    if value.abs() > 1e6 {
        eprintln!("WARNING: Logic parameter {} exceeds safe range (±1e6)", value);
    }
    if value.is_nan() {
        eprintln!("WARNING: Logic parameter is NaN, clamping to 0");
        return 0.0;
    }
    value.clamp(-1_000_000.0, 1_000_000.0)
}
```

**Add to config.toml schema**:
```toml
[logic.params.p1]
type = "float"
min = 0.0
max = 10.0
default = 1.0
description = "Animation speed multiplier"
```

**Estimated Effort**: 1 day

---

### 2.4 Thermal Throttling Protection
**File**: `src/engine.rs`  
**Priority**: P2 (Medium)

**Current State**: 
- No performance monitoring
- Could overheat on low-end hardware

**Implementation**:
1. Add `PerformanceMonitor` struct:
```rust
struct PerformanceMonitor {
    frame_times: RingBuffer<Duration>,
    gpu_usage: f32,
    cpu_temp: Option<f32>,
    throttled: bool,
}
```

2. Adaptive quality scaling:
```rust
impl PerformanceMonitor {
    fn adjust_quality(&mut self) -> QualityLevel {
        if self.avg_frame_time() > Duration::from_millis(20) {
            // Below 50 FPS, reduce quality
            self.throttled = true;
            QualityLevel::Reduced
        } else if self.avg_frame_time() < Duration::from_millis(12) {
            // Above 83 FPS, increase quality
            QualityLevel::High
        } else {
            QualityLevel::Normal
        }
    }
}
```

3. Integration with render loop:
```rust
// In render loop
let quality = perf_monitor.adjust_quality();
match quality {
    QualityLevel::High => render_full_resolution(),
    QualityLevel::Reduced => render_half_resolution(),
    _ => render_normal(),
}
```

**Estimated Effort**: 3 days

---

### 2.5 Package Validation Tool
**File**: `src/builder.rs` (new), `src/loader.rs`  
**Priority**: P2 (Medium)

**Current State**: 
- No validation of .flow packages
- Could produce invalid packages silently

**Create `src/builder.rs`**:
```rust
pub struct PackageBuilder {
    output_path: PathBuf,
    compression_level: i32,
}

impl PackageBuilder {
    pub fn new(path: impl Into<PathBuf>) -> Self;
    
    pub fn validate(&self, flow_path: impl AsRef<Path>) -> Result<ValidationReport>;
    
    pub fn build(&self, source: impl AsRef<Path>) -> Result<()>;
    
    pub fn set_compression_level(&mut self, level: i32) -> &mut Self;
}
```

**Validation Checks**:
1. ✅ config.toml present and valid TOML
2. ✅ Required fields: `[meta]`, `[logic]`
3. ✅ All referenced files exist (shader.wgsl, background.png)
4. ✅ File sizes within limits (8K max dimensions)
5. ✅ No path traversal in file paths
6. ✅ Shader compiles without errors
7. ✅ Logic parameters within safe ranges

**CLI Integration** (`src/cli.rs`):
```rust
Commands::Builder { 
    action: BuilderAction,
}

enum BuilderAction {
    Create { source: PathBuf, output: PathBuf },
    Validate { flow_file: PathBuf },
    Inspect { flow_file: PathBuf },
}
```

**Estimated Effort**: 3 days

---

### 2.6 GUI Settings Editor
**Priority**: P2 (Medium)  
**Scope**: New binary (`settings_editor.rs`)

**Technology Choice**: 
- Option A: Tauri (Rust + WebView2) - Modern, lightweight
- Option B: egui (immediate-mode GUI) - Pure Rust
- Option C: WinUI 3 via windows-rs - Native Windows look

**Recommended**: egui for simplicity and cross-platform compatibility

**Features**:
1. Load/save config.toml
2. Live preview of parameter changes
3. Shader reload button
4. Multi-monitor config
5. Performance metrics display

**Estimated Effort**: 5-7 days (complete UI)

---

## Phase 3: Low Priority (Week 4+)

### 3.1 Additional Waveforms
**File**: `src/soundgenerator.rs`  
**Priority**: P3 (Low)

**Add Support**:
- Square wave
- Triangle wave
- Sawtooth wave
- Custom waveform (samples array)

**Implementation**:
```rust
pub enum Waveform {
    Sine,
    Square { duty_cycle: f32 },
    Triangle,
    Sawtooth,
    Noise,
    Custom(Vec<f32>),
}

pub fn generate_wave(wave: Waveform, freq: f32, duration: f32, sample_rate: u32) -> Vec<f32> {
    match wave {
        Waveform::Square => generate_square_wave(freq, duration, sample_rate),
        // ...
    }
}
```

**Estimated Effort**: 1 day

---

### 3.2 Package Signing
**File**: `src/builder.rs`  
**Priority**: P3 (Low)

**Features**:
- Sign .flow packages with Ed25519 keys
- Verify signatures before loading
- Trust on first use (TOFU) model

**Estimated Effort**: 2-3 days

---

### 3.3 Settings Cloud Sync
**Priority**: P3 (Low)

**Options**:
- Dropbox API
- Google Drive API
- Custom sync server
- Git-based (commit config.toml to private repo)

**Estimated Effort**: 3-5 days

---

## Implementation Order

### Sprint 1 (Week 1): Critical Fixes
- [ ] Day 1-2: Fix DXGI capture (screenshot.rs)
- [ ] Day 3-4: Add integration tests
- [ ] Day 5: Fix performance benchmarks

**Deliverable**: Working DXGI capture, test suite, real benchmarks  
**Impact**: Screenshot module goes from 58→90/100

### Sprint 2 (Week 2-3): Core Features
- [ ] Day 1-2: Parameter interpolation (logic.rs)
- [ ] Day 3: Runtime parameter updates
- [ ] Day 4: Parameter validation
- [ ] Day 5-7: Thermal throttling protection

**Deliverable**: Animated parameters, adaptive performance  
**Impact**: Logic module goes from 92→95/100

### Sprint 3 (Week 3-4): Tools & Polish
- [ ] Day 1-3: Package validation tool (builder.rs)
- [ ] Day 4-7: CLI enhancements

**Deliverable**: Builder tool, validation, package inspection  
**Impact**: Builder module goes from 78→85/100

### Sprint 4+ (Optional): Nice-to-Have
- [ ] GUI settings editor
- [ ] Additional waveforms
- [ ] Cloud sync
- [ ] Package signing

---

## Success Metrics

1. **DXGI Capture**: 0 black frames in 10,000 frame test
2. **Performance**: <1ms capture latency (vs current 5-7ms fallback)
3. **Test Coverage**: >80% line coverage (from current ~40%)
4. **Benchmark Validity**: All critical paths benchmarked (from current 3 to 8+)
5. **Features**: Zero critical issues in evaluation report

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| DXGI wgpu integration too complex | Medium | High | Fallback to CPU readback (still 5× faster) |
| Windows API testing difficulties | Medium | Medium | Mock where possible, Windows CI runner |
| Thermal monitoring not portable | Low | Medium | Windows-only feature, use appropriate APIs |
| GUI editor scope creep | High | Medium | Start with CLI alternative, minimal UI |

---

## Notes

1. **DXGI Capture**: The biggest impact item. Even a basic implementation 
   (without wgpu integration) would be 5× faster than BitBlt.

2. **Integration Tests**: Essential before making DXGI changes to ensure 
   no regressions in existing functionality.

3. **Incremental Delivery**: Each sprint delivers standalone value. 
   Sprint 1 alone would bring the project from 83→90/100.

4. **Testing Strategy**: Focus on platform-specific tests (Windows) 
   using GitHub Actions with Windows runners.