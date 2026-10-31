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
use xtask::checks::cargo_cmds;
use xtask::verdict::{Check, Verdict};
use xtask::{Error, ci, report, tools};

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
    Tools {
        /// Install the pinned tools that are missing or at another version (takes minutes and
        /// needs the network), then check them.
        #[arg(long, conflicts_with = "print_install_spec")]
        install: bool,
        /// Print the pins as `name@version,…`, the form the setup action takes, and nothing
        /// else.
        #[arg(long)]
        print_install_spec: bool,
    },
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
        Command::Tools {
            print_install_spec: true,
            ..
        } => {
            // Only the spec line goes to stdout: the setup action captures it.
            let pins = tools::load_pins(&root)?;
            writeln!(out, "{}", tools::install_spec(&pins))?;
            return Ok(true);
        }
        Command::Tools { install, .. } => {
            if *install {
                tools::install_missing(&root)?;
            }
            tools::run(&root, ci.as_ref(), &mut out)?
        }
        Command::Fmt => cargo_cmds::run(Check::Fmt, &root, ci.as_ref(), &mut out)?,
        Command::Clippy => cargo_cmds::run(Check::Clippy, &root, ci.as_ref(), &mut out)?,
        Command::Test => cargo_cmds::run(Check::Test, &root, ci.as_ref(), &mut out)?,
        Command::Docs => cargo_cmds::run(Check::Docs, &root, ci.as_ref(), &mut out)?,
        Command::Deny => cargo_cmds::run(Check::Deny, &root, ci.as_ref(), &mut out)?,
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
