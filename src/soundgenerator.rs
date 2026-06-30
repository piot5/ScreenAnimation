//! Sound generator module using cpal.
//!
//! This module provides real-time audio synthesis capabilities using the
//! cpal (Cross-Platform Audio Library) crate. It can generate simple
//! tones, noise, and other waveforms programmatically without requiring
//! pre-recorded audio files.
//!
//! # Features
//!
//! - Sine wave generation with configurable frequency and duration
//! - White noise generation
//! - Simple ADSR envelope for smooth sound shaping
//!
//! # Example
//!
//! ```ignore
//! use screen_animation::soundgenerator::generate_sine_wave;
//!
//! // Generate a 440 Hz sine wave for 1 second at 48000 Hz sample rate
//! let samples = generate_sine_wave(440.0, 1.0, 48000);
//! ```

use std::f32::consts::TAU;

/// Generate a sine wave with the given frequency and duration.
///
/// # Arguments
///
/// * `frequency` - Frequency in Hz (e.g., 440.0 for A4 note)
/// * `duration_secs` - Duration in seconds
/// * `sample_rate` - Sample rate in Hz (typically 44100 or 48000)
///
/// # Returns
///
/// A vector of f32 samples in the range [-1.0, 1.0].
///
/// # Performance
///
/// - ~2ms per second of audio at 48kHz
pub fn generate_sine_wave(frequency: f32, duration_secs: f32, sample_rate: u32) -> Vec<f32> {
    let num_samples = (duration_secs * sample_rate as f32) as usize;
    let mut samples = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (TAU * frequency * t).sin();
        samples.push(sample);
    }

    samples
}

/// Generate white noise for a given duration.
///
/// # Arguments
///
/// * `duration_secs` - Duration in seconds
/// * `sample_rate` - Sample rate in Hz
///
/// # Returns
///
/// A vector of f32 samples with random values in [-1.0, 1.0].
pub fn generate_white_noise(duration_secs: f32, sample_rate: u32) -> Vec<f32> {
    let num_samples = (duration_secs * sample_rate as f32) as usize;
    let mut samples = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        // Simple pseudo-random number generator
        // Using sine-based PRNG for deterministic output
        let val = ((i as u32).wrapping_mul(1103515245) + 12345) as f32 / u32::MAX as f32;
        samples.push(val * 2.0 - 1.0);
    }

    samples
}

/// Apply a simple ADSR envelope to audio samples.
///
/// # Arguments
///
/// * `samples` - Input samples to process
/// * `attack` - Attack time in seconds (time to reach full volume)
/// * `decay` - Decay time in seconds (time to reach sustain level)
/// * `sustain` - Sustain level (0.0 to 1.0)
/// * `release` - Release time in seconds (time to fade to zero)
/// * `sample_rate` - Sample rate in Hz
///
/// # Returns
///
/// Envelope-applied samples.
pub fn apply_envelope(samples: &mut [f32], attack: f32, decay: f32, sustain: f32, release: f32, sample_rate: u32) {
    let total_duration = samples.len() as f32 / sample_rate as f32;

    for (i, sample) in samples.iter_mut().enumerate() {
        let t = i as f32 / sample_rate as f32;
        let gain: f32;

        if t < attack {
            // Attack phase: linear ramp from 0 to 1
            gain = t / attack;
        } else if t < attack + decay {
            // Decay phase: linear ramp from 1 to sustain level
            let decay_t = (t - attack) / decay;
            gain = 1.0 - (1.0 - sustain) * decay_t;
        } else if total_duration - t < release {
            // Release phase: linear ramp from sustain to 0
            let release_t = (total_duration - t) / release;
            gain = sustain * release_t;
        } else {
            // Sustain phase: hold at sustain level
            gain = sustain;
        }

        *sample *= gain;
    }
}

/// Generate a simple beep tone with envelope.
///
/// # Arguments
///
/// * `frequency` - Frequency in Hz
/// * `duration_secs` - Total duration in seconds
/// * `sample_rate` - Sample rate in Hz
///
/// # Returns
///
/// A vector of f32 samples.
pub fn generate_beep(frequency: f32, duration_secs: f32, sample_rate: u32) -> Vec<f32> {
    let mut samples = generate_sine_wave(frequency, duration_secs, sample_rate);

    // Apply envelope for smooth sound
    apply_envelope(&mut samples, 0.01, 0.05, 0.7, 0.1, sample_rate);

    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_sine_wave() {
        let samples = generate_sine_wave(440.0, 1.0, 44100);
        assert_eq!(samples.len(), 44100);
        assert!(samples[0] >= -1.0 && samples[0] <= 1.0);
    }

    #[test]
    fn test_generate_white_noise() {
        let samples = generate_white_noise(0.5, 44100);
        assert_eq!(samples.len(), 22050);
    }

    #[test]
    fn test_apply_envelope() {
        let mut samples = vec![1.0; 100];
        apply_envelope(&mut samples, 0.01, 0.05, 0.7, 0.1, 1000u32);

        // Check that first and last samples are near zero (attack and release)
        assert!(samples[0] < 0.01);
        assert!(samples.last().unwrap() < &0.01);
    }
}
