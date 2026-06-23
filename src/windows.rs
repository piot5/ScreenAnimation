//! Windows integration module.
//!
//! This module handles native Windows API integration for window management,
//! desktop embedding, and screen capture. It's responsible for creating
//! animation windows on each monitor and configuring GPU surfaces.
//!
//! # Key Responsibilities
//!
//! - Create windows for each monitor (overlay or wallpaper mode)
//! - Embed content behind desktop icons (WorkerW trick)
//! - Capture desktop background for wallpaper mode
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

use crate::engine::{GpuCore, WindowWrapper};
use crate::loader::FlowPackage;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::w;

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
    /// V1-style: creates window with background image/capture.
    ///
    /// This constructor creates a monitor window with a pre-rendered background.
    /// The background is uploaded to a GPU texture and bound for rendering.
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
    /// - `hwnd` must be a valid window created by `CreateWindowExW`
    /// - `bg_buf` must contain valid BGRA pixel data (w × h × 4 bytes)
    /// - Caller must ensure window message pump is running
    ///
    /// # Performance
    ///
    /// - Texture creation: ~5ms
    /// - Texture upload: ~10ms (w × h × 4 bytes)
    /// - Surface creation: ~20ms
    /// - Total: ~35ms per monitor
    pub unsafe fn new_v1(
        gpu: &GpuCore,
        inst: &wgpu::Instance,
        hwnd: HWND,
        w: u32,
        h: u32,
        bg_buf: &[u8],
        rect: RECT,
    ) -> Self {
        // Create GPU texture for background image
        // Format: BGRA8UnormSrgb (matches Windows DIB format)
        let bg_tex = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        // Upload background image data to GPU texture
        // Layout: tightly packed BGRA rows (w * 4 bytes per row)
        gpu.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &bg_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bg_buf,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: None,
            },
            bg_tex.size(),
        );

        // Create texture view (required for binding)
        let bg_view = bg_tex.create_view(&wgpu::TextureViewDescriptor::default());
        
        // Create bind group for background texture + sampler
        // Uses all 4 bindings to support both desktop and custom textures
        // Bindings 0-1: Background texture + sampler
        // Bindings 2-3: Duplicate of background (allows shader to use tex1)
        let t_bg = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
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
        // Size: 64 bytes (matches Uniforms struct)
        let u_buf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: std::mem::size_of::<crate::engine::Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group for uniform buffer
        let u_bg = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &gpu.uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: u_buf.as_entire_binding(),
            }],
        });

        // Create WGPU surface from native HWND
        // WindowWrapper implements the required traits for surface creation
        let surface = inst.create_surface(WindowWrapper(hwnd)).unwrap();
        surface.configure(
            &gpu.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                width: w,
                height: h,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            },
        );

        Self {
            hwnd,
            surface,
            texture_bind_group: t_bg,
            uniform_buffer: u_buf,
            uniform_bind_group: u_bg,
            desktop_tex: bg_tex,
            rect,
            // Pre-allocate buffer for desktop capture (4 bytes per pixel: BGRA)
            buffer: vec![0u8; (w * h * 4) as usize],
        }
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
    unsafe extern "system" fn monitor_enum(
        _: HMONITOR,
        _: HDC,
        r: *mut RECT,
        d: LPARAM,
    ) -> BOOL {
        let rects = &mut *(d.0 as *mut Vec<RECT>);
        rects.push(*r);
        true.into()  // Continue enumeration
    }

    // Enumerate all display monitors
    let _ = EnumDisplayMonitors(
        HDC(0),           // All monitors
        None,             // No clipping region
        Some(monitor_enum),  // Callback function
        LPARAM(&mut rects as *mut _ as isize),  // User data (pointer to rects)
    );

    // For wallpaper mode: find WorkerW window behind desktop icons
    let workerw = if is_wp {
        GpuCore::fetch_worker_w()
    } else {
        HWND(0)  // Not used in overlay mode
    };
    
    // Create a window for each monitor
    let mut windows = Vec::new();

    for &r in rects.iter() {
        // Calculate window dimensions from monitor RECT
        let (w, h) = ((r.right - r.left) as u32, (r.bottom - r.top) as u32);

        // Create native window with appropriate styles
        let hwnd = CreateWindowExW(
            // Extended window styles
            if is_wp {
                WINDOW_EX_STYLE(0)  // No extended styles for wallpaper
            } else {
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_TRANSPARENT
            },
            class,       // Window class name
            w!(""),      // No window title
            // Window styles
            if is_wp {
                WS_CHILD | WS_VISIBLE  // Child of WorkerW
            } else {
                WS_POPUP | WS_VISIBLE  // Borderless overlay
            },
            // Position and size
            if is_wp { 0 } else { r.left },  // X position
            if is_wp { 0 } else { r.top },   // Y position
            w as i32,   // Width
            h as i32,   // Height
            // Parent window (WorkerW for wallpaper, none for overlay)
            if is_wp { workerw } else { HWND(0) },
            None,       // No menu
            hi,         // Instance handle
            None,       // No creation data
        );

        // Load background image or capture desktop
        let buf = capture_or_load(flow, w, h, &r);
        
        // Create MonitorWindow with all GPU resources
        let mw = MonitorWindow::new_v1(gpu, inst, hwnd, w, h, &buf, r);
        windows.push(mw);
    }

    windows
}

/// Capture desktop background or load image from flow package.
///
/// This function provides the background texture for each monitor window.
/// It either:
/// 1. Loads `background.png` from the .flow package and resizes to monitor resolution
/// 2. Falls back to capturing the current desktop via BitBlt
///
/// # Arguments
///
/// * `f` - Loaded flow package (may contain background.png)
/// * `w` - Target width (monitor width)
/// * `h` - Target height (monitor height)
/// * `r` - Monitor rectangle (for BitBlt coordinates)
///
/// # Returns
///
/// BGRA pixel data (w × h × 4 bytes) suitable for GPU texture upload.
///
/// # Safety
///
/// - Must be called from main thread with GDI initialized
/// - `r` must be a valid monitor RECT from EnumDisplayMonitors
///
/// # Performance
///
/// - Image load + resize: ~10ms
/// - BitBlt capture: ~5ms
unsafe fn capture_or_load(f: &FlowPackage, w: u32, h: u32, r: &RECT) -> Vec<u8> {
    // Try to load background image from flow package
    if let Some(ref d) = f.image_data {
        if let Ok(img) = image::load_from_memory(d) {
            // Resize to monitor resolution using triangle filter (bilinear)
            let rgba = img
                .resize_exact(w, h, image::imageops::FilterType::Triangle)
                .to_rgba8();
            // Convert RGBA to BGRA (Windows DIB format)
            // Swizzle: R↔B, keep G and A
            return rgba
                .chunks_exact(4)
                .flat_map(|s| [s[2], s[1], s[0], s[3]])
                .collect();
        }
    }
    
    // Fallback: Capture current desktop using BitBlt
    let s_dc = GetDC(None);  // Get screen DC
    let m_dc = CreateCompatibleDC(s_dc);  // Create compatible memory DC
    let bm = CreateCompatibleBitmap(s_dc, w as i32, h as i32);  // Create bitmap
    SelectObject(m_dc, bm);  // Select bitmap into DC
    
    // Copy screen to bitmap (source: screen DC, dest: memory DC)
    // SRCCOPY = direct copy, no blending
    let _ = BitBlt(m_dc, 0, 0, w as i32, h as i32, s_dc, r.left, r.top, SRCCOPY);
    
    // Describe the bitmap format for GetDIBits
    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w as i32,
            biHeight: -(h as i32),  // Negative = top-down DIB (origin at top-left)
            biPlanes: 1,
            biBitCount: 32,  // 32 bits per pixel (BGRA)
            ..Default::default()
        },
        ..Default::default()
    };
    
    // Allocate buffer for pixel data
    let mut b = vec![0u8; (w * h * 4) as usize];
    
    // Convert bitmap to raw BGRA pixels
    GetDIBits(
        m_dc,      // Source DC
        bm,        // Bitmap handle
        0,         // Start scan line
        h,         // Number of scan lines
        Some(b.as_mut_ptr() as *mut _),  // Destination buffer
        &mut bmi,  // Bitmap format info
        DIB_RGB_COLORS,  // Color usage
    );
    
    // Cleanup GDI resources (critical to prevent leaks)
    DeleteObject(bm);     // Delete bitmap
    DeleteDC(m_dc);       // Delete memory DC
    ReleaseDC(None, s_dc);  // Release screen DC
    
    b
}