//! The errors of the studio.

use std::path::PathBuf;

use theos_content::{IndexError, KindRegistry, ProjectError, ReadError, RegistryError};

use crate::CommandError;
use crate::create::CreateError;
use crate::hook::HookError;
use crate::lock::LockError;
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
    /// A path or id names no record.
    #[error("no record `{what}`")]
    UnknownRecord {
        /// What was asked for.
        what: String,
    },
    /// A record file could not be read.
    #[error("{}: {source}", path.display())]
    Read {
        /// The file.
        path: PathBuf,
        /// What went wrong.
        source: ReadError,
    },
    /// A filter of a read tool is not valid.
    #[error("{message}")]
    InvalidFilter {
        /// What is wrong with it.
        message: String,
    },
    /// `theos new` could not make the record.
    #[error(transparent)]
    Create(#[from] CreateError),
    /// `theos hook install` refused.
    #[error(transparent)]
    Hook(#[from] HookError),
    /// The repo lock could not be taken.
    #[error(transparent)]
    Lock(#[from] LockError),
    /// The record or file is in a merge conflict; resolve it first.
    #[error("`{id}` is in a merge conflict; resolve it first")]
    Conflicted {
        /// The record's id, or the repository path of a file.
        id: String,
    },
    /// The change cannot be discarded.
    #[error("cannot discard `{path}`: {why}")]
    NotDiscardable {
        /// The record's project path or the file's repository path.
        path: String,
        /// Why not.
        why: String,
    },
    /// A status entry of a kind this studio does not know how to write.
    #[error("cannot write to git for the status entry {entry}")]
    UnsupportedEntry {
        /// The entry, as debug text.
        entry: String,
    },
    /// A command of the command layer failed. Boxed, since a [`CommandError`] can hold a
    /// [`StudioError`].
    #[error(transparent)]
    Command(Box<CommandError>),
}

impl From<CommandError> for StudioError {
    /// The error of a command; a studio error the command layer passed on is that error again.
    fn from(error: CommandError) -> StudioError {
        match error {
            CommandError::Studio(inner) => inner,
            other => StudioError::Command(Box::new(other)),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestResult;

    #[test]
    fn a_command_error_is_boxed_and_shown_as_it_is() -> TestResult {
        let error = StudioError::from(CommandError::NothingToUndo);
        let StudioError::Command(inner) = &error else {
            return Err(format!("expected a command error, got {error:?}").into());
        };
        assert!(matches!(**inner, CommandError::NothingToUndo), "{inner:?}");
        assert_eq!(error.to_string(), "there is nothing to undo");
        Ok(())
    }

    #[test]
    fn a_studio_error_a_command_passed_on_is_that_error_again() -> TestResult {
        let error = StudioError::from(CommandError::Studio(StudioError::NotAProject {
            start: PathBuf::from("synthetic"),
        }));
        let StudioError::NotAProject { start } = &error else {
            return Err(format!("expected the studio error itself, got {error:?}").into());
        };
        assert_eq!(start, &PathBuf::from("synthetic"));
        Ok(())
    }
}
