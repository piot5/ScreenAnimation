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
use windows::Win32::System::LibraryLoader::{GetModuleHandleW};
use windows::Win32::UI::WindowsAndMessaging::*;

use screen_animation::{
    settings::AppSettings,
    gpu_init::init_gpu,
    loader::FlowPackage,
    windows::init_windows,
};

/// Run the Windows message pump to keep windows alive and processing events.
///
/// This is required for WGPU surfaces to work correctly. Without a message pump,
/// windows will not receive WM_PAINT, WM_SIZE, or input messages, causing them
/// to appear frozen or not render at all.
///
/// The loop blocks until WM_QUIT is received (e.g., Alt+F4, window close).
///
/// # Safety
///
/// Must be called from the main thread with a valid Windows message queue.
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

    /// List current settings and exit
    #[arg(long, help = "Print settings and exit")]
    list_settings: bool,

    /// List available .flow packages in current directory and exit
    #[arg(long, help = "Scan for .flow packages and exit")]
    list_packages: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Enable debug logging if requested
    if args.debug {
        eprintln!("[cli] Debug mode enabled");
        eprintln!("[cli] Args: package={:?}, wallpaper={}, overlay={}", 
            args.package, args.wallpaper, args.overlay);
    }

    // Load user settings (uses defaults if no file exists)
    let settings = AppSettings::load().context("Failed to load settings")?;

    // Handle --list-settings flag: print current settings and exit
    if args.list_settings {
        println!("Current settings:");
        println!("  render_mode: {}", settings.render_mode);
        println!("  fps_limit: {}", settings.fps_limit);
        println!("  window_opacity: {}", settings.window_opacity);
        println!("  enable_sound: {}", settings.enable_sound);
        println!("  enable_mouse: {}", settings.enable_mouse);
        println!("  vsync: {}", settings.vsync);
        println!("  use_dxgi_capture: {}", settings.use_dxgi_capture);
        println!("  multi_monitor: {}", settings.multi_monitor);
        println!("  log_level: {}", settings.log_level);
        println!("  schema_version: {}", settings.schema_version);
        if let Some(ref path) = settings.last_package_path {
            println!("  last_package_path: {}", path);
        }
        println!("\nEnvironment overrides:");
        println!("  SCREENANIMATION_RENDER_MODE, SCREENANIMATION_FPS_LIMIT, etc.");
        return Ok(());
    }

    // Handle --list-packages flag: scan for .flow packages and exit
    if args.list_packages {
        println!("Scanning for .flow packages in current directory...");
        let mut found = 0;
        let entries = std::fs::read_dir(".")
            .context("Failed to read current directory")?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "flow").unwrap_or(false) {
                println!("  {}", path.display());
                found += 1;
            }
        }
        println!("Found {} .flow packages", found);
        return Ok(());
    }

    eprintln!(
        "[cli] Settings loaded: render_mode={}, fps_limit={}, multi_monitor={}",
        settings.render_mode, settings.fps_limit, settings.multi_monitor
    );

    // Determine mode: CLI flags override settings
    // Priority: --overlay > --wallpaper > settings default
    let is_wallpaper = if args.overlay {
        false // Explicit --overlay overrides everything
    } else if args.wallpaper {
        true // Explicit --wallpaper overrides settings
    } else {
        // Use settings default
        match settings.render_mode.as_str() {
            "wallpaper" => true,
            "overlay" => false,
            _ => {
                eprintln!("[cli] Unknown render_mode '{}', defaulting to overlay", settings.render_mode);
                false
            }
        }
    };
    let mode_str = if is_wallpaper { "wallpaper" } else { "overlay" };
    eprintln!("[cli] Mode: {}", mode_str);

    // Load the .flow package from filesystem
    let flow = if let Some(ref path) = args.package {
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
    
    // Validate package content
    if flow.shader_src.is_empty() {
        anyhow::bail!("Package contains empty shader source");
    }
    
    // Log package details for debugging
    if !flow.config.sequence.is_empty() {
        eprintln!("[cli] Sequence mode: {} steps", flow.config.sequence.len());
        for (i, step) in flow.config.sequence.iter().enumerate() {
            eprintln!("[cli]   Step {}: {} ({}ms, entry: {})", 
                i, step.name, step.duration_ms, step.shader_entry);
        }
    } else {
        let entry = flow.config.shader.as_deref().unwrap_or("fs_default");
        eprintln!("[cli] V1 mode: continuous shader '{}'", entry);
    }

    // Determine shader entry points based on package type
    let entries: Vec<&str> = if flow.config.sequence.is_empty() {
        vec![flow.config.shader.as_deref().unwrap_or("fs_main")]
    } else {
        flow.config.sequence.iter()
            .map(|s| s.shader_entry.as_str())
            .collect::<Vec<_>>()
    };

    // Get module handle for window creation
    let hi = unsafe {
        GetModuleHandleW(None)
            .map_err(|e| anyhow::anyhow!("Failed to get module handle: {}", e))?
            .into()
    };

    // Create WGPU instance (backend-agnostic: Vulkan/DX12/Metal)
    let inst = Instance::default();
    eprintln!("[cli] WGPU instance created");

    // Initialize GPU core using shared gpu_init module
    eprintln!("[cli] Initializing GPU with {} shader entry points...", entries.len());
    let gpu = init_gpu(
        &inst,
        &flow.shader_src,
        &entries,
    ).context("Failed to initialize GPU")?;
    eprintln!("[cli] GPU initialized ({} pipelines)", gpu.pipelines.len());

    // Create windows on all monitors
    eprintln!("[cli] Creating {} windows...", if is_wallpaper { "wallpaper" } else { "overlay" });
    let windows = unsafe { init_windows(&gpu, &inst, hi, is_wallpaper, &flow) };
    
    if windows.is_empty() {
        eprintln!("[cli] Warning: No windows created (no monitors found?)");
    } else {
        eprintln!("[cli] {} windows created successfully across {} monitors", 
            windows.len(), windows.len());
    }

    // Keep windows alive in scope until message pump exits
    let _windows = windows;

    // Run the Windows message pump to keep windows alive and responsive.
    // This blocks until WM_QUIT is received (e.g., Alt+F4, window close).
    // Set up Ctrl+C handler for graceful shutdown
    eprintln!("[cli] Press Ctrl+C to exit gracefully");
    run_message_pump();

    eprintln!("[cli] Shutdown complete.");
    Ok(())
}