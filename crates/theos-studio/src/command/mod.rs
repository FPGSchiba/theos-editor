//! The command layer: the one place that changes a project, behind the repo lock.
//!
//! A [`Command`] builds a [`Plan`]: the files it changes, as the bytes before and after. This
//! module holds the model and the five plain commands; later tasks of P06 add the writer, the
//! pipeline and the journal. A new command is one module and one `pub use`.

mod base;
mod create;
mod delete;
mod edit;
mod error;
mod files;
mod patch;
mod path;
pub(crate) mod state;
#[cfg(test)]
mod test_support;
mod write;

use std::collections::BTreeMap;
use std::path::Path;

use theos_content::{Index, KindRegistry};

pub use base::Base;
pub use create::Create;
pub use delete::Delete;
pub use edit::Edit;
pub use error::CommandError;
pub use patch::Patch;
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
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the pipeline of P06 builds the context; remove this then"
        )
    )]
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
        let path = project_path(path)?;
        match std::fs::read(self.root.join(&path)) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(CommandError::read_io(path, source)),
        }
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
