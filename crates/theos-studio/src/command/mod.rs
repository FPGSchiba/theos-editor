//! The command layer: the one place that changes a project, behind the repo lock.
//!
//! A [`Command`] builds a [`Plan`]: the files it changes, as the bytes before and after.
//! [`Workspace::run`](crate::Workspace::run) takes it through the pipeline: the repo lock, the
//! preview, the verdict, the write, the reindex and the [`Route`]'s finish. A new command is one
//! module and one `pub use`.

mod base;
mod create;
mod delete;
mod edit;
mod error;
mod files;
mod journal;
mod patch;
mod path;
mod pipeline;
mod route;
mod run;
pub(crate) mod state;
#[cfg(test)]
mod test_support;
mod verdict;
mod write;

use std::collections::BTreeMap;
use std::path::Path;

use theos_content::{Index, KindRegistry};

pub use base::Base;
pub use create::Create;
pub use delete::Delete;
pub use edit::Edit;
pub use error::CommandError;
pub use journal::{EntryId, EntrySummary};
pub use patch::Patch;
pub use pipeline::Outcome;
pub use route::{Applied, HumanRoute, Route};
pub use write::Write;

pub(crate) use path::project_path;

/// A change to a project, planned before anything is written.
pub trait Command {
    /// The line shown in the undo list and the log.
    fn label(&self) -> String;
    /// What the index must have fresh before [`Command::plan`] runs.
    fn reads(&self) -> Reads;
    /// The files this command changes, read from the workspace as it is now.
    ///
    /// # Errors
    ///
    /// A [`CommandError`] naming why the command cannot be planned; nothing was changed.
    fn plan(&self, ctx: &PlanContext<'_>) -> Result<Plan, CommandError>;
}

/// What the index must have fresh before a command plans.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reads {
    /// These project-relative paths.
    Paths(Vec<String>),
    /// Every record.
    Everything,
}

/// What a command reads while it plans.
pub struct PlanContext<'a> {
    root: &'a Path,
    index: &'a Index,
    registry: &'a KindRegistry,
}

impl<'a> PlanContext<'a> {
    /// The context of a plan on the project folder `root`, its index and its registry.
    pub(crate) fn new(root: &'a Path, index: &'a Index, registry: &'a KindRegistry) -> Self {
        PlanContext {
            root,
            index,
            registry,
        }
    }

    /// The project folder.
    #[must_use]
    pub fn root(&self) -> &Path {
        self.root
    }

    /// The index.
    #[must_use]
    pub fn index(&self) -> &Index {
        self.index
    }

    /// The editor-part registry.
    #[must_use]
    pub fn registry(&self) -> &KindRegistry {
        self.registry
    }

    /// The bytes of a project file, `None` when it is absent; the path is checked first.
    ///
    /// # Errors
    ///
    /// [`CommandError::Path`] for a path outside the rules, [`CommandError::Read`] when the file
    /// exists but cannot be read.
    pub fn read(&self, path: &str) -> Result<Option<Vec<u8>>, CommandError> {
        read_file(self.root, path)
    }

    /// The bytes of the file `path` when they hash to `base`.
    ///
    /// # Errors
    ///
    /// [`CommandError::NotFound`] when the file is absent, [`CommandError::Stale`] when its bytes
    /// differ from what the caller saw, else as [`PlanContext::read`].
    pub(crate) fn read_based(&self, path: &str, base: &Base) -> Result<Vec<u8>, CommandError> {
        let Some(bytes) = self.read(path)? else {
            return Err(CommandError::NotFound {
                what: path.to_owned(),
            });
        };
        if Base::of(&bytes) == *base {
            Ok(bytes)
        } else {
            Err(CommandError::Stale {
                path: path.to_owned(),
            })
        }
    }
}

/// The bytes of the project file `path` under `root`, `None` when it is absent; the path is
/// checked first.
///
/// # Errors
///
/// [`CommandError::Path`] for a path outside the rules, [`CommandError::Read`] when the file
/// exists but cannot be read.
pub(crate) fn read_file(root: &Path, path: &str) -> Result<Option<Vec<u8>>, CommandError> {
    let path = project_path(path)?;
    match std::fs::read(root.join(&path)) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(CommandError::read_io(path, source)),
    }
}

/// One file a command changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileChange {
    /// Project-relative, with `/`.
    pub path: String,
    /// The bytes the file has now; `None` when it does not exist.
    pub before: Option<Vec<u8>>,
    /// The bytes it will have; `None` when it is deleted.
    pub after: Option<Vec<u8>>,
}

/// The files a command changes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    /// The changes, in the order they are written.
    pub changes: Vec<FileChange>,
    /// Old path to new path, for the files a command moved.
    pub moved: BTreeMap<String, String>,
}

impl Plan {
    /// Whether the plan changes nothing: every file has the same bytes before and after.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changes
            .iter()
            .all(|change| change.before == change.after)
    }

    /// The paths of the changes.
    #[must_use]
    pub fn paths(&self) -> Vec<String> {
        self.changes
            .iter()
            .map(|change| change.path.clone())
            .collect()
    }
}
