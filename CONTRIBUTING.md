# Contributing to ScreenAnimation

Thank you for your interest in contributing! This document outlines the process and guidelines.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/ScreenAnimation.git`
3. Create a feature branch: `git checkout -b feature/my-feature`
4. Make your changes
5. Push and submit a pull request

## Code Guidelines

### Rust Style

- Follow [Rust naming conventions](https://rust-lang.github.io/api-guidelines/naming.html)
- Use `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Add tests for new functionality

```bash
cargo fmt --all
cargo clippy --all-targets
cargo test --all
```

### Shader Guidelines

- WGSL shaders should follow [WebGPU best practices](https://www.w3.org/TR/WGSL/)
- Use meaningful variable names
- Comment complex calculations
- Optimize for mobile GPUs when possible

## Contributing Animations

Want to contribute a `.flow` animation?

1. Create your animation directory with `config.toml`, `shader.wgsl`, and optional assets
2. Test locally: `./target/release/animationengine Wallpaper your_animation.flow`
3. Create a PR with:
   - Your `.flow` file (or compressed archive if >100MB)
   - Brief description of the effect
   - Any special requirements (GPU tier, OS)

## Pull Request Process

1. **Describe your change**: Explain what and why
2. **Test thoroughly**: Ensure all tests pass
3. **Update docs**: If applicable, update README or TUTORIAL
4. **Keep commits clean**: One feature per commit
5. **Respond to feedback**: Maintainers may suggest changes

## Reporting Issues

Before submitting an issue, please:

1. Check existing issues (closed or open)
2. Try reproducing on latest `master`
3. Include:
   - OS version (Windows 10/11, Linux distro, etc.)
   - GPU model
   - Steps to reproduce
   - Error message or screenshot

## Code of Conduct

- Be respectful and inclusive
- No harassment or discrimination
- Constructive feedback only
- Respect intellectual property

## Questions?

Open a [Discussion](https://github.com/piot5/ScreenAnimation/discussions) or ask in an Issue.

Thank you for contributing! 🎨
