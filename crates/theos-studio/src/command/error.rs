//! Why a command fails.

use theos_content::rename::RenameError;
use theos_content::{Finding, IndexError, PatchError, ReadError, RecordEditError, RecordError};

use crate::StudioError;
use crate::lock::LockError;

/// Why a command did not run. A command that fails changes nothing, except [`Partial`].
///
/// [`Partial`]: CommandError::Partial
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    /// The change would introduce errors.
    #[error("the change would introduce errors in `{}`; fix them first", paths(.blocking))]
    Rejected {
        /// The new errors.
        blocking: Vec<Finding>,
        /// Every finding of the touched records after the change.
        findings: Vec<Finding>,
    },
    /// The file changed on disk since the caller read it.
    #[error("`{path}` changed on disk since it was read; reload it")]
    Stale {
        /// The file.
        path: String,
    },
    /// The file or record does not exist.
    #[error("`{what}` does not exist")]
    NotFound {
        /// The path or id.
        what: String,
    },
    /// The file to create exists.
    #[error("`{path}` exists already")]
    Exists {
        /// The file.
        path: String,
    },
    /// The new id belongs to another record.
    #[error("the id `{id}` is taken by `{path}`")]
    IdTaken {
        /// The id.
        id: String,
        /// The record that has it.
        path: String,
    },
    /// The id names several records.
    #[error("the id `{id}` names several records: {}", paths.join(", "))]
    Ambiguous {
        /// The id.
        id: String,
        /// The records.
        paths: Vec<String>,
    },
    /// A rename that only changes the case of an id.
    #[error("cannot rename `{id}` to `{to}`: they differ only in case")]
    CaseOnly {
        /// The old id.
        id: String,
        /// The new id.
        to: String,
    },
    /// A path that breaks a rule of the project.
    #[error("the path `{path}` {reason}")]
    Path {
        /// The path as given.
        path: String,
        /// The rule it breaks.
        reason: &'static str,
    },
    /// Undo or redo found a file of its journal entry changed after the command, or after its
    /// undo: the file no longer holds the bytes the entry left there, so playing the entry back
    /// would overwrite that later change.
    #[error(
        "`{path}` changed after that command; undoing or redoing it would overwrite the later \
         change"
    )]
    Changed {
        /// The file.
        path: String,
    },
    /// Undo with an empty journal.
    #[error("there is nothing to undo")]
    NothingToUndo,
    /// Redo with nothing undone.
    #[error("there is nothing to redo")]
    NothingToRedo,
    /// A file cannot be read.
    #[error("cannot read `{path}`: {source}")]
    Read {
        /// The file.
        path: String,
        /// The cause.
        source: ReadError,
    },
    /// A file is not a record.
    #[error("`{path}` is not a record: {source}")]
    Record {
        /// The file.
        path: String,
        /// The cause.
        source: RecordError,
    },
    /// A patch does not apply.
    #[error("cannot patch `{path}`: {source}")]
    Patch {
        /// The record.
        path: String,
        /// The cause.
        source: PatchError,
    },
    /// Edits do not apply.
    #[error("cannot edit `{path}`: {source}")]
    Edit {
        /// The record.
        path: String,
        /// The cause.
        source: RecordEditError,
    },
    /// Ids cannot be renamed in a record.
    #[error("cannot rename ids in `{path}`: {source}")]
    Rename {
        /// The record.
        path: String,
        /// The cause.
        source: RenameError,
    },
    /// A file cannot be written.
    #[error("cannot write `{path}`: {source}; another program may have it open")]
    Write {
        /// The file.
        path: String,
        /// The cause.
        source: std::io::Error,
    },
    /// A write failed and the files written before it could not all be put back.
    #[error(
        "cannot write `{path}`: {source}; these files could not be restored and keep the new \
         bytes: {}",
        unrestored.join(", ")
    )]
    Partial {
        /// The file that failed.
        path: String,
        /// The cause.
        source: std::io::Error,
        /// The files that keep the new bytes.
        unrestored: Vec<String>,
    },
    /// The index failed.
    #[error(transparent)]
    Index(#[from] IndexError),
    /// The repo lock could not be taken.
    #[error(transparent)]
    Lock(#[from] LockError),
    /// The studio failed.
    #[error(transparent)]
    Studio(#[from] StudioError),
}

impl CommandError {
    /// An I/O failure while reading `path`.
    pub(crate) fn read_io(path: String, source: std::io::Error) -> CommandError {
        CommandError::Read {
            path: path.clone(),
            source: ReadError::Io {
                path: path.into(),
                source,
            },
        }
    }
}

/// The distinct paths of `findings`, joined.
fn paths(findings: &[Finding]) -> String {
    let mut paths: Vec<&str> = findings.iter().map(|f| f.path.as_str()).collect();
    paths.sort_unstable();
    paths.dedup();
    paths.join("`, `")
}
