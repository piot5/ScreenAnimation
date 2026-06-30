//! Unit tests for logic module.
//!
//! Tests the LogicEngine and uniform buffer calculation.

use screen_animation::loader::FlowPackage;
use screen_animation::logic::LogicEngine;

// Mock a minimal FlowPackage for testing
fn create_mock_flow() -> FlowPackage {
    let config_str = r#"
mode = "animation"
shader = "fs_default"

[logic]
p1 = 1.5
p2 = 2.5
p3 = 3.5
p4 = 4.5

[features]
f1 = true
f2 = false
"#;
    let config: screen_animation::loader::Config = toml::from_str(config_str).unwrap();

    FlowPackage {
        config,
        sounds: std::collections::HashMap::new(),
        image_data: None,
        textures: std::collections::HashMap::new(),
        shader_src: String::new(),
    }
}

#[test]
fn test_logic_engine_creation() {
    let flow = create_mock_flow();
    let logic = LogicEngine::new(&flow);
    // Verify start_time is recent (within last second)
    let elapsed = logic.start_time.elapsed().as_secs_f32();
    assert!(elapsed < 1.0, "LogicEngine start_time should be recent");
}

#[test]
fn test_uniforms_mouse_position() {
    let flow = create_mock_flow();
    let logic = LogicEngine::new(&flow);

    // Test normalized mouse position (0.0 to 1.0)
    let uniforms = logic.update(&flow, [0.5, 0.5]);
    assert_eq!(uniforms.mouse, [0.5, 0.5], "Mouse position should be preserved");
}

#[test]
fn test_uniforms_logic_params() {
    let flow = create_mock_flow();
    let logic = LogicEngine::new(&flow);

    let uniforms = logic.update(&flow, [0.0, 0.0]);

    // Verify logic parameters from config
    assert_eq!(uniforms.logic_params[0], 1.5, "p1 should be 1.5");
    assert_eq!(uniforms.logic_params[1], 2.5, "p2 should be 2.5");
    assert_eq!(uniforms.logic_params[2], 3.5, "p3 should be 3.5");
    assert_eq!(uniforms.logic_params[3], 4.5, "p4 should be 4.5");
}

#[test]
fn test_uniforms_feature_flags() {
    let flow = create_mock_flow();
    let logic = LogicEngine::new(&flow);

    let uniforms = logic.update(&flow, [0.0, 0.0]);

    // Verify feature flags (true → 1.0, false → 0.0)
    assert_eq!(uniforms.feature_flags[0], 1.0, "f1 should be 1.0 (true)");
    assert_eq!(uniforms.feature_flags[1], 0.0, "f2 should be 0.0 (false)");
    assert_eq!(uniforms.feature_flags[2], 0.0, "f3 should be 0.0 (default)");
    assert_eq!(uniforms.feature_flags[3], 0.0, "f4 should be 0.0 (default)");
}

#[test]
fn test_uniforms_time_increment() {
    let flow = create_mock_flow();
    let logic = LogicEngine::new(&flow);

    let uniforms1 = logic.update(&flow, [0.0, 0.0]);
    std::thread::sleep(std::time::Duration::from_millis(100));
    let uniforms2 = logic.update(&flow, [0.0, 0.0]);

    // Time should have increased
    assert!(uniforms2.time > uniforms1.time, "Time should increase between frames");
    assert!(uniforms2.time - uniforms1.time >= 0.1, "Time difference should be ~100ms");
}

#[test]
fn test_uniforms_default_values() {
    // Create flow with missing parameters (should use defaults)
    let config_str = r#"mode = "animation""#;
    let config: screen_animation::loader::Config = toml::from_str(config_str).unwrap();
    let flow_empty = FlowPackage {
        config,
        sounds: std::collections::HashMap::new(),
        image_data: None,
        textures: std::collections::HashMap::new(),
        shader_src: String::new(),
    };

    let logic = LogicEngine::new(&flow_empty);
    let uniforms = logic.update(&flow_empty, [0.0, 0.0]);

    // Verify defaults
    assert_eq!(uniforms.logic_params[0], 0.0, "p1 should default to 0.0");
    assert_eq!(uniforms.feature_flags[0], 0.0, "f1 should default to false (0.0)");
    assert_eq!(uniforms.scale, 1.0, "Scale should be 1.0");
    assert_eq!(uniforms.offset, [0.0, 0.0], "Offset should be zero");
    assert_eq!(uniforms._padding, [0.0, 0.0], "Padding should be zero");
}

#[test]
fn test_uniforms_offset_mirrors_mouse() {
    let flow = create_mock_flow();
    let logic = LogicEngine::new(&flow);

    let uniforms = logic.update(&flow, [0.3, 0.7]);

    // Offset currently mirrors mouse position (reserved for future)
    assert_eq!(uniforms.offset, [0.3, 0.7], "Offset should mirror mouse");
}
