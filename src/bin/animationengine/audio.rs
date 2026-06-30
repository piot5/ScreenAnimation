//! Audio initialization module.
//!
//! Provides audio stream and sink setup for the animation engine.
//! Uses cpal for stream creation and rodio for audio playback.

use anyhow::Context;
use rodio::{OutputStream, Sink};
use screen_animation::loader::FlowPackage;

/// Initialize audio system with optional preloaded sounds.
///
/// # Arguments
///
/// * `_flow` - Loaded flow package (may contain .wav files)
///
/// # Returns
///
/// Tuple of (OutputStream, Sink) for audio playback.
/// Keep OutputStream alive for the duration of the application.
///
/// # Errors
///
/// Returns error if audio output device cannot be found or initialized.
/// This is non-fatal — animation will run without sound.
pub fn init_audio(_flow: &FlowPackage) -> anyhow::Result<(OutputStream, Sink)> {
    let (_stream, handle) = OutputStream::try_default()
        .context("Failed to initialize audio output. Animation will run without sound.")?;
    let sink = Sink::try_new(&handle)
        .context("Failed to create audio sink")?;
    Ok((_stream, sink))
}