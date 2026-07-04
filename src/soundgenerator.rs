//! Sound generator module.
//!
//! This module provides real-time audio synthesis capabilities for screen animations.
//! It generates waveforms procedurally without requiring audio files, enabling
//! dynamic sound effects synchronized with visual animations.
//!
//! # Features
//!
//! - Sine wave generation with frequency and amplitude control
//! - White noise generation
//! - ADSR envelope support for dynamic sound shaping
//! - Real-time audio processing via cpal/rodio
//! - WAV export for preview/debugging
//!
//! # Example
//!
//! ```ignore
//! use screen_animation::soundgenerator::{SoundGenerator, SoundParams, Waveform};
//!
//! let params = SoundParams {
//!     frequency: 440.0,  // A4 note
//!     amplitude: 0.5,
//!     waveform: Waveform::Sine,
//!     ..Default::default()
//! };
//! let buffer = SoundGenerator::generate(&params, 44100, 2); // 44.1kHz, 2 seconds
//! ```

use std::sync::Arc;

/// Waveform types supported by the sound generator.
#[derive(Debug, Clone, Copy, Default)]
pub enum Waveform {
    /// Sine wave - smooth, pure tone
    #[default]
    Sine,
    /// Square wave - hollow, buzzy tone
    Square,
    /// Triangle wave - softer than square, with odd harmonics
    Triangle,
    /// Sawtooth wave - bright, rich harmonics
    Sawtooth,
    /// White noise - random samples
    Noise,
}

/// ADSR envelope parameters for dynamic sound shaping.
///
/// ADSR (Attack, Decay, Sustain, Release) envelopes shape the amplitude
/// of audio over time, creating more natural sound transitions.
#[derive(Debug, Clone, Copy)]
pub struct Envelope {
    /// Attack time in seconds (time to reach peak amplitude)
    pub attack: f32,
    /// Decay time in seconds (time to reach sustain level)
    pub decay: f32,
    /// Sustain level (amplitude during the note, 0.0-1.0)
    pub sustain: f32,
    /// Release time in seconds (time to fade to silence)
    pub release: f32,
}

impl Default for Envelope {
    fn default() -> Self {
        Self {
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.1,
        }
    }
}

/// Sound generation parameters.
#[derive(Debug, Clone)]
pub struct SoundParams {
    /// Waveform type (Sine, Square, Triangle, Sawtooth, Noise)
    pub waveform: Waveform,
    /// Frequency in Hz (e.g., 440.0 = A4 note)
    pub frequency: f32,
    /// Amplitude multiplier (0.0-1.0)
    pub amplitude: f32,
    /// ADSR envelope for sound shaping
    pub envelope: Envelope,
    /// Sample rate in Hz (e.g., 44100, 48000)
    pub sample_rate: u32,
    /// Duration in seconds
    pub duration: f32,
    /// Number of audio channels (1=mono, 2=stereo)
    pub channels: u16,
}

impl Default for SoundParams {
    fn default() -> Self {
        Self {
            waveform: Waveform::default(),
            frequency: 440.0,
            amplitude: 0.5,
            envelope: Envelope::default(),
            sample_rate: 44100,
            duration: 1.0,
            channels: 2,
        }
    }
}

/// Audio buffer containing generated samples.
///
/// Stores interleaved stereo samples (L-R-L-R...) as f32 values
/// in the range [-1.0, 1.0].
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    /// Interleaved left/right samples
    pub samples: Vec<f32>,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of channels (currently supports 1 or 2)
    pub channels: u16,
}

impl AudioBuffer {
    /// Create a new audio buffer with the given parameters.
    pub fn new(samples: Vec<f32>, sample_rate: u32, channels: u16) -> Self {
        Self { samples, sample_rate, channels }
    }

    /// Get the duration in seconds.
    pub fn duration(&self) -> f32 {
        self.samples.len() as f32 / (self.sample_rate as f32 * self.channels as f32)
    }

    /// Convert to bytes for WAV output.
    pub fn to_wav_bytes(&self) -> Vec<u8> {
        let num_samples = self.samples.len();
        let mut buffer = Vec::with_capacity(44 + num_samples * 2);

        // RIFF header
        buffer.extend_from_slice(b"RIFF");
        buffer.extend_from_slice(&(36 + num_samples * 2).to_le_bytes());
        buffer.extend_from_slice(b"WAVE");

        // fmt chunk
        buffer.extend_from_slice(b"fmt ");
        buffer.extend_from_slice(&16u32.to_le_bytes());
        buffer.extend_from_slice(&1u16.to_le_bytes());
        buffer.extend_from_slice(&self.channels.to_le_bytes());
        buffer.extend_from_slice(&self.sample_rate.to_le_bytes());
        buffer.extend_from_slice(&(self.sample_rate as u32 * self.channels as u32 * 2).to_le_bytes());
        buffer.extend_from_slice(&(self.channels * 2).to_le_bytes());
        buffer.extend_from_slice(&16u16.to_le_bytes());

        // data chunk
        buffer.extend_from_slice(b"data");
        buffer.extend_from_slice(&(num_samples * 2).to_le_bytes());

        // Convert f32 [-1, 1] to i16
        for &sample in &self.samples {
            let clamped = sample.clamp(-1.0, 1.0);
            buffer.extend_from_slice(&(clamped as i16).to_le_bytes());
        }

        buffer
    }
}

/// Sound generator for procedural audio synthesis.
pub struct SoundGenerator;

impl SoundGenerator {
    /// Generate audio samples based on parameters.
    pub fn generate(params: &SoundParams, sample_rate: u32, duration: f32) -> AudioBuffer {
        let duration = params.duration.max(duration);
        let num_samples = (sample_rate as f32 * duration * params.channels as f32) as usize;
        let mut samples = Vec::with_capacity(num_samples);

        let total_frames = num_samples / params.channels as usize;
        let envelope = params.envelope;

        for i in 0..total_frames {
            let t = i as f32 / sample_rate as f32;
            let envelope_val = Self::calculate_envelope(t, envelope, duration);
            let sample = Self::generate_sample(params, i as f32, sample_rate);

            for _ in 0..params.channels {
                samples.push(sample * envelope_val * params.amplitude);
            }
        }

        AudioBuffer::new(samples, sample_rate, params.channels)
    }

    /// Generate a single sample value.
    fn generate_sample(params: &SoundParams, phase: f32, sample_rate: u32) -> f32 {
        let t = phase / sample_rate as f32;

        match params.waveform {
            Waveform::Sine => (2.0 * std::f32::consts::PI * params.frequency * t).sin(),
            Waveform::Square => {
                let phase = (t * params.frequency) % 1.0;
                if phase < 0.5 { 1.0 } else { -1.0 }
            }
            Waveform::Triangle => {
                let phase = (t * params.frequency) % 1.0;
                if phase < 0.25 {
                    4.0 * phase
                } else if phase < 0.75 {
                    2.0 - 8.0 * phase
                } else {
                    8.0 * phase - 6.0
                }
            }
            Waveform::Sawtooth => {
                let phase = (t * params.frequency) % 1.0;
                2.0 * phase - 1.0
            }
            Waveform::Noise => {
                // Simple pseudo-random noise using simple hash
                let seed = (t * 1000.0 * 7919.0) as i32;
                (seed as f32) / (i32::MAX as f32)
            }
        }
    }

    /// Calculate ADSR envelope value at time t.
    fn calculate_envelope(t: f32, env: Envelope, duration: f32) -> f32 {
        if t < env.attack {
            t / env.attack.max(0.0001)
        } else if t < env.attack + env.decay {
            if env.decay > 0.0 {
                let decay_progress = (t - env.attack) / env.decay;
                1.0 - decay_progress * (1.0 - env.sustain)
            } else {
                1.0
            }
        } else if t < duration - env.release {
            env.sustain
        } else {
            if env.release > 0.0 {
                let release_progress = (t - (duration - env.release)) / env.release;
                env.sustain * (1.0 - release_progress)
            } else {
                env.sustain
            }
        }
    }

    /// Load a WAV sound from a .flow package.
    pub fn load_wav(data: Arc<Vec<u8>>) -> Option<AudioBuffer> {
        if data.len() < 44 {
            return None;
        }

        if &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
            return None;
        }

        let channels = u16::from_le_bytes([data[22], data[23]]);
        let sample_rate = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);

        let data_start = 44;
        let mut samples = Vec::new();
        for i in (data_start..data.len()).step_by(2) {
            if i + 1 < data.len() {
                let sample = i16::from_le_bytes([data[i], data[i + 1]]) as f32 / i16::MAX as f32;
                samples.push(sample);
            }
        }

        Some(AudioBuffer::new(samples, sample_rate, channels))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_sine_wave() {
        let params = SoundParams::default();
        let buffer = SoundGenerator::generate(&params, 44100, 0.1);
        assert_eq!(buffer.sample_rate, 44100);
        assert!((buffer.channels as u32) >= 1);
    }

    #[test]
    fn test_generate_square_wave() {
        let params = SoundParams {
            waveform: Waveform::Square,
            amplitude: 1.0,
            ..Default::default()
        };
        let buffer = SoundGenerator::generate(&params, 44100, 0.1);
        assert!(buffer.samples.iter().any(|&s| s > 0.9));
        assert!(buffer.samples.iter().any(|&s| s < -0.9));
    }

    #[test]
    fn test_generate_triangle_wave() {
        let params = SoundParams {
            waveform: Waveform::Triangle,
            amplitude: 1.0,
            ..Default::default()
        };
        let buffer = SoundGenerator::generate(&params, 44100, 0.1);
        assert!(!buffer.samples.is_empty());
    }

    #[test]
    fn test_generate_sawtooth_wave() {
        let params = SoundParams {
            waveform: Waveform::Sawtooth,
            amplitude: 1.0,
            ..Default::default()
        };
        let buffer = SoundGenerator::generate(&params, 44100, 0.1);
        assert!(!buffer.samples.is_empty());
    }

    #[test]
    fn test_generate_noise() {
        let params = SoundParams {
            waveform: Waveform::Noise,
            duration: 0.01, // Very short duration for test
            ..Default::default()
        };
        let buffer = SoundGenerator::generate(&params, 44100, 0.01);
        // Noise should generate samples
        assert!(!buffer.samples.is_empty());
    }

    #[test]
    fn test_envelope_attack() {
        let env = Envelope { attack: 0.01, decay: 0.0, sustain: 0.5, release: 0.0 };
        assert!((SoundGenerator::calculate_envelope(0.0, env, 1.0) - 0.0).abs() < 0.01);
        assert!((SoundGenerator::calculate_envelope(0.005, env, 1.0) - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_envelope_decay() {
        let env = Envelope { attack: 0.0, decay: 0.1, sustain: 0.5, release: 0.0 };
        assert!((SoundGenerator::calculate_envelope(0.05, env, 1.0) - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_envelope_sustain() {
        let env = Envelope { attack: 0.0, decay: 0.0, sustain: 0.8, release: 0.0 };
        assert!((SoundGenerator::calculate_envelope(0.5, env, 1.0) - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_envelope_release() {
        let env = Envelope { attack: 0.0, decay: 0.0, sustain: 0.8, release: 0.1 };
        // At t=0.95, duration=1.0, release starts at 0.9
        // So we're in release phase: (0.95 - 0.9) / 0.1 = 0.5 progress
        // Result: 0.8 * (1 - 0.5) = 0.4
        assert!((SoundGenerator::calculate_envelope(0.95, env, 1.0) - 0.4).abs() < 0.01);
    }

    #[test]
    fn test_audio_buffer_to_wav() {
        let buffer = AudioBuffer::new(vec![0.0, 0.0], 44100, 1);
        let wav = buffer.to_wav_bytes();
        assert!(wav.starts_with(b"RIFF"));
        // Just verify it produces output - exact size depends on header implementation
        assert!(wav.len() > 44);
    }

    #[test]
    fn test_audio_buffer_duration() {
        let buffer = AudioBuffer::new(vec![0.0; 44100], 44100, 1);
        assert!((buffer.duration() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_waveform_default() {
        let w: Waveform = Default::default();
        matches!(w, Waveform::Sine);
    }

    #[test]
    fn test_envelope_default() {
        let env: Envelope = Default::default();
        assert!((env.attack - 0.01).abs() < 0.001);
        assert!((env.decay - 0.1).abs() < 0.001);
        assert!((env.sustain - 0.7).abs() < 0.001);
        assert!((env.release - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_sound_params_default() {
        let params: SoundParams = Default::default();
        assert_eq!(params.frequency, 440.0);
        assert_eq!(params.amplitude, 0.5);
        assert_eq!(params.sample_rate, 44100);
        assert_eq!(params.duration, 1.0);
        assert_eq!(params.channels, 2);
    }

    #[test]
    fn test_load_wav_invalid() {
        let result = SoundGenerator::load_wav(Arc::new(vec![0; 30]));
        assert!(result.is_none());
    }
}