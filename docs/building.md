# Building Guide

## Prerequisites

- **Rust** 1.70+ (install via [rustup](https://rustup.rs/))
- **Windows 10/11** (required for Windows API integration)
- **Git** (optional, for cloning)

## Quick Build

```bash
# Clone repository
git clone https://github.com/piot5/ScreenAnimation.git
cd ScreenAnimation

# Build release binaries
cargo build --release
```

## Build Outputs

```
target/release/
├── animationengine.exe    # Main animation engine
└── builder.exe            # Package builder tool
```

## Project Structure

```
ScreenAnimation/
├── Cargo.toml              # Package manifest
├── src/
│   ├── lib.rs             # Library root, module exports
│   ├── engine.rs          # WGPU core (device, pipelines)
│   ├── loader.rs          # .flow package loader
│   ├── logic.rs           # Uniform buffer calculations
│   ├── windows.rs         # Windows API integration
│   ├── background.rs      # Image loading/GPU upload
│   ├── screenshot.rs      # Desktop capture
│   ├── soundgenerator.rs  # Audio synthesis
│   └── bin/
│       ├── animationengine.rs  # Main executable
│       └── builder.rs          # Package builder
├── tests/
│   ├── flow_loading.rs    # Integration tests
│   └── logic_tests.rs     # Unit tests
└── examples/              # Example .flow packages
```

## Building Documentation

```bash
# Generate and open documentation
cargo doc --open

# Generate with private items included
cargo doc --document-private-items --open
```

## Code Quality Checks

```bash
# Format code according to rustfmt.toml
cargo fmt

# Check formatting without modifying
cargo fmt -- --check

# Run Clippy linter
cargo clippy

# Run Clippy with stricter checks
cargo clippy -- -D warnings -W clippy::pedantic
```

## Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_logic_engine_creation

# Run tests with all features
cargo test --all-features
```

## Creating Example Packages

### 1. Create directory structure

```bash
mkdir my_animation
cd my_animation
```

### 2. Add required files

Create `config.toml`:
```toml
mode = "animation"
shader = "fs_default"

[p1]
p2 = 0.0
p3 = 0.0
p4 = 0.0

enable_effect = true
```

Create `shader.wgsl`:
```wgsl
struct Uniforms {
    mouse: vec2f,
    offset: vec2f,
    scale: f32,
    time: f32,
    logic_params: vec4f,
    feature_flags: vec4f
};

@group(1) @binding(0) var<uniform> u: Uniforms;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4f {
    var pos = array<vec2f, 6>(
        vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(-1.0, 1.0),
        vec2f(-1.0, 1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0)
    );
    return vec4f(pos[vi], 0.0, 1.0);
}

@fragment
fn fs_default(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    let uv = coord.xy / vec2f(textureDimensions(tex0));
    let bg = textureSample(tex0, samp0, uv);
    return vec4f(bg.rgb, 1.0);
}
```

### 3. Build package

```bash
# Using builder.exe
target\release\builder.exe --input my_animation --output my_animation.flow

# Or using PowerShell
Compress-Archive -Path config.toml, shader.wgsl -DestinationPath my_animation.flow -CompressionLevel None
```

### 4. Run animation

```bash
# Overlay mode
target\release\animationengine.exe Animation my_animation.flow

# Wallpaper mode
target\release\animationengine.exe Wallpaper my_animation.flow
```

## Development Workflow

### 1. Make changes to source code

Edit files in `src/`

### 2. Format and lint

```bash
cargo fmt
cargo clippy --fix
```

### 3. Run tests

```bash
cargo test
```

### 4. Build and test manually

```bash
cargo build
cargo run -- Animation examples\livewallpaper.flow
```

### 5. Build release

```bash
cargo build --release
```

## Troubleshooting

### Linker errors on Windows

Ensure you have the Visual Studio C++ Build Tools installed:
```bash
rustup default stable-x86_64-pc-windows-msvc
```

### WGPU adapter not found

- Update graphics drivers (Vulkan, DX12, or Metal)
- Check `wgpu` backend support: ` cargo run --features=wgpu/trace`

### Shader compilation fails

- Validate WGSL syntax: Use `naga` or online validator
- Check entry point names match config.toml
- Verify bind group layout matches shader

### Audio not playing

- Check default audio device is available
- Verify WAV format: PCM, 16-bit, 44100Hz
- Test with `rodio` example: `cargo test soundgenerator_tests`

## Advanced Builds

### Debug build with logging

```bash
cargo build
set RUST_LOG=debug
cargo run -- Animation my_animation.flow
```

### Profile-guided optimization

```bash
cargo build --profile release-with-debug
```

### Cross-compilation (Linux → Windows)

```bash
# Install mingw target
rustup target add x86_64-pc-windows-gnu

# Build
cargo build --target x86_64-pc-windows-gnu --release
```

## CI/CD

The project includes GitHub Actions workflow (`.github/workflows/ci.yml`):

- **Code Quality**: Format check, Clippy, trailing whitespace
- **Build & Test**: Windows build, test, release artifacts
- **Documentation**: Doc build verification
- **Security**: `cargo audit` for dependency vulnerabilities

### Running CI locally

```bash
# Install act (GitHub Actions local runner)
brew install act  # macOS
# or download from https://github.com/nektos/act

# Run workflow
act -j build-test
```

## Performance Profiling

### CPU Profiling (Windows)

```bash
# Build with debug symbols
cargo build --profile release-with-debug

# Run with Windows Performance Recorder
wpr -start CPU -record
target\release\animationengine.exe Animation my_animation.flow
wpr -stop profile.etl
```

### GPU Profiling

- Use **RenderDoc** or **Nsight Graphics** to capture frames
- Check pipeline bottlenecks, draw calls, GPU time

## Release Checklist

Before creating a release:

- [ ] All tests pass: `cargo test`
- [ ] No Clippy warnings: `cargo clippy -- -D warnings`
- [ ] Formatted: `cargo fmt -- --check`
- [ ] Documentation builds: `cargo doc --no-deps`
- [ ] Security audit: `cargo audit` (install via `cargo install cargo-audit`)
- [ ] Version bumped in `Cargo.toml`
- [ ] `CHANGELOG.md` updated
- [ ] Binaries tested on clean Windows machine