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
use xtask::branch::Phase;
use xtask::checks::{ac_trace, cargo_cmds, commit_msg, lints, pr_body, pr_title};
use xtask::git::Git;
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
    /// Check that every acceptance criterion of the phase has a tagged test, and that every tag
    /// names a criterion.
    ///
    /// The phase is the one of the branch: the head branch of the pull request under GitHub
    /// Actions, else the branch that is checked out. Only `phase/PNN-…` has a phase; elsewhere
    /// only the rules for tags are checked.
    AcTrace {
        /// The phase to check, instead of the phase of the branch.
        #[arg(long, value_name = "PNN")]
        phase: Option<Phase>,
    },
    /// Check that every commit of the branch is a Conventional Commit.
    ///
    /// Under GitHub Actions the commits and the branch come from the event. Any flag here
    /// replaces the event and takes the rest from git instead.
    CommitMsg {
        /// The commit before the first one to check. Default: the merge base of the head and
        /// `origin/main`.
        #[arg(long, value_name = "REV")]
        base: Option<String>,
        /// The last commit to check. Default: `HEAD`.
        #[arg(long, value_name = "REV")]
        head: Option<String>,
        /// The branch the commits are on, which decides whether `Phase:` is needed. Default:
        /// the branch that is checked out.
        #[arg(long, value_name = "NAME")]
        branch: Option<String>,
    },
    /// Check that the pull request title is a Conventional Commit that matches the branch.
    ///
    /// Under GitHub Actions the title and the branch come from the pull request event. A title
    /// only exists on a pull request, so anywhere else `--title` is needed; it replaces the event.
    PrTitle {
        /// The title to check.
        #[arg(long, value_name = "TEXT")]
        title: Option<String>,
        /// The branch the pull request is from, which decides how the title must end. Default:
        /// the branch that is checked out.
        #[arg(long, value_name = "NAME", requires = "title")]
        branch: Option<String>,
    },
    /// Check that the pull request description has every section of the template, filled.
    ///
    /// Under GitHub Actions the description comes from the pull request event. A description
    /// only exists on a pull request, so anywhere else `--body-file` is needed; it replaces the
    /// event.
    PrBody {
        /// A file with the description to check.
        #[arg(long, value_name = "PATH")]
        body_file: Option<PathBuf>,
    },
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
        Command::Lints => lints::run(&root, ci.as_ref(), &mut out)?,
        Command::AcTrace { phase: None } => ac_trace::run(&root, ci.as_ref(), &mut out)?,
        Command::AcTrace { phase } => ac_trace::run_phase(&root, *phase, ci.is_some(), &mut out)?,
        Command::CommitMsg {
            base: None,
            head: None,
            branch: None,
        } => commit_msg::run(&root, ci.as_ref(), &mut out)?,
        Command::CommitMsg { base, head, branch } => {
            let git = Git::new(&root);
            let range = commit_msg::local_range(&git, base.as_deref(), head.as_deref())?;
            let branch = commit_msg::branch_kind(&git, branch.as_deref())?;
            let known_acs = commit_msg::known_acs(&root)?;
            commit_msg::check(&git, &range, &branch, &known_acs)?
        }
        Command::PrTitle { title: None, .. } => pr_title::run(&root, ci.as_ref(), &mut out)?,
        Command::PrTitle {
            title: Some(title),
            branch,
        } => {
            let branch = commit_msg::branch_kind(&Git::new(&root), branch.as_deref())?;
            pr_title::check_title(title, &branch)
        }
        Command::PrBody { body_file: None } => pr_body::run(&root, ci.as_ref(), &mut out)?,
        Command::PrBody {
            body_file: Some(path),
        } => {
            let template = pr_body::read_template(&root)?;
            let body = std::fs::read_to_string(path).map_err(|source| Error::Io {
                path: path.clone(),
                source,
            })?;
            pr_body::check_body(&template, Some(&body))
        }
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
