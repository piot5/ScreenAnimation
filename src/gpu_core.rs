use crate::system_integration::WindowWrapper;
use raw_window_handle::{HasWindowHandle, HasDisplayHandle, RawWindowHandle, Win32WindowHandle, WindowHandle, HandleError};
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::w;
use anyhow::Context;
use std::collections::HashMap;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub offset: [f32; 2],
    pub scale: f32,
    pub time: f32,
    pub logic_params: [f32; 4],
    pub feature_flags: [f32; 4],
}

impl HasWindowHandle for WindowWrapper {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let handle = Win32WindowHandle::new(std::num::NonZeroIsize::new(self.0.0 as isize).unwrap());
        unsafe { Ok(WindowHandle::borrow_raw(RawWindowHandle::Win32(handle))) }
    }
}

impl HasDisplayHandle for WindowWrapper {
    fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>, HandleError> {
        unsafe { Ok(raw_window_handle::DisplayHandle::borrow_raw(raw_window_handle::RawDisplayHandle::Windows(raw_window_handle::WindowsDisplayHandle::new()))) }
    }
}

pub struct GpuCore {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub uniform_layout: wgpu::BindGroupLayout,
    pub sampler: wgpu::Sampler,
    pub pipelines: HashMap<String, wgpu::RenderPipeline>,
}

impl GpuCore {
    pub async fn new(instance: &wgpu::Instance, shader_src: &str, target_shader: &str) -> anyhow::Result<Self> {
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default()).await.context("NO_ADAPTER")?;
        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor::default(), None).await?;
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(shader_src.into()) });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });

        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX_FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { bind_group_layouts: &[&bind_group_layout, &uniform_layout], ..Default::default() });

        let mut pipelines = HashMap::new();
        for ep in ["fs_default", target_shader] {
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(ep),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState { module: &shader_module, entry_point: "vs_main", buffers: &[] },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: ep,
                    targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Bgra8UnormSrgb, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });
            pipelines.insert(ep.to_string(), pipeline);
        }

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor { mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, ..Default::default() });
        Ok(Self { device, queue, bind_group_layout, uniform_layout, sampler, pipelines })
    }

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