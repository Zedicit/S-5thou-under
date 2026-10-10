//! `s5k`: understand Studio 5000 L5X projects from the command line.

#![forbid(unsafe_code)]

use std::fs::File;
use std::io::{self, BufReader, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "s5k", version, about = "Understand Studio 5000 L5X projects")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print a summary of an L5X file.
    Inspect {
        /// Path to the .L5X file.
        file: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            #[allow(clippy::print_stderr)]
            {
                eprintln!("error: {err:#}");
            }
            // Exit code 3: I/O or parse failure (FR-CLI-003).
            ExitCode::from(3)
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Inspect { file } => {
            let f = File::open(&file).with_context(|| format!("opening {}", file.display()))?;
            let header = s5k_l5x::read_header(BufReader::new(f))
                .with_context(|| format!("reading {}", file.display()))?;
            let mut out = io::stdout().lock();
            let show = |v: &Option<String>| v.clone().unwrap_or_else(|| "-".into());
            writeln!(out, "Target name:       {}", show(&header.target_name))?;
            writeln!(out, "Target type:       {:?}", header.target_type)?;
            writeln!(
                out,
                "Software revision: {}",
                show(&header.software_revision)
            )?;
            writeln!(out, "Schema revision:   {}", show(&header.schema_revision))?;
            writeln!(out, "Contains context:  {}", header.contains_context)?;
            writeln!(out, "Export date:       {}", show(&header.export_date))?;
        }
    }
    Ok(())
}
