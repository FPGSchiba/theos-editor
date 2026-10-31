//! The `cargo xtask` command line: gate tooling (`docs/gate.md`).
//!
//! Every check is a subcommand named like the check. Exit code 0 means the check passed, 1 that
//! it found something, and 2 that it could not run. A subcommand that is not built yet is an
//! error, so a gate that depends on it cannot pass by accident.
//!
//! Adding a check is one arm in `run`: call the check's `run(root, ci, out)`, which gathers its
//! own inputs and returns a verdict, and let `report::emit` write it.

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Context;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;
use xtask::verdict::Verdict;
use xtask::{Error, ci, report};

/// Gate tooling for the theos-editor workspace (`docs/gate.md`).
#[derive(Parser)]
#[command(name = "xtask")]
struct Cli {
    /// The repository root. Defaults to the workspace this xtask was built from.
    #[arg(long, global = true, value_name = "DIR")]
    root: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check that the installed tools have the versions pinned in `gate/tools.toml`.
    Tools,
    /// Check the formatting of every crate.
    Fmt,
    /// Run clippy on every crate with warnings denied.
    Clippy,
    /// Run the tests and the doc tests of every crate.
    Test,
    /// Build the documentation of every crate with warnings denied.
    Docs,
    /// Check licences, advisories, bans and sources.
    Deny,
    /// Check that no engine crate is a dependency without the feature `engine`.
    EngineFree,
    /// Measure line coverage per crate and compare it with the floors and the baseline.
    Coverage,
    /// Check the lint list, the tool configuration and the attributes in the sources.
    Lints,
    /// Check that every acceptance criterion of the phase has a tagged test.
    AcTrace,
    /// Check that every commit of the branch is a Conventional Commit.
    CommitMsg,
    /// Check that the pull request title is a Conventional Commit that matches the branch.
    PrTitle,
    /// Check that the pull request description has every section of the template.
    PrBody,
    /// Check that a pull request touching a protected path has the label `approved`.
    Approval,
    /// Run every check of the gate that needs no pull request, as CI does.
    Gate,
    /// Check or apply the repository settings of `docs/gate.md`.
    Github {
        #[command(subcommand)]
        action: GithubAction,
    },
}

#[derive(Subcommand)]
enum GithubAction {
    /// Compare the repository settings with `docs/gate.md` and report the differences.
    Check,
    /// Change the repository settings to match `docs/gate.md`.
    Apply,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    init_logging();
    match run(&cli) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(error) => {
            // If stderr is closed there is nobody left to tell.
            let _ = writeln!(std::io::stderr(), "error: {error:#}");
            ExitCode::from(2)
        }
    }
}

/// Runs the subcommand and says whether it passed.
fn run(cli: &Cli) -> anyhow::Result<bool> {
    let root = cli.root.clone().unwrap_or_else(default_root);
    tracing::debug!(root = %root.display(), "repository root");
    let ci = ci::from_env().context("cannot read the GitHub Actions event")?;
    let mut out = std::io::stdout().lock();
    // One arm per subcommand. A built check reads `<module>::run(&root, ci.as_ref(), &mut out)?`.
    let verdict: Verdict = match &cli.command {
        Command::Tools => not_built("tools")?,
        Command::Fmt => not_built("fmt")?,
        Command::Clippy => not_built("clippy")?,
        Command::Test => not_built("test")?,
        Command::Docs => not_built("docs")?,
        Command::Deny => not_built("deny")?,
        Command::EngineFree => not_built("engine-free")?,
        Command::Coverage => not_built("coverage")?,
        Command::Lints => not_built("lints")?,
        Command::AcTrace => not_built("ac-trace")?,
        Command::CommitMsg => not_built("commit-msg")?,
        Command::PrTitle => not_built("pr-title")?,
        Command::PrBody => not_built("pr-body")?,
        Command::Approval => not_built("approval")?,
        Command::Gate => not_built("gate")?,
        Command::Github {
            action: GithubAction::Check,
        } => not_built("github check")?,
        Command::Github {
            action: GithubAction::Apply,
        } => not_built("github apply")?,
    };
    report::emit(&verdict, ci.is_some(), &mut out)?;
    Ok(verdict.passed())
}

fn not_built(command: &str) -> xtask::Result<Verdict> {
    Err(Error::Missing {
        what: format!("the `{command}` command (not built yet)"),
    })
}

/// The workspace root: the parent of the directory this xtask was built from.
fn default_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map_or_else(|| manifest_dir.to_path_buf(), Path::to_path_buf)
}

/// Logs to stderr, filtered by `XTASK_LOG` (default: warnings and errors).
fn init_logging() {
    let filter = EnvFilter::builder()
        .with_env_var("XTASK_LOG")
        .with_default_directive(LevelFilter::WARN.into())
        .from_env_lossy();
    // A subscriber that is already installed is as good as ours.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
        .with_target(false)
        .without_time()
        .try_init();
}
