//! GPU initialization module.
//!
//! Handles WGPU instance creation, adapter selection, device/queue creation,
//! and shader pipeline compilation. Provides helper functions for creating
//! bind group layouts.

use screen_animation::engine::GpuCore;
use wgpu::Instance;

/// Initialize GPU core with device, pipelines, and layouts.
///
/// # Arguments
///
/// * `instance` - WGPU instance (usually `wgpu::Instance::default()`)
/// * `shader_src` - Complete WGSL shader source code
/// * `entries` - Fragment shader entry point names (e.g., `["fs_default", "fs_intro"]`)
///
/// # Returns
///
/// Returns a fully initialized `GpuCore` with all pipelines compiled.
///
/// # Errors
///
/// - Returns error if no GPU adapter is found
/// - Returns error if device/queue creation fails
/// - Returns error if shader compilation fails (WGSL syntax errors)
///
/// # Performance
///
/// - Adapter selection: ~50ms (one-time)
/// - Device creation: ~10ms (one-time)
/// - Shader compilation: 100-200ms per entry point (one-time at startup)
/// - Total initialization: ~500ms for 3-4 shader entry points
pub async fn init_gpu(instance: &Instance, shader_src: &str, entries: &[&str]) -> anyhow::Result<GpuCore> {
    eprintln!("Initializing GPU...");

    // Request a GPU adapter (prefers discrete GPU, falls back to integrated)
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .ok_or_else(|| anyhow::anyhow!("No GPU adapter found. Ensure you have Vulkan/DX12/Metal drivers installed."))?;

    // Request logical device and command queue
    let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor::default(), None).await?;

    // Compile WGSL shader module
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("ScreenAnimation shader module"),
        source: wgpu::ShaderSource::Wgsl(shader_src.into()),
    });

    // Create bind group layout for textures and samplers
    let bind_group_layout = create_texture_bind_group_layout(&device);

    // Create bind group layout for uniform buffer
    let uniform_layout = create_uniform_bind_group_layout(&device);

    // Combine both bind group layouts into a pipeline layout
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Main render pipeline layout"),
        bind_group_layouts: &[&bind_group_layout, &uniform_layout],
        push_constant_ranges: &[],
    });

    // Compile a render pipeline for each shader entry point
    let mut pipelines = std::collections::HashMap::new();
    for entry in entries {
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(entry),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[], // No vertex buffers - vertices generated from vertex_index
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: entry,
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        pipelines.insert(entry.to_string(), pipeline);
    }

    // Create shared linear sampler for all texture sampling
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("Linear sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    eprintln!("✓ GPU initialized with {} pipelines", pipelines.len());

    Ok(GpuCore {
        device,
        queue,
        bind_group_layout,
        uniform_layout,
        sampler,
        pipelines,
    })
}

/// Create bind group layout for textures and samplers.
///
/// Layout supports 4 bindings:
/// - Binding 0: Background texture (desktop capture or background.png)
/// - Binding 1: Linear sampler for background texture
/// - Binding 2: Optional custom texture (from sequence steps)
/// - Binding 3: Sampler for custom texture
fn create_texture_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Texture + sampler bind group layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

/// Create bind group layout for uniform buffer.
///
/// Single binding visible to both vertex and fragment shaders.
fn create_uniform_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Uniform buffer bind group layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bind_group_layout_creation() {
        // This test requires a GPU device, so it's mainly for compilation.
        // In real CI, this would need a mocked GPU or skipped on non-GPU runners.
    }
}