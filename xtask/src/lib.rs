//! Gate tooling (`docs/gate.md`) and generator conformance checks.
//!
//! The library holds the checks; the `xtask` binary is the command line in front of them.
//!
//! This is the core every check builds on:
//!
//! - [`verdict`]: what a check returns, the findings in it, and the order of the gate.
//! - [`report`]: how verdicts are written, as a table, as workflow annotations and as a summary.
//! - [`ci`]: the GitHub Actions event the run belongs to, if any.
//! - [`branch`]: the branch naming rules of `CONTRIBUTING.md` §1.
//! - [`conventional`]: the commit message rules of `CONTRIBUTING.md` §2, as a parser and the
//!   list of what it forbids; `commit-msg` and `pr-title` share it.
//! - [`git`], [`files`] and [`process`]: the only places that start git and cargo or walk the tree.
//! - [`changes`]: the paths a pull request or the working tree changes, which `approval`,
//!   `slow-tests` and the scope of the gate all read.
//! - [`globs`]: the glob lists of the `gate/*.toml` files, read by `globset` the same way
//!   everywhere.
//! - [`markdown`] and [`repo_path`]: the one place that reads Markdown, and lexical paths from
//!   the text other tools print.
//! - [`github`]: the labels and comments of a pull request, and the rulesets, settings and labels
//!   of the repository, through the `gh` command line; [`github::settings`] keeps the latter as
//!   files in `gate/` and compares them with the live repository.
//! - [`error`]: the one error type of the library.
//!
//! And the checks built on it:
//!
//! - [`checks`]: the checks of `docs/gate.md` §1; [`checks::cargo_cmds`] has the ones that are
//!   cargo commands (`fmt`, `clippy`, `test`, `docs`, `deny`), [`checks::commit_msg`] the
//!   `commit-msg` check, [`checks::pr_title`] the `pr-title` check, [`checks::pr_body`] the
//!   `pr-body` check, [`checks::ac_trace`] the `ac-trace` check, [`checks::lints`] the `lints`
//!   check, [`checks::engine_free`] the `engine-free` check, [`checks::coverage`] the
//!   `coverage` check, [`checks::slow`] the `slow-tests` check and [`checks::approval`] the
//!   `approval` check.
//! - [`umbrella`]: the umbrella spec's phases and acceptance criteria, which `ac-trace` and
//!   `commit-msg` read their ids from.
//! - [`tools`]: the pinned tools (`gate/tools.toml`) and the `tools` check.
//! - [`ui_tools`]: node and pnpm, which the `tools` check verifies when `editor-ui/` exists.
//! - [`view`]: the workspace view, the files cargo compiles and what `syn` reads in them, which
//!   the checks that read code share.
//! - [`code_paths`]: the paths no check reads (`gate/code-paths.toml`).
//! - [`scope`]: whether a run needs every check or, for a change that touches no code, only
//!   `commit-msg`.
//! - [`needs`]: `cargo xtask needs`, whether a run needs any of some checks, which CI asks
//!   before it installs what they need.
//! - [`gate`]: `cargo xtask gate`, which runs the checks above that need no pull request, in
//!   order, and writes one verdict table.

pub mod branch;
pub mod changes;
pub mod checks;
pub mod ci;
pub mod code_paths;
pub mod conventional;
pub mod error;
pub mod files;
pub mod gate;
pub mod git;
pub mod github;
pub mod globs;
pub mod markdown;
pub mod needs;
pub mod process;
pub mod repo_path;
pub mod report;
pub mod scope;
pub mod tools;
pub mod ui_tools;
pub mod umbrella;
pub mod verdict;
pub mod view;

pub use error::{Error, Result};
