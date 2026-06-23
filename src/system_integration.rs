use crate::gpu_core::{GpuCore, Uniforms};
use crate::loader::FlowPackage;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub struct WindowWrapper(pub HWND);

pub struct MonitorWindow {
    pub hwnd: HWND,
    pub surface: wgpu::Surface<'static>,
    pub texture_bind_group: wgpu::BindGroup,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
}

pub unsafe fn init_windows(
    gpu: &GpuCore,
    inst: &wgpu::Instance,
    class: windows::core::PCWSTR,
    hi: HINSTANCE,
    is_wp: bool,
    flow: &FlowPackage,
) -> Vec<MonitorWindow> {
    let mut rects: Vec<RECT> = Vec::new();
    
    unsafe extern "system" fn monitor_enum(_: HMONITOR, _: HDC, r: *mut RECT, d: LPARAM) -> BOOL {
        let rects = &mut *(d.0 as *mut Vec<RECT>);
        rects.push(*r);
        true.into()
    }

    let _ = EnumDisplayMonitors(HDC(0), None, Some(monitor_enum), LPARAM(&mut rects as *mut _ as isize));
    
    let workerw = if is_wp { GpuCore::fetch_worker_w() } else { HWND(0) };
    let mut windows = Vec::new();

    for &r in rects.iter() {
        let (w, h) = ((r.right - r.left) as u32, (r.bottom - r.top) as u32);
        let hwnd = CreateWindowExW(
            if is_wp { WINDOW_EX_STYLE(0) } else { WS_EX_TOPMOST | WS_EX_TOOLWINDOW },
            class, windows::core::w!(""),
            if is_wp { WS_CHILD | WS_VISIBLE } else { WS_POPUP | WS_VISIBLE },
            if is_wp { 0 } else { r.left }, if is_wp { 0 } else { r.top },
            w as i32, h as i32, 
            if is_wp { workerw } else { HWND(0) }, 
            None, hi, None
        );

        let buf = capture_or_load(flow, w, h, &r);
        let tex = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: None, size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[],
        });
        gpu.queue.write_texture(wgpu::ImageCopyTexture { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All }, &buf, wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: None }, tex.size());
        
        if let Ok(surface) = inst.create_surface(WindowWrapper(hwnd)) {
            surface.configure(&gpu.device, &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT, format: wgpu::TextureFormat::Bgra8UnormSrgb,
                width: w, height: h, present_mode: wgpu::PresentMode::Fifo, alpha_mode: wgpu::CompositeAlphaMode::Auto, view_formats: vec![], desired_maximum_frame_latency: 2,
            });

            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            let t_bg = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None, layout: &gpu.bind_group_layout,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&gpu.sampler) }],
            });
            let u_buf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: None, size: std::mem::size_of::<Uniforms>() as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
            });
            let u_bg = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None, layout: &gpu.uniform_layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: u_buf.as_entire_binding() }],
            });
            windows.push(MonitorWindow { hwnd, surface, texture_bind_group: t_bg, uniform_buffer: u_buf, uniform_bind_group: u_bg });
        }
    }
    windows
}

unsafe fn capture_or_load(f: &FlowPackage, w: u32, h: u32, r: &RECT) -> Vec<u8> {
    if let Some(ref d) = f.image_data {
        if let Ok(img) = image::load_from_memory(d) {
            let rgba = img.resize_exact(w, h, image::imageops::FilterType::Triangle).to_rgba8();
            return rgba.chunks_exact(4).flat_map(|s| [s[2], s[1], s[0], s[3]]).collect();
        }
    }
    let s_dc = GetDC(None);
    let m_dc = CreateCompatibleDC(s_dc);
    let bm = CreateCompatibleBitmap(s_dc, w as i32, h as i32);
    SelectObject(m_dc, bm);
    let _ = BitBlt(m_dc, 0, 0, w as i32, h as i32, s_dc, r.left, r.top, SRCCOPY);
    let mut bmi = BITMAPINFO { bmiHeader: BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: w as i32, biHeight: -(h as i32), biPlanes: 1, biBitCount: 32, ..Default::default() }, ..Default::default() };
    let mut b = vec![0u8; (w * h * 4) as usize];
    GetDIBits(m_dc, bm, 0, h, Some(b.as_mut_ptr() as *mut _), &mut bmi, DIB_RGB_COLORS);
    DeleteObject(bm); DeleteDC(m_dc); ReleaseDC(None, s_dc);
    b
}