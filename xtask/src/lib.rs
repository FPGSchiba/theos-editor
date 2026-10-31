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
//! - [`error`]: the one error type of the library.
//!
//! And the checks built on it:
//!
//! - [`checks`]: the checks of `docs/gate.md` §1; [`checks::cargo_cmds`] has the ones that are
//!   cargo commands (`fmt`, `clippy`, `test`, `docs`, `deny`), [`checks::commit_msg`] the
//!   `commit-msg` check, [`checks::pr_title`] the `pr-title` check and [`checks::pr_body`] the
//!   `pr-body` check.
//! - [`tools`]: the pinned tools (`gate/tools.toml`) and the `tools` check.

pub mod branch;
pub mod checks;
pub mod ci;
pub mod conventional;
pub mod error;
pub mod files;
pub mod git;
pub mod process;
pub mod report;
pub mod tools;
pub mod verdict;

pub use error::{Error, Result};
