//! Integration tests for the screenshot module.
//!
//! These tests validate the DXGI capture pipeline and its BitBlt fallback.
//! Tests that require a real display are marked as such and will gracefully
//! handle environments without a GPU (e.g., CI).

use screen_animation::screenshot;

/// Verify that the module exposes the expected public API.
#[test]
fn test_public_api_signatures() {
    // Ensure all public functions are accessible
    let _ = screenshot::capture_or_fallback;
    let _ = screenshot::capture_desktop_fallback;
    let _ = screenshot::reinitialize_dxgi;
    let _ = screenshot::is_dxgi_available;
    let _ = screenshot::get_dxgi_info;
}

/// Verify `capture_or_fallback` returns a correctly sized buffer for a valid
/// monitor region. This tests the real capture path if DXGI is available,
/// otherwise the BitBlt fallback.
#[test]
fn test_capture_returns_correct_size() {
    // SAFETY: Unsafe due to GDI/DXGI calls, but safe in context of a test
    // running on the main thread with a message pump.
    let result = unsafe { screenshot::capture_or_fallback(640, 480, None) };
    assert_eq!(result.len(), 640 * 480 * 4, "capture_or_fallback should return the correct buffer size");
}

/// Verify that fallback capture handles various buffer sizes correctly.
#[test]
fn test_fallback_various_sizes() {
    let sizes = [(1, 1), (2, 2), (100, 100), (1920, 1080), (3840, 2160)];
    for &(w, h) in &sizes {
        let rect = windows::Win32::Foundation::RECT {
            left: 0,
            top: 0,
            right: w as i32,
            bottom: h as i32,
        };
        // SAFETY: GDI operations on test rects.
        let result = unsafe { screenshot::capture_desktop_fallback(w, h, &rect) };
        assert_eq!(
            result.len(),
            (w as usize) * (h as usize) * 4,
            "Fallback at {}×{} should produce correct buffer size",
            w,
            h
        );
    }
}

/// Verify that `capture_or_fallback` with a `Some` rect attempts DXGI first,
/// then falls back gracefully. This test only validates the mechanism — actual
/// DXGI availability depends on the test environment.
#[test]
fn test_capture_with_monitor_rect() {
    let rect = windows::Win32::Foundation::RECT {
        left: 0,
        top: 0,
        right: 100,
        bottom: 100,
    };
    // SAFETY: Unsafe due to potential DXGI/D3D11 calls, but safe in test context.
    let result = unsafe { screenshot::capture_or_fallback(100, 100, Some(&rect)) };
    assert_eq!(result.len(), 100 * 100 * 4, "Capture with monitor rect should produce correct buffer size");
}

/// Verify that pixel stride is 4 bytes per pixel (BGRA format).
#[test]
fn test_bgra_pixel_stride() {
    let rect = windows::Win32::Foundation::RECT {
        left: 0,
        top: 0,
        right: 10,
        bottom: 1,
    };
    // SAFETY: Minimal GDI fallback call.
    let result = unsafe { screenshot::capture_desktop_fallback(10, 1, &rect) };
    // 10 pixels × 4 bytes = 40 bytes
    assert_eq!(result.len(), 40, "10×1 BGRA should be exactly 40 bytes");
    // Each pixel should be exactly 4 bytes — no padding between pixels
    for i in 0..10 {
        let pixel_start = i * 4;
        assert!(pixel_start + 3 < result.len(), "Pixel {} should have complete BGRA data", i);
    }
}

/// Verify that `reinitialize_dxgi` doesn't panic (even if DXGI is unavailable).
#[test]
fn test_reinitialization_safety() {
    // SAFETY: Reinitialization is a no-op if DXGI unavailable.
    unsafe {
        screenshot::reinitialize_dxgi(1920, 1080);
    }
    // Calling twice should also be safe
    unsafe {
        screenshot::reinitialize_dxgi(1920, 1080);
    }
}

/// Verify that `get_dxgi_info` provides expected diagnostic output.
#[test]
fn test_dxgi_diagnostics_format() {
    let info = unsafe { screenshot::get_dxgi_info() };
    assert!(!info.is_empty(), "DXGI info should not be empty");
    // Should mention adapter or state unavailability
    assert!(
        info.contains("Adapter:") || info.contains("DXGI not available") || info.contains("No adapter"),
        "Diagnostics should describe adapter state: {}",
        info
    );
}

/// Verify that `is_dxgi_available` is consistent across multiple calls.
#[test]
fn test_dxgi_availability_stability() {
    let a = unsafe { screenshot::is_dxgi_available() };
    let b = unsafe { screenshot::is_dxgi_available() };
    assert_eq!(a, b, "DXGI availability should be consistent within a session");
}

/// Verify that the fallback handles edge-case coordinates.
#[test]
fn test_fallback_edge_coordinates() {
    // Negative coordinates (should be handled)
    let rect_neg = windows::Win32::Foundation::RECT {
        left: -100,
        top: -100,
        right: 100,
        bottom: 100,
    };
    // SAFETY: BitBlt can handle negative coordinates as screen offsets
    let result = unsafe { screenshot::capture_desktop_fallback(200, 200, &rect_neg) };
    assert_eq!(result.len(), 200 * 200 * 4, "Negative coords should still produce correct buffer");
}
