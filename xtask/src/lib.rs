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
//! - [`git`], [`files`] and [`process`]: the only places that start git and cargo or walk the tree.
//! - [`error`]: the one error type of the library.

pub mod branch;
pub mod ci;
pub mod error;
pub mod files;
pub mod git;
pub mod process;
pub mod report;
pub mod verdict;

pub use error::{Error, Result};
