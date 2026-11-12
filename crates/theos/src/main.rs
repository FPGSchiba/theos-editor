//! The `theos` command line: `ledger` and `formalism` today; `check`, `new`, `schema`, `review`
//! and `hook install` follow in phase P05.

mod formalism;
mod ledger;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// The `theos` command line.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// A command of the binary.
#[derive(Subcommand)]
enum Command {
    /// Print the reveal ledger of every truth.
    Ledger {
        /// The project folder; the current folder by default.
        folder: Option<PathBuf>,
    },
    /// Regenerate the Shared Formalism mirror and print each path written.
    Formalism {
        /// The project folder; the current folder by default.
        folder: Option<PathBuf>,
    },
}

/// The folder of a command: the one given, else the current folder.
fn folder_or_current(folder: Option<PathBuf>) -> PathBuf {
    folder.unwrap_or_else(|| Path::new(".").to_path_buf())
}

fn run(cli: Cli, out: &mut impl Write) -> anyhow::Result<()> {
    match cli.command {
        Command::Ledger { folder } => ledger::run(&folder_or_current(folder), out),
        Command::Formalism { folder } => formalism::run(&folder_or_current(folder), out),
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let mut stdout = std::io::stdout().lock();
    match run(cli, &mut stdout) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(std::io::stderr().lock(), "error: {error:#}");
            ExitCode::FAILURE
        }
    }
}
