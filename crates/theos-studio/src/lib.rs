//! Projects, layered settings, editor-part registry, command layer, agent sessions, review queue,
//! generators, LLM client and binaries folder.
//!
//! P06 adds the command layer; so far it has the repo lock ([`lock`]), which lets one process at
//! a time change a project, the [`Workspace`] (a project with its registry and index) and
//! [`introduced`], the count of errors a command introduces. The rest arrives in P06 and P07.

mod command;
pub mod error;
pub mod introduced;
pub mod lock;
pub mod workspace;

pub use command::{
    Base, Command, CommandError, Create, Delete, Edit, FileChange, Patch, Plan, PlanContext, Reads,
    Write,
};
pub use error::StudioError;
pub use introduced::{FindingCounts, introduced};
pub use workspace::{Workspace, find_root};
