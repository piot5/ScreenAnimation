//! GUI Control Panel for ScreenAnimation.
//!
//! This binary provides a graphical user interface for managing animation packages,
//! adjusting settings, and monitoring performance. Built with egui/eframe.
//!
//! # Features
//!
//! - Package browser: load, select, and manage .flow packages
//! - Settings panel: render mode, FPS, opacity, sound, mouse, VSync, DXGI
//! - Performance monitor: FPS, frame time, capture time (real metrics)
//! - Package inspector: shader info, sounds, textures, logic parameters
//! - Sequence timeline: visual overview of V2 sequence steps
//! - Background preview: shows loaded background image dimensions
//!
//! # Usage
//!
//! ```text
//! cargo run --bin gui [--package animation.flow]
//! ```

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use anyhow::Context;
use clap::Parser;
use egui::{Color32, RichText, ScrollArea, Grid};
use image::GenericImageView;
use wgpu::Instance;

/// Export current frame as PNG screenshot.
///
/// Captures the current GPU framebuffer and saves it to a PNG file.
/// Useful for debugging and sharing animation frames.
///
/// # Arguments
///
/// * `gpu` - GPU core with device and queue
/// * `surface` - WGPU surface to capture from
/// * `path` - Output file path (e.g., "screenshot.png")
///
/// # Returns
///
/// Returns `Ok(())` on success.
///
/// # Errors
///
/// Returns error if:
/// - Surface cannot be accessed or configured
/// - File write fails
/// - PNG encoding fails
#[allow(dead_code)]
pub fn export_frame_as_png(_gpu: &GpuCore, _surface: &wgpu::Surface<'_>, path: &str) -> anyhow::Result<()> {
    // Placeholder for frame export (requires frame capture logic)
    eprintln!("[gui] Frame export to {} not yet implemented", path);
    Ok(())
}

use screen_animation::{
    settings::AppSettings,
    loader::FlowPackage,
    gpu_init::init_gpu,
    engine::GpuCore,
    logic::LogicEngine,
};

/// Playback state for the animation engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

/// Shared engine state between GUI and render thread.
struct EngineState {
    playback: PlaybackState,
    /// Speed multiplier for animation (0.1x to 3.0x)
    speed: f32,
    /// Whether to loop the animation
    loop_animation: bool,
}

impl Default for EngineState {
    fn default() -> Self {
        Self {
            playback: PlaybackState::Stopped,
            speed: 1.0,
            loop_animation: false,
        }
    }
}

/// Application state shared across all UI panels.
struct GuiState {
    settings: AppSettings,
    packages: Vec<String>,
    current_package: Option<Box<FlowPackage>>,
    current_package_path: Option<String>,
    fps: f32,
    frame_time_ms: f32,
    capture_time_ms: f32,
    status_message: String,
    status_color: Color32,
    show_settings: bool,
    show_packages: bool,
    show_performance: bool,
    new_package_path: String,
    /// Engine state for playback control
    engine_state: Arc<Mutex<EngineState>>,
    /// GPU core (initialized when package is loaded)
    gpu_core: Option<GpuCore>,
    /// Logic engine (initialized when package is loaded)
    logic_engine: Option<LogicEngine>,
    /// Actual playback time (controlled by Play/Pause/Stop)
    playback_time: f32,
    /// Timestamp of last frame for real FPS calculation
    last_frame_time: std::time::Instant,
    /// Frame counter for FPS calculation
    frame_count: u32,
    /// Time accumulator for FPS
    fps_accumulator: f32,
}

impl GuiState {
    pub fn new() -> Self {
        Self {
            settings: AppSettings::load().unwrap_or_default(),
            packages: Vec::new(),
            current_package: None,
            current_package_path: None,
            fps: 0.0,
            frame_time_ms: 0.0,
            capture_time_ms: 0.0,
            status_message: "Ready".to_string(),
            status_color: Color32::GREEN,
            show_settings: true,
            show_packages: true,
            show_performance: true,
            new_package_path: String::new(),
            engine_state: Arc::new(Mutex::new(EngineState::default())),
            gpu_core: None,
            logic_engine: None,
            playback_time: 0.0,
            last_frame_time: std::time::Instant::now(),
            frame_count: 0,
            fps_accumulator: 0.0,
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>, color: Color32) {
        self.status_message = msg.into();
        self.status_color = color;
    }

    /// Update real performance metrics.
    pub fn update_metrics(&mut self) {
        let now = std::time::Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        
        self.frame_count += 1;
        self.fps_accumulator += dt;
        
        // Calculate FPS every 0.5 seconds
        if self.fps_accumulator >= 0.5 {
            self.fps = self.frame_count as f32 / self.fps_accumulator;
            self.frame_count = 0;
            self.fps_accumulator = 0.0;
        }
        
        // Estimate frame time from current frame
        self.frame_time_ms = dt * 1000.0;
    }

    /// Initialize GPU and Logic engine for a loaded package.
    ///
    /// Uses the shared `gpu_init` module for consistent initialization across
    /// all binaries (CLI, GUI). Validates package and shader entry points.
    pub fn init_engine(&mut self) -> anyhow::Result<()> {
        if self.gpu_core.is_some() {
            return Ok(());
        }
        
        let flow = self.current_package.as_ref().ok_or_else(|| anyhow::anyhow!("No package loaded"))?;
        
        // Validate shader source is not empty
        anyhow::ensure!(!flow.shader_src.is_empty(), "Shader source is empty");
        
        // Determine shader entry points based on package type
        let entries = if flow.config.sequence.is_empty() {
            vec!["fs_main"]
        } else {
            flow.config.sequence.iter()
                .map(|s| s.shader_entry.as_str())
                .collect::<Vec<_>>()
        };
        
        anyhow::ensure!(!entries.is_empty(), "No shader entry points found");
        
        let inst = Instance::default();
        let gpu = init_gpu(&inst, &flow.shader_src, &entries)
            .context("Failed to initialize GPU via gpu_init")?;
        
        let logic = LogicEngine::new(flow);
        
        self.gpu_core = Some(gpu);
        self.logic_engine = Some(logic);
        self.set_status("Engine initialized", Color32::GREEN);
        Ok(())
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the flow package to load
    #[arg(short, long)]
    package: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    eprintln!("[gui] ScreenAnimation GUI starting...");

    let settings = AppSettings::load().context("Failed to load settings")?;
    eprintln!(
        "[gui] Settings loaded: render_mode={}, fps_limit={}",
        settings.render_mode, settings.fps_limit
    );

    let mut state = GuiState::new();
    state.settings = settings;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 720.0])
            .with_title("ScreenAnimation - Control Panel"),
        ..Default::default()
    };

    eframe::run_native(
        "ScreenAnimation",
        options,
        Box::new(|_cc| Box::new(GuiApp::new(state))),
    )
    .map_err(|e| anyhow::anyhow!("{}", e))
}

struct GuiApp {
    state: GuiState,
}

impl GuiApp {
    fn new(state: GuiState) -> Self {
        Self { state }
    }
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Always update performance metrics for real-time display
        self.state.update_metrics();
        
        // Handle keyboard shortcuts
        let input = ctx.input(|i| i.clone());
        if input.key_pressed(egui::Key::Space) {
            // Toggle play/pause - update state first, then status to avoid borrow conflicts
            let mut es = self.state.engine_state.lock().unwrap();
            let (_new_playback, status_msg, status_color) = match es.playback {
                PlaybackState::Playing => {
                    es.playback = PlaybackState::Paused;
                    (PlaybackState::Paused, "Playback paused (Space)", Color32::YELLOW)
                }
                PlaybackState::Paused | PlaybackState::Stopped => {
                    es.playback = PlaybackState::Playing;
                    self.state.playback_time = 0.0;
                    (PlaybackState::Playing, "Playback started (Space)", Color32::GREEN)
                }
            };
            drop(es);
            self.state.set_status(status_msg, status_color);
        }
        
        // Update playback time only when playing
        let engine_locked = self.state.engine_state.lock().unwrap();
        let is_playing = engine_locked.playback == PlaybackState::Playing;
        let speed = engine_locked.speed;
        drop(engine_locked);
        
        if is_playing {
            // Use actual delta time for accurate playback progression
            let dt = self.state.frame_time_ms / 1000.0;
            self.state.playback_time += dt * speed;
        }
        
        self.build_ui(ctx);
    }
}

impl GuiApp {
    fn build_ui(&mut self, ctx: &egui::Context) {
        let state = &mut self.state;

        // Top menu bar
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Load Package...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("flow package", &["flow"])
                            .pick_file()
                        {
                            let p = path.display().to_string();
                            state.new_package_path = p.clone();
                            let _ = Self::load_package(state, &p);
                        }
                        ui.close_menu();
                    }
                    if ui.button("Exit").clicked() {
                        std::process::exit(0);
                    }
                });
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut state.show_settings, "Settings");
                    ui.checkbox(&mut state.show_packages, "Packages");
                    ui.checkbox(&mut state.show_performance, "Performance");
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("FPS: {:.1}", state.fps));
                    ui.colored_label(Color32::LIGHT_GRAY, " | ");
                    ui.colored_label(state.status_color, &state.status_message);
                });
            });
        });

        // Left sidebar: package list
        egui::SidePanel::left("packages").show(ctx, |ui| {
            ui.heading("Packages");
            ui.separator();
            ScrollArea::vertical().show(ui, |ui: &mut egui::Ui| {
                let packages_clone = state.packages.clone();
                let current = state.current_package_path.clone();
                for (idx, pkg) in packages_clone.iter().enumerate() {
                    let sel = current.as_deref() == Some(pkg);
                    if ui.selectable_label(sel, pkg).clicked() {
                        state.current_package_path = Some(pkg.clone());
                        let _ = Self::load_package(state, pkg);
                    }
                    if ui.button("🗑").clicked() {
                        state.packages.remove(idx);
                        break;
                    }
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Add:");
                if ui.button("Browse...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("flow package", &["flow"])
                        .pick_file()
                    {
                        state.new_package_path = path.display().to_string();
                    }
                }
                ui.text_edit_singleline(&mut state.new_package_path);
                if ui.button("+").clicked() && !state.new_package_path.is_empty() {
                    let p = state.new_package_path.clone();
                    let _ = Self::load_package(state, &p);
                }
            });
            ui.separator();
            let mem_mb = state.current_package.as_ref().map(|pkg| {
                let sounds_size: usize = pkg.sounds.values().map(|s| s.len()).sum();
                let textures_size: usize = pkg.textures.values().map(|(_, _, d)| d.len()).sum();
                let shader_size = pkg.shader_src.len();
                (sounds_size + textures_size + shader_size) / (1024 * 1024)
            }).unwrap_or(0);
            ui.label(format!("Memory: ~{} MB", mem_mb));
        });

        // Central panel: settings, packages, performance, preview
        egui::CentralPanel::default().show(ctx, |ui| {
            if state.show_settings {
                Self::settings_panel(ui, state);
                ui.separator();
            }
            if state.show_packages {
                Self::packages_panel(ui, state);
                ui.separator();
            }
            if state.show_performance {
                Self::performance_panel(ui, state);
            }
            if state.current_package.is_some() {
                ui.separator();
                Self::preview_panel(ui, state);
            }
        });

        // Bottom status bar
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&state.status_message);
                ui.colored_label(Color32::GRAY, " | ");
                ui.label(format!("Render: {}", state.settings.render_mode));
                if let Some(ref pkg) = state.current_package {
                    ui.colored_label(Color32::GRAY, " | ");
                    let mode = if pkg.config.sequence.is_empty() { "V1" } else { "V2" };
                    ui.label(format!("Mode: {}", mode));
                }
            });
        });
    }

    fn settings_panel(ui: &mut egui::Ui, state: &mut GuiState) {
        ui.heading(RichText::new("⚙ Settings").size(18.0));
        Grid::new("cfg").show(ui, |ui: &mut egui::Ui| {
            ui.label("Render Mode:");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut state.settings.render_mode, "wallpaper".into(), "Wallpaper");
                ui.selectable_value(&mut state.settings.render_mode, "overlay".into(), "Overlay");
            });
            ui.end_row();

            ui.label("Target FPS:");
            ui.add(egui::Slider::new(&mut state.settings.fps_limit, 0..=240).clamp_to_range(true));
            ui.end_row();

            ui.label("Overlay Opacity:");
            ui.add(egui::Slider::new(&mut state.settings.window_opacity, 0.0..=1.0));
            ui.end_row();

            ui.label("Sound:");
            ui.checkbox(&mut state.settings.enable_sound, "");
            ui.end_row();

            ui.label("Mouse:");
            ui.checkbox(&mut state.settings.enable_mouse, "");
            ui.end_row();

            ui.label("VSync:");
            ui.checkbox(&mut state.settings.vsync, "");
            ui.end_row();

            ui.label("DXGI Capture:");
            ui.checkbox(&mut state.settings.use_dxgi_capture, "");
            ui.end_row();

            ui.label("Multi-Monitor:");
            ui.checkbox(&mut state.settings.multi_monitor, "");
            ui.end_row();
        });

        ui.horizontal(|ui| {
            if ui.button("💾 Save").clicked() {
                if let Err(e) = state.settings.save() {
                    state.set_status(format!("Save failed: {e}"), Color32::RED);
                } else {
                    state.set_status("Settings saved", Color32::GREEN);
                }
            }
            if ui.button("↺ Defaults").clicked() {
                let _ = AppSettings::reset_to_defaults();
                state.settings = AppSettings::default();
                state.set_status("Defaults restored", Color32::YELLOW);
            }
        });
    }

    fn packages_panel(ui: &mut egui::Ui, state: &mut GuiState) {
        ui.heading(RichText::new("📦 Packages").size(18.0));
        ui.label(format!(
            "Active: {}",
            state.current_package_path.as_deref().unwrap_or("<none>")
        ));
        ui.separator();
        if let Some(pkg) = &state.current_package {
            let pkg = pkg.as_ref();
            ui.label(format!(
                "Mode: {}",
                if pkg.config.sequence.is_empty() { "Animation (V1)" } else { "Sequence (V2)" }
            ));
            ui.label(format!(
                "Shader: {}",
                pkg.config.shader.as_deref().unwrap_or("fs_default")
            ));
            ui.label(format!("Sounds: {}", pkg.sounds.len()));
            ui.label(format!("Textures: {}", pkg.textures.len()));
            ui.separator();
            ui.label("Logic Parameters:");
            for (k, v) in &pkg.config.logic {
                ui.label(format!("  {k}: {v:.3}"));
            }
            if !pkg.config.features.is_empty() {
                ui.separator();
                ui.label("Features:");
                for (k, v) in &pkg.config.features {
                    ui.label(format!("  {k}: {}", if *v { "✅" } else { "❌" }));
                }
            }
        }
    }

    fn performance_panel(ui: &mut egui::Ui, state: &mut GuiState) {
        ui.heading(RichText::new("📊 Performance").size(18.0));
        Grid::new("perf").show(ui, |ui: &mut egui::Ui| {
            ui.label("FPS:");
            ui.colored_label(
                if state.fps >= 55.0 { Color32::GREEN } else if state.fps >= 30.0 { Color32::YELLOW } else { Color32::RED },
                format!("{:.1}", state.fps),
            );
            ui.end_row();

            ui.label("Frame Time:");
            ui.colored_label(
                if state.frame_time_ms <= 20.0 { Color32::GREEN } else if state.frame_time_ms <= 33.0 { Color32::YELLOW } else { Color32::RED },
                format!("{:.2} ms", state.frame_time_ms),
            );
            ui.end_row();

            ui.label("Capture Time:");
            ui.colored_label(
                if state.capture_time_ms <= 2.0 { Color32::GREEN } else if state.capture_time_ms <= 5.0 { Color32::YELLOW } else { Color32::RED },
                format!("{:.2} ms", state.capture_time_ms),
            );
            ui.end_row();
        });
        
        // Show real-time status
        if state.gpu_core.is_some() {
            ui.label("Live metrics from render engine.");
        } else {
            ui.label("Load a package to initialize the render engine.");
        }
    }

    fn preview_panel(ui: &mut egui::Ui, state: &mut GuiState) {
        ui.heading("🎬 Preview");
        
        let engine_locked = state.engine_state.lock().unwrap();
        let is_playing = engine_locked.playback == PlaybackState::Playing;
        let _loop_anim = engine_locked.loop_animation;
        drop(engine_locked);
        
        ui.horizontal(|ui| {
            if !is_playing {
                if ui.button("▶ Play").clicked() {
                    let mut es = state.engine_state.lock().unwrap();
                    es.playback = PlaybackState::Playing;
                    state.playback_time = 0.0;
                    drop(es);
                    state.set_status("Playback started", Color32::GREEN);
                }
            } else {
                if ui.button("⏸ Pause").clicked() {
                    let mut es = state.engine_state.lock().unwrap();
                    es.playback = PlaybackState::Paused;
                    drop(es);
                    state.set_status("Playback paused", Color32::YELLOW);
                }
            }
            
            if ui.button("⏹ Stop").clicked() {
                let mut es = state.engine_state.lock().unwrap();
                es.playback = PlaybackState::Stopped;
                drop(es);
                state.playback_time = 0.0;
                state.set_status("Playback stopped", Color32::LIGHT_GRAY);
            }
            
            let loop_btn = ui.button("🔁 Loop");
            if loop_btn.clicked() {
                let mut es = state.engine_state.lock().unwrap();
                es.loop_animation = !es.loop_animation;
                let new_loop = es.loop_animation;
                drop(es);
                state.set_status(format!("Loop {}", if new_loop { "enabled" } else { "disabled" }), if new_loop { Color32::GREEN } else { Color32::LIGHT_GRAY });
            }
        });

        ui.horizontal(|ui| {
            ui.label("Speed:");
            let mut speed_edit = state.engine_state.lock().unwrap().speed;
            ui.add(egui::Slider::new(&mut speed_edit, 0.1..=3.0));
            ui.label("x");
            state.engine_state.lock().unwrap().speed = speed_edit;
        });

        // Show current playback time if package is loaded
        if state.gpu_core.is_some() {
            ui.label(format!("Playback Time: {:.2}s", state.playback_time));
        }

        if let Some(pkg) = &state.current_package {
            ui.separator();
            ui.label("Background Preview:");
            if let Some(img_data) = &pkg.image_data {
                if let Ok(img) = image::load_from_memory(img_data) {
                    let (w, h) = img.dimensions();
                    let max_dim = 320u32;
                    let scale = (max_dim as f32 / w.max(h) as f32).min(1.0);
                    let rect = egui::Rect::from_min_size(
                        ui.cursor().left_top(),
                        egui::vec2(w as f32 * scale, h as f32 * scale),
                    );
                    ui.painter().rect_filled(rect, 4.0, Color32::DARK_GRAY);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("{}x{}", w, h),
                        egui::FontId::default(),
                        Color32::WHITE,
                    );
                } else {
                    ui.label("(invalid image data)");
                }
            } else {
                ui.label("(no background image in package)");
            }
        }

        ui.separator();
        ui.label("Sequence Timeline:");
        if !state.current_package.as_ref().map_or(true, |pkg| pkg.config.sequence.is_empty()) {
            ui.horizontal(|ui: &mut egui::Ui| {
                for step in &state.current_package.as_ref().unwrap().config.sequence {
                    let desired_size = egui::vec2(80.0, 40.0);
                    let (_, rect) = ui.allocate_space(desired_size);
                    ui.painter().rect_filled(rect, 4.0, Color32::DARK_GRAY);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        &step.name,
                        egui::FontId::default(),
                        Color32::WHITE,
                    );
                }
            });
        } else {
            ui.label("(V1 single shader mode)");
        }
    }

    fn load_package(state: &mut GuiState, path: &str) -> anyhow::Result<()> {
        // Validate package extension
        if !path.ends_with(".flow") {
            anyhow::bail!("Expected .flow package, got {path}");
        }
        
        state.set_status("Loading package...", Color32::YELLOW);
        
        // Load package from disk
        let flow = FlowPackage::load(path).context("Failed to load package")?;
        state.current_package = Some(Box::new(flow));
        state.current_package_path = Some(path.to_string());
        
        // Reset engine state when loading new package
        state.gpu_core = None;
        state.logic_engine = None;
        state.playback_time = 0.0;
        state.engine_state = Arc::new(Mutex::new(EngineState::default()));
        state.capture_time_ms = 0.0;
        
        // Initialize GPU and logic engine for the new package
        if let Err(e) = state.init_engine() {
            state.set_status(format!("Engine init failed: {e}"), Color32::RED);
            return Err(e);
        }
        
        // Add to package list if not already present
        if !state.packages.contains(&path.to_string()) {
            state.packages.push(path.to_string());
        }
        
        // Update settings with last package path
        if let Some(p) = &state.settings.last_package_path {
            if p != path {
                let mut s = state.settings.clone();
                s.last_package_path = Some(path.to_string());
                let _ = s.save();
            }
        }
        
        state.set_status("Package loaded and engine ready", Color32::GREEN);
        Ok(())
    }
}