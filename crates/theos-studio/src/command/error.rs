//! Why a command fails.

use theos_content::rename::RenameError;
use theos_content::{Finding, IndexError, PatchError, ReadError, RecordEditError, RecordError};

use super::files::was_retried;
use crate::StudioError;
use crate::lock::LockError;

/// Why a command did not run. A command that fails changes nothing, except [`Partial`] and an
/// error of [`Route::finish`], which comes after the write; [`Workspace::run_with`] says what
/// each error leaves behind.
///
/// [`Partial`]: CommandError::Partial
/// [`Route::finish`]: super::Route::finish
/// [`Workspace::run_with`]: crate::Workspace::run_with
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
    /// A record the command moves would not be read as a record at its new path: the path is
    /// hidden, a `.gitignore` keeps it out, or its name is no record name (such as `README.md`).
    /// The verdict refuses it before anything is written, so a command never takes a record out
    /// of the index (ruling R16).
    #[error("`{path}` would not be read as a record: hidden, ignored by git, or not a record name")]
    NotARecord {
        /// The new path.
        path: String,
    },
    /// A path that breaks a rule of the project, or a change whose effect on which files are
    /// records the verdict could not judge (ruling R21): of a `.gitignore`, on Windows the
    /// replacement of a hidden file, or of a path that the disk spells otherwise or that passes a
    /// symbolic link to a folder.
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
    /// A file the command would replace or remove has no write permission. It is refused before
    /// anything is written (ruling R9).
    #[error("`{path}` is read-only; make it writable first")]
    ReadOnly {
        /// The file.
        path: String,
    },
    /// A file cannot be written. Every file the command wrote before it is put back.
    #[error("cannot write `{path}`: {source}{}", retried_hint(.source))]
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

/// The hint for a write error whose kind the writer retried (spec §5.6): the retries are used up,
/// so another program probably holds the file. Other errors get no hint.
fn retried_hint(source: &std::io::Error) -> &'static str {
    if was_retried(source) {
        "; another program may have it open"
    } else {
        ""
    }
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
