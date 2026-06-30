//! # Audio Module
//!
//! Audio playback and real-time synthesis.
//!
//! - `player`: Stream/sink management via rodio
//! - `synthesis`: Procedural audio generation (sine waves, noise, envelopes)

pub mod player;
pub mod synthesis;

pub use player::init_audio;
pub use synthesis::{generate_sine_wave, generate_white_noise, apply_envelope, generate_beep};