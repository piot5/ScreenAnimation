use clap::Parser;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use walkdir::WalkDir;
use zip::write::FileOptions;

#[derive(Parser, Debug)]
#[command(author, version, about = "Packt Assets in eine .flow Datei")]
struct Args {
    #[arg(short, long)]
    input: String,
    #[arg(short, long)]
    output: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let input_path = Path::new(&args.input);
    let output_path = Path::new(&args.output);

    if !input_path.is_dir() {
        eprintln!("Fehler: Der Input-Pfad muss ein Verzeichnis sein.");
        std::process::exit(1);
    }

    let file = File::create(output_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    println!("Packe Flow-Paket: {} -> {}", args.input, args.output);

    for entry in WalkDir::new(input_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = path.strip_prefix(input_path)?;

        if path.is_file() {
            println!("  + {}", name.display());
            zip.start_file(name.to_string_lossy(), options)?;
            let mut f = File::open(path)?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
        } else if !name.as_os_str().is_empty() {
            println!("  [D] {}", name.display());
            zip.add_directory(name.to_string_lossy(), options)?;
        }
    }

    zip.finish()?;
    println!("\nErfolgreich erstellt: {}", output_path.display());

    Ok(())
}