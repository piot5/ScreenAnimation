//! Integration tests for .flow package loading.
//!
//! These tests verify that the builder produces valid .flow packages
//! that can be loaded by the animation engine.
//!
//! # Usage
//!
//! ```bash
//! cargo test --test flow_loading
//! ```
//!
//! Run with `--nocapture` to see verbose output:
//! ```bash
//! cargo test --test flow_loading -- --nocapture
//! ```

use screen_animation::loader::FlowPackage;

/// Verify that a .flow package can be loaded and contains required fields.
fn verify_package(path: &str, expected_shader: Option<&str>, expected_mode: Option<&str>, has_sequence: bool) {
    let flow = FlowPackage::load(path).unwrap_or_else(|e| panic!("Failed to load package '{}': {}", path, e));

    // Verify shader source is non-empty
    assert!(!flow.shader_src.is_empty(), "shader.wgsl must not be empty in '{}'", path);
    assert!(flow.shader_src.contains("vs_main"), "shader.wgsl must contain vs_main entry point in '{}'", path);

    // Verify config has valid mode
    if let Some(expected) = expected_mode {
        assert_eq!(
            flow.config.mode.as_deref(),
            Some(expected),
            "mode mismatch in '{}': expected Some({:?}), got {:?}",
            path,
            expected,
            flow.config.mode
        );
    }

    // Verify shader entry point
    if let Some(expected) = expected_shader {
        assert_eq!(flow.config.shader.as_deref(), Some(expected), "shader entry point mismatch in '{}'", path);
    }

    // Verify sequence config matches
    assert_eq!(
        !flow.config.sequence.is_empty(),
        has_sequence,
        "sequence presence mismatch in '{}': expected has_sequence={}",
        path,
        has_sequence
    );
}

#[test]
fn test_livewallpaper_package() {
    verify_package("examples/livewallpaper.flow", Some("fs_live_wallpaper"), Some("wallpaper"), false);

    // Verify logic parameters
    let flow = FlowPackage::load("examples/livewallpaper.flow").unwrap();
    assert_eq!(flow.val("p1", 0.0), 0.5, "p1 (wave_speed)");
    assert_eq!(flow.val("p2", 0.0), 0.03, "p2 (wave_amplitude)");
    assert_eq!(flow.val("p3", 0.0), 10.0, "p3 (wave_frequency)");
    assert_eq!(flow.val("p4", 0.0), 1.0, "p4 (brightness)");

    // Verify feature flags
    assert!(flow.feature("f1"), "f1 (enable_wave)");
    assert!(flow.feature("f2"), "f2 (enable_mouse_warp)");
    assert!(flow.feature("f3"), "f3 (show_vignette)");

    // Verify audio files loaded
    assert!(flow.sounds.contains_key("pulse.wav"), "pulse.wav missing");
    assert!(flow.sounds.contains_key("woosh.wav"), "woosh.wav missing");
}

#[test]
fn test_screentransition_package() {
    verify_package("examples/screentransition.flow", Some("fs_stable"), Some("animation"), true);

    // Verify sequence steps
    let flow = FlowPackage::load("examples/screentransition.flow").unwrap();
    let seq = &flow.config.sequence;
    assert_eq!(seq.len(), 5, "Expected 5 sequence steps");

    // Verify step names and durations
    assert_eq!(seq[0].name, "capture");
    assert_eq!(seq[0].duration_ms, 500);
    assert_eq!(seq[0].shader_entry, "fs_capture");
    // Media moved into sequence.media[]; at least one entry present for this step
    assert!(
        seq[0].media.iter().any(|m| m.sound.as_deref() == Some("woosh.wav")),
        "Expected woosh.wav in step '{}' media events",
        seq[0].name
    );

    assert_eq!(seq[1].name, "detach");
    assert_eq!(seq[1].duration_ms, 1500);
    assert_eq!(seq[1].shader_entry, "fs_detach");

    assert_eq!(seq[2].name, "move");
    assert_eq!(seq[2].duration_ms, 2000);
    assert_eq!(seq[2].shader_entry, "fs_move");

    assert_eq!(seq[3].name, "land");
    assert_eq!(seq[3].duration_ms, 1000);
    assert_eq!(seq[3].shader_entry, "fs_land");

    assert_eq!(seq[4].name, "stable");
    assert_eq!(seq[4].duration_ms, 0); // 0 = infinite
    assert_eq!(seq[4].shader_entry, "fs_stable");

    // Verify logic parameters
    assert_eq!(flow.val("p1", 0.0), 1.5, "p1 (detach_speed)");
    assert_eq!(flow.val("p2", 0.0), 0.3, "p2 (move_distance)");
    assert_eq!(flow.val("p3", 0.0), 0.5, "p3 (rotation_speed)");
    assert_eq!(flow.val("p4", 0.0), 0.3, "p4 (perspective)");

    // Verify feature flags
    assert!(flow.feature("f1"), "f1 (enable_3d)");
    assert!(flow.feature("f2"), "f2 (enable_sound)");

    // Verify audio files
    assert!(flow.sounds.contains_key("pulse.wav"), "pulse.wav missing");
    assert!(flow.sounds.contains_key("woosh.wav"), "woosh.wav missing");
}

#[test]
fn test_assets_packages() {
    // Test that the asset packages under assets/ also load
    // These use the older shader Uniforms format but still load
    let flow = FlowPackage::load("assets/wallpaper1/config.toml");
    // assets are not .flow files, so loading should fail
    assert!(
        flow.is_err(),
        "assets/wallpaper1/config.toml should not be a valid .flow package (missing .flow extension)"
    );
}
