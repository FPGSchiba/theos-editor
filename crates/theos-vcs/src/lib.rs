//! Wrapper over the git command line (G6).
//!
//! P05 provides [`Git`], a runner that starts the `git` program without a shell, [`repo`], queries
//! for a repository's top folder, a folder's place in it, its boolean settings and its hooks
//! folder, [`Folding`], how the repository compares paths (`core.ignorecase`,
//! `core.precomposeunicode`), and [`staged`], a reader of the staged snapshot: the changes, the
//! tracked paths in git's spelling, the staged bytes of edited files and of any object.
//!
//! P46 adds git's version check to [`Git::find`] ([`GitVersion`], [`GitError::TooOld`]), a
//! [`Cancel`] flag that stops git with its whole process tree ([`Git::run_cancellable`],
//! [`GitError::Cancelled`]), and the reads: [`read`], a file's bytes at a version, [`status`],
//! the status with unstaged renames paired, [`history`], a file's commits across renames,
//! [`index`], the paths of git's index in git's spelling, and [`repo::first_parent`]. The writes
//! (commit, worktrees, merge, pull and push) arrive in P47.

mod cancel;
pub mod commit;
mod commit_id;
mod folding;
mod git;
pub mod history;
pub mod housekeeping;
pub mod index;
pub mod merge;
mod no_hooks;
pub mod push;
pub mod read;
mod read_args;
pub mod remote;
pub mod repo;
pub mod stage;
pub mod staged;
pub mod status;
mod tree;
mod version;
mod wait;
mod write_args;

pub use cancel::Cancel;
pub use commit_id::CommitId;
pub use folding::Folding;
pub use git::{Git, GitError, Output};
pub use version::GitVersion;
