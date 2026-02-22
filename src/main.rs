use clap::{Parser, Subcommand};
use mdpack::{PackOptions, UnpackOptions, pack_to_path, pack_to_string, unpack_from_path};
use std::io::{self, Write};
use std::path::PathBuf;

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
        } => {
            let options = PackOptions { include_hidden };
            match output {
                Some(output) => pack_to_path(&path, &output, options)?,
                None => {
                    let bundle = pack_to_string(&path, options)?;
                    let mut stdout = io::stdout();
                    stdout.write_all(bundle.as_bytes())?;
                }
            }
        }
        Commands::Unpack {
            input,
            output,
            force,
        } => {
            let options = UnpackOptions { force };
            unpack_from_path(&input, output.as_deref(), options)?;
        }
    }
    Ok(())
}
