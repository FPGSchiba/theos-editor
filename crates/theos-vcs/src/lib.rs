//! Wrapper over the git command line (G6).
//!
//! It provides [`Git`], a runner that starts the `git` program without a shell and stops it with
//! its whole process tree when a [`Cancel`] flag is set, [`repo`], queries for a repository's top
//! folder, a folder's place in it, its boolean settings and its hooks folder, [`Folding`], how the repository compares paths (`core.ignorecase`,
//! `core.precomposeunicode`), and [`staged`], a reader of the staged snapshot: the changes, the
//! tracked paths in git's spelling, the staged bytes of edited files and of any object, and
//! [`read`], a file's bytes at a version, and [`status`], the status with unstaged renames
//! paired.

mod cancel;
mod folding;
mod git;
pub mod read;
mod read_args;
pub mod repo;
pub mod staged;
pub mod status;
mod tree;
mod version;
mod wait;

pub use cancel::Cancel;
pub use folding::Folding;
pub use git::{Git, GitError};
pub use version::GitVersion;
