//! ScreenAnimation - GPU-accelerated screen animations and wallpaper engine for Windows
//!
//! This crate provides a high-performance rendering engine for real-time animations
//! using WGPU and WGSL shaders. It supports two operation modes:
//! - V1 (Simple): Continuous animation with mouse interaction
//! - V2 (Sequence): Time-based sequences with multiple shader/sound steps
//!
//! # Architecture
//!
//! The crate is organized into four main modules:
//! - `engine`: WGPU core abstraction (device, pipelines, bind groups)
//! - `loader`: .flow package parsing (ZIP archives with shaders, audio, configs)
//! - `logic`: Uniform buffer calculation per frame
//! - `windows`: Windows API integration for window management and desktop embedding
//!
//! # Example
//!
//! ```no_run
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

pub mod engine;
pub mod loader;
pub mod logic;
pub mod windows;

/// Re-export of core GPU types for convenience
pub use engine::{GpuCore, Uniforms, WindowWrapper};
/// Re-export of package loader
pub use loader::FlowPackage;
/// Re-export of logic engine
pub use logic::LogicEngine;
/// Re-export of Windows integration functions
pub use windows::{init_windows, MonitorWindow};
