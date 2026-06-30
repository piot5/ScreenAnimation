//! Integration tests for the engine module.

use screen_animation::engine::Uniforms;
use screen_animation::core::uniform::UniformManager;
use wgpu::{Device, Queue};
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::automock;

    #[automock]
    trait MockDevice {
        fn create_buffer(&self, _desc: &wgpu::BufferDescriptor) -> wgpu::Buffer;
    }

    #[automock]
    trait MockQueue {
        fn write_buffer(&self, _buffer: &wgpu::Buffer, _offset: wgpu::BufferAddress, _data: &[u8]);
    }

    /// Test Uniforms struct size and alignment.
    ///
    /// This is critical for WGSL uniform buffer compatibility.
    #[test]
    fn test_uniforms_size() {
        // Uniforms must be exactly 64 bytes for WGSL compatibility
        assert_eq!(std::mem::size_of::<Uniforms>(), 64);
    }

    #[test]
    fn test_uniforms_alignment() {
        // Ensure proper alignment for GPU buffer upload
        let alignment = std::mem::align_of::<Uniforms>();
        assert!(alignment >= 4, "Alignment should be at least 4 bytes, got {}", alignment);
    }

    #[test]
    fn test_uniforms_default_values() {
        let uniforms = Uniforms {
            mouse: [0.0, 0.0],
            offset: [0.0, 0.0],
            scale: 1.0,
            time: 0.0,
            _padding: [0.0; 2],
            logic_params: [0.0; 4],
            feature_flags: [0.0; 4],
        };

        assert_eq!(uniforms.mouse, [0.0, 0.0]);
        assert_eq!(uniforms.scale, 1.0);
        assert_eq!(uniforms.time, 0.0);
        assert_eq!(uniforms.logic_params, [0.0; 4]);
        assert_eq!(uniforms.feature_flags, [0.0; 4]);
    }

    #[test]
    fn test_uniforms_parameter_validation() {
        // Test NaN handling
        let uniforms = Uniforms {
            mouse: [0.5, 0.5],
            offset: [0.0, 0.0],
            scale: 1.0,
            time: 0.0,
            _padding: [0.0; 2],
            logic_params: [1.0, 2.0, 3.0, 4.0],
            feature_flags: [1.0, 0.0, 1.0, 0.0],
        };

        assert!(!uniforms.logic_params.iter().any(|&x| x.is_nan()));
        assert!(!uniforms.feature_flags.iter().any(|&x| x.is_nan()));
    }

    /// Test for UniformManager buffer creation and writing
    #[test]
    fn test_uniform_manager_buffer_creation() {
        // Mock device and queue for testing
        let mock_device = MockDevice::new();
        let mock_queue = MockQueue::new();

        // Create a mock device and queue
        let device = mock_device;
        let queue = mock_queue;

        // Create a buffer using UniformManager
        let buffer = UniformManager::create_buffer(&device, "Test Buffer");

        // Verify buffer size matches Uniforms size
        assert_eq!(buffer.size(), std::mem::size_of::<Uniforms>() as u64);
    }

    /// Test for UniformManager buffer writing
    #[test]
    fn test_uniform_manager_write_buffer() {
        // Mock device and queue for testing
        let mock_device = MockDevice::new();
        let mock_queue = MockQueue::new();

        // Create a mock device and queue
        let device = mock_device;
        let queue = mock_queue;

        // Create a buffer using UniformManager
        let buffer = UniformManager::create_buffer(&device, "Test Buffer");

        // Create a sample Uniforms struct
        let uniforms = Uniforms {
            mouse: [0.5, 0.5],
            offset: [0.0, 0.0],
            scale: 1.0,
            time: 1.0,
            _padding: [0.0; 2],
            logic_params: [1.0, 2.0, 3.0, 4.0],
            feature_flags: [1.0, 0.0, 1.0, 0.0],
        };

        // Write the uniforms to the buffer
        UniformManager::write_buffer(&queue, &buffer, &uniforms);
    }
}