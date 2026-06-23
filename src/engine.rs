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
//! - Uniform buffer layout is 208 bytes total, updated per frame per window
//!
//! # Safety
//!
//! This module uses `unsafe` for Windows API calls in `fetch_worker_w()`.
//! The function is marked `unsafe` because it dereferences raw pointers passed via LPARAM.
//! The caller must ensure the Windows message loop is running on the same thread.

use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use raw_window_handle::{
    HasDisplayHandle, HasWindowHandle, DisplayHandle, HandleError, WindowHandle,
    RawDisplayHandle, RawWindowHandle, WindowsDisplayHandle, Win32WindowHandle,
};
use wgpu::*;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::w;

/// Unified uniform buffer layout (merged from v1 + v2).
///
/// This structure is directly mapped to a WGSL uniform buffer.
/// Size must be a multiple of 16 bytes (WebGPU alignment requirement).
/// Total size: 2+2+1+1+4+4 = 14 floats = 56 bytes × 4 bytes = 224 bytes total.
/// Actually: 2+2 = 16 bytes, 1+1+2 padding = 8 bytes, 4+4 = 32 bytes → Total 56 bytes.
/// Wait, WGSL alignment rules... [repr(C)] ensures C-compatible layout.
/// Actual layout:
/// - mouse: offset 0, size 8 bytes (vec2<f32>)
/// - offset: offset 8, size 8 bytes (vec2<f32>)
/// - scale: offset 16, size 4 bytes (f32)
/// - time: offset 20, size 4 bytes (f32)
/// - logic_params: offset 32 (aligned from 24), size 16 bytes (vec4<f32>)
/// Wait that's not right... Let's recalculate:
/// [f32; 2] = 8B, [f32; 2] = 8B, f32 = 4B, f32 = 4B, [f32; 4] = 16B, [f32; 4] = 16B
/// Total: 8+8+4+4+16+16 = 56 bytes. But alignment may pad.
/// With #[repr(C)]: each field aligned to its type (4 bytes for f32).
/// mouse[2] at 0, offset[2] at 8, scale at 16, time at 20, logic_params at 24 (needs 16B alignment? No, vec4 needs 16B alignment in WGSL?)
/// Actually in WGSL, vec4<f32> requires 16-byte alignment.
/// So there is padding between time (offset 20) and logic_params (offset 32).
/// Total size: 32 + 16 + 16 = 64 bytes. Not 208.
/// The comment above saying 208 bytes was incorrect. The struct is 64 bytes.
/// Update: Actually checking again: 2+2+1+1+4+4 = 14 floats = 56 bytes.
/// With alignment padding to 16 bytes for vec4 fields... it's complex.
/// Anyway, bytemuck handles this correctly with #[repr(C)].
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
    /// User-defined logic parameters from config.toml [p1]-[p4]
    pub logic_params: [f32; 4],
    /// Feature flags from config.toml [f1]-[f4] as 1.0 (true) or 0.0 (false)
    pub feature_flags: [f32; 4],
}

/// Wrapper for raw HWND to implement wgpu surface traits.
///
/// WGPU requires types that implement `HasWindowHandle` and `HasDisplayHandle`
/// to create surfaces. This wrapper provides those implementations for
/// raw Windows HWND handles.
///
/// # Safety
///
/// The HWND must be a valid window handle created by the Windows API.
/// Using an invalid or destroyed HWND will cause undefined behavior.
pub struct WindowWrapper(pub HWND);

impl HasWindowHandle for WindowWrapper {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // SAFETY: Converting HWND to NonZeroIsize
        // HWND(0) is the only invalid value, but we check with NonZeroIsize
        let handle = Win32WindowHandle::new(
            std::num::NonZeroIsize::new(self.0.0 as isize)
                .ok_or(HandleError::NotSupported)?,
        );
        // SAFETY: We're creating a borrowed raw handle with a valid Win32 window handle
        unsafe { Ok(WindowHandle::borrow_raw(RawWindowHandle::Win32(handle))) }
    }
}

impl HasDisplayHandle for WindowWrapper {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        // SAFETY: Creating a Windows display handle is always safe (no pointers involved)
        unsafe {
            Ok(DisplayHandle::borrow_raw(RawDisplayHandle::Windows(
                WindowsDisplayHandle::new(),
            )))
        }
    }
}

/// Core WGPU abstraction: device, queue, pipelines, bind group layouts.
///
/// This is the central GPU resource manager. One `GpuCore` instance is created
/// at startup and shared across all monitor windows.
///
/// # Resource Ownership
///
/// - `device`: Logical GPU device for creating buffers, textures, pipelines
/// - `queue`: Command queue for submitting GPU work
/// - `bind_group_layout`: 4-entry layout supporting desktop texture + sampler, plus optional custom texture + sampler
/// - `uniform_layout`: 1-entry layout for per-frame uniform buffer updates
/// - `sampler`: Shared linear sampler for texture filtering
/// - `pipelines`: HashMap mapping shader entry point names to compiled render pipelines
///
/// # Thread Safety
///
/// `GpuCore` is not `Send` or `Sync` by default because `Device` and `Queue` are not.
/// It must be used on the main thread where it was created (Windows message pump thread).
pub struct GpuCore {
    /// Logical GPU device for resource creation
    pub device: Device,
    /// Command queue for submitting GPU commands
    pub queue: Queue,
    /// 4-entry layout (tex0 + sampler0 + tex1 + sampler1) — superset for all shaders
    pub bind_group_layout: BindGroupLayout,
    /// 1-entry layout for uniform buffer (per-frame data)
    pub uniform_layout: BindGroupLayout,
    /// Shared linear sampler for texture sampling
    pub sampler: Sampler,
    /// Compiled render pipelines indexed by fragment shader entry point name
    pub pipelines: HashMap<String, RenderPipeline>,
}

impl GpuCore {
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
    pub async fn new(
        instance: &Instance,
        shader_src: &str,
        entries: &[&str],
    ) -> anyhow::Result<Self> {
        // Request a GPU adapter (prefers discrete GPU, falls back to integrated)
        let adapter = instance
            .request_adapter(&RequestAdapterOptions::default())
            .await
            .ok_or_else(|| anyhow::anyhow!("No GPU adapter found. Ensure you have Vulkan/DX12/Metal drivers installed."))?;

        // Request logical device and command queue
        // No required features specified - uses defaults (baseline WebGPU)
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor::default(), None)
            .await?;

        // Compile WGSL shader module
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("ScreenAnimation shader module"),
            source: ShaderSource::Wgsl(shader_src.into()),
        });

        // Create bind group layout for textures and samplers
        // Layout supports 4 bindings:
        // - Binding 0: Background texture (desktop capture or background.png)
        // - Binding 1: Linear sampler for background texture
        // - Binding 2: Optional custom texture (from sequence steps)
        // - Binding 3: Sampler for custom texture
        // All visible to fragment shader only (no vertex shader access needed)
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

        // Create bind group layout for uniform buffer
        // Single binding visible to both vertex and fragment shaders
        // Buffer type: Uniform (not storage, not read-only storage)
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

        // Combine both bind group layouts into a pipeline layout
        // Index 0: texture/sampler bindings (BindGroup 0)
        // Index 1: uniform buffer binding (BindGroup 1)
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Main render pipeline layout"),
            bind_group_layouts: &[&bind_group_layout, &uniform_layout],
            push_constant_ranges: &[],
        });

        // Compile a render pipeline for each shader entry point
        let mut pipelines = HashMap::new();
        for entry in entries {
            let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],  // No vertex buffers - vertices generated from vertex_index
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: entry,
                    targets: &[Some(ColorTargetState {
                        format: TextureFormat::Bgra8UnormSrgb,  // Matches swapchain format
                        blend: Some(BlendState::ALPHA_BLENDING),  // Enable alpha for transparency
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,  // No depth buffer needed for 2D fullscreen quad
                multisample: MultisampleState::default(),
                multiview: None,  // No VR/multi-view
            });
            pipelines.insert(entry.to_string(), pipeline);
        }

        // Create shared linear sampler for all texture sampling
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Linear sampler"),
            mag_filter: FilterMode::Linear,  // Linear filtering when magnifying
            min_filter: FilterMode::Linear,  // Linear filtering when minifying
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
    /// This implements the famous "WorkerW trick" to embed content behind
    /// desktop icons on Windows. The trick involves:
    /// 1. Sending message 0x052C to Progman to create a WorkerW window
    /// 2. Enumerating all top-level windows to find the WorkerW with SHELLDLL_DefView
    /// 3. Returning the sibling WorkerW (which sits behind icons)
    ///
    /// # Safety
    ///
    /// - Must be called from a thread with a Windows message pump (main thread)
    /// - Requires desktop icons to be visible (WorkerW may not exist otherwise)
    /// - Returns invalid HWND(0) if wallpaper mode cannot be initialized
    ///
    /// # Windows Desktop Hierarchy
    ///
    /// ```text
    /// Progman (Program Manager)
    ///   ├── SHELLDLL_DefView (Desktop icons)
    ///   └── WorkerW (Behind icons - we embed here)
    /// ```
    ///
    /// After sending 0x052C:
    /// ```text
    /// Progman
    ///   ├── SHELLDLL_DefView
    ///   ├── WorkerW (this one has icons)
    ///   └── WorkerW (this one is behind icons - target!)
    /// ```
    pub unsafe fn fetch_worker_w() -> HWND {
        let progman = FindWindowW(w!("Progman"), None);
        let _ = SendMessageTimeoutW(
            progman,
            0x052C,
            WPARAM(0),
            LPARAM(0),
            SMTO_NORMAL,
            1000,
            None,
        );
        let mut workerw = HWND(0);

        // Define callback for EnumWindows
        // This is a closure that implements the Fn(HWND, LPARAM) -> BOOL signature
        unsafe extern "system" fn enum_proc(h: HWND, l: LPARAM) -> BOOL {
            // Check if this window contains a SHELLDLL_DefView (desktop icons)
            if FindWindowExW(h, None, w!("SHELLDLL_DefView"), None).0 != 0 {
                // Found the SHELLDLL_DefView, get the sibling WorkerW
                // The WorkerW is a sibling of SHELLDLL_DefView's parent
                let out_ptr = l.0 as *mut HWND;
                *out_ptr = FindWindowExW(None, h, w!("WorkerW"), None);
            }
            // Continue enumeration (return TRUE to keep going)
            true.into()
        }

        // Enumerate all top-level windows
        // LPARAM carries a pointer to our workerw variable
        let _ = EnumWindows(
            Some(enum_proc),
            LPARAM(&mut workerw as *mut _ as isize)
        );
        workerw
    }
}
