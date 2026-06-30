use std::path::PathBuf;
use anyhow::{Context, Result};
use clap::Parser;
use pollster::block_on;
use wgpu::Instance;
use windows::core::w;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::System::LibraryLoader::{GetModuleHandleW};

use screen_animation::{
    settings::AppSettings,
    engine::GpuCore,
    loader::FlowPackage,
    windows::init_windows,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the flow package to load
    #[arg(short, long)]
    package: Option<PathBuf>,
    /// Run in wallpaper mode (WorkerW)
    #[arg(short, long)]
    wallpaper: bool,
    /// Run in overlay mode
    #[arg(short, long)]
    overlay: bool,
    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

async fn run_engine(
    inst: &Instance,
    gpu: &GpuCore,
    flow: &FlowPackage,
    is_wallpaper: bool,
    hi: HINSTANCE,
) -> Result<()> {
    eprintln!("Creating windows...");
    let _ = unsafe { init_windows(gpu, inst, w!("WgpuAnim"), hi, is_wallpaper, flow) };
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let settings = AppSettings::load().context("Failed to load settings")?;

    let inst = Instance::default();

    let flow = if let Some(path) = &args.package {
        let path_str = path.to_str().context("Invalid path")?;
        FlowPackage::load(path_str).context("Failed to load package")?
    } else {
        return Err(anyhow::anyhow!("No package path provided"));
    };

    let hi = unsafe {
        GetModuleHandleW(None)
            .map_err(|e| anyhow::anyhow!("Failed to get module handle: {}", e))?
            .into()
    };

    let is_wallpaper = args.wallpaper;
    let gpu = pollster::block_on(GpuCore::new(&inst, &flow.shader_src, &["fs_main"])).context("Failed to initialize GPU")?;
    block_on(run_engine(&inst, &gpu, &flow, is_wallpaper, hi))?;
    Ok(())
}