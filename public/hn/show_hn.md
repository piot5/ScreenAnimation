Show HN: ScreenAnimation – GPU-accelerated screen animations in Rust/WGPU

I built a real-time animation engine for Windows that renders WGSL shaders directly to your screen. Can be used as transparent overlays or embedded as desktop wallpapers.

Tech stack:
- Rust + WGPU 0.19
- WGSL fragment shaders
- windows-rs for Win32 integration
- Multi-monitor support

The engine loads .flow packages (ZIP format) containing shaders, audio, and configs. Supports both simple mode (V1) and sequence-based mode (V2) with timed shader/sound transitions.

Performance: 60 FPS, ~50 MB base memory, single draw call per frame.

Source + docs: https://github.com/<user>/Build_ScreenAnimation

Curious for feedback on:
- Multi-monitor synchronization approaches
- Shader debugging workflows
- Security considerations for loading external shader packages