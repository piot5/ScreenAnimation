use screen_animation::loader::FlowPackage;

#[test]
fn test_monitorswitch_flow_loads() {
    let flow = FlowPackage::load("examples/monitorswitch.flow")
        .expect("monitorswitch.flow should load successfully");

    assert!(!flow.shader_src.is_empty(), "shader source must not be empty");
    assert_eq!(flow.config.mode.as_deref(), Some("animation"));
    assert_eq!(flow.config.sequence.len(), 2);
    assert!(flow.sounds.contains_key("pulse.wav"));
    assert!(flow.sounds.contains_key("woosh.wav"));
}