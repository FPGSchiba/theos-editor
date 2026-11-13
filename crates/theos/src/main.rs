//! The `theos` command line: `check`, `ledger`, `schema`, `formalism` and `new` today;
//! `hook install` follows in phase P05.
//!
//! The binary only parses arguments, calls `theos-studio` and prints. Exit codes: 0 when the
//! command ran without error findings, 1 when `check` found errors, 2 when it could not run or `new` refused.

mod check;
mod formalism;
mod ledger;
mod new;
mod output;
mod schema;

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use theos_studio::report;

/// The `theos` command line.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Print one JSON document on stdout instead of text.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

/// A command of the binary.
#[derive(Subcommand)]
enum Command {
    /// Check the project and print its findings; exit 1 when there is an error.
    Check {
        /// The project folder; the project above the current folder by default.
        folder: Option<PathBuf>,
        /// Check what git would commit (the index) instead of the working tree.
        #[arg(long)]
        staged: bool,
        /// The folder of the binaries to check; the user settings name it by default.
        #[arg(long, conflicts_with = "staged")]
        binaries: Option<PathBuf>,
    },
    /// Print the reveal ledger of every truth.
    Ledger {
        /// The project folder; the project above the current folder by default.
        folder: Option<PathBuf>,
    },
    /// Print every kind and rule, or the fields and body of one kind.
    Schema {
        /// The kind to describe; every kind and rule by default.
        kind: Option<String>,
        /// The project whose configured severities to show; the one above the current folder
        /// by default.
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// Regenerate the Shared Formalism mirror and print each path written.
    Formalism {
        /// The project folder; the project above the current folder by default.
        folder: Option<PathBuf>,
    },
    /// Make a record of a kind from its template; exit 2 when it would introduce an error.
    New {
        /// The kind of the record.
        kind: String,
        /// The id; the next number of a numbered kind by default.
        id: Option<String>,
        /// The title of the record.
        #[arg(long)]
        title: Option<String>,
        /// A field to set, as `field=value`; a list is written `[a, b]`.
        #[arg(long = "set", value_parser = new::field_value)]
        set: Vec<(String, String)>,
        /// The project folder; the project above the current folder by default.
        #[arg(long)]
        project: Option<PathBuf>,
    },
}

/// How a command ended.
enum Exit {
    /// It ran and found no error.
    Ok,
    /// `check` ran and found at least one error.
    Errors,
    /// It could not run.
    Refused,
}

impl From<Exit> for ExitCode {
    fn from(exit: Exit) -> ExitCode {
        match exit {
            Exit::Ok => ExitCode::SUCCESS,
            Exit::Errors => ExitCode::from(1),
            Exit::Refused => ExitCode::from(2),
        }
    }
}

/// Runs the command, writing results to `out` and notes to `err`.
fn run(cli: Cli, out: &mut impl Write, err: &mut impl Write) -> anyhow::Result<Exit> {
    let json = cli.json;
    match cli.command {
        Command::Check {
            folder,
            staged,
            binaries,
        } => check::run(
            check::Options {
                folder,
                staged,
                binaries,
                json,
            },
            out,
            err,
        ),
        Command::Ledger { folder } => {
            ledger::run(folder.as_deref(), json, out, err).map(|()| Exit::Ok)
        }
        Command::Schema { kind, project } => {
            schema::run(kind.as_deref(), project.as_deref(), json, out).map(|()| Exit::Ok)
        }
        Command::Formalism { folder } => {
            formalism::run(folder.as_deref(), json, out).map(|()| Exit::Ok)
        }
        Command::New {
            kind,
            id,
            title,
            set,
            project,
        } => new::run(
            &new::Options {
                kind,
                id,
                title,
                set,
                project,
                json,
            },
            out,
            err,
        ),
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    match run(cli, &mut stdout, &mut stderr) {
        Ok(exit) => exit.into(),
        Err(error) => {
            let _ = writeln!(stderr, "error: {}", report(error.as_ref()));
            Exit::Refused.into()
        }
    }
}
