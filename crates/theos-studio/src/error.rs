//! The errors of the studio.

use std::path::PathBuf;

use theos_content::{IndexError, ProjectError, RegistryError};
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
    /// The folder is not in a git repository.
    #[error("`{}` is not in a git repository", folder.display())]
    NotInRepository {
        /// The folder.
        folder: PathBuf,
    },
    /// The user settings file is not valid.
    #[error("{}: {message}", path.display())]
    UserSettings {
        /// The settings file.
        path: PathBuf,
        /// What is wrong with it.
        message: String,
    },
}
