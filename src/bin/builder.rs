use clap::Parser;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use walkdir::WalkDir;
use zip::write::FileOptions;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Pack assets into a .flow package (ZIP archive with animation assets)",
    long_about = "Scans a directory and packs all files into a .flow file.\n\
                   The output file is automatically skipped if it is inside the input directory.\n\n\
                   Expected files:\n  \
                   - config.toml: Animation configuration (required)\n  \
                   - shader.wgsl: WGSL shader source (required)\n  \
                   - background.png: Background image (optional)\n  \
                   - *.wav: Audio files (optional)\n  \
                   - *.png, *.jpg: Textures (optional)"
)]
struct Args {
    #[arg(short, long, help = "Input directory with animation assets")]
    input: String,

    #[arg(short, long, help = "Output path for the .flow file")]
    output: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let input_path = Path::new(&args.input);
    let output_path = Path::new(&args.output);

    if !input_path.is_dir() {
        eprintln!("Error: Input path '{}' is not a directory or does not exist.", args.input);
        std::process::exit(1);
    }

    if !args.output.ends_with(".flow") {
        eprintln!("Warning: Output file '{}' does not end with .flow", args.output);
    }

    let file = File::create(output_path)?;
    let mut zip = zip::ZipWriter::new(file);

    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    eprintln!("Building flow package: {} -> {} (Deflate)", args.input, args.output);

    let output_canonical = output_path.canonicalize().unwrap_or_else(|_| output_path.to_path_buf());

    let mut file_count = 0;
    let mut dir_count = 0;

    for entry in WalkDir::new(input_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = path.strip_prefix(input_path)?;

        if path.is_file() {
            if let Ok(canonical) = path.canonicalize() {
                if canonical == output_canonical {
                    eprintln!("  (skipping output file: {})", name.display());
                    continue;
                }
            }
        }

        if path.is_file() {
            eprintln!("  + {}", name.display());
            zip.start_file(name.to_string_lossy(), options)?;
            let mut f = File::open(path)?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
            file_count += 1;
        } else if !name.as_os_str().is_empty() {
            eprintln!("  [D] {}", name.display());
            zip.add_directory(name.to_string_lossy(), options)?;
            dir_count += 1;
        }
    }

    zip.finish()?;

    eprintln!("\nCreated: {} ({} files, {} directories)", output_path.display(), file_count, dir_count);
    Ok(())
}