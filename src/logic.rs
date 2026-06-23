//! Logic engine module.
//!
//! This module calculates per-frame uniform buffer values from the animation configuration.
//! It translates user-defined parameters and runtime state (time, mouse position) into
//! the `Uniforms` structure that gets uploaded to the GPU each frame.
//!
//! # Responsibilities
//!
//! - Track elapsed time since animation start
//! - Read logic parameters (p1-p4) from config.toml
//! - Read feature flags (f1-f4) from config.toml
//! - Combine with runtime mouse position to produce uniform buffer
//!
//! # Design
//!
//! `LogicEngine` is intentionally stateless except for `start_time`.
//! This makes it easy to test and reason about - given the same inputs,
//! it always produces the same outputs.

use crate::engine::Uniforms;
use crate::loader::FlowPackage;
use std::time::Instant;

/// Computes uniform values from flow config at each frame.
///
/// This is the bridge between the high-level animation configuration (config.toml)
/// and the low-level GPU uniform buffer. It runs every frame (60 times per second)
/// and produces the data that drives shader animations.
///
/// # State
///
/// - `start_time`: Reference point for `time` uniform (seconds since engine creation)
///
/// # Thread Safety
///
/// `LogicEngine` is cheap to clone (just an `Instant`), but typically used on
/// the main thread. It could be made `Send` if needed for multi-threaded rendering.
pub struct LogicEngine {
    /// Reference time for calculating elapsed animation time
    pub start_time: Instant,
}

impl LogicEngine {
    /// Create a new logic engine with current time as start.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let logic = LogicEngine::new();
    /// // Animation starts counting from now
    /// ```
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }

    /// Calculate uniform buffer values for one frame.
    ///
    /// # Arguments
    ///
    /// * `flow` - Loaded animation package with config and assets
    /// * `mouse_rel` - Normalized mouse position (0.0 to 1.0) relative to window
    ///
    /// # Returns
    ///
    /// A fully populated `Uniforms` structure ready for GPU upload.
    ///
    /// # Performance
    ///
    /// - Hash map lookups: 8× (4 logic params + 4 feature flags)
    /// - Time calculation: 1× `Instant::elapsed()`
    /// - Total: <1μs per call
    ///
    /// # Uniform Buffer Layout
    ///
    /// ```text
    /// Offset 0:  mouse.x, mouse.y          (vec2<f32>)
    /// Offset 8:  offset.x, offset.y        (vec2<f32>)
    /// Offset 16: scale                      (f32)
    /// Offset 20: time                       (f32)
    /// Offset 32: logic_params[0..4]         (vec4<f32>) - aligned to 16 bytes
    /// Offset 48: feature_flags[0..4]        (vec4<f32>) - aligned to 16 bytes
    /// Total: 64 bytes
    /// ```
    ///
    /// Note: The layout above shows conceptual organization. Actual memory layout
    /// is determined by `#[repr(C)]` on the `Uniforms` struct and may include
    /// padding for alignment.
    pub fn update(&self, flow: &FlowPackage, mouse_rel: [f32; 2]) -> Uniforms {
        // Calculate elapsed time since animation start
        // Used for time-based shader effects (oscillations, progress, etc.)
        let elapsed = self.start_time.elapsed().as_secs_f32();

        Uniforms {
            // Mouse position in normalized coordinates (0-1)
            // Calculated by caller from raw cursor position / window size
            mouse: mouse_rel,
            // Offset: currently mirrors mouse position
            // Reserved for future pan/scroll functionality
            offset: mouse_rel,
            // Uniform scale factor
            // Currently hardcoded to 1.0, could be animated via config
            scale: 1.0,
            // Elapsed time in seconds (floating point for smooth animation)
            // Resets when LogicEngine is recreated
            time: elapsed,
            // User-defined logic parameters from [p1], [p2], [p3], [p4] in config.toml
            // These are exposed to shaders as vec4<f32> for customization
            // Examples: animation speed, color intensity, effect strength
            logic_params: [
                flow.val("p1", 0.0),
                flow.val("p2", 0.0),
                flow.val("p3", 0.0),
                flow.val("p4", 0.0),
            ],
            // Feature flags from [f1], [f2], [f3], [f4] in config.toml
            // Converted from bool to f32 (1.0 = true, 0.0 = false)
            // Used in shaders to enable/disable effects conditionally
            feature_flags: [
                if flow.feature("f1") { 1.0 } else { 0.0 },
                if flow.feature("f2") { 1.0 } else { 0.0 },
                if flow.feature("f3") { 1.0 } else { 0.0 },
                if flow.feature("f4") { 1.0 } else { 0.0 },
            ],
        }
    }
}
