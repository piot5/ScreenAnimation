//! CLI argument parsing module.
//!
//! Handles command-line argument parsing using clap derive macros.

use clap::{Parser, Subcommand};

/// CLI argument definitions for ScreenAnimation.
///
/// Supports two operation modes:
/// - Animation: Transparent overlay mode
/// - Wallpaper: Desktop wallpaper embedding mode
#[derive(Parser, Debug)]
pub struct AppArgs {
    #[command(subcommand)]
    pub command: Commands,
}

/// Available animation modes.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Single animation mode (v1) - transparent overlay on desktop
    Animation { path: String },
    /// Wallpaper mode (v1) - embed behind desktop icons
    Wallpaper { path: String },
}

impl AppArgs {
    /// Parse command-line arguments and return (path, is_wallpaper).
    pub fn parse_args() -> (String, bool) {
        let args = AppArgs::parse();
        match &args.command {
            Commands::Animation { path } => (path.clone(), false),
            Commands::Wallpaper { path } => (path.clone(), true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_parsing() {
        // This would require clap testing utilities
        // For now, just verify structures compile
        let _ = AppArgs::parse_from(&["test", "Animation", "test.flow"]);
    }
}
