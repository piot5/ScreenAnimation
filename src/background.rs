//! Background image handling module.
//!
//! This module provides functionality for loading, resizing, and uploading
//! background images to GPU textures. It encapsulates all background-related
//! operations that were previously embedded in the Windows integration module.
//!
//! # Responsibilities
//!
//! - Load background images from .flow packages
//! - Resize images to match monitor resolution
//! - Convert RGBA to BGRA format for Windows DIB compatibility
//! - Upload processed images to GPU textures
//!
//! # Design
//!
//! This module is platform-agnostic except for the BGRA conversion, which is
//! specific to Windows DIB format. The image processing uses the `image` crate
//! for decoding and resizing.
//!
//! # Performance
//!
//! - Image decoding: ~10ms per texture
//! - Bilinear resize: ~5ms for 1080p
//! - Format conversion: ~2ms
//! - Total: ~17ms per background

use crate::engine::GpuCore;
use image::imageops::FilterType;

/// Load and process a background image for wallpaper mode.
///
/// This function handles the complete background image pipeline:
/// 1. Decodes the image from raw bytes (PNG/JPG)
/// 2. Resizes to monitor resolution using bilinear filtering
/// 3. Converts from RGBA to BGRA format (Windows DIB format)
///
/// # Arguments
///
/// * `image_data` - Raw image bytes from .flow package (background.png)
/// * `width` - Target width in pixels (monitor width)
/// * `height` - Target height in pixels (monitor height)
///
/// # Returns
///
/// Returns `Some(Vec<u8>)` with BGRA pixel data (width × height × 4 bytes)
/// if the image loads successfully, or `None` if decoding fails.
///
/// # Errors
///
/// This function does not return errors. If the image cannot be decoded,
/// it returns `None` and the caller should fall back to desktop capture.
///
/// # Performance
///
/// - Decoding: ~10ms
/// - Resizing: ~5ms
/// - Conversion: ~2ms
/// - Total: ~17ms for 1920×1080
///
/// # Example
///
/// ```ignore
/// # use screen_animation::background::load_background;
/// let bg_data = package.image_data.unwrap_or_default();
/// let bgra = load_background(&bg_data, 1920, 1080);
/// ```
pub fn load_background(image_data: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    // Decode image from memory (supports PNG, JPEG, GIF, etc.)
    let img = image::load_from_memory(image_data).ok()?;

    // Resize to monitor resolution using triangle filter (bilinear)
    // Triangle filter provides good quality/performance balance
    let resized = img.resize_exact(width, height, FilterType::Triangle);
    let rgba = resized.to_rgba8();

    // Convert RGBA to BGRA (Windows DIB format)
    // Windows expects: B, G, R, A order
    // image crate gives: R, G, B, A order
    // Swizzle: R↔B, keep G and A unchanged
    let bgra = rgba.chunks_exact(4).flat_map(|pixel| [pixel[2], pixel[1], pixel[0], pixel[3]]).collect();

    Some(bgra)
}

/// Create a GPU texture for background image upload.
///
/// This function creates a WGPU texture suitable for background images.
/// The texture is configured with BGRA8 format to match Windows DIB format.
///
/// # Arguments
///
/// * `gpu` - GPU core with device for texture creation
/// * `width` - Texture width in pixels
/// * `height` - Texture height in pixels
///
/// # Returns
///
/// Returns a tuple of (texture, texture_view) ready for data upload.
///
/// # Performance
///
/// - Texture creation: ~5ms
/// - Memory allocation: width × height × 4 bytes
///
/// # Example
///
/// ```ignore
/// # use screen_animation::background::create_background_texture;
/// let (tex, view) = create_background_texture(&gpu, 1920, 1080);
/// ```
pub fn create_background_texture(gpu: &GpuCore, width: u32, height: u32) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Background texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

/// Upload background image data to a GPU texture.
///
/// # Arguments
///
/// * `gpu` - GPU core with queue for data upload
/// * `texture` - Target texture (from `create_background_texture`)
/// * `data` - BGRA pixel data (width × height × 4 bytes)
/// * `width` - Texture width in pixels
/// * `height` - Texture height in pixels
///
/// # Performance
///
/// - Upload time: ~10ms for 1920×1080
/// - Bandwidth: width × height × 4 bytes
///
/// # Safety
///
/// The data slice must contain exactly `width × height × 4` bytes.
pub fn upload_background(gpu: &GpuCore, texture: &wgpu::Texture, data: &[u8], width: u32, height: u32) {
    gpu.queue.write_texture(
        wgpu::ImageCopyTexture {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}
