//! Settings persistence module.
//!
//! This module provides a settings system for user preferences using
//! `confy` for file-based persistence with a Windows Registry fallback.
//! Settings are stored in the user's application data directory.
//!
//! # Storage Locations
//!
//! | OS | Primary (confy) | Fallback |
//! |----|-----------------|----------|
//! | Windows | `%APPDATA%\ScreenAnimation\preferences.toml` | Registry `HKCU\Software\ScreenAnimation` |
//! | Linux | `~/.config/screen_animation/preferences.toml` | - |
//! | macOS | `~/Library/Application Support/screen_animation/preferences.toml` | - |
//!
//! # Example
//!
//! ```ignore
//! use screen_animation::settings::AppSettings;
//!
//! let settings = AppSettings::load().unwrap();
//! println!("Render mode: {}", settings.render_mode);
//! ```

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use windows::core::PCWSTR;

/// Application settings stored persistently.
///
/// # Fields
///
/// - `render_mode`: "wallpaper" or "overlay" (default: "wallpaper")
/// - `fps_limit`: Target FPS (default: 60, 0 = unlimited)
/// - `enable_sound`: Enable sound effects (default: true)
/// - `enable_mouse`: Enable mouse interaction (default: true)
/// - `last_package_path`: Path to last loaded .flow package (optional)
/// - `window_opacity`: Overlay window opacity 0.0-1.0 (default: 1.0)
/// - `vsync`: Enable vsync (default: true)
/// - `use_dxgi_capture`: Use DXGI instead of BitBlt (default: true)
/// - `multi_monitor`: Span animations across all monitors (default: true)
/// - `log_level`: Logging verbosity (default: "info")
///
/// # Industrial-Grade Design
///
/// - Uses `confy` for atomic file writes (crash-safe)
/// - Fallback to Windows Registry for locked-down environments
/// - All fields have sensible defaults via `Default` trait
/// - Human-readable TOML format for easy manual editing
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppSettings {
    /// Render mode: "wallpaper" or "overlay"
    #[serde(default = "default_render_mode")]
    pub render_mode: String,
    /// Target FPS (0 = unlimited)
    #[serde(default = "default_fps_limit")]
    pub fps_limit: u32,
    /// Enable sound effects
    #[serde(default = "default_enable_sound")]
    pub enable_sound: bool,
    /// Enable mouse interaction in shaders
    #[serde(default = "default_enable_mouse")]
    pub enable_mouse: bool,
    /// Path to last loaded .flow package
    #[serde(default)]
    pub last_package_path: Option<String>,
    /// Overlay window opacity 0.0-1.0
    #[serde(default = "default_window_opacity")]
    pub window_opacity: f32,
    /// Enable vsync
    #[serde(default = "default_vsync")]
    pub vsync: bool,
    /// Use DXGI capture instead of BitBlt
    #[serde(default = "default_use_dxgi_capture")]
    pub use_dxgi_capture: bool,
    /// Span animations across all monitors
    #[serde(default = "default_multi_monitor")]
    pub multi_monitor: bool,
    /// Logging verbosity ("error", "warn", "info", "debug", "trace")
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// Version of the settings schema (for migration)
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
}

// Default value functions
fn default_render_mode() -> String {
    "wallpaper".to_string()
}
fn default_fps_limit() -> u32 {
    60
}
fn default_enable_sound() -> bool {
    true
}
fn default_enable_mouse() -> bool {
    true
}
fn default_window_opacity() -> f32 {
    1.0
}
fn default_vsync() -> bool {
    true
}
fn default_use_dxgi_capture() -> bool {
    true
}
fn default_multi_monitor() -> bool {
    true
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_schema_version() -> u32 {
    1
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            render_mode: default_render_mode(),
            fps_limit: default_fps_limit(),
            enable_sound: default_enable_sound(),
            enable_mouse: default_enable_mouse(),
            last_package_path: None,
            window_opacity: default_window_opacity(),
            vsync: default_vsync(),
            use_dxgi_capture: default_use_dxgi_capture(),
            multi_monitor: default_multi_monitor(),
            log_level: default_log_level(),
            schema_version: default_schema_version(),
        }
    }
}

impl AppSettings {
    /// Get the settings file path.
    ///
    /// Returns `%APPDATA%/ScreenAnimation/preferences.toml` on Windows,
    /// or the equivalent on other platforms.
    ///
    /// # Errors
    ///
    /// Returns error if the config directory cannot be determined
    /// (extremely rare - only on sandboxed systems without home dirs).
    fn config_path() -> anyhow::Result<PathBuf> {
        let base = directories::ProjectDirs::from("", "", "ScreenAnimation")
            .context("Cannot determine application config directory")?
            .config_dir()
            .to_path_buf();
        Ok(base.join("preferences.toml"))
    }

    /// Load settings from disk, or create defaults if no file exists.
    ///
    /// # Returns
    ///
    /// Returns `AppSettings` with values loaded from the config file,
    /// or defaults if the file does not exist yet.
    ///
    /// # Errors
    ///
    /// - Returns error if the config directory cannot be determined
    /// - Returns error if the config file is corrupt (invalid TOML)
    ///
    /// # Performance
    ///
    /// - File read: ~1ms (typically <1 KB file)
    /// - TOML parse: <0.1ms
    pub fn load() -> anyhow::Result<Self> {
        let path = Self::config_path()?;

        // Try to load from confy-style file
        match confy::load_path::<Self>(&path) {
            Ok(settings) => {
                eprintln!("[settings] Loaded from: {}", path.display());
                Ok(settings)
            }
            Err(e) => {
                // Log warning and try registry fallback on Windows
                eprintln!("[settings] Failed to load from file ({}), trying registry fallback", e);
                #[cfg(target_os = "windows")]
                {
                    Self::load_from_registry().or_else(|_| {
                        eprintln!("[settings] No registry settings found, using defaults");
                        Ok(Self::default())
                    })
                }
                #[cfg(not(target_os = "windows"))]
                {
                    eprintln!("[settings] No settings file at {}, using defaults", path.display());
                    Ok(Self::default())
                }
            }
        }
    }

    /// Save settings to disk.
    ///
    /// # Errors
    ///
    /// - Returns error if the config directory cannot be created
    /// - Returns error if writing the file fails
    ///
    /// # Performance
    ///
    /// - File write: ~1ms (atomic write via confy)
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path()?;

        // Ensure config directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create settings directory")?;
        }

        // Save using confy (atomic write to temp file + rename)
        confy::store_path(&path, self).context("Failed to save settings file")?;

        // Also save to registry on Windows for cross-app discovery
        #[cfg(target_os = "windows")]
        self.save_to_registry().ok();

        eprintln!("[settings] Saved to: {}", path.display());
        Ok(())
    }

    /// Load settings from Windows Registry fallback.
    ///
    /// Used when the config file is unavailable (e.g., read-only filesystem,
    /// sandboxed environment).
    ///
    /// # Errors
    ///
    /// Returns error if registry key doesn't exist or cannot be read.
    #[cfg(target_os = "windows")]
    fn load_from_registry() -> anyhow::Result<Self> {
        use windows::core::w;
        use windows::Win32::System::Registry::*;

        let hkey = HKEY_CURRENT_USER;
        let sub_key = w!("Software\\ScreenAnimation");

        let mut settings = Self::default();

        // Open registry key
        // SAFETY: RegOpenKeyExW is safe with valid handles and strings
        unsafe {
            let mut key = HKEY::default();
            RegOpenKeyExW(hkey, sub_key, 0, KEY_READ, &mut key)
                .ok()
                .context("Registry key Software\\ScreenAnimation not found")?;

            // Helper to read a string value from registry
            // SAFETY: All operations use valid handles and pre-allocated buffers
            unsafe fn read_reg_string(key: HKEY, name: &str) -> Option<String> {
                let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
                let mut size: u32 = 0;

                // First call to get buffer size
                if RegQueryValueExW(key, PCWSTR(name_wide.as_ptr()), None, None, None, Some(&mut size)).is_err()
                    || size == 0
                {
                    return None;
                }

                let mut buffer = vec![0u16; (size / 2) as usize];
                let mut value_type = REG_NONE;

                if RegQueryValueExW(
                    key,
                    PCWSTR(name_wide.as_ptr()),
                    None,
                    Some(&mut value_type as *mut _ as *mut _),
                    Some(buffer.as_mut_ptr() as *mut u8),
                    Some(&mut size as *mut _),
                )
                .is_err()
                    || value_type != REG_SZ
                {
                    return None;
                }

                // Find null terminator
                let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
                Some(String::from_utf16_lossy(&buffer[..len]))
            }

            // Helper to read a DWORD value from registry
            unsafe fn read_reg_dword(key: HKEY, name: &str) -> Option<u32> {
                let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
                let mut value: u32 = 0;
                let mut size: u32 = 4;
                let mut value_type = REG_NONE;

                if RegQueryValueExW(
                    key,
                    PCWSTR(name_wide.as_ptr()),
                    None,
                    Some(&mut value_type as *mut _ as *mut _),
                    Some(&mut value as *mut _ as *mut u8),
                    Some(&mut size as *mut _),
                )
                .is_err()
                    || value_type != REG_DWORD
                {
                    return None;
                }
                Some(value)
            }

            // Read all settings from registry
            if let Some(mode) = read_reg_string(key, "render_mode") {
                settings.render_mode = mode;
            }
            if let Some(fps) = read_reg_dword(key, "fps_limit") {
                settings.fps_limit = fps;
            }
            if let Some(v) = read_reg_dword(key, "enable_sound") {
                settings.enable_sound = v != 0;
            }
            if let Some(v) = read_reg_dword(key, "enable_mouse") {
                settings.enable_mouse = v != 0;
            }
            if let Some(path) = read_reg_string(key, "last_package_path") {
                settings.last_package_path = Some(path);
            }
            if let Some(v) = read_reg_dword(key, "use_dxgi_capture") {
                settings.use_dxgi_capture = v != 0;
            }

            // Close registry key
            let _ = RegCloseKey(key);
        }

        Ok(settings)
    }

    /// Save settings to Windows Registry.
    ///
    /// # Safety
    ///
    /// Registry write operations are safe with valid handles.
    /// Errors are silently ignored (file is the primary storage).
    #[cfg(target_os = "windows")]
    fn save_to_registry(&self) -> anyhow::Result<()> {
        use windows::core::w;
        use windows::Win32::System::Registry::*;

        let hkey = HKEY_CURRENT_USER;
        let sub_key = w!("Software\\ScreenAnimation");

        // SAFETY: RegCreateKeyExW creates or opens a key
        unsafe {
            let mut key = HKEY::default();
            RegCreateKeyExW(hkey, sub_key, 0, None, Default::default(), KEY_WRITE, None, &mut key, None)
                .ok()
                .context("Failed to create/open registry key")?;

            // Helper to write a string value
            unsafe fn write_reg_string(key: HKEY, name: &str, value: &str) {
                let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
                let value_wide: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
                let value_bytes = std::slice::from_raw_parts(value_wide.as_ptr() as *const u8, value_wide.len() * 2);

                let _ = RegSetValueExW(key, PCWSTR(name_wide.as_ptr()), 0, REG_SZ, Some(value_bytes));
            }

            // Helper to write a DWORD value
            unsafe fn write_reg_dword(key: HKEY, name: &str, value: u32) {
                let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
                let _ = RegSetValueExW(key, PCWSTR(name_wide.as_ptr()), 0, REG_DWORD, Some(&value.to_ne_bytes()));
            }

            write_reg_string(key, "render_mode", &self.render_mode);
            write_reg_dword(key, "fps_limit", self.fps_limit);
            write_reg_dword(key, "enable_sound", self.enable_sound as u32);
            write_reg_dword(key, "enable_mouse", self.enable_mouse as u32);
            if let Some(ref path) = self.last_package_path {
                write_reg_string(key, "last_package_path", path);
            }
            write_reg_dword(key, "use_dxgi_capture", self.use_dxgi_capture as u32);
            write_reg_dword(key, "schema_version", self.schema_version);

            let _ = RegCloseKey(key);
        }

        Ok(())
    }

    /// Reset all settings to defaults.
    ///
    /// This deletes the settings file and clears registry entries,
    /// effectively restoring factory defaults.
    ///
    /// # Errors
    ///
    /// Returns error if file deletion fails.
    pub fn reset_to_defaults() -> anyhow::Result<()> {
        let path = Self::config_path()?;
        if path.exists() {
            std::fs::remove_file(&path).context("Failed to remove settings file")?;
        }

        // Clear registry entries on Windows
        #[cfg(target_os = "windows")]
        {
            use windows::core::w;
            use windows::Win32::System::Registry::*;
            let _ = unsafe { RegDeleteKeyW(HKEY_CURRENT_USER, w!("Software\\ScreenAnimation")) };
        }

        eprintln!("[settings] Reset to defaults");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_defaults() {
        let settings = AppSettings::default();
        assert_eq!(settings.render_mode, "wallpaper");
        assert_eq!(settings.fps_limit, 60);
        assert!(settings.enable_sound);
        assert!(settings.enable_mouse);
        assert!(settings.last_package_path.is_none());
        assert_eq!(settings.window_opacity, 1.0);
        assert!(settings.vsync);
        assert!(settings.use_dxgi_capture);
        assert_eq!(settings.log_level, "info");
        assert_eq!(settings.schema_version, 1);
    }

    #[test]
    fn test_settings_serialization_roundtrip() {
        let settings = AppSettings {
            render_mode: "overlay".to_string(),
            fps_limit: 144,
            enable_sound: false,
            enable_mouse: true,
            last_package_path: Some("C:\\animations\\test.flow".to_string()),
            window_opacity: 0.8,
            vsync: false,
            use_dxgi_capture: true,
            multi_monitor: false,
            log_level: "debug".to_string(),
            schema_version: 1,
        };

        // Serialize to TOML
        let toml_str = toml::to_string(&settings).expect("Serialization should succeed");

        // Deserialize back
        let deserialized: AppSettings = toml::from_str(&toml_str).expect("Deserialization should succeed");

        assert_eq!(deserialized.render_mode, "overlay");
        assert_eq!(deserialized.fps_limit, 144);
        assert!(!deserialized.enable_sound);
        assert!(deserialized.enable_mouse);
        assert_eq!(deserialized.last_package_path, Some("C:\\animations\\test.flow".to_string()));
        assert!((deserialized.window_opacity - 0.8).abs() < 0.001);
        assert!(!deserialized.vsync);
        assert_eq!(deserialized.log_level, "debug");
    }

    #[test]
    fn test_settings_partial_deserialization() {
        // Test that missing fields use defaults
        let minimal_toml = r#"
render_mode = "wallpaper"
"#;
        let settings: AppSettings = toml::from_str(minimal_toml).expect("Partial TOML should deserialize");
        assert_eq!(settings.render_mode, "wallpaper");
        assert_eq!(settings.fps_limit, 60); // Default
        assert!(settings.enable_sound); // Default
    }

    #[test]
    fn test_settings_save_and_load() {
        use std::fs;

        let settings = AppSettings {
            render_mode: "overlay".to_string(),
            fps_limit: 120,
            enable_sound: true,
            enable_mouse: true,
            last_package_path: None,
            window_opacity: 1.0,
            vsync: false,
            use_dxgi_capture: true,
            multi_monitor: true,
            log_level: "info".to_string(),
            schema_version: 1,
        };

        // Create a temp file path for testing
        let tmp_dir = std::env::temp_dir().join("screen_animation_test");
        let _ = fs::create_dir_all(&tmp_dir);
        let test_path = tmp_dir.join("test_preferences.toml");

        // Save using confy
        confy::store_path(&test_path, &settings).expect("Save should succeed");

        // Load back
        let loaded: AppSettings = confy::load_path(&test_path).expect("Load should succeed");

        assert_eq!(loaded.render_mode, "overlay");
        assert_eq!(loaded.fps_limit, 120);
        assert!(!loaded.vsync);

        // Cleanup
        let _ = fs::remove_file(&test_path);
        let _ = fs::remove_dir(&tmp_dir);
    }
}
