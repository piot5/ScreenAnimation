//! Performance benchmarks for ScreenAnimation.
//!
//! Measures critical performance metrics:
//! - Logic engine uniform calculation (real LogicEngine)
//! - Sound generation (real audio synthesis)
//! - Memory allocation patterns
//! - Screenshot capture (DXGI & BitBlt)
//! - Staging texture readback

use criterion::{criterion_group, criterion_main, Criterion};
use screen_animation::loader::FlowPackage;
use screen_animation::logic::LogicEngine;
use screen_animation::soundgenerator::{apply_envelope, generate_sine_wave, generate_white_noise};

// Real logic engine benchmark
fn bench_logic_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("logic");

    // Load a known .flow package for benchmarking
    let flow = FlowPackage::load("examples/screenshot.flow").unwrap_or_else(|_| {
        FlowPackage::load("assets/animation1.flow").expect("At least one .flow file must exist for benchmarking")
    });

    let engine = LogicEngine::new(&flow);

    group.bench_function("uniforms_calculation", |b| {
        b.iter(|| {
            let mouse = [0.5, 0.5];
            let uniforms = engine.update(&flow, mouse);
            // Prevent compiler from optimizing away
            criterion::black_box(uniforms);
        })
    });

    group.finish();
}

// Sound generation benchmarks
fn bench_sound_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("sound");

    group.bench_function("sine_wave_1s_44kHz", |b| b.iter(|| generate_sine_wave(440.0, 1.0, 44100)));

    group.bench_function("sine_wave_0_1s_48kHz", |b| b.iter(|| generate_sine_wave(880.0, 0.1, 48000)));

    group.bench_function("white_noise_0_5s_44kHz", |b| b.iter(|| generate_white_noise(0.5, 44100)));

    group.bench_function("envelope_apply", |b| {
        b.iter(|| {
            let mut samples = vec![1.0; 1000];
            apply_envelope(&mut samples, 0.1, 0.1, 0.7, 0.1, 44100);
            samples
        })
    });

    group.finish();
}

// Memory allocation benchmarks
fn bench_memory_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory");

    group.bench_function("vec_u8_1920x1080x4", |b| b.iter(|| vec![0u8; 1920 * 1080 * 4]));

    group.bench_function("vec_f32_44100_samples", |b| b.iter(|| vec![0.0f32; 44100]));

    group.bench_function("vec_u8_4K_frame", |b| b.iter(|| vec![0u8; 3840 * 2160 * 4]));

    group.finish();
}

// Screenshot capture benchmarks (DXGI and BitBlt)
fn bench_screenshot_capture(c: &mut Criterion) {
    let mut group = c.benchmark_group("screenshot");

    // Benchmark: fallback capture at various sizes (always available)
    group.bench_function("fallback_640x480", |b| {
        let rect = windows::Win32::Foundation::RECT {
            left: 0,
            top: 0,
            right: 640,
            bottom: 480,
        };
        b.iter(|| {
            // SAFETY: GDI fallback capture in benchmark context
            let pixels = unsafe { screen_animation::screenshot::capture_desktop_fallback(640, 480, &rect) };
            criterion::black_box(pixels);
        })
    });

    group.bench_function("fallback_1920x1080", |b| {
        let rect = windows::Win32::Foundation::RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        b.iter(|| {
            let pixels = unsafe { screen_animation::screenshot::capture_desktop_fallback(1920, 1080, &rect) };
            criterion::black_box(pixels);
        })
    });

    group.bench_function("or_fallback_640x480", |b| {
        b.iter(|| {
            // Without monitor_rect, this uses the fallback path
            let pixels = unsafe { screen_animation::screenshot::capture_or_fallback(640, 480, None) };
            criterion::black_box(pixels);
        })
    });

    group.finish();
}

// DXGI availability check benchmark
fn bench_dxgi_diagnostics(c: &mut Criterion) {
    let mut group = c.benchmark_group("dxgi");

    group.bench_function("availability_check", |b| {
        b.iter(|| {
            // SAFETY: COM initialized for benchmark
            let available = unsafe { screen_animation::screenshot::is_dxgi_available() };
            criterion::black_box(available);
        })
    });

    group.bench_function("get_info", |b| {
        b.iter(|| {
            let info = unsafe { screen_animation::screenshot::get_dxgi_info() };
            criterion::black_box(info);
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_logic_update,
    bench_sound_generation,
    bench_memory_allocation,
    bench_screenshot_capture,
    bench_dxgi_diagnostics,
);
criterion_main!(benches);
