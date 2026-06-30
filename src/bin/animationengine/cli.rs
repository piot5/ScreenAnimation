//! CLI argument parsing module.
//!
//! Handles command-line argument parsing for the animation engine binary.
//! Supports two modes: Wallpaper mode and Animation (overlay) mode.

/// Application command-line arguments.
pub struct AppArgs;

impl AppArgs {
    /// Parse command-line arguments.
    ///
    /// Expected format: `animationengine.exe Wallpaper <package.flow>` or
    /// `animationengine.exe Animation <package.flow>`
    ///
    /// Returns (path_to_flow_file, is_wallpaper_mode).
    pub fn parse_args() -> (String, bool) {
        let args: Vec<String> = std::env::args().collect();
        if args.len() < 3 {
            eprintln!("Usage: {} (Wallpaper|Animation) <path_to.flow>", args.first().map(|s| s.as_str()).unwrap_or("animationengine"));
            eprintln!("  Wallpaper  - Set as desktop background wallpaper");
            eprintln!("  Animation  - Run as overlay animation");
            std::process::exit(1);
        }

        let mode = &args[1];
        let path = args[2].clone();
        let is_wp = match mode.to_lowercase().as_str() {
            "wallpaper" => true,
            "animation" => false,
            _ => {
                eprintln!("Invalid mode '{}'. Use 'Wallpaper' or 'Animation'.", mode);
                std::process::exit(1);
            }
        };

        (path, is_wp)
    }
}