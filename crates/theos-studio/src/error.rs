//! The errors of the studio.

use std::path::PathBuf;

use theos_content::{IndexError, KindRegistry, ProjectError, RegistryError};

use crate::create::CreateError;
use crate::hook::HookError;
use theos_vcs::GitError;

/// A failure of the studio.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum StudioError {
    /// The project could not be opened.
    #[error(transparent)]
    Project(#[from] ProjectError),
    /// The editor-part registry could not be built.
    #[error(transparent)]
    Registry(#[from] RegistryError),
    /// The index failed.
    #[error(transparent)]
    Index(#[from] IndexError),
    /// Running git failed.
    #[error(transparent)]
    Git(#[from] GitError),
    /// A file operation failed.
    #[error("{}: {source}", path.display())]
    Io {
        /// The file or folder.
        path: PathBuf,
        /// What went wrong.
        source: std::io::Error,
    },
    /// No `theos.toml` in the start folder or any folder above it.
    #[error("no theos.toml in `{}` or any folder above it", start.display())]
    NotAProject {
        /// Where the search started.
        start: PathBuf,
    },
    /// The folder is not in a git repository, or git cannot use the repository it is in.
    #[error("`{}` is not in a git repository", folder.display())]
    NotInRepository {
        /// The folder.
        folder: PathBuf,
        /// git's own answer, which names the reason (no repository, dubious ownership, …).
        #[source]
        source: GitError,
    },
    /// The user settings file is not valid.
    #[error("{}: {message}", path.display())]
    UserSettings {
        /// The settings file.
        path: PathBuf,
        /// What is wrong with it.
        message: String,
    },
    /// The kind is not registered.
    #[error("unknown kind `{kind}`; the kinds are: {}", known.join(", "))]
    UnknownKind {
        /// What was asked for.
        kind: String,
        /// The registered kinds, in order.
        known: Vec<String>,
    },
    /// `theos new` could not make the record.
    #[error(transparent)]
    Create(#[from] CreateError),
    /// `theos hook install` refused.
    #[error(transparent)]
    Hook(#[from] HookError),
}

impl StudioError {
    /// The error for a kind that `registry` does not have, with the kinds it has.
    #[must_use]
    pub fn unknown_kind(registry: &KindRegistry, kind: &str) -> StudioError {
        StudioError::UnknownKind {
            kind: kind.to_owned(),
            known: registry.kinds().map(|spec| spec.name.to_owned()).collect(),
        }
    }
}
