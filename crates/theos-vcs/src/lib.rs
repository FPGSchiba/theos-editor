//! Wrapper over the git command line (G6).
//!
//! It provides [`Git`], a runner that starts the `git` program without a shell, and [`repo`],
//! queries for a repository's top folder, a folder's place in it and its hooks folder.
//! The staged-snapshot reader joins them in a later task.

mod git;
pub mod repo;

pub use git::{Git, GitError};
