//! The top folder of the repository that holds a project, with the studio's error for "none".

use std::path::{Path, PathBuf};

use theos_vcs::{Git, GitError, repo};

use crate::StudioError;

/// The top folder of the work tree that holds `folder`.
///
/// # Errors
///
/// [`StudioError::NotInRepository`], with git's answer as its source, when git finds no usable
/// repository for `folder`, and [`StudioError::Git`] for every other failure (a missing git
/// stays a git error).
pub(crate) fn repo_top(git: &Git, folder: &Path) -> Result<PathBuf, StudioError> {
    repo::top(git, folder).map_err(|error| match error {
        GitError::Failed { .. } => StudioError::NotInRepository {
            folder: folder.to_path_buf(),
            source: error,
        },
        other => StudioError::Git(other),
    })
}
