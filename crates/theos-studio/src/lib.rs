//! Projects, layered settings, editor-part registry, command layer, agent sessions, review queue,
//! generators, LLM client and binaries folder.
//!
//! P05 adds the [`Workspace`] (a project with its registry and index, which falls back to an
//! index in memory while another process holds the cache), [`find_root`], the [`StudioError`] of
//! the studio and [`report`], which writes an error with each cause once. Later tasks of P05
//! extend this crate; the rest arrives in P06, P07 and later phases.

pub mod error;
pub mod report;
#[cfg(test)]
mod test_support;
pub mod workspace;

pub use error::StudioError;
pub use report::report;
pub use workspace::{Workspace, find_root};
