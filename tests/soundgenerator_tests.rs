use screen_animation::soundgenerator::{apply_envelope, generate_beep, generate_sine_wave, generate_white_noise};

/// Helper to ensure samples are in valid range and have some signal content.
fn validate_samples(samples: &[f32], _sample_rate: u32) {
    assert!(!samples.is_empty(), "Samples should not be empty");
    assert!(samples.len() > 10, "Should have more than 10 samples");

    // Check all samples are in valid f32 audio range
    for &s in samples {
        assert!(s >= -1.0 && s <= 1.0, "Sample {} out of range [-1, 1]", s);
    }

    // Check there is some audio content (not all silence)
    let max_amp = samples.iter().map(|&x: &f32| x.abs()).reduce(f32::max).unwrap_or(0.0);
    assert!(max_amp > 0.0, "Should have some signal content");
}

#[test]
fn test_sine_wave_properties() {
    let sample_rate: u32 = 44100;
    let samples = generate_sine_wave(440.0, 1.0, sample_rate);
    assert_eq!(samples.len(), 44100, "Expected 44100 samples for 1 second at 44.1kHz");
    validate_samples(&samples, sample_rate);
}

#[test]
fn test_sine_wave_frequency() {
    let sample_rate: u32 = 44100;
    // Generate a 1Hz tone for 1 second = exactly 1 full cycle
    let samples = generate_sine_wave(1.0, 1.0, sample_rate);
    assert_eq!(samples.len(), 44100);

    // Start at 0 (sine of 0)
    assert!((samples[0]).abs() < 0.01, "Sine wave should start near 0");

    // Quarter cycle should be near 1.0 (peak)
    let quarter = (sample_rate / 4) as usize;
    assert!((samples[quarter] - 1.0).abs() < 0.01, "Quarter cycle should be near 1.0");
}

#[test]
fn test_white_noise_properties() {
    let sample_rate: u32 = 44100;
    let samples = generate_white_noise(0.5, sample_rate);
    assert_eq!(samples.len(), 22050, "Expected 22050 samples for 0.5s at 44.1kHz");
    validate_samples(&samples, sample_rate);

    // White noise should have some variance
    let mean = samples.iter().sum::<f32>() / samples.len() as f32;
    assert!(mean.abs() < 0.2, "White noise mean should be near 0, got {}", mean);
}

#[test]
fn test_envelope_attack() {
    let sample_rate: u32 = 1000;
    let mut samples = vec![1.0f32; 100];
    apply_envelope(&mut samples, 0.01, 0.05, 0.7, 0.1, sample_rate);

    // Attack phase: first sample should be near 0
    assert!(samples[0] < 0.01, "Start of attack should be near 0");
    // Decay phase: after attack+decay, should be near sustain
    assert!((samples[60] - 0.7).abs() < 0.1, "After decay should be near sustain 0.7");
}

#[test]
fn test_envelope_release() {
    let sample_rate: u32 = 1000;
    let mut samples = vec![1.0f32; 100];
    apply_envelope(&mut samples, 0.01, 0.05, 0.7, 0.1, sample_rate);

    // Release phase: last samples should be near 0
    assert!(samples[99] < 0.01, "End of release should be near 0");
}

#[test]
fn test_envelope_sustain() {
    let sample_rate: u32 = 1000;
    let mut samples = vec![1.0f32; 600];
    apply_envelope(&mut samples, 0.01, 0.05, 0.6, 0.1, sample_rate);

    // Sustain phase (between decay and release)
    assert!((samples[200] - 0.6).abs() < 0.1, "Decay end should be near sustain");
    assert!((samples[500] - 0.6).abs() < 0.1, "Sustain should be maintained");
}

#[test]
fn test_generate_beep() {
    let sample_rate: u32 = 44100;
    let samples = generate_beep(880.0, 0.5, sample_rate);
    assert_eq!(samples.len(), 22050, "Expected 22050 samples for 0.5s beep");
    validate_samples(&samples, sample_rate);

    // Beep should have audible content
    assert!(samples.iter().any(|&x: &f32| x.abs() > 0.1), "Beep should have audible content");
}

#[test]
fn test_empty_duration() {
    let sample_rate: u32 = 44100;
    let samples = generate_sine_wave(440.0, 0.0, sample_rate);
    assert_eq!(samples.len(), 0, "Zero duration should produce no samples");
}

#[test]
fn test_different_sample_rates() {
    // Test at different sample rates
    for &sample_rate in &[8000u32, 22050u32, 44100u32, 48000u32] {
        let samples = generate_sine_wave(440.0, 0.1, sample_rate);
        let expected_len = (0.1 * sample_rate as f32) as usize;
        assert_eq!(samples.len(), expected_len, "Mismatch at sample rate {}", sample_rate);
    }
}
