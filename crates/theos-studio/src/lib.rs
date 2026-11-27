//! Projects, layered settings, editor-part registry, command layer, agent sessions, review queue,
//! generators, LLM client and binaries folder.
//!
//! P05 adds the [`Workspace`] (a project with its registry and index, which falls back to an
//! index in memory while another process holds the cache), [`find_root`], the [`StudioError`] of
//! the studio, [`report()`], which writes an error with each cause once, [`introduced()`], the
//! error findings a change introduces beyond the counts a [`FindingCounts`] allows,
//! [`check_staged`], which judges git's staged snapshot, [`check()`], which adds the
//! binaries-folder warnings, [`user_settings`], which reads where that folder is, and the JSON
//! [`views`] of findings, the [`schema`] of kinds and rules, and
//! [`create()`](create::create), which makes a new record from its template and writes it with
//! P06's [`Create`] only when it introduces no error, and [`hook::install_hook`], which writes
//! git's `pre-commit` and `pre-merge-commit` hooks.
//!
//! P06 adds the command layer: the repo lock ([`lock`]), which lets one process at a time change
//! a project; the [`Workspace`] (a project with its registry and index); the commands [`Create`],
//! [`Write`], [`Edit`], [`Patch`], [`Delete`] and [`Rename`], which [`Workspace::run`] takes
//! through one pipeline of preview, verdict, write and reindex on a [`Route`]; undo and redo of
//! the in-memory journal ([`Workspace::undo`], [`Workspace::redo`]), for the newest entry or,
//! with [`Scope::Record`], the newest entry of one file; and [`introduced()`], the count of errors
//! a command introduces.
//!
//! P46 adds [`changes()`], git's status of a project grouped by record. The rest arrives in P07
//! and later phases.

pub mod changes;
pub mod check;
mod command;
pub mod create;
pub mod error;
pub mod hook;
pub mod introduced;
pub mod lock;
mod repo_path;
pub mod report;
pub mod schema;
pub mod staged;
#[cfg(test)]
mod test_support;
mod top;
pub mod user_settings;
pub mod views;
pub mod workspace;

pub use changes::{Changes, RecordChange, changes};
pub use check::{Binaries, BinariesFolder, Check, binaries_folder, check};
pub use command::*;
pub use error::StudioError;
pub use introduced::{FindingCounts, introduced};
pub use report::report;
pub use staged::{StagedCheck, check_staged};
pub use workspace::{Workspace, find_root};
