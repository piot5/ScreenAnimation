//! Audio playback initialization.
//!
//! Creates an audio output stream and a rodio Sink for non-blocking
//! sound playback. Uses cpal for stream creation and rodio for
//! audio decoding and playback.

use anyhow::Context;
use rodio::{OutputStream, Sink};

/// Initialize audio output stream and playback sink.
///
/// # Returns
///
/// Tuple of `(OutputStream, Sink)`. Keep the `OutputStream` alive for
/// the application's lifetime to prevent audio device disconnection.
///
/// # Errors
///
/// Returns error if no audio output device is available (non-fatal –
/// animation can run without sound).
pub fn init_audio() -> anyhow::Result<(OutputStream, Sink)> {
    let (_stream, handle) = OutputStream::try_default()
        .context("Failed to initialize audio output. Animation will run without sound.")?;
    let sink = Sink::try_new(&handle)
        .context("Failed to create audio sink")?;
    Ok((_stream, sink))
}