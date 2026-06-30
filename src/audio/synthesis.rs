//! Real-time audio synthesis.
//!
//! Procedural sound generation utility functions:
//! - Sine waves (pure tones)
//! - White noise
//! - ADSR envelope shaping
//! - Full beep generator

use std::f32::consts::TAU;

/// Generate a sine wave.
pub fn generate_sine_wave(frequency: f32, duration_secs: f32, sample_rate: u32) -> Vec<f32> {
    let n = (duration_secs * sample_rate as f32) as usize;
    let mut samples = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sample_rate as f32;
        samples.push((TAU * frequency * t).sin());
    }
    samples
}

/// Generate white noise (deterministic PRNG).
pub fn generate_white_noise(duration_secs: f32, sample_rate: u32) -> Vec<f32> {
    let n = (duration_secs * sample_rate as f32) as usize;
    let mut samples = Vec::with_capacity(n);
    for i in 0..n {
        let val = ((i as u32).wrapping_mul(1103515245) + 12345) as f32 / u32::MAX as f32;
        samples.push(val * 2.0 - 1.0);
    }
    samples
}

/// Apply ADSR envelope to samples.
pub fn apply_envelope(samples: &mut [f32], attack: f32, decay: f32, sustain: f32, release: f32, sample_rate: u32) {
    let total = samples.len() as f32 / sample_rate as f32;
    for (i, s) in samples.iter_mut().enumerate() {
        let t = i as f32 / sample_rate as f32;
        let gain = if t < attack {
            t / attack
        } else if t < attack + decay {
            let dt = (t - attack) / decay;
            1.0 - (1.0 - sustain) * dt
        } else if total - t < release {
            let rt = (total - t) / release;
            sustain * rt
        } else {
            sustain
        };
        *s *= gain;
    }
}

/// Generate a beep (sine + envelope).
pub fn generate_beep(frequency: f32, duration_secs: f32, sample_rate: u32) -> Vec<f32> {
    let mut samples = generate_sine_wave(frequency, duration_secs, sample_rate);
    apply_envelope(&mut samples, 0.01, 0.05, 0.7, 0.1, sample_rate);
    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sine_wave_length() {
        let s = generate_sine_wave(440.0, 1.0, 44100);
        assert_eq!(s.len(), 44100);
        assert!(s[0] >= -1.0 && s[0] <= 1.0);
    }

    #[test]
    fn test_noise_length() {
        let s = generate_white_noise(0.5, 44100);
        assert_eq!(s.len(), 22050);
    }

    #[test]
    fn test_envelope() {
        let mut s = vec![1.0; 100];
        apply_envelope(&mut s, 0.01, 0.05, 0.7, 0.1, 1000);
        assert!(s[0] < 0.01);
        assert!(s.last().unwrap() < &0.01);
    }

    #[test]
    fn test_beep() {
        let s = generate_beep(440.0, 0.5, 44100);
        assert_eq!(s.len(), 22050);
    }
}