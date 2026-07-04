//! ScreenAnimation - GPU-accelerated screen animations and wallpaper engine for Windows
//!
//! This crate provides a high-performance rendering engine for real-time animations
//! using WGPU and WGSL shaders. It supports two operation modes:
//! - V1 (Simple): Continuous animation with mouse interaction
//! - V2 (Sequence): Time-based sequences with multiple shader/sound steps
//!
//! # Architecture
//!
//! The crate is organized into eight main modules:
//! - `engine`: WGPU core abstraction (device, pipelines, bind groups)
//! - `loader`: .flow package parsing (ZIP archives with shaders, audio, configs)
//! - `logic`: Uniform buffer calculation per frame
//! - `windows`: Windows API integration for window management and desktop embedding
//! - `background`: Background image loading, resizing, and GPU texture upload
//! - `screenshot`: Desktop capture via DXGI (with BitBlt fallback) for wallpaper mode
//! - `soundgenerator`: Real-time audio synthesis (sine waves, noise, envelopes)
//! - `settings`: Persistent user settings via confy + Windows Registry fallback
//!
//! # wgpu Version
//!
//! This crate uses wgpu 0.19. Key API notes:
//! - `ShaderSource::Wgsl` takes `Cow<'_, str>`
//! - `DeviceDescriptor` uses `label: Some("...")`
//! - `Surface::get_supported_formats()` is used (not `get_capabilities()`)
//! - `RenderPipelineDescriptor` entry points use `&str`
//! - `Instance::create_surface()` takes `impl Into<SurfaceTarget<'a>>` via raw-window-handle
//!
//! # DXGI Migration
//!
//! Desktop capture was migrated from BitBlt (GDI, CPU-bound, ~7ms) to
//! DXGI Output Duplication (GPU-accelerated, ~0.5ms). BitBlt is kept
//! as a fallback for RDP sessions, VMs, and Windows 7 systems.
//!
//! # Example
//!
//! ```ignore
//! // NOTE: Requires Windows SDK and GPU. Run with `cargo run --bin cli -- --package animation.flow`
//! use screen_animation::{engine::GpuCore, loader::FlowPackage, logic::LogicEngine, windows::init_windows};
//!
//! // Load animation package
//! let flow = FlowPackage::load("animation.flow").unwrap();
//!
//! // Initialize GPU
//! let instance = wgpu::Instance::default();
//! let gpu = pollster::block_on(GpuCore::new(&instance, &flow.shader_src, &["fs_default"])).unwrap();
//!
//! // Create windows on all monitors
//! let windows = unsafe { init_windows(&gpu, &instance, w!("WgpuAnim"), HINSTANCE(0), false, &flow) };
//! ```

pub mod background;
pub mod engine;
pub mod gpu_init;
pub mod loader;
pub mod logic;
pub mod screenshot;
pub mod settings;
pub mod soundgenerator;
pub mod windows;

/// Re-export of core GPU types for convenience
pub use engine::{GpuCore, Uniforms, WindowWrapper};
/// Re-export of package loader
pub use loader::FlowPackage;
/// Re-export of logic engine
pub use logic::LogicEngine;
/// Re-export of sound generator
pub use soundgenerator::{SoundGenerator, AudioBuffer, SoundParams};
/// Re-export of settings
pub use settings::AppSettings;
/// Re-export of Windows integration functions
pub use windows::{init_windows, register_window_class, MonitorWindow};
