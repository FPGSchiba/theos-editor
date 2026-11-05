//! The index of a project: a persistent cache in SQLite with full-text search.

use std::path::PathBuf;

use crate::walk::WalkError;

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "wired into the index by Task 10")
)]
mod identity;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "wired into the index by Task 10")
)]
mod store;

/// A failure of the index.
#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    /// SQLite reported an error.
    #[error("the index cache failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Another process holds the cache for longer than the busy timeout.
    #[error("the index cache {} is busy: another theos process is writing to it", path.display())]
    Busy {
        /// The cache file.
        path: PathBuf,
    },
    /// The cache folder or file cannot be created or deleted.
    #[error("cannot create or delete the index cache {}: {source}", path.display())]
    Cache {
        /// The cache file.
        path: PathBuf,
        /// The cause.
        source: std::io::Error,
    },
    /// The cache is damaged and another process holds it open, so it cannot be replaced.
    #[error("the index cache {} is damaged and in use: close the other theos processes", path.display())]
    CacheInUse {
        /// The cache file.
        path: PathBuf,
    },
    /// The walk of the project failed.
    #[error(transparent)]
    Walk(#[from] WalkError),
}
