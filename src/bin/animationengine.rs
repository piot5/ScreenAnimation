//! ScreenAnimation binary entry point.
//!
//! This is the main executable that parses CLI arguments, initializes GPU and audio,
//! creates windows for each monitor, and runs the render loop.
//!
//! # Operation Modes
//!
//! **V1 Simple Mode**: Continuous animation with mouse interaction
//! - Loads a single shader and renders it continuously
//! - Tracks mouse position and passes to shader
//! - 60 FPS render loop
//!
//! **V2 Sequence Mode**: Timed sequence of shader/sound steps
//! - Loads multiple shader entry points from sequence config
//! - Plays sounds synchronously at step start
//! - Each step runs for a configured duration
//!
//! # Architecture
//!
//! ```text
//! CLI Args → Load Package → Init Audio → Init GPU → Create Windows → Render Loop
//!                                                              ├── V1: Simple loop
//!                                                              └── V2: Sequence loop
//! ```

#![windows_subsystem = "windows"]

use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{Duration, Instant};

use anyhow::Context;
use clap::{Parser, Subcommand};
use screen_animation::engine::GpuCore;
use screen_animation::loader::FlowPackage;
use screen_animation::logic::LogicEngine;
use screen_animation::windows::init_windows;
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::w;

/// Global atomic variables for lock-free mouse position tracking.
///
/// These atomics are written by the Windows WndProc (on the message pump thread)
/// and read by the render loop (also on the main thread). Since both operations
/// happen on the same thread, `Relaxed` ordering is sufficient.
///
/// # Why atomics?
///
/// Mouse position needs to be communicated from the window procedure to the render loop.
/// Using atomics avoids mutex overhead and is simpler than channels for this use case.
static MOUSE_X: AtomicI32 = AtomicI32::new(0);
static MOUSE_Y: AtomicI32 = AtomicI32::new(0);

/// Window procedure for handling Windows messages.
///
/// This function is called by the Windows message pump for each message
/// sent to our animation windows. It handles:
/// - `WM_MOUSEMOVE`: Tracks mouse position in window coordinates
/// - `WM_DESTROY`: Signals the application to exit
/// - All other messages: Passed to default handler
///
/// # Safety
///
/// This is an unsafe extern "system" function called by Windows.
/// It must follow the stdcall calling convention and Windows WndProc signature.
unsafe extern "system" fn wnd_proc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if m == WM_MOUSEMOVE {
        // Extract mouse X coordinate from LPARAM (lower 16 bits)
        // Cast through i16 to i32 to preserve sign extension
        MOUSE_X.store((l.0 & 0xffff) as i16 as i32, Ordering::Relaxed);
        // Extract mouse Y coordinate from LPARAM (upper 16 bits)
        MOUSE_Y.store(((l.0 >> 16) & 0xffff) as i16 as i32, Ordering::Relaxed);
    }
    if m == WM_DESTROY {
        // Window is being destroyed, signal main loop to exit
        PostQuitMessage(0);
    }
    // Pass all other messages to default window procedure
    DefWindowProcW(h, m, w, l)
}

/// CLI argument definitions using clap derive macros.
///
/// Supports two subcommands:
/// - `Animation`: Transparent overlay mode
/// - `Wallpaper`: Desktop wallpaper embedding mode
#[derive(Parser, Debug)]
struct AppArgs {
    #[command(subcommand)]
    command: Commands,
}

/// Available animation modes.
#[derive(Subcommand, Debug)]
enum Commands {
    /// Single animation mode (v1) - transparent overlay on desktop
    Animation { path: String },
    /// Wallpaper mode (v1) - embed behind desktop icons
    Wallpaper { path: String },
}

/// Application entry point.
///
/// # Execution Flow
///
/// 1. Parse CLI arguments
/// 2. Load .flow animation package
/// 3. Initialize audio system
/// 4. Create GPU instance and compile shaders
/// 5. Create windows for all monitors
/// 6. Enter render loop (V1 or V2 mode)
///
/// # Errors
///
/// Returns error if:
/// - .flow package cannot be loaded
/// - Audio system initialization fails
/// - GPU adapter/device creation fails
/// - Window creation fails
/// - Shader compilation fails
///
/// # Performance
///
/// - Package loading: ~100-200ms
/// - Audio init: ~50ms
/// - GPU init: ~500ms
/// - Window creation: ~35ms per monitor
/// - Total startup: ~1s for typical setup
fn main() -> anyhow::Result<()> {
    // Parse command-line arguments
    let args = AppArgs::parse();
    let (path, is_wp) = match &args.command {
        Commands::Animation { path } => (path.clone(), false),
        Commands::Wallpaper { path } => (path.clone(), true),
    };

    eprintln!("Loading package: {}", path);
    // Load animation package from .flow file
    let flow = FlowPackage::load(&path)?;
    eprintln!("✓ Package loaded successfully");

    // Audio setup
    // Initialize default audio output device (usually speakers/headphones)
    let (_stream, handle) = rodio::OutputStream::try_default().map_err(|e| anyhow::anyhow!(e))?;
    // Create audio sink for playing sounds
    let sink = rodio::Sink::try_new(&handle).map_err(|e| anyhow::anyhow!(e))?;
    // Set master volume from config (default 0.5 = 50%)
    sink.set_volume(flow.val("volume", 0.5));

    unsafe {
        // Set DPI awareness to per-monitor aware (V2)
        // Required for correct rendering on mixed-DPI setups
        // Windows 10 1703+ feature
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        eprintln!("✓ DPI awareness set");
        
        // Get instance handle (required for window creation)
        let hi = GetModuleHandleW(None)?;
        eprintln!("✓ Got instance handle");
        let class_name = w!("WgpuAnim");

        // Register window class with our custom WndProc
        RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: hi.into(),
            lpszClassName: class_name,
            ..Default::default()
        });

        // Create WGPU instance (uses default backends: Vulkan, DX12, Metal)
        let inst = wgpu::Instance::default();

        eprintln!("Mode: {}", if is_wp { "Wallpaper" } else { "Animation" });
        
        // Determine which shader entry points to compile
        // V2 sequences may use multiple shaders, V1 uses one
        let entries: Vec<String> = if !flow.config.sequence.is_empty() {
            // V2: Collect unique shader entries from sequence steps
            let mut set: Vec<String> = Vec::new();
            for step in &flow.config.sequence {
                if !set.contains(&step.shader_entry) {
                    set.push(step.shader_entry.clone());
                }
            }
            // Always include default shader as fallback
            if !set.contains(&"fs_default".to_string()) {
                set.push("fs_default".to_string());
            }
            set
        } else {
            // V1: Use configured shader + default fallback
            let sh_n = flow
                .config
                .shader
                .clone()
                .unwrap_or_else(|| "fs_default".into());
            vec!["fs_default".to_string(), sh_n]
        };

        eprintln!("Compiling {} shader entries: {:?}", entries.len(), entries);
        // Convert to string slices for GPU initialization
        let entry_refs: Vec<&str> = entries.iter().map(|s: &String| s.as_str()).collect();
        
        // Initialize GPU core (compile shaders, create pipelines)
        eprintln!("Initializing GPU...");
        let gpu = pollster::block_on(GpuCore::new(&inst, &flow.shader_src, &entry_refs))?;
        eprintln!("✓ GPU initialized");
        
        // Create logic engine for uniform buffer calculations
        let logic = LogicEngine::new();
        
        // Create windows on all monitors
        eprintln!("Creating windows...");
        let mut wins = init_windows(&gpu, &inst, class_name, hi.into(), is_wp, &flow);
        eprintln!("✓ Created {} monitor windows", wins.len());

        let has_sequence = !flow.config.sequence.is_empty();
        eprintln!("Has sequence: {}", has_sequence);

        if has_sequence {
            // === V2: SEQUENCE MODE ===
            // Iterate through sequence steps defined in config.toml
            for step in &flow.config.sequence {
                let step_start = Instant::now();
                let dur = Duration::from_millis(step.duration_ms);

                // Play step sound
                if let Some(snd) = &step.sound {
                    if let Some(data) = flow.sounds.get(snd) {
                        if let Ok(source) = rodio::Decoder::new(std::io::Cursor::new((**data).clone())) {
                            let _ = sink.append(source);
                        }
                    }
                }

                // Get render pipeline for this step's shader entry
                // Falls back to "fs_default" if specific entry not found
                let pipe = gpu
                    .pipelines
                    .get(&step.shader_entry)
                    .or_else(|| gpu.pipelines.get("fs_default"))
                    .context("MISSING_PIPELINE")?;

                while step_start.elapsed() < dur || step.duration_ms == 0 {
                    let mut msg = MSG::default();
                    while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                        if msg.message == WM_QUIT {
                            return Ok(());
                        }
                    }

                    for win in &mut wins {
                        let mut rect = RECT::default();
                        let _ = GetWindowRect(win.hwnd, &mut rect);
                        let mx = MOUSE_X.load(Ordering::Relaxed) as f32;
                        let my = MOUSE_Y.load(Ordering::Relaxed) as f32;

                        // Build uniform buffer for this frame
                        // Mouse position is normalized to 0-1 range
                        let uniforms = screen_animation::engine::Uniforms {
                            mouse: [
                                mx / (rect.right - rect.left) as f32,
                                my / (rect.bottom - rect.top) as f32
                            ],
                            offset: [0.0, 0.0],
                            scale: 1.0,
                            time: step_start.elapsed().as_secs_f32(),
                            logic_params: [
                                flow.val("p1", 1.0),
                                flow.val("p2", 0.0),
                                flow.val("p3", 0.0),
                                flow.val("p4", 0.0),
                            ],
                            feature_flags: [0.0, 0.0, 0.0, 0.0],
                        };

                        gpu.queue
                            .write_buffer(&win.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

                        if let Ok(fr) = win.surface.get_current_texture() {
                            let v = fr
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());
                            let mut enc = gpu
                                .device
                                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                            {
                                let mut rp =
                                    enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                                        color_attachments: &[Some(
                                            wgpu::RenderPassColorAttachment {
                                                view: &v,
                                                resolve_target: None,
                                                ops: wgpu::Operations {
                                                    load: wgpu::LoadOp::Clear(
                                                        wgpu::Color::BLACK,
                                                    ),
                                                    store: wgpu::StoreOp::Store,
                                                },
                                            },
                                        )],
                                        ..Default::default()
                                    });
                                rp.set_pipeline(pipe);
                                rp.set_bind_group(0, &win.texture_bind_group, &[]);
                                rp.set_bind_group(1, &win.uniform_bind_group, &[]);
                                rp.draw(0..6, 0..1);
                            }
                            gpu.queue.submit(Some(enc.finish()));
                            fr.present();
                        }
                    }

                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        } else {
            // === V1: SIMPLE MODE ===
            let sh_n = flow
                .config
                .shader
                .clone()
                .unwrap_or_else(|| "fs_default".into());
            let pipe = gpu
                .pipelines
                .get(&sh_n)
                .or_else(|| gpu.pipelines.get("fs_default"))
                .context("MISSING_PIPELINE")?;

            let frame_time = Duration::from_secs_f64(1.0 / 60.0);

            loop {
                let frame_start = Instant::now();
                let mut msg = MSG::default();
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    if msg.message == WM_QUIT {
                        return Ok(());
                    }
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                let mut cp = POINT::default();
                let _ = GetCursorPos(&mut cp);

                for w in &mut wins {
                    let mut rect = RECT::default();
                    let _ = GetWindowRect(w.hwnd, &mut rect);

                    let rel_x = (cp.x - rect.left) as f32 / (rect.right - rect.left) as f32;
                    let rel_y = (cp.y - rect.top) as f32 / (rect.bottom - rect.top) as f32;

                    let u = logic.update(&flow, [rel_x, rel_y]);
                    gpu.queue
                        .write_buffer(&w.uniform_buffer, 0, bytemuck::bytes_of(&u));

                    if let Ok(fr) = w.surface.get_current_texture() {
                        let v = fr
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());
                        let mut enc = gpu
                            .device
                            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                        {
                            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &v,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                ..Default::default()
                            });
                            rp.set_pipeline(pipe);
                            rp.set_bind_group(0, &w.texture_bind_group, &[]);
                            rp.set_bind_group(1, &w.uniform_bind_group, &[]);
                            rp.draw(0..6, 0..1);
                        }
                        gpu.queue.submit(Some(enc.finish()));
                        fr.present();
                    }
                }

                let elapsed = frame_start.elapsed();
                if elapsed < frame_time {
                    std::thread::sleep(frame_time - elapsed);
                }
            }
        }
    }

    Ok(())
}