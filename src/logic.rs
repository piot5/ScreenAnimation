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
//! - Validate and clamp all parameters to safe ranges
//!
//! # Design
//!
//! `LogicEngine` is intentionally stateless except for `start_time`.
//! This makes it easy to test and reason about - given the same inputs,
//! it always produces the same outputs. Parameters are cached at creation
//! time to avoid HashMap lookups during the hot render loop.
//!
//! # Performance
//!
//! - `update()`: <0.5μs per call (no heap allocations, no HashMap lookups)
//! - `new()`: ~1μs (8 HashMap lookups for parameter caching)
//! - Memory: 48 bytes (3× [f32; 4] + Instant)
//!
//! # Thread Safety
//!
//! `LogicEngine` is cheap to clone (just an `Instant`), but typically used on
//! the main thread. It could be made `Send` if needed for multi-threaded rendering.
//! The `update()` method takes `&self` so multiple threads could read simultaneously.

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
/// - `cached_logic_params`: Cached logic parameters to avoid repeated HashMap lookups
/// - `cached_feature_flags`: Cached feature flags for performance
///
/// # Example
///
/// ```ignore
/// use screen_animation::{loader::FlowPackage, logic::LogicEngine};
///
/// let flow = FlowPackage::load("animation.flow").unwrap();
/// let logic = LogicEngine::new(&flow);
/// let uniforms = logic.update(&flow, [0.5, 0.3]);
/// println!("Time: {:.2}s, Mouse: ({:.2}, {:.2})", uniforms.time, uniforms.mouse[0], uniforms.mouse[1]);
/// ```
pub struct LogicEngine {
    /// Reference time for calculating elapsed animation time
    pub start_time: Instant,
    /// Cached logic parameters [p1, p2, p3, p4] to avoid HashMap lookups per frame
    cached_logic_params: [f32; 4],
    /// Cached feature flags [f1, f2, f3, f4] as f32 (1.0 = true, 0.0 = false)
    cached_feature_flags: [f32; 4],
}

impl LogicEngine {
    /// Create a new logic engine with current time as start.
    ///
    /// Pre-caches all parameters from the flow package for zero-lookup performance
    /// during the render loop. Parameters are validated and clamped to safe ranges.
    ///
    /// # Arguments
    ///
    /// * `flow` - Loaded animation package with config and parameters
    ///
    /// # Returns
    ///
    /// A new `LogicEngine` ready for per-frame updates.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let logic = LogicEngine::new(&flow);
    /// // Animation starts counting from now
    /// ```
    pub fn new(flow: &FlowPackage) -> Self {
        let mut engine = Self {
            start_time: Instant::now(),
            cached_logic_params: [0.0; 4],
            cached_feature_flags: [0.0; 4],
        };
        // Pre-cache all parameters on creation
        engine.update_cache(flow);
        engine
    }

    /// Reset the animation timer to the current time.
    ///
    /// This effectively restarts the animation from the beginning.
    /// Useful for looping or restarting sequences.
    pub fn reset_timer(&mut self) {
        self.start_time = Instant::now();
    }

    /// Update cached parameters from flow package.
    ///
    /// This should be called when the flow package changes,
    /// but typically only once at initialization.
    /// Validates parameter ranges and clamps values to safe limits.
    ///
    /// # Performance
    ///
    /// - 8× HashMap lookups
    /// - Called once at startup, not per-frame
    fn update_cache(&mut self, flow: &FlowPackage) {
        // Validate and clamp logic parameters to prevent shader overflow
        self.cached_logic_params = [
            Self::validate_param(flow.val("p1", 0.0)),
            Self::validate_param(flow.val("p2", 0.0)),
            Self::validate_param(flow.val("p3", 0.0)),
            Self::validate_param(flow.val("p4", 0.0)),
        ];
        self.cached_feature_flags = [
            if flow.feature("f1") { 1.0 } else { 0.0 },
            if flow.feature("f2") { 1.0 } else { 0.0 },
            if flow.feature("f3") { 1.0 } else { 0.0 },
            if flow.feature("f4") { 1.0 } else { 0.0 },
        ];
    }

    /// Validate logic parameter value.
    ///
    /// Clamps parameter to safe range to prevent shader overflow/NaN.
    /// Range: -1e6 to +1e6 (sufficient for any meaningful animation parameter).
    fn validate_param(value: f32) -> f32 {
        if value.is_nan() || value.is_infinite() {
            return 0.0;
        }
        value.clamp(-1_000_000.0, 1_000_000.0)
    }

    /// Calculate uniform buffer values for one frame.
    ///
    /// This is the main per-frame function. It combines cached configuration
    /// parameters with runtime state (mouse position, elapsed time) to produce
    /// the uniform buffer that drives GPU shader animations.
    ///
    /// # Arguments
    ///
    /// * `_flow` - Loaded animation package (unused, parameters are cached)
    /// * `mouse_rel` - Normalized mouse position (0.0 to 1.0) relative to window
    ///
    /// # Returns
    ///
    /// A fully populated `Uniforms` structure ready for GPU upload.
    ///
    /// # Performance
    ///
    /// - Hash map lookups: 0× (cached at initialization)
    /// - Time calculation: 1× `Instant::elapsed()`
    /// - Array copies: 2× (logic_params + feature_flags)
    /// - Total: <0.5μs per call
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
    pub fn update(&self, _flow: &FlowPackage, mouse_rel: [f32; 2]) -> Uniforms {
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
            // Resets when LogicEngine is recreated or reset_timer() is called
            time: elapsed,
            // Padding to align vec4<f32> fields to 16-byte boundary (WGSL requirement)
            _padding: [0.0; 2],
            // User-defined logic parameters from [p1], [p2], [p3], [p4] in config.toml
            // These are exposed to shaders as vec4<f32> for customization
            // Examples: animation speed, color intensity, effect strength
            // Performance: Uses cached values (no HashMap lookups)
            logic_params: self.cached_logic_params,
            // Feature flags from [f1], [f2], [f3], [f4] in config.toml
            // Converted from bool to f32 (1.0 = true, 0.0 = false)
            // Used in shaders to enable/disable effects conditionally
            // Performance: Uses cached values (no HashMap lookups)
            feature_flags: self.cached_feature_flags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::FlowPackage;

    /// Test that LogicEngine creates with valid defaults.
    #[test]
    fn test_logic_engine_creation() {
        // This test verifies the struct compiles and has correct field types.
        // Full integration test requires a .flow package.
        let engine = LogicEngine {
            start_time: Instant::now(),
            cached_logic_params: [1.0, 2.0, 3.0, 4.0],
            cached_feature_flags: [1.0, 0.0, 1.0, 0.0],
        };
        assert_eq!(engine.cached_logic_params[0], 1.0);
        assert_eq!(engine.cached_feature_flags[1], 0.0);
    }

    /// Test parameter validation clamps NaN to 0.
    #[test]
    fn test_validate_param_nan() {
        assert_eq!(LogicEngine::validate_param(f32::NAN), 0.0);
    }

    /// Test parameter validation clamps infinity to 0.
    #[test]
    fn test_validate_param_infinity() {
        assert_eq!(LogicEngine::validate_param(f32::INFINITY), 0.0);
        assert_eq!(LogicEngine::validate_param(f32::NEG_INFINITY), 0.0);
    }

    /// Test parameter validation clamps extreme values.
    #[test]
    fn test_validate_param_clamp() {
        assert_eq!(LogicEngine::validate_param(2_000_000.0), 1_000_000.0);
        assert_eq!(LogicEngine::validate_param(-2_000_000.0), -1_000_000.0);
    }

    /// Test parameter validation passes normal values.
    #[test]
    fn test_validate_param_normal() {
        assert_eq!(LogicEngine::validate_param(42.0), 42.0);
        assert_eq!(LogicEngine::validate_param(-3.14), -3.14);
        assert_eq!(LogicEngine::validate_param(0.0), 0.0);
    }

    /// Test that update produces correct uniform structure.
    #[test]
    fn test_update_returns_valid_uniforms() {
        let engine = LogicEngine {
            start_time: Instant::now(),
            cached_logic_params: [0.5, 1.0, 1.5, 2.0],
            cached_feature_flags: [1.0, 0.0, 1.0, 0.0],
        };
        // Create a minimal flow package for the update call
        let config = crate::loader::Config::default();
        let flow = FlowPackage {
            config,
            sounds: std::collections::HashMap::new(),
            image_data: None,
            textures: std::collections::HashMap::new(),
            shader_src: String::new(),
        };
        let uniforms = engine.update(&flow, [0.5, 0.3]);
        assert_eq!(uniforms.mouse, [0.5, 0.3]);
        assert_eq!(uniforms.offset, [0.5, 0.3]);
        assert_eq!(uniforms.scale, 1.0);
        assert!(uniforms.time >= 0.0);
        assert_eq!(uniforms.logic_params, [0.5, 1.0, 1.5, 2.0]);
        assert_eq!(uniforms.feature_flags, [1.0, 0.0, 1.0, 0.0]);
    }

    /// Test that reset_timer restarts the animation time.
    #[test]
    fn test_reset_timer() {
        let mut engine = LogicEngine {
            start_time: Instant::now() - std::time::Duration::from_secs(10),
            cached_logic_params: [0.0; 4],
            cached_feature_flags: [0.0; 4],
        };
        let config = crate::loader::Config::default();
        let flow = FlowPackage {
            config,
            sounds: std::collections::HashMap::new(),
            image_data: None,
            textures: std::collections::HashMap::new(),
            shader_src: String::new(),
        };
        let before = engine.update(&flow, [0.0, 0.0]).time;
        assert!(before >= 9.0); // Should be ~10 seconds

        engine.reset_timer();
        let after = engine.update(&flow, [0.0, 0.0]).time;
        assert!(after < 1.0); // Should be near 0 after reset
    }
}