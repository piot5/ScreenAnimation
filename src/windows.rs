//! Windows integration module.
//!
//! This module handles native Windows API integration for window management,
//! desktop embedding, and screen capture. It's responsible for creating
//! animation windows on each monitor and configuring WGPU surfaces.
//!
//! # Key Responsibilities
//!
//! - Create windows for each monitor (overlay or wallpaper mode)
//! - Embed content behind desktop icons (WorkerW trick)
//! - Capture desktop background for wallpaper mode using DXGI
//! - Configure WGPU surfaces from native HWNDs
//!
//! # Safety
//!
//! This module makes extensive use of `unsafe` because:
//! - Windows API functions have undefined behavior with invalid handles
//! - Raw pointer manipulation for window enumeration callbacks
//! - GDI operations require careful resource cleanup
//!
//! All unsafe functions are documented with safety invariants.

use crate::background::{create_background_texture, load_background, upload_background};
use crate::engine::{GpuCore, WindowWrapper};
use crate::loader::FlowPackage;
use anyhow::Context;
use windows::core::w;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

/// Represents a fullscreen animation window on one monitor.
///
/// Each `MonitorWindow` contains all GPU resources needed to render one
/// animation instance on a single display. In multi-monitor setups, one
/// `MonitorWindow` is created per monitor.
///
/// # Memory Ownership
///
/// - `hwnd`: Owned by Windows, must be destroyed via `DestroyWindow` (not implemented)
/// - `surface`: Owned by wgpu, tied to HWND lifetime
/// - `texture_bind_group`: Owned, references GPU resources in GpuCore
/// - `uniform_buffer`: Owned, updated every frame with new uniform data
/// - `uniform_bind_group`: Owned, binds uniform_buffer to pipeline
/// - `desktop_tex`: Owned, contains background image/capture
/// - `rect`: Copied from monitor enumeration (RECT is Copy)
/// - `buffer`: Temporary buffer for desktop capture (reused each frame in some modes)
pub struct MonitorWindow {
    /// Native Windows window handle (HWND)
    pub hwnd: HWND,
    /// WGPU swapchain surface for rendering to this window
    pub surface: wgpu::Surface<'static>,
    /// Bind group for background texture + sampler (BindGroup 0)
    pub texture_bind_group: wgpu::BindGroup,
    /// Uniform buffer for per-frame data (mouse, time, etc.)
    pub uniform_buffer: wgpu::Buffer,
    /// Bind group for uniform buffer (BindGroup 1)
    pub uniform_bind_group: wgpu::BindGroup,
    /// Desktop background texture (BGRA8 format)
    pub desktop_tex: wgpu::Texture,
    /// Monitor rectangle in screen coordinates
    pub rect: RECT,
    /// Temporary buffer for desktop capture (BGRA format)
    pub buffer: Vec<u8>,
}

impl MonitorWindow {
    /// Create a new monitor window with background texture.
    ///
    /// # Arguments
    ///
    /// * `gpu` - GPU core with device, queue, layouts
    /// * `inst` - WGPU instance for surface creation
    /// * `hwnd` - Native window handle (must be valid)
    /// * `w` - Window width in pixels
    /// * `h` - Window height in pixels
    /// * `bg_buf` - Background image data in BGRA format
    /// * `rect` - Monitor rectangle (for mouse coordinate calculation)
    ///
    /// # Safety
    ///
    /// - `hwnd` must be a valid window created by CreateWindowExW
    /// - `bg_buf` must contain valid BGRA pixel data (w × h × 4 bytes)
    /// - Caller must ensure window message pump is running
    pub unsafe fn new_v1(
        gpu: &GpuCore,
        inst: &wgpu::Instance,
        hwnd: HWND,
        w: u32,
        h: u32,
        bg_buf: &[u8],
        rect: RECT,
    ) -> anyhow::Result<Self> {
        // Create GPU texture for background image using dedicated module
        let (bg_tex, bg_view) = create_background_texture(gpu, w, h);

        // Upload background image data to GPU texture using dedicated module
        upload_background(gpu, &bg_tex, bg_buf, w, h);

        // Create bind group for background texture + sampler
        // Uses all 4 bindings to support both desktop and custom textures
        // Bindings 0-1: Background texture + sampler
        // Bindings 2-3: Duplicate of background (allows shader to use tex1)
        let t_bg = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Background texture bind group"),
            layout: &gpu.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&bg_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&gpu.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&bg_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&gpu.sampler),
                },
            ],
        });

        // Create uniform buffer for per-frame data
        // Size is determined by the Rust struct (now 64 bytes with padding)
        let u_buf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniform buffer"),
            size: std::mem::size_of::<crate::engine::Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group for uniform buffer
        let u_bg = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform bind group"),
            layout: &gpu.uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: u_buf.as_entire_binding(),
            }],
        });

        // Create WGPU surface from native HWND
        // WindowWrapper implements the required traits for surface creation
        // wgpu 0.24: Instance::create_surface takes impl Into<SurfaceTarget<'a>>
        let surface = inst
            .create_surface(WindowWrapper(hwnd))
            .context("Failed to create WGPU surface for monitor window")?;

        // wgpu 0.24: Use get_capabilities instead of the removed get_supported_formats
        // Get surface capabilities from the adapter (not instance)
        // Note: we don't have the adapter here, so we use the standard configuration
        // that matches our pipeline format
        surface.configure(
            &gpu.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                width: w,
                height: h,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: wgpu::CompositeAlphaMode::PreMultiplied,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            },
        );

        Ok(Self {
            hwnd,
            surface,
            texture_bind_group: t_bg,
            uniform_buffer: u_buf,
            uniform_bind_group: u_bg,
            desktop_tex: bg_tex,
            rect,
            // Pre-allocate buffer for desktop capture (4 bytes per pixel: BGRA)
            buffer: vec![0u8; (w * h * 4) as usize],
        })
    }

    /// Clean up MonitorWindow resources.
    ///
    /// This destroys the native window and releases GPU resources.
    /// Note: WGPU resources are managed by the device and will be cleaned up automatically.
    ///
    /// # Safety
    ///
    /// - Must not be called while the window is still being used by the render loop
    /// - After this call, the MonitorWindow must not be used again
    pub unsafe fn destroy(&self) {
        // Destroy the native window
        // SAFETY: hwnd is a valid window handle created by CreateWindowExW
        let _ = DestroyWindow(self.hwnd);
    }
}

/// Create animation/wallpaper windows across all monitors.
///
/// This is the main entry point for window creation. It:
/// 1. Enumerates all connected monitors via `EnumDisplayMonitors`
/// 2. For wallpaper mode: finds the WorkerW window behind desktop icons
/// 3. Creates a native window for each monitor
/// 4. Initializes GPU resources for each window
///
/// # Arguments
///
/// * `gpu` - Initialized GPU core
/// * `inst` - WGPU instance
/// * `class` - Window class name (registered with `RegisterClassW`)
/// * `hi` - Instance handle from `GetModuleHandleW`
/// * `is_wp` - True for wallpaper mode, false for overlay animation
/// * `flow` - Loaded animation package (for background image)
///
/// # Returns
///
/// A vector of `MonitorWindow`, one per monitor. The vector is empty if
/// no monitors are found or window creation fails.
///
/// # Safety
///
/// - Must be called from the main thread with a Windows message pump
/// - `class` must be a valid registered window class
/// - `hi` must be a valid HINSTANCE
/// - For wallpaper mode: requires desktop icons to be visible
///
/// # Windows Styles
///
/// **Overlay mode**:
/// - `WS_EX_TOPMOST`: Stay above all windows
/// - `WS_EX_TOOLWINDOW`: Don't show in taskbar
/// - `WS_EX_LAYERED`: Support per-pixel alpha
/// - `WS_EX_TRANSPARENT`: Click-through (mouse events pass through)
/// - `WS_POPUP`: Borderless window
///
/// **Wallpaper mode**:
/// - `WS_CHILD`: Child of WorkerW window
/// - `WS_VISIBLE`: Visible (no WS_POPUP, no extended styles)
pub unsafe fn init_windows(
    gpu: &GpuCore,
    inst: &wgpu::Instance,
    class: windows::core::PCWSTR,
    hi: HINSTANCE,
    is_wp: bool,
    flow: &FlowPackage,
) -> Vec<MonitorWindow> {
    // Collect monitor rectangles via Windows API callback
    let mut rects: Vec<RECT> = Vec::new();

    // Callback function for EnumDisplayMonitors
    // Receives monitor RECT via LPARAM (pointer to our Vec<RECT>)
    unsafe extern "system" fn monitor_enum(_: HMONITOR, _: HDC, r: *mut RECT, d: LPARAM) -> BOOL {
        let rects = &mut *(d.0 as *mut Vec<RECT>);
        rects.push(*r);
        true.into() // Continue enumeration
    }

    // Enumerate all display monitors
    let _ = EnumDisplayMonitors(
        HDC(0),                                // All monitors
        None,                                  // No clipping region
        Some(monitor_enum),                    // Callback function
        LPARAM(&mut rects as *mut _ as isize), // User data (pointer to rects)
    );

    // For wallpaper mode: find WorkerW window behind desktop icons
    let workerw = if is_wp {
        GpuCore::fetch_worker_w()
    } else {
        HWND(0) // Not used in overlay mode
    };

    // Create a window for each monitor
    let mut windows = Vec::new();

    for &r in rects.iter() {
        // Calculate window dimensions from monitor RECT
        let (w, h) = ((r.right - r.left) as u32, (r.bottom - r.top) as u32);

        // CAPTURE DESKTOP FIRST (before creating the overlay window).
        // If we create the window first, the capture will get the transparent
        // overlay (which is black) instead of the actual desktop content.
        let buf = capture_or_load(flow, w, h, &r);

        // Create native window with appropriate styles
        let hwnd = CreateWindowExW(
            // Extended window styles
            if is_wp {
                WINDOW_EX_STYLE(0) // No extended styles for wallpaper
            } else {
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_TRANSPARENT
            },
            class,  // Window class name
            w!(""), // No window title
            // Window styles
            if is_wp {
                WS_CHILD | WS_VISIBLE // Child of WorkerW
            } else {
                WS_POPUP | WS_VISIBLE // Borderless overlay
            },
            // Position and size
            if is_wp { 0 } else { r.left }, // X position
            if is_wp { 0 } else { r.top },  // Y position
            w as i32,                       // Width
            h as i32,                       // Height
            // Parent window (WorkerW for wallpaper, none for overlay)
            if is_wp { workerw } else { HWND(0) },
            None, // No menu
            hi,   // Instance handle
            None, // No creation data
        );

        // Create MonitorWindow with all GPU resources
        match MonitorWindow::new_v1(gpu, inst, hwnd, w, h, &buf, r) {
            Ok(mw) => windows.push(mw),
            Err(e) => {
                eprintln!("Monitor window failed: {}", e);
                if is_wp {
                    let _ = DestroyWindow(hwnd);
                }
            }
        }
    }

    windows
}

/// Capture desktop background or load image from flow package.
///
/// This function provides the background texture for each monitor window.
/// It either:
/// 1. Loads `background.png` from the .flow package and resizes to monitor resolution
/// 2. Falls back to capturing the current desktop via DXGI (or BitBlt as fallback)
///
/// # Arguments
///
/// * `f` - Loaded flow package (may contain background.png)
/// * `w` - Target width (monitor width)
/// * `h` - Target height (monitor height)
/// * `r` - Monitor rectangle (for BitBlt/DXGI coordinates)
///
/// # Returns
///
/// BGRA pixel data (w × h × 4 bytes) suitable for GPU texture upload.
///
/// # Performance
///
/// - Image load + resize: ~10ms
/// - DXGI capture: ~0.5-1ms (when available)
/// - BitBlt fallback: ~5-7ms
unsafe fn capture_or_load(f: &FlowPackage, w: u32, h: u32, r: &RECT) -> Vec<u8> {
    // Try to load background image from flow package using dedicated module
    if let Some(ref d) = f.image_data {
        if let Some(bgra) = load_background(d, w, h) {
            return bgra;
        }
    }

    // Fallback: Capture current desktop using the screenshot module
    // The screenshot module automatically tries DXGI first, then BitBlt
    // SAFETY: capture_or_fallback handles DXGI initialization internally
    crate::screenshot::capture_or_fallback(w, h, Some(r))
}
