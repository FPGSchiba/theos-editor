//! Projects, layered settings, editor-part registry, command layer, agent sessions, review queue,
//! generators, LLM client and binaries folder.
//!
//! P05 adds the [`Workspace`] (a project with its registry and index, which falls back to an
//! index in memory while another process holds the cache), [`find_root`], the [`StudioError`] of
//! the studio, [`report()`], which writes an error with each cause once, [`introduced()`], the
//! count of errors a change introduces, [`check_staged`], which judges git's staged snapshot,
//! [`check()`], which adds the binaries-folder warnings, [`user_settings`], which reads where that
//! folder is, and the JSON [`views`] of findings, the [`schema`] of kinds and rules, and
//! [`create()`](create::create), which makes a new record from its template and writes it only
//! when it introduces no error, and [`hook::install_hook`], which writes git's pre-commit hook.
//! Later tasks of P05 extend this crate; the rest arrives in P06,
//! P07 and later phases.

pub mod check;
pub mod create;
pub mod error;
pub mod hook;
pub mod introduced;
pub mod report;
pub mod schema;
pub mod staged;
#[cfg(test)]
mod test_support;
mod top;
pub mod user_settings;
pub mod views;
pub mod workspace;

pub use check::{Binaries, BinariesFolder, Check, binaries_folder, check};
pub use error::StudioError;
pub use introduced::{FindingCounts, introduced};
pub use report::report;
pub use staged::{StagedCheck, check_staged};
pub use workspace::{Workspace, find_root};
