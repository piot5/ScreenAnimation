//! Unit tests for loader module.
//!
//! Tests .flow package loading, validation, and parsing.

use screen_animation::loader::FlowPackage;

#[test]
fn test_loader_constants() {
    // Verify security constants are set correctly
    const MAX_PACKAGE_SIZE: u64 = 100 * 1024 * 1024;
    const MAX_TEXTURE_DIMENSION: u32 = 8192;
    const MAX_AUDIO_FILES: usize = 32;
    const MAX_TEXTURE_FILES: usize = 16;

    assert_eq!(MAX_PACKAGE_SIZE, 104857600, "Package size limit should be 100MB");
    assert_eq!(MAX_TEXTURE_DIMENSION, 8192, "Texture dimension limit should be 8192");
    assert_eq!(MAX_AUDIO_FILES, 32, "Audio file limit should be 32");
    assert_eq!(MAX_TEXTURE_FILES, 16, "Texture file limit should be 16");
}

#[test]
fn test_config_parsing_with_all_fields() {
    let config_str = r#"
mode = "animation"
shader = "fs_default"
direction = "forward"
z_order = "top"
screenshot_capture = true

[logic]
speed = 1.5
amplitude = 0.3
frequency = 10.0
brightness = 2.0

[features]
enable_effect = true
secondary_feature = false
tertiary_feature = true
quaternary_feature = false
"#;

    let config: screen_animation::loader::Config = toml::from_str(config_str).unwrap();

    assert_eq!(config.mode, Some("animation".to_string()));
    assert_eq!(config.shader, Some("fs_default".to_string()));
    assert_eq!(config.direction, Some("forward".to_string()));
    assert_eq!(config.z_order, "top");
    assert_eq!(config.screenshot_capture, true);

    assert_eq!(config.logic.get("speed"), Some(&1.5));
    assert_eq!(config.logic.get("amplitude"), Some(&0.3));
    assert_eq!(config.features.get("enable_effect"), Some(&true));
    assert_eq!(config.features.get("secondary_feature"), Some(&false));
}

#[test]
fn test_config_parsing_with_minimal_fields() {
    let config_str = r#"mode = "wallpaper""#;

    let config: screen_animation::loader::Config = toml::from_str(config_str).unwrap();

    assert_eq!(config.mode, Some("wallpaper".to_string()));
    assert!(config.logic.is_empty());
    assert!(config.features.is_empty());
    assert!(config.sequence.is_empty());
}

#[test]
fn test_config_parsing_with_empty_hashmaps() {
    let config_str = r#"
mode = "animation"
shader = "fs_main"

[logic]
[features]
"#;

    let config: screen_animation::loader::Config = toml::from_str(config_str).unwrap();

    assert!(config.logic.is_empty());
    assert!(config.features.is_empty());
}

#[test]
fn test_config_sequence_steps() {
    let config_str = r#"
mode = "sequence"

[[sequence]]
name = "intro"
duration_ms = 3000
shader_entry = "fs_intro"
easing = "easeInOut"

[[sequence]]
name = "main"
duration_ms = 0
shader_entry = "fs_main"
"#;

    let config: screen_animation::loader::Config = toml::from_str(config_str).unwrap();

    assert_eq!(config.sequence.len(), 2);
    assert_eq!(config.sequence[0].name, "intro");
    assert_eq!(config.sequence[0].duration_ms, 3000);
    assert_eq!(config.sequence[0].shader_entry, "fs_intro");
    assert_eq!(config.sequence[1].name, "main");
    assert_eq!(config.sequence[1].duration_ms, 0); // Infinite loop
    assert!(config.sequence[0].media.is_empty());
}

#[test]
fn test_config_sequence_with_media() {
    let config_str = r#"
mode = "sequence"

[[sequence]]
name = "transition"
duration_ms = 2000
shader_entry = "fs_transition"

  [[sequence.media]]
  at_ms = 0
  sound = "start.wav"
  
  [[sequence.media]]
  at_ms = 1000
  texture = "flash.png"
  
  [[sequence.media]]
  at_ms = 1500
  sound = "end.wav"
"#;

    let config: screen_animation::loader::Config = toml::from_str(config_str).unwrap();

    assert_eq!(config.sequence.len(), 1);
    assert_eq!(config.sequence[0].media.len(), 3);
    assert_eq!(config.sequence[0].media[0].sound, Some("start.wav".to_string()));
    assert_eq!(config.sequence[0].media[1].texture, Some("flash.png".to_string()));
    assert_eq!(config.sequence[0].media[2].sound, Some("end.wav".to_string()));
    assert_eq!(config.sequence[0].media[0].at_ms, 0);
    assert_eq!(config.sequence[0].media[1].at_ms, 1000);
}

#[test]
fn test_flow_package_val_accessor() {
    // Create mock flow package with logic params under [logic] table
    let config_str = r#"
mode = "animation"

[logic]
p1 = 42.0
p2 = 3.14
p3 = 2.71
p4 = 1.41
"#;
    let config: screen_animation::loader::Config = toml::from_str(config_str).unwrap();
    let flow = FlowPackage {
        config,
        sounds: std::collections::HashMap::new(),
        image_data: None,
        textures: std::collections::HashMap::new(),
        shader_src: String::new(),
    };

    assert_eq!(flow.val("p1", 0.0), 42.0);
    assert_eq!(flow.val("p2", 0.0), 3.14);
    assert_eq!(flow.val("p3", 99.0), 2.71);
    assert_eq!(flow.val("p4", 0.0), 1.41);
    assert_eq!(flow.val("nonexistent", 77.0), 77.0);
}

#[test]
fn test_flow_package_feature_accessor() {
    let config_str = r#"
mode = "animation"

[features]
feature1 = true
feature2 = false
feature3 = true
"#;
    let config: screen_animation::loader::Config = toml::from_str(config_str).unwrap();
    let flow = FlowPackage {
        config,
        sounds: std::collections::HashMap::new(),
        image_data: None,
        textures: std::collections::HashMap::new(),
        shader_src: String::new(),
    };

    assert!(flow.feature("feature1"));
    assert!(!flow.feature("feature2"));
    assert!(flow.feature("feature3"));
    assert!(!flow.feature("nonexistent"));
}

#[test]
fn test_config_serialization() {
    let mut config = screen_animation::loader::Config::default();
    config.mode = Some("animation".to_string());
    config.shader = Some("fs_default".to_string());
    config.logic.insert("speed".to_string(), 1.5);
    config.features.insert("enable_effect".to_string(), true);

    // Serialize to TOML
    let toml_str = toml::to_string(&config).unwrap();

    // Deserialize back
    let parsed: screen_animation::loader::Config = toml::from_str(&toml_str).unwrap();

    assert_eq!(parsed.mode, Some("animation".to_string()));
    assert_eq!(parsed.shader, Some("fs_default".to_string()));
    assert_eq!(parsed.logic.get("speed"), Some(&1.5));
    assert_eq!(parsed.features.get("enable_effect"), Some(&true));
}

#[test]
fn test_media_event_serialization() {
    let media = screen_animation::loader::MediaEvent {
        at_ms: 500,
        sound: Some("click.wav".to_string()),
        texture: None,
    };

    let toml_str = toml::to_string(&media).unwrap();
    let parsed: screen_animation::loader::MediaEvent = toml::from_str(&toml_str).unwrap();

    assert_eq!(parsed.at_ms, 500);
    assert_eq!(parsed.sound, Some("click.wav".to_string()));
    assert!(parsed.texture.is_none());
}

#[test]
fn test_step_serialization() {
    let step = screen_animation::loader::Step {
        name: "test_step".to_string(),
        duration_ms: 2500,
        shader_entry: "fs_test".to_string(),
        easing: Some("easeInOut".to_string()),
        media: vec![],
    };

    let toml_str = toml::to_string(&step).unwrap();
    let parsed: screen_animation::loader::Step = toml::from_str(&toml_str).unwrap();

    assert_eq!(parsed.name, "test_step");
    assert_eq!(parsed.duration_ms, 2500);
    assert_eq!(parsed.shader_entry, "fs_test");
    assert_eq!(parsed.easing, Some("easeInOut".to_string()));
}

#[test]
fn test_config_defaults() {
    let config = screen_animation::loader::Config::default();

    assert!(config.mode.is_none());
    assert!(config.shader.is_none());
    assert!(config.logic.is_empty());
    assert!(config.features.is_empty());
    assert!(config.sequence.is_empty());
    assert_eq!(config.z_order, "");
    assert!(!config.screenshot_capture);
}
