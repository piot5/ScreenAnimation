# Building ScreenAnimation

## Prerequisites

### Required Software

- **Rust Toolchain**: 1.70+ (stable channel)
  ```bash
  rustup update stable
  rustc --version  # Should show 1.70.0 or higher
  ```

- **Visual Studio Build Tools**: 2019 or 2022
  - C++ Build Tools workload
  - Windows 10/11 SDK
  - MSVC compiler (x64)

- **Git** (for cloning repository)

### GPU Driver Requirements

- **NVIDIA**: GeForce GTX 900+ or Quadro K2200+
- **AMD**: Radeon RX 400+ or Radeon Pro WX 7100+
- **Intel**: Arc Graphics (Gen 11+) or UHD 770+
- **Vulkan Support**: Required for WGPU fallback

## Quick Start

```bash
# Clone repository
git clone <repository-url>
cd Build_ScreenAnimation

# Build debug version
cargo build

# Run tests
cargo test

# Build release (optimized)
cargo build --release
```

## Build Modes

### Debug Build

```bash
cargo build
```

**Characteristics**:
- No optimizations
- Full debug symbols
- Assertions enabled
- Slower execution (~30 FPS cap due to debug overhead)
- Fast compilation

**Use for**: Development, debugging, testing

### Release Build

```bash
cargo build --release
```

**Optimizations** (from `Cargo.toml`):
```toml
[profile.release]
opt-level = "z"        # Optimize for size
lto = true             # Link-time optimization
codegen-units = 1      # Maximum optimization
panic = "abort"        # No unwinding
strip = true           # Remove symbols
```

**Characteristics**:
- Maximally optimized binary size
- No debug symbols
- ~60 FPS target performance
- Smaller executable (~2-3 MB)
- Slower compilation (2-3× longer)

**Use for**: Deployment, distribution

## Platform-Specific Instructions

### Windows x64 (MSVC)

This is the primary target platform.

```powershell
# Install Visual Studio Build Tools
# Download from: https://visualstudio.microsoft.com/downloads/
# Select: "Desktop development with C++"

# Verify MSVC compiler
cl.exe  # Should open Visual Studio command prompt

# Set up Rust for MSVC
rustup default stable-x86_64-pc-windows-msvc

# Build
cargo build --release
```

### Windows x64 (GNU)

Alternative toolchain using MinGW:

```powershell
# Install MinGW-w64
# Download from: https://www.mingw-w64.org/

# Set up Rust for GNU
rustup default stable-x86_64-pc-windows-gnu

# Configure linker (in config.toml or .cargo/config.toml)
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"

# Build
cargo build --release
```

### Windows ARM64

Experimental support:

```powershell
# Requires Visual Studio 2022 with ARM64 tools
rustup default stable-aarch64-pc-windows-msvc

# Build
cargo build --release
```

## Troubleshooting Build Errors

### Error: `link.exe not found`

**Cause**: Visual Studio C++ tools not installed or not in PATH.

**Solution**:
```powershell
# Open "Developer Command Prompt for VS"
# Or run:
& "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
cargo build
```

### Error: `Windows SDK version not found`

**Cause**: Missing Windows 10/11 SDK.

**Solution**:
```powershell
# Install via Visual Studio Installer:
# Individual components → Windows 10/11 SDK
```

### Error: `wgpu build fails`

**Cause**: Missing Vulkan/DX12 headers or incompatible GPU.

**Solution**:
```bash
# Update GPU drivers
# For NVIDIA: https://www.nvidia.com/drivers
# For AMD: https://www.amd.com/drivers

# Try forcing Vulkan backend:
cargo build --features wgpu/vulkan
```

### Error: `DllMain` or `GetModuleHandleW` linkage errors

**Cause**: Windows crate features missing in `Cargo.toml`.

**Solution**: Ensure `Cargo.toml` contains:
```toml
windows = {
    version = "0.54",
    features = [
        "Win32_Graphics_Gdi",
        "Win32_UI_WindowsAndMessaging",
        "Win32_System_LibraryLoader",
        "Win32_UI_HiDpi"
    ]
}
```

### Warning: `LNK4098` (Library mismatch)

**Cause**: Mixing debug/release CRTs.

**Solution**:
```powershell
# Clean build artifacts
cargo clean

# Rebuild with consistent flags
cargo build --release
```

## Dependencies

### Crate Versions (from `Cargo.toml`)

| Crate | Version | Purpose |
|-------|---------|---------|
| `wgpu` | 0.19 | GPU API abstraction layer |
| `windows` | 0.54 | Win32 API bindings |
| `rodio` | 0.17 | Audio playback |
| `clap` | 4.4 | Command-line argument parsing |
| `serde` | 1.0 | Serialization/deserialization |
| `toml` | 0.8 | TOML config parsing |
| `zip` | 0.6 | .flow package reading |
| `image` | 0.24 | Image decoding/resizing |
| `bytemuck` | 1.14 | Safe byte casting for GPU buffers |
| `pollster` | 0.3 | Async runtime (block_on) |
| `raw-window-handle` | 0.6 | Window handle abstraction |

### Upgrading Dependencies

```bash
# Check for updates
cargo outdated

# Update all to latest compatible versions
cargo update

# Update specific crate
cargo update -p wgpu
```

## Feature Flags

Currently no custom feature flags. Future plans:

- `debug`: Verbose logging, WGPU debug layers
- `no-audio`: Disable rodio for silent operation
- `validation`: Extra GPU validation layers

## Cross-Compilation

### Linux → Windows (not recommended)

WGPU requires native Windows APIs, so cross-compilation is not feasible.

### Windows → Linux (not supported)

Heavy Windows API usage (HWND, WorkerW, BitBlt) makes this impossible.

## Continuous Integration

### GitHub Actions (Example)

```yaml
# .github/workflows/build.yml
name: Build

on: [push, pull_request]

jobs:
  build:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo build --release
      - run: cargo test
```

## Packaging for Distribution

### Create Portable Binary

```powershell
# Build release
cargo build --release

# Copy binary
Copy-Item target\release\animationengine.exe dist\

# Include example packages
Copy-Item assets\animation1.flow dist\
Copy-Item assets\wallpaper1.flow dist\

# Create ZIP
Compress-Archive -Path dist\* -DestinationPath ScreenAnimation-v0.1.0-windows-x64.zip
```

### Installer (Inno Setup)

Create `installer.iss`:
```iss
[Setup]
AppName=ScreenAnimation
AppVersion=0.1.0
DefaultDirName={pf}\ScreenAnimation
OutputDir=installer

[Files]
Source: "dist\animationengine.exe"; DestDir: "{app}"
Source: "dist\*.flow"; DestDir: "{app}\examples"

[Icons]
Name: "{group}\ScreenAnimation"; Filename: "{app}\animationengine.exe"
```

Build with:
```bash
iscc installer.iss
```

## Development Workflow

### Test Loop

```bash
# 1. Build and run
cargo run -- Animation assets\animation1\animation1.flow

# 2. In separate terminal, monitor logs
# (Add logging in future versions)

# 3. Kill process (Ctrl+C)
# 4. Edit source
# 5. Repeat
```

### Hot Reload (Manual)

Since WGPU shaders are strings, you can implement runtime reloading:

```rust
// In main loop:
if std::fs::metadata("shader.wgsl").unwrap().modified()
    != last_modified
{
    let new_src = std::fs::read_to_string("shader.wgsl")?;
    // Recompile pipelines...
}
```

### Debug Logging

Add to `src/lib.rs`:
```rust
#[cfg(debug_assertions)]
macro_rules! log {
    ($($arg:tt)*) => {
        eprintln!("[DEBUG] {}", format!($($arg)*));
    };
}

#[cfg(not(debug_assertions))]
macro_rules! log {
    ($($arg:tt)*) => {};
}
```

## Performance Profiling

### CPU Profiling (Windows Performance Analyzer)

```powershell
# Record trace
wpr -start GPUScreenAnimation -onoff -record
animationengine.exe Animation test.flow
wpr -stop trace.etl

# Analyze with WPA
wpa.exe trace.etl
```

### GPU Profiling (NVIDIA Nsight)

```bash
# Launch with Nsight Graphics
NvidiaProfileGui.exe --launch animationengine.exe
```

### Memory Profiling (Valgrind equivalent)

```powershell
# Windows Performance Recorder
wpr -start Heap -heap -record
animationengine.exe Animation test.flow
wpr -stop heap.etl
```

## Release Checklist

Before tagging a release:

- [ ] Build passes in release mode: `cargo build --release`
- [ ] All tests pass: `cargo test --release`
- [ ] Binary runs without panics on test packages
- [ ] README.md updated with version number
- [ ] CHANGELOG.md updated
- [ ] Version bumped in `Cargo.toml`
- [ ] Create git tag: `git tag v0.2.0`
- [ ] Build distribution ZIP
- [ ] Test installer on clean Windows machine
- [ ] Update website/documentation

## Environment Variables

| Variable | Description |
|----------|-------------|
| `WGPU_DEBUG=1` | Enable WGPU debug layers |
| `WGPU_BACKEND=vulkan` | Force Vulkan backend |
| `WGPU_BACKEND=dx12` | Force DirectX 12 backend |
| `WGPU_BACKEND=gl` | Force OpenGL backend |
| `RUST_BACKTRACE=1` | Full stack traces on panic |

Example:
```bash
set WGPU_DEBUG=1
animationengine.exe Animation test.flow
```

## Common Build Configurations

### Minimum Size Build

```toml
# Cargo.toml
[profile.release]
opt-level = "z"
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true
```

Result: ~2 MB binary

### Maximum Performance Build

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
```

Result: ~4 MB binary, fastest execution

### Fast Compilation Build

```toml
[profile.dev]
opt-level = 0
debug = "line-tables-only"
split-debuginfo = "unpacked"
```

Result: Fast compilation, reasonable debug experience

## Notes

- **No macOS/Linux Support**: Windows-only due to platform-specific APIs
- **No Static Linking**: WGPU requires dynamic system libraries
- **UWP Not Supported**: Requires Win32 desktop API
- **Safe Rust**: No `unsafe` in application code except FFI boundaries