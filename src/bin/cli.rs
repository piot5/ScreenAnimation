//! CLI binary for ScreenAnimation.
//!
//! This binary loads a `.flow` package, initializes the GPU, creates animation
//! windows on all monitors, and runs the Windows message pump to keep them alive.
//!
//! # Usage
//!
//! ```text
//! cargo run --bin cli -- --package animation.flow [--wallpaper] [--overlay] [--debug]
//! ```
//!
//! # Architecture
//!
//! 1. Parse CLI arguments via clap
//! 2. Load user settings from disk
//! 3. Load the .flow package (ZIP with shader, config, assets)
//! 4. Initialize WGPU GPU core (device, pipelines, bind groups)
//! 5. Create windows on all monitors (overlay or wallpaper mode)
//! 6. Run Windows message pump until WM_QUIT
//! 7. Cleanup via Drop (MonitorWindow destroys HWNDs)

use std::path::PathBuf;
use anyhow::{Context, Result};
use clap::Parser;
use wgpu::Instance;
use windows::core::w;
use windows::Win32::System::LibraryLoader::{GetModuleHandleW};
use windows::Win32::UI::WindowsAndMessaging::*;

use screen_animation::{
    settings::AppSettings,
    engine::GpuCore,
    loader::FlowPackage,
    windows::init_windows,
};

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "ScreenAnimation - GPU-accelerated screen animations and wallpaper engine",
    long_about = "Loads a .flow animation package and displays it on all monitors.\n\
                   Supports two modes:\n  \
                   - Overlay (default): Borderless window on top of everything\n  \
                   - Wallpaper: Behind desktop icons (WorkerW embedding)\n\n\
                   Example:\n  \
                   cargo run --bin cli -- --package my_animation.flow --wallpaper"
)]
struct Args {
    /// Path to the .flow package to load
    #[arg(short, long, help = "Path to .flow animation package")]
    package: Option<PathBuf>,

    /// Run in wallpaper mode (behind desktop icons via WorkerW)
    #[arg(short, long, help = "Wallpaper mode (behind desktop icons)")]
    wallpaper: bool,

    /// Run in overlay mode (borderless window on top)
    #[arg(short, long, help = "Overlay mode (on top of all windows)")]
    overlay: bool,

    /// Enable debug logging
    #[arg(short, long, help = "Enable debug output")]
    debug: bool,
}

/// Windows message pump: keeps windows alive and processing events.
///
/// This is required for WGPU surfaces to work correctly. Without a message pump,
/// windows will not receive WM_PAINT, WM_SIZE, or input messages, causing them
/// to appear frozen or not render at all.
///
/// The loop blocks until WM_QUIT is received (e.g., Alt+F4, window close).
fn run_message_pump() {
    unsafe {
        let mut msg = MSG::default();
        loop {
            let ret = GetMessageW(&mut msg, None, 0, 0);
            if ret.0 == 0 {
                // WM_QUIT received - exit gracefully
                eprintln!("[cli] WM_QUIT received, shutting down");
                break;
            }
            if ret.0 == -1 {
                // Error in GetMessageW
                eprintln!("[cli] GetMessageW error: {}", std::io::Error::last_os_error());
                break;
            }
            let _ = TranslateMessage(&msg);
            let _ = DispatchMessageW(&msg);
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Enable debug logging if requested
    if args.debug {
        eprintln!("[cli] Debug mode enabled");
    }

    // Load user settings (uses defaults if no file exists)
    let _settings = AppSettings::load().context("Failed to load settings")?;
    eprintln!("[cli] Settings loaded");

    // Create WGPU instance (backend-agnostic: Vulkan/DX12/Metal)
    let inst = Instance::default();
    eprintln!("[cli] WGPU instance created");

    // Load the .flow package from filesystem
    let flow = if let Some(path) = &args.package {
        let path_str = path.to_str().context("Invalid package path (non-UTF8)")?;
        eprintln!("[cli] Loading package: {}", path_str);
        FlowPackage::load(path_str).context("Failed to load .flow package")?
    } else {
        return Err(anyhow::anyhow!(
            "No package path provided. Use --package <path/to/animation.flow>"
        ));
    };
    eprintln!(
        "[cli] Package loaded: {} sounds, {} textures, {} shader bytes",
        flow.sounds.len(),
        flow.textures.len(),
        flow.shader_src.len()
    );

    // Get module handle for window creation
    let hi = unsafe {
        GetModuleHandleW(None)
            .map_err(|e| anyhow::anyhow!("Failed to get module handle: {}", e))?
            .into()
    };

    // Determine mode: wallpaper or overlay
    let is_wallpaper = args.wallpaper;
    eprintln!("[cli] Mode: {}", if is_wallpaper { "wallpaper" } else { "overlay" });

    // Initialize GPU core (compiles shader, creates pipelines)
    eprintln!("[cli] Initializing GPU...");
    let gpu = pollster::block_on(GpuCore::new(&inst, &flow.shader_src, &["fs_main"]))
        .context("Failed to initialize GPU")?;
    eprintln!("[cli] GPU initialized ({} pipelines)", gpu.pipelines.len());

    // Create windows on all monitors
    eprintln!("[cli] Creating windows...");
    let _windows = unsafe { init_windows(&gpu, &inst, w!("WgpuAnim"), hi, is_wallpaper, &flow) };
    eprintln!("[cli] Windows created. Starting message pump...");

    // Run the Windows message pump to keep windows alive and responsive.
    // This blocks until WM_QUIT is received (e.g., window close).
    run_message_pump();

    eprintln!("[cli] Shutting down.");
    Ok(())
}