//! Wrapper over the git command line (G6).
//!
//! It provides [`Git`], a runner that starts the `git` program without a shell, [`repo`],
//! queries for a repository's top folder, a folder's place in it, its boolean settings and its
//! hooks folder, [`Folding`], how the repository compares paths (`core.ignorecase`,
//! `core.precomposeunicode`), and [`staged`], a reader of the staged snapshot: the changes, the
//! tracked paths in git's spelling, the staged bytes of edited files and of any object.

mod folding;
mod git;
pub mod repo;
pub mod staged;

pub use folding::Folding;
pub use git::{Git, GitError};
