//! .flow package loader module.
//!
//! This module handles reading and parsing `.flow` packages, which are ZIP archives
//! containing animation configuration, WGSL shaders, audio files, and textures.
//!
//! # Package Format
//!
//! A `.flow` file is a ZIP archive with the following structure:
//! ```text
//! animation.flow
//! ├── config.toml       (required) Configuration and parameters
//! ├── shader.wgsl       (required) WGSL shader source code
//! ├── background.png    (optional) Wallpaper background image
//! ├── *.wav             (optional) Audio files (PCM WAV format)
//! └── *.png/*.jpg       (optional) Additional textures
//! ```
//!
//! # Memory Management
//!
//! - Sound data is stored as `Arc<Vec<u8>>` for zero-copy sharing between loader and audio decoder
//! - Textures are decoded to RGBA8 and stored with dimensions
//! - Config is parsed from TOML with fallback to defaults for missing fields
//!
//! # Performance
//!
//! - ZIP extraction: ~50ms for typical packages (<10 MB)
//! - Image decoding: ~10ms per texture (PNG/JPG)
//! - Total load time: ~100-200ms

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::Read, sync::Arc};
use zip::ZipArchive;

/// A step in a sequence-based animation (v2 format).
///
/// Sequence steps define timed transitions between different shader/sound combinations.
/// They enable complex animations like intro loops, transitions, and multi-phase effects.
///
/// # Example
///
/// ```toml
/// [[sequence]]
/// name = "intro"
/// duration_ms = 3000
/// shader_entry = "fs_intro"
/// sound = "intro.wav"
/// ```
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Step {
    /// Unique identifier for this sequence step
    pub name: String,
    /// Duration in milliseconds (0 = infinite loop)
    pub duration_ms: u64,
    /// Fragment shader entry point name (must match a @fragment function in shader.wgsl)
    pub shader_entry: String,
    /// Optional audio file to play at step start (must exist in ZIP)
    pub sound: Option<String>,
    /// Optional texture to load for this step (bound to tex1)
    pub texture: Option<String>,
    /// Easing function hint (currently unused, reserved for future)
    pub easing: Option<String>,
}

/// Merged configuration supporting both v1 and v2 formats.
///
/// This structure is deserialized from `config.toml` inside the .flow package.
/// It uses `#[serde(default)]` to gracefully handle missing fields.
///
/// # V1 vs V2
///
/// - **V1 (Simple)**: Uses `shader`, `mode` fields. Single shader continuously rendered.
/// - **V2 (Sequence)**: Uses `sequence` array. Multiple timed steps with transitions.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Config {
    /// Logic parameters passed to shaders as `logic_params` uniform
    #[serde(default)]
    pub logic: HashMap<String, f32>,
    /// Feature flags passed to shaders as `feature_flags` uniform (1.0 = true, 0.0 = false)
    #[serde(default)]
    pub features: HashMap<String, bool>,
    /// Operation mode: "animation" (overlay), "wallpaper", or "sequence"
    pub mode: Option<String>,
    /// Fragment shader entry point (V1 only)
    pub shader: Option<String>,
    /// Rendering direction hint (V1 only, currently unused)
    pub direction: Option<String>,
    /// Sequence steps (V2 only). Empty = V1 mode.
    #[serde(default)]
    pub sequence: Vec<Step>,
    /// Z-order for layering multiple packages: "top", "bottom", "middle"
    #[serde(default)]
    pub z_order: String,
    /// Allow Windows capture API (for game overlays, etc.)
    #[serde(default)]
    pub screenshot_capture: bool,
}

/// Loaded .flow package with all assets ready for rendering.
///
/// This is the main container returned by `FlowPackage::load()`.
/// It holds all decoded assets and configuration needed to run an animation.
///
/// # Memory Layout
///
/// - `config`: ~1 KB (parsed TOML)
/// - `sounds`: Shared via `Arc`, typically 1-10 MB total
/// - `image_data`: Optional wallpaper background, up to 8K resolution (~32 MB max)
/// - `textures`: Decoded RGBA images, shared dimensions with raw pixel data
/// - `shader_src`: WGSL source code, typically 1-10 KB
pub struct FlowPackage {
    /// Parsed configuration from config.toml
    pub config: Config,
    /// Audio files indexed by filename (e.g., "intro.wav")
    /// Stored as Arc<Vec<u8>> for zero-copy sharing with rodio decoder
    pub sounds: HashMap<String, Arc<Vec<u8>>>,
    /// Wallpaper background image (background.png)
    /// Loaded once at initialization, resized to monitor resolution
    pub image_data: Option<Vec<u8>>,
    /// Sequence textures loaded as raw RGBA with dimensions
    /// Key: filename, Value: (width, height, RGBA pixel data)
    pub textures: HashMap<String, (u32, u32, Vec<u8>)>,
    /// Complete WGSL shader source code from shader.wgsl
    pub shader_src: String,
}

impl FlowPackage {
    /// Load a .flow package from filesystem.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to .flow file (ZIP archive)
    ///
    /// # Returns
    ///
    /// Returns a fully loaded `FlowPackage` with all assets decoded.
    ///
    /// # Errors
    ///
    /// - File not found or inaccessible
    /// - Invalid ZIP archive
    /// - Missing required files (config.toml, shader.wgsl)
    /// - TOML parse error (falls back to defaults)
    /// - Image decode error (skips invalid images)
    ///
    /// # Performance
    ///
    /// - ZIP reading: ~50ms
    /// - Image decoding: ~10ms per texture
    /// - Total: ~100-200ms for typical packages
    pub fn load(path: &str) -> anyhow::Result<Self> {
        // Open .flow file (ZIP archive)
        let file = std::fs::File::open(path)?;
        let mut archive = ZipArchive::new(file)?;

        // Read and parse config.toml
        let mut config_str = String::new();
        archive
            .by_name("config.toml")?
            .read_to_string(&mut config_str)?;
        // Use unwrap_or_default to gracefully handle malformed configs
        let config: Config = toml::from_str(&config_str).unwrap_or_default();

        // Read WGSL shader source (required)
        let mut shader_src = String::new();
        archive
            .by_name("shader.wgsl")?
            .read_to_string(&mut shader_src)?;

        // Initialize asset containers
        let mut sounds = HashMap::new();
        let mut image_data = None;
        let mut textures = HashMap::new();

        // Iterate all files in ZIP
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();

            // Extract WAV audio files
            if name.ends_with(".wav") {
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;
                // Wrap in Arc for zero-copy sharing with audio decoder
                sounds.insert(name, Arc::new(buffer));
            } 
            // Extract wallpaper background
            else if name == "background.png" {
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;
                image_data = Some(buffer);
            } 
            // Extract texture images (PNG/JPG)
            else if name.ends_with(".png") || name.ends_with(".jpg") {
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;
                // Decode image and convert to RGBA8
                if let Ok(img) = image::load_from_memory(&buffer) {
                    let rgba = img.to_rgba8();
                    let (w, h) = rgba.dimensions();
                    // Store dimensions + raw pixel data
                    textures.insert(name, (w, h, rgba.into_raw()));
                }
            }
        }

        Ok(Self {
            config,
            sounds,
            image_data,
            textures,
            shader_src,
        })
    }

    /// Retrieve a logic parameter value from config.
    ///
    /// # Arguments
    ///
    /// * `key` - Parameter name (e.g., "p1", "p2")
    /// * `default` - Fallback value if key not found
    ///
    /// # Returns
    ///
    /// The f32 value from config, or default if missing.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let speed = flow.val("p1", 1.0);
    /// ```
    pub fn val(&self, key: &str, default: f32) -> f32 {
        *self.config.logic.get(key).unwrap_or(&default)
    }

    /// Retrieve a feature flag from config.
    ///
    /// # Arguments
    ///
    /// * `key` - Feature name (e.g., "f1", "f2")
    ///
    /// # Returns
    ///
    /// true if feature is enabled in config, false otherwise.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if flow.feature("f1") {
    ///     // Enable special effect
    /// }
    /// ```
    pub fn feature(&self, key: &str) -> bool {
        *self.config.features.get(key).unwrap_or(&false)
    }
}
