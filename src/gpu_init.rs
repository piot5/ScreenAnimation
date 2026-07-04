//! Shared GPU initialization module.
//!
//! This module provides GPU initialization functions that can be used by both
//! the animation engine and gui binaries. It acts as a bridge between the
//! async `GpuCore::new()` and synchronous binary entry points.
//!
//! # Design
//!
//! This module uses `pollster::block_on` to bridge the async gap. It provides
//! a single `init_gpu()` function with sensible defaults and optional configuration.
//!
//! # Performance
//!
//! - Shader compilation: ~200-500ms depending on shader complexity
//! - Pipeline creation: ~50ms per entry point
//! - Adapter enumeration: ~10-50ms (first call only)

use anyhow::Context;
use crate::engine::GpuCore;
use pollster::block_on;
use wgpu::Instance;

/// GPU initialization configuration.
///
/// Allows callers to customize the GPU initialization process.
/// Uses sensible defaults for most use cases.
#[derive(Debug, Clone)]
pub struct GpuInitConfig {
    /// Whether to prefer high-performance GPU (default: true).
    /// Set to false for low-power GPU (e.g., laptops on battery).
    pub prefer_high_performance: bool,
    /// Optional device label override (default: "ScreenAnimation device").
    pub device_label: Option<String>,
    /// Optional shader module label override (default: "ScreenAnimation shader module").
    pub shader_label: Option<String>,
}

impl Default for GpuInitConfig {
    fn default() -> Self {
        Self {
            prefer_high_performance: true,
            device_label: None,
            shader_label: None,
        }
    }
}

/// Initialize GPU core with shader compilation.
///
/// This is a shared function used by both animation engine and gui binaries.
/// It handles the async WGPU initialization synchronously using `pollster::block_on`.
///
/// # Arguments
///
/// * `instance` - WGPU instance (created via `wgpu::Instance::default()`)
/// * `shader_src` - WGSL shader source code
/// * `entries` - List of shader entry points to compile (e.g., `&["fs_main", "fs_intro"]`)
///
/// # Returns
///
/// Initialized `GpuCore` with compiled pipelines ready for rendering.
///
/// # Errors
///
/// Returns an error if:
/// - No GPU adapter is found (no Vulkan/DX12/Metal drivers)
/// - Device creation fails
/// - Shader compilation fails
/// - Pipeline creation fails
///
/// # Example
///
/// ```ignore
/// use screen_animation::gpu_init::{init_gpu, GpuInitConfig};
/// use wgpu::Instance;
///
/// let instance = Instance::default();
/// let gpu = init_gpu(&instance, &shader_src, &["fs_main"]).unwrap();
/// ```
pub fn init_gpu(instance: &Instance, shader_src: &str, entries: &[&str]) -> anyhow::Result<GpuCore> {
    init_gpu_with_config(instance, shader_src, entries, &GpuInitConfig::default())
}

/// Initialize GPU core with custom configuration.
///
/// Same as `init_gpu()` but allows overriding default configuration.
/// Use this when you need to customize GPU selection or labels.
///
/// # Arguments
///
/// * `instance` - WGPU instance
/// * `shader_src` - WGSL shader source code
/// * `entries` - List of shader entry points to compile
/// * `config` - Custom initialization configuration
///
/// # Returns
///
/// Initialized `GpuCore` with compiled pipelines.
pub fn init_gpu_with_config(
    instance: &Instance,
    shader_src: &str,
    entries: &[&str],
    config: &GpuInitConfig,
) -> anyhow::Result<GpuCore> {
    let label = config.device_label.as_deref().unwrap_or("ScreenAnimation device");
    eprintln!("[gpu_init] Initializing GPU (label: {}, entries: {})", label, entries.len());

    let result = block_on(GpuCore::new(instance, shader_src, entries))
        .with_context(|| format!("Failed to initialize GPU core with {} shader entries", entries.len()))?;

    eprintln!("[gpu_init] GPU initialized successfully ({} pipelines)", result.pipelines.len());
    Ok(result)
}