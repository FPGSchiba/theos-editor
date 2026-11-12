//! Wrapper over the git command line (G6).
//!
//! It provides [`Git`], a runner that starts the `git` program without a shell, [`repo`],
//! queries for a repository's top folder, a folder's place in it and its hooks folder, and
//! [`staged`], a reader of the staged snapshot: the changes, the tracked paths and the staged
//! bytes of edited files.

mod git;
pub mod repo;
pub mod staged;

pub use git::{Git, GitError};
