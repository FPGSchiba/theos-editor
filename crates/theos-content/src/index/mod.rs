//! The index of a project: a persistent cache in SQLite with full-text search.

use std::path::PathBuf;

use crate::kinds::KindRegistry;
use crate::project::Project;
use crate::walk::WalkError;

mod dump;
mod identity;
mod lens;
mod query;
mod resolve;
mod rows;
mod search;
pub mod slots;
mod store;
mod sync;
mod write;

pub use lens::{
    EffectiveLens, HeldBelief, IgnoredEntry, Layer, LensData, LensValue, Origin, Overridden,
};
pub use query::{LinkRef, RecordSummary};
pub use search::{Column, Hit, Match};
pub use slots::Slot;
pub use sync::{Change, Preview, Update};

use identity::Identity;
use store::Store;

/// The index of a content project: its records, links, beats, findings and search rows, kept in
/// SQLite. Queries read the store; the index keeps no copy of the rows.
pub struct Index {
    root: PathBuf,
    registry: KindRegistry,
    store: Store,
}

impl Index {
    /// Opens the cache `<root>/.theos/cache/index.sqlite` of `project` and syncs it with the
    /// folder, reading only the files whose size or time changed.
    ///
    /// # Errors
    ///
    /// Fails when the cache cannot be opened or replaced, the walk fails, or SQLite fails.
    pub fn open(project: &Project, registry: &KindRegistry) -> Result<(Index, Update), IndexError> {
        let file = project
            .root()
            .join(".theos")
            .join("cache")
            .join("index.sqlite");
        let store = Store::open(&file, &Identity::current(registry))?;
        let mut index = Index {
            root: project.root().to_path_buf(),
            registry: registry.clone(),
            store,
        };
        let update = index.sync()?;
        Ok((index, update))
    }

    /// An empty index in memory: [`Index::sync`] fills it from the folder, [`Index::apply`] from
    /// given texts.
    ///
    /// # Errors
    ///
    /// Fails when SQLite fails.
    pub fn in_memory(project: &Project, registry: &KindRegistry) -> Result<Index, IndexError> {
        Ok(Index {
            root: project.root().to_path_buf(),
            registry: registry.clone(),
            store: Store::in_memory(&Identity::current(registry))?,
        })
    }
}

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
