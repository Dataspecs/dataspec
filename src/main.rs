use std::path::PathBuf;

use clap::{Parser, Subcommand};
use dataspec::{create_project, default_dataspec_path};

#[derive(Parser, Debug)]
#[command(name = "dataspec")]
#[command(about = "Scaffold data-spec Rust projects", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create a new data-spec Rust project
    New {
        name: String,
        #[arg(short, long)]
        path: Option<String>,
        #[arg(long, help = "Path to dataspec crate for generated project dependencies")]
        dataspec_path: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New {
            name,
            path,
            dataspec_path,
        } => {
            let base_path = path
                .map(PathBuf::from)
                .unwrap_or_else(|| std::env::current_dir().expect("current dir"));
            let ds_path = dataspec_path
                .map(PathBuf::from)
                .unwrap_or_else(default_dataspec_path);

            if let Err(err) = create_project(&name, &base_path, &ds_path) {
                eprintln!("Error: {err}");
                std::process::exit(1);
            }

            println!(
                "Created data-spec project '{}' at {}",
                name,
                base_path.join(&name).display()
            );
        }
    }
}
