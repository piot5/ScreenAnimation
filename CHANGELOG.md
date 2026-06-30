# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial project structure with WGPU-based rendering
- V1 simple animation mode with mouse interaction
- V2 sequence mode with timed steps
- Multi-monitor support
- Audio playback via rodio
- ZIP-based .flow package format
- Builder tool for creating animation packages

### Security
- Added path traversal protection in ZIP extraction
- Added package size limits (100MB)
- Added texture dimension limits (8192×8192)
- Added audio file count limits (32 files)

### Documentation
- Added comprehensive .flow format specification
- Added architecture documentation
- Added building guide
- Added MIT License

### Fixed
- Corrected TOML syntax in documentation examples
- Fixed missing Drop implementation for MonitorWindow
- Added proper error handling for Windows API calls

## [0.1.0] - 2024-01-01

### Added
- Initial release
- Basic GPU-accelerated wallpaper engine
- Support for custom WGSL shaders
- Desktop background capture
- Mouse-reactive animations