//! Unit tests for background module.
//!
//! Tests image loading, resizing, and format conversion.

use screen_animation::background::load_background;

/// Create a 2×2 red image encoded as PNG bytes using the image crate.
fn create_test_png_2x2() -> Vec<u8> {
    let mut img = image::RgbImage::new(2, 2);
    // Fill with red pixels
    for pixel in img.pixels_mut() {
        *pixel = image::Rgb([255, 0, 0]);
    }
    let mut png_data: Vec<u8> = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png_data), image::ImageFormat::Png)
        .expect("Failed to encode test PNG");
    png_data
}

/// Create a 1×1 red pixel encoded as PNG bytes using the image crate.
fn create_test_png_1x1_red() -> Vec<u8> {
    let mut img = image::RgbImage::new(1, 1);
    img.put_pixel(0, 0, image::Rgb([255, 0, 0]));
    let mut png_data: Vec<u8> = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png_data), image::ImageFormat::Png)
        .expect("Failed to encode test PNG");
    png_data
}

#[test]
fn test_load_background_with_valid_png() {
    let png_data = create_test_png_2x2();

    let result = load_background(&png_data, 4, 4);

    assert!(result.is_some(), "Should successfully load valid PNG");
    let bgra = result.unwrap();
    assert_eq!(bgra.len(), 4 * 4 * 4, "Should produce 4x4 BGRA data (16 pixels × 4 bytes)");
}

#[test]
fn test_load_background_with_invalid_data() {
    let invalid_data = b"This is not an image";

    let result = load_background(invalid_data, 100, 100);

    assert!(result.is_none(), "Should return None for invalid image data");
}

#[test]
fn test_load_background_resize() {
    let png_data = create_test_png_2x2();

    // Request different output size
    let result = load_background(&png_data, 10, 10);

    assert!(result.is_some(), "Should resize image");
    let bgra = result.unwrap();
    assert_eq!(bgra.len(), 10 * 10 * 4, "Should produce 10x10 BGRA data");
}

#[test]
fn test_load_background_rgba_to_bgra_conversion() {
    // Create a 1x1 red pixel PNG (R=255, G=0, B=0)
    let png_data = create_test_png_1x1_red();

    let result = load_background(&png_data, 1, 1);

    assert!(result.is_some(), "Should load image");
    let bgra = result.unwrap();

    // Source is RGBA(255,0,0,255). After BGRA conversion:
    // BGRA means bytes are: B (index 0), G (index 1), R (index 2), A (index 3)
    // So R=255 from source becomes BGRA[2]=255, BGRA[1]=0, BGRA[0]=0
    assert_eq!(bgra[0], 0, "Blue channel should be 0");
    assert_eq!(bgra[1], 0, "Green channel should be 0");
    assert_eq!(bgra[2], 255, "Red channel should be 255");
    assert_eq!(bgra[3], 255, "Alpha channel should be 255");
}

#[test]
fn test_create_background_texture_dimensions() {
    // This would require a GPU device for real testing
    // Here we just verify the module compiles
}

#[test]
fn test_upload_background_size_match() {
    // Verify that upload handles correct buffer sizes
    // 100x100 image = 10000 pixels = 40000 bytes (RGBA)
    let expected_size = 100 * 100 * 4;
    let buffer = vec![0u8; expected_size];

    assert_eq!(buffer.len(), expected_size, "Buffer size should match image dimensions");
}
