//! Projects, layered settings, editor-part registry, command layer, agent sessions, review queue,
//! generators, LLM client and binaries folder.
//!
//! P06 adds the command layer: the repo lock ([`lock`]), which lets one process at a time change
//! a project; the [`Workspace`] (a project with its registry and index); the commands [`Create`],
//! [`Write`], [`Edit`], [`Patch`], [`Delete`] and [`Rename`], which [`Workspace::run`] takes
//! through one pipeline of preview, verdict, write and reindex on a [`Route`]; undo and redo of
//! the in-memory journal ([`Workspace::undo`], [`Workspace::redo`]), for the newest entry or,
//! with [`Scope::Record`], the newest entry of one file; and [`introduced()`], the count of errors
//! a command introduces. The rest arrives in P07 and later phases.

mod command;
pub mod error;
pub mod introduced;
pub mod lock;
pub mod workspace;

pub use command::*;
pub use error::StudioError;
pub use introduced::{FindingCounts, introduced};
pub use workspace::{Workspace, find_root};
