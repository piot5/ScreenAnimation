//! GPU core abstraction module.
//!
//! This module provides the central WGPU interface for screen_animation.
//! It manages the GPU device, command queue, render pipelines, and bind group layouts.
//! All rendering operations flow through this module.
//!
//! # Design
//!
//! - `GpuCore` owns all GPU resources and is shared across all monitor windows
//! - One pipeline per unique shader entry point (cached in HashMap)
//! - Bind group layout supports up to 4 texture/sampler pairs (2 for desktop, 2 for custom)
//!
//! # wgpu Migration Notes (0.19 → 22)
//!
//! - `Instance::create_surface()` takes `impl Into<SurfaceTarget<'a>>` from raw-window-handle
//! - `DeviceDescriptor` has `label` field (anyhow::Context not needed for simple errors)
//! - `ShaderSource::Wgsl(Cow::Borrowed(s))` format
//! - `RenderPipelineDescriptor` entry_point takes `&str`
//! - `Surface::get_supported_formats()` → `Surface::get_capabilities()`
//!
//! # Safety
//!
//! This module uses `unsafe` for Windows API calls in `fetch_worker_w()`.
//! The function is marked `unsafe` because it dereferences raw pointers passed via LPARAM.
//! The caller must ensure the Windows message loop is running on the same thread.

use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle,
    Win32WindowHandle, WindowHandle, WindowsDisplayHandle,
};
use wgpu::*;
use windows::core::w;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

/// Unified uniform buffer layout (merged from v1 + v2).
///
/// This structure is directly mapped to a WGSL uniform buffer.
/// Size must be a multiple of 16 bytes (WebGPU alignment requirement).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Uniforms {
    /// Normalized mouse position (0.0 to 1.0) relative to window
    pub mouse: [f32; 2],
    /// Translation offset (currently unused, reserved for future)
    pub offset: [f32; 2],
    /// Uniform scale factor (currently unused, always 1.0)
    pub scale: f32,
    /// Elapsed time in seconds since animation start
    pub time: f32,
    /// Padding to align vec4<f32> fields to 16-byte boundary (WGSL requirement)
    pub _padding: [f32; 2],
    /// User-defined logic parameters from config.toml [p1]-[p4]
    pub logic_params: [f32; 4],
    /// Feature flags from config.toml [f1]-[f4] as 1.0 (true) or 0.0 (false)
    pub feature_flags: [f32; 4],
}

/// Wrapper for raw HWND to implement wgpu surface traits.
pub struct WindowWrapper(pub HWND);

impl HasWindowHandle for WindowWrapper {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // SAFETY: Converting HWND to NonZeroIsize
        let handle = Win32WindowHandle::new(std::num::NonZeroIsize::new(self.0 .0).ok_or(HandleError::NotSupported)?);
        // SAFETY: raw handle with valid Win32 window
        unsafe { Ok(WindowHandle::borrow_raw(RawWindowHandle::Win32(handle))) }
    }
}

impl HasDisplayHandle for WindowWrapper {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        // SAFETY: Windows display handle is always safe
        unsafe { Ok(DisplayHandle::borrow_raw(RawDisplayHandle::Windows(WindowsDisplayHandle::new()))) }
    }
}

/// Core WGPU abstraction: device, queue, pipelines, bind group layouts.
///
/// One `GpuCore` instance is created at startup and shared across all monitor windows.
/// wgpu 22: Uses updated API with Cow-based shader sources and adjusted struct fields.
pub struct GpuCore {
    pub device: Device,
    pub queue: Queue,
    pub bind_group_layout: BindGroupLayout,
    pub uniform_layout: BindGroupLayout,
    pub sampler: Sampler,
    pub pipelines: HashMap<String, RenderPipeline>,
}

impl GpuCore {
    /// Initialize GPU core with device, pipelines, and layouts.
    ///
    /// wgpu 22: DeviceDescriptor with label, ShaderSource::Wgsl(Cow), entry_point as &str.
    pub async fn new(instance: &Instance, shader_src: &str, entries: &[&str]) -> anyhow::Result<Self> {
        // Try multiple adapter fallback strategies for maximum compatibility
        // 1. Try high-performance GPU first
        // 2. Fall back to low-power adapter
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await;

        let adapter = if adapter.is_some() {
            adapter
        } else {
            // Fallback: try low-power GPU
            instance
                .request_adapter(&RequestAdapterOptions {
                    power_preference: PowerPreference::LowPower,
                    ..Default::default()
                })
                .await
        }
        .ok_or_else(|| anyhow::anyhow!("No GPU adapter found. Ensure you have Vulkan/DX12/Metal drivers installed."))?;

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("ScreenAnimation device"),
                    ..Default::default()
                },
                None,
            )
            .await?;

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("ScreenAnimation shader module"),
            source: ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader_src)),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Texture + sampler bind group layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let uniform_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Uniform buffer bind group layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Main render pipeline layout"),
            bind_group_layouts: &[&bind_group_layout, &uniform_layout],
            push_constant_ranges: &[],
        });

        let mut pipelines = HashMap::new();
        for entry in entries {
            let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some(*entry),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: entry,
                    targets: &[Some(ColorTargetState {
                        format: TextureFormat::Bgra8UnormSrgb,
                        blend: Some(BlendState::ALPHA_BLENDING),
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
            });
            pipelines.insert(entry.to_string(), pipeline);
        }

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Linear sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        Ok(Self {
            device,
            queue,
            bind_group_layout,
            uniform_layout,
            sampler,
            pipelines,
        })
    }

    /// Find WorkerW window for wallpaper embedding (behind desktop icons).
    ///
    /// Implements the "WorkerW trick": send 0x052C to Progman to create WorkerW,
    /// then find the WorkerW that's behind desktop icons.
    ///
    /// # Safety
    ///
    /// - Must be called from thread with Windows message pump
    /// - Requires desktop icons to be visible
    pub unsafe fn fetch_worker_w() -> HWND {
        let progman = FindWindowW(w!("Progman"), None);
        let _ = SendMessageTimeoutW(progman, 0x052C, WPARAM(0), LPARAM(0), SMTO_NORMAL, 1000, None);

        let mut workerw = HWND(0);

        unsafe extern "system" fn enum_proc(h: HWND, l: LPARAM) -> BOOL {
            if FindWindowExW(h, None, w!("SHELLDLL_DefView"), None).0 != 0 {
                let out_ptr = l.0 as *mut HWND;
                *out_ptr = FindWindowExW(None, h, w!("WorkerW"), None);
            }
            true.into()
        }

        let _ = EnumWindows(Some(enum_proc), LPARAM(&mut workerw as *mut _ as isize));
        workerw
    }
}
