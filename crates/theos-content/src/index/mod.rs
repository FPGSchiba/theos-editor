//! The index of a project: a persistent cache in SQLite with full-text search.

use std::path::PathBuf;

use crate::kinds::KindRegistry;
use crate::lore::FactError;
use crate::project::Project;
use crate::walk::WalkError;

pub(crate) use sync::sha256_hex;

mod configured;
mod dump;
mod identity;
pub(crate) mod inputs;
mod ledger;
pub(crate) mod lens;
mod lore_source;
mod lore_step;
mod query;
pub(crate) mod resolve;
pub(crate) mod rows;
mod search;
pub mod slots;
mod store;
mod sync;
mod write;

pub use ledger::{BeatRef, LedgerEntry, TruthLedger};
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
///
/// Another build (another registry or `theos-content` source) may open the same cache file and
/// rebuild it for itself. Every write of this index first checks the cache's identity and, when
/// it is another build's, rebuilds the cache for this one before it does its own work. A cache
/// file that another build keeps rebuilding is rebuilt at most three times per write; then the
/// write fails with [`IndexError::Contended`].
pub struct Index {
    root: PathBuf,
    registry: KindRegistry,
    /// The identity of this build, which every write checks against the cache's `meta`.
    identity: Identity,
    store: Store,
}

impl Index {
    /// Opens the cache `<root>/.theos/cache/index.sqlite` of `project` and syncs it with the
    /// folder, reading only the files whose size or time changed. A cache that turns out to be
    /// damaged during that first sync is deleted and built again once.
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
        let identity = Identity::current(registry);
        let open = || -> Result<(Index, Update), IndexError> {
            let mut index = Index {
                root: project.root().to_path_buf(),
                registry: registry.clone(),
                store: Store::open(&file, &identity)?,
                identity: identity.clone(),
            };
            let update = index.sync()?;
            Ok((index, update))
        };
        // Opening reads only page 1 and `meta`; damage elsewhere shows in the sync. The failed
        // index is dropped by then, so the cache files are closed and can be deleted.
        match open() {
            Err(IndexError::Sqlite(error)) if store::is_damage(&error) => {
                store::discard(&file, &error)?;
                open()
            }
            other => other,
        }
    }

    /// An empty index in memory: [`Index::sync`] fills it from the folder, [`Index::apply`] from
    /// given texts.
    ///
    /// # Errors
    ///
    /// Fails when SQLite fails.
    pub fn in_memory(project: &Project, registry: &KindRegistry) -> Result<Index, IndexError> {
        let identity = Identity::current(registry);
        Ok(Index {
            root: project.root().to_path_buf(),
            registry: registry.clone(),
            store: Store::in_memory(&identity)?,
            identity,
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
    /// Another theos build kept rebuilding the cache while this write tried to own it.
    #[error(
        "the index cache {} keeps being rebuilt by another theos build: close one of the theos \
         processes",
        path.display()
    )]
    Contended {
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
    /// The cache is damaged and a file of it cannot be deleted, mostly because another process
    /// holds it open, so it cannot be replaced.
    #[error(
        "the index cache {} is damaged and in use: close the other theos processes ({source})",
        path.display()
    )]
    CacheInUse {
        /// The cache file.
        path: PathBuf,
        /// Why the delete failed.
        source: std::io::Error,
    },
    /// The walk of the project failed.
    #[error(transparent)]
    Walk(#[from] WalkError),
}

impl From<FactError> for IndexError {
    /// A lore rule failed because the store failed: the SQLite error, so that a damaged cache is
    /// still recognised as damaged.
    fn from(error: FactError) -> IndexError {
        match error {
            FactError::Sqlite(error) => IndexError::Sqlite(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_read_of_a_rule_is_a_failure_of_the_cache() {
        let error = IndexError::from(FactError::Sqlite(rusqlite::Error::QueryReturnedNoRows));
        assert!(matches!(
            error,
            IndexError::Sqlite(rusqlite::Error::QueryReturnedNoRows)
        ));
    }
}
