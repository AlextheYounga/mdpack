use clap::{Parser, Subcommand};
use mdpack::{PackOptions, UnpackOptions, pack_to_path, unpack_from_path};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "mdpack",
    version,
    about = "Bundle and expand code2prompt-style markdown"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Pack {
        #[arg(value_name = "PATH", default_value = ".")]
        path: PathBuf,
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
        #[arg(long)]
        include_hidden: bool,
        #[arg(long)]
        ignored: bool,
    },
    Unpack {
        #[arg(value_name = "FILE")]
        input: PathBuf,
        #[arg(short, long, value_name = "DIR")]
        output: Option<PathBuf>,
        #[arg(long)]
        force: bool,
    },
}

fn main() -> mdpack::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Pack {
            path,
            output,
            include_hidden,
            ignored,
        } => {
            let options = PackOptions {
                include_hidden,
                include_ignored: ignored,
            };
            let output = output.unwrap_or_else(|| PathBuf::from("bundle.md"));
            pack_to_path(&path, &output, options)?;
            println!("Wrote bundle to {}", display_path(&output));
        }
        Commands::Unpack {
            input,
            output,
            force,
        } => {
            let options = UnpackOptions { force };
            let output_dir = unpack_from_path(&input, output.as_deref(), options)?;
            println!("Unpacked to {}", display_path(&output_dir));
        }
    }
    Ok(())
}

fn display_path(path: &Path) -> String {
    if path.is_absolute() {
        return path.display().to_string();
    }
    match std::env::current_dir() {
        Ok(cwd) => cwd.join(path).display().to_string(),
        Err(_) => path.display().to_string(),
    }
}
