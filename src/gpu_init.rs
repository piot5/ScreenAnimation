//! Shared GPU initialization module.
//!
//! This module provides GPU initialization functions that can be used by both
//! the animationengine and gui binaries.
//!
//! # Design
//!
//! This module acts as a bridge between the async `GpuCore::new()` and synchronous
//! binary entry points. It uses `pollster::block_on` to bridge the async gap.
//!
//! # Performance
//!
//! - Shader compilation: ~200-500ms depending on shader complexity
//! - Pipeline creation: ~50ms per entry point

use anyhow::Context;
use crate::engine::GpuCore;
use pollster::block_on;
use wgpu::Instance;

/// Initialize GPU core with shader compilation.
///
/// This is a shared function used by both animationengine and gui binaries.
///
/// # Arguments
///
/// * `instance` - WGPU instance
/// * `shader_src` - WGSL shader source code
/// * `entries` - List of shader entry points to compile
///
/// # Returns
///
/// Initialized `GpuCore` with compiled pipelines
pub fn init_gpu(instance: &Instance, shader_src: &str, entries: &[&str]) -> anyhow::Result<GpuCore> {
    block_on(GpuCore::new(instance, shader_src, entries))
        .context("Failed to initialize GPU core")
}
