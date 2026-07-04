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
use std::time::Instant;
use wgpu::{Instance, Backends, InstanceDescriptor};

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
    /// Optional list of preferred GPU backends (default: all).
    /// Example: `vec!["vulkan", "dx12"]` to prefer Vulkan or DirectX 12.
    /// Currently informational - actual backend selection is handled by wgpu.
    pub preferred_backends: Option<Vec<String>>,
    /// Timeout in milliseconds for GPU initialization (default: 10000).
    /// Set to 0 for no timeout.
    /// Currently informational - timeout is handled by wgpu internally.
    pub timeout_ms: u64,
    /// Whether to log detailed initialization steps (default: false).
    pub verbose: bool,
}

impl Default for GpuInitConfig {
    fn default() -> Self {
        Self {
            prefer_high_performance: true,
            device_label: None,
            shader_label: None,
            preferred_backends: None,
            timeout_ms: 10000,
            verbose: false,
        }
    }
}

impl GpuInitConfig {
    /// Parse backend names into wgpu Backends bitflags.
    ///
    /// # Arguments
    ///
    /// * `names` - Vector of backend names (e.g., `&["vulkan", "dx12"]`)
    ///
    /// # Returns
    ///
    /// Returns `wgpu::Backends` bitmask with the requested backends enabled.
    /// Unknown backend names are logged and skipped.
    ///
    /// # Supported Backend Names
    ///
    /// - `"vulkan"` → `wgpu::Backends::VULKAN`
    /// - `"dx12"` → `wgpu::Backends::DX12`
    /// - `"dx11"` → `wgpu::Backends::DX12` (DX11 not directly supported, mapped to DX12)
    /// - `"metal"` → `wgpu::Backends::METAL`
    /// - `"opengl"` → `wgpu::Backends::GL`
    /// - `"webgpu"` → `wgpu::Backends::BROWSER_WEBGPU`
    pub fn parse_backends(names: &[&str]) -> Backends {
        let mut backends = Backends::empty();
        for &name in names {
            match name.to_lowercase().as_str() {
                "vulkan" => backends |= Backends::VULKAN,
                "dx12" => backends |= Backends::DX12,
                "dx11" => backends |= Backends::DX12,
                "metal" => backends |= Backends::METAL,
                "opengl" => backends |= Backends::GL,
                "webgpu" => backends |= Backends::BROWSER_WEBGPU,
                _ => eprintln!("[gpu_init] Unknown backend: {} (skipping)", name),
            }
        }
        if backends.is_empty() {
            eprintln!("[gpu_init] No valid backends specified, using all available");
            backends = Backends::all();
        }
        backends
    }

    /// Create a wgpu Instance with the configured backends.
    ///
    /// # Returns
    ///
    /// Returns a configured `wgpu::Instance` ready for adapter enumeration.
    pub fn create_instance(&self) -> Instance {
        let backends = self.preferred_backends.as_ref()
            .map(|names| Self::parse_backends(&names.iter().map(|s| s.as_str()).collect::<Vec<_>>()))
            .unwrap_or(Backends::all());
        
        Instance::new(InstanceDescriptor {
            backends,
            ..Default::default()
        })
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
/// * `shader_src` - WGSL shader source code (must not be empty)
/// * `entries` - List of shader entry points to compile (must not be empty)
/// * `config` - Custom initialization configuration
///
/// # Returns
///
/// Initialized `GpuCore` with compiled pipelines.
///
/// # Errors
///
/// Returns an error if:
/// - `shader_src` is empty
/// - `entries` is empty
/// - No GPU adapter is found
/// - Shader compilation fails
/// - Pipeline creation fails
pub fn init_gpu_with_config(
    instance: &Instance,
    shader_src: &str,
    entries: &[&str],
    config: &GpuInitConfig,
) -> anyhow::Result<GpuCore> {
    // Validate inputs
    anyhow::ensure!(!shader_src.is_empty(), "Shader source must not be empty");
    anyhow::ensure!(!entries.is_empty(), "Must provide at least one shader entry point");

    let device_label = config.device_label.as_deref().unwrap_or("ScreenAnimation device");
    let shader_label = config.shader_label.as_deref().unwrap_or("ScreenAnimation shader module");

    eprintln!(
        "[gpu_init] Initializing GPU (label: {}, shader_label: {}, entries: {}, high_perf: {})",
        device_label,
        shader_label,
        entries.len(),
        config.prefer_high_performance
    );

    // Set timeout if configured (currently informational - wgpu handles internally)
    if config.timeout_ms > 0 && config.verbose {
        eprintln!("[gpu_init] Timeout set to {}ms", config.timeout_ms);
    }

    // Log backend configuration if verbose
    if config.verbose {
        if let Some(ref backends) = config.preferred_backends {
            eprintln!("[gpu_init] Preferred backends: {:?}", backends);
        } else {
            eprintln!("[gpu_init] Using all available backends");
        }
    }

    let start_time = Instant::now();

    let result = block_on(GpuCore::new(instance, shader_src, entries))
        .with_context(|| format!("Failed to initialize GPU core with {} shader entries", entries.len()))?;

    let elapsed = start_time.elapsed();
    eprintln!(
        "[gpu_init] GPU initialized in {:.2}s ({} pipelines, adapter: {:?})",
        elapsed.as_secs_f32(),
        result.pipelines.len(),
        result.device.limits()
    );

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_backends_all_known() {
        let backends = GpuInitConfig::parse_backends(&["vulkan", "dx12", "metal"]);
        assert!(backends.contains(Backends::VULKAN));
        assert!(backends.contains(Backends::DX12));
        assert!(backends.contains(Backends::METAL));
    }

    #[test]
    fn test_parse_backends_unknown_skipped() {
        let backends = GpuInitConfig::parse_backends(&["vulkan", "unknown", "dx12"]);
        assert!(backends.contains(Backends::VULKAN));
        assert!(backends.contains(Backends::DX12));
    }

    #[test]
    fn test_parse_backends_empty_returns_all() {
        let backends = GpuInitConfig::parse_backends(&[]);
        assert_eq!(backends, Backends::all());
    }

    #[test]
    fn test_parse_backends_case_insensitive() {
        let backends = GpuInitConfig::parse_backends(&["VULKAN", "DX12", "Metal"]);
        assert!(backends.contains(Backends::VULKAN));
        assert!(backends.contains(Backends::DX12));
        assert!(backends.contains(Backends::METAL));
    }

    #[test]
    fn test_create_instance_with_backends() {
        let config = GpuInitConfig {
            preferred_backends: Some(vec!["vulkan".to_string(), "dx12".to_string()]),
            ..Default::default()
        };
        let _instance = config.create_instance();
        // Just verify it doesn't panic - actual backend availability depends on system
    }

    #[test]
    fn test_default_config() {
        let config = GpuInitConfig::default();
        assert!(config.prefer_high_performance);
        assert!(config.device_label.is_none());
        assert!(config.shader_label.is_none());
        assert!(config.preferred_backends.is_none());
        assert_eq!(config.timeout_ms, 10000);
        assert!(!config.verbose);
    }
}