//! The SQLite file of the cache: opening it, checking its identity, rebuilding it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, ErrorCode, TransactionBehavior};

use crate::index::IndexError;
use crate::index::identity::Identity;

/// The tables `schema.sql` creates, the virtual `search` first so it is dropped first.
const TABLES: [&str; 8] = [
    "search",
    "findings",
    "beats",
    "links",
    "field_values",
    "records",
    "files",
    "meta",
];

const SCHEMA: &str = include_str!("schema.sql");

/// How long a connection waits for another process holding the write lock.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// An open cache.
pub(crate) struct Store {
    pub(crate) conn: Connection,
    pub(crate) path: Option<PathBuf>,
}

impl Store {
    /// Opens the cache `file`, creating or rebuilding it when its identity differs or it is
    /// damaged.
    ///
    /// # Errors
    ///
    /// Fails when the folder cannot be created, the cache is busy or damaged and cannot be
    /// replaced, or SQLite fails.
    pub(crate) fn open(file: &Path, identity: &Identity) -> Result<Store, IndexError> {
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent).map_err(|source| IndexError::Cache {
                path: file.to_path_buf(),
                source,
            })?;
        }
        match Self::try_open(file, identity) {
            Err(IndexError::Sqlite(error)) if is_damage(&error) => {
                tracing::warn!(path = %file.display(), %error, "the index cache is damaged; rebuilding it");
                remove_cache(file)?;
                Self::try_open(file, identity)
            }
            other => other,
        }
    }

    /// An in-memory cache with the same schema, for tests and one-shot runs.
    ///
    /// # Errors
    ///
    /// Fails when SQLite fails.
    pub(crate) fn in_memory(identity: &Identity) -> Result<Store, IndexError> {
        let mut conn = Connection::open_in_memory()?;
        prepare(&mut conn, identity)?;
        Ok(Store { conn, path: None })
    }

    fn try_open(file: &Path, identity: &Identity) -> Result<Store, IndexError> {
        let mut conn = Connection::open(file).map_err(|e| map_error(e, file))?;
        conn.busy_timeout(BUSY_TIMEOUT)
            .map_err(|e| map_error(e, file))?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| map_error(e, file))?;
        prepare(&mut conn, identity).map_err(|error| match error {
            IndexError::Sqlite(error) => map_error(error, file),
            other => other,
        })?;
        Ok(Store {
            conn,
            path: Some(file.to_path_buf()),
        })
    }
}

/// Rebuilds the schema in one write transaction when the identity or the tables do not match.
fn prepare(conn: &mut Connection, identity: &Identity) -> Result<(), IndexError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let wanted: BTreeMap<String, String> = identity
        .pairs()
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect();
    if stored_meta(&tx)? != Some(wanted.clone()) || !has_every_table(&tx)? {
        for table in TABLES {
            tx.execute_batch(&format!("DROP TABLE IF EXISTS {table};"))?;
        }
        tx.execute_batch(SCHEMA)?;
        let mut insert = tx.prepare("INSERT INTO meta (key, value) VALUES (?1, ?2)")?;
        for (key, value) in &wanted {
            insert.execute([key, value])?;
        }
        drop(insert);
    }
    tx.commit()?;
    Ok(())
}

/// The `meta` rows, or `None` when the table is missing.
fn stored_meta(conn: &Connection) -> Result<Option<BTreeMap<String, String>>, rusqlite::Error> {
    let exists: bool = conn.query_row(
        "SELECT count(*) > 0 FROM sqlite_master WHERE type = 'table' AND name = 'meta'",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        return Ok(None);
    }
    let mut stmt = conn.prepare("SELECT key, value FROM meta")?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect::<Result<_, _>>().map(Some)
}

fn has_every_table(conn: &Connection) -> Result<bool, rusqlite::Error> {
    let present: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN \
         ('meta', 'files', 'records', 'field_values', 'links', 'beats', 'findings', 'search')",
        [],
        |row| row.get(0),
    )?;
    Ok(usize::try_from(present).is_ok_and(|n| n == TABLES.len()))
}

fn is_damage(error: &rusqlite::Error) -> bool {
    matches!(
        error.sqlite_error_code(),
        Some(ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt)
    )
}

/// Maps `SQLITE_BUSY` to [`IndexError::Busy`]; everything else stays a SQLite error.
pub(crate) fn map_error(error: rusqlite::Error, path: &Path) -> IndexError {
    if error.sqlite_error_code() == Some(ErrorCode::DatabaseBusy) {
        IndexError::Busy {
            path: path.to_path_buf(),
        }
    } else {
        IndexError::Sqlite(error)
    }
}

/// Deletes the cache file with its `-wal` and `-shm`; a file that cannot be deleted is held open.
fn remove_cache(file: &Path) -> Result<(), IndexError> {
    for suffix in ["", "-wal", "-shm"] {
        let mut name = file.as_os_str().to_os_string();
        name.push(suffix);
        match std::fs::remove_file(PathBuf::from(name)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {
                return Err(IndexError::CacheInUse {
                    path: file.to_path_buf(),
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::identity::Identity;

    const TABLES: [&str; 8] = [
        "meta",
        "files",
        "records",
        "field_values",
        "links",
        "beats",
        "findings",
        "search",
    ];

    fn identity() -> Identity {
        Identity {
            format: 1,
            crate_version: "0.0.0",
            schema_version: 1,
            registry: "r".to_owned(),
            rules: "u".to_owned(),
            source: "s",
        }
    }

    fn tables(conn: &rusqlite::Connection) -> Result<Vec<String>, rusqlite::Error> {
        let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table'")?;
        stmt.query_map([], |row| row.get(0))?.collect()
    }

    fn meta(conn: &rusqlite::Connection) -> Result<Vec<(String, String)>, rusqlite::Error> {
        let mut stmt = conn.prepare("SELECT key, value FROM meta ORDER BY key")?;
        stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect()
    }

    fn pairs(identity: &Identity) -> Vec<(String, String)> {
        identity
            .pairs()
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect()
    }

    fn assert_complete(store: &Store) -> Result<(), Box<dyn std::error::Error>> {
        let names = tables(&store.conn)?;
        for table in TABLES {
            assert!(names.iter().any(|n| n == table), "missing {table}");
        }
        Ok(())
    }

    #[test]
    fn a_new_cache_has_its_tables_and_identity() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let store = Store::open(&dir.path().join("a/b/index.db"), &identity())?;
        assert_complete(&store)?;
        let mut expected = pairs(&identity());
        expected.sort();
        assert_eq!(meta(&store.conn)?, expected);
        let mode: String = store
            .conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))?;
        assert_eq!(mode, "wal");
        Ok(())
    }

    #[test]
    fn in_memory_has_the_schema() -> Result<(), Box<dyn std::error::Error>> {
        let store = Store::in_memory(&identity())?;
        assert!(store.path.is_none());
        assert_complete(&store)
    }

    #[test]
    fn a_changed_identity_empties_the_cache() -> Result<(), Box<dyn std::error::Error>> {
        let edits: [fn(&mut Identity); 6] = [
            |i| i.format = 2,
            |i| i.crate_version = "9.9.9",
            |i| i.schema_version = 2,
            |i| i.registry = "other".to_owned(),
            |i| i.rules = "other".to_owned(),
            |i| i.source = "other",
        ];
        for apply in edits {
            let dir = tempfile::tempdir()?;
            let file = dir.path().join("index.db");
            let store = Store::open(&file, &identity())?;
            store.conn.execute(
                "INSERT INTO records (path, id, id_fold, facts) VALUES ('a.md', 'a', 'a', '')",
                [],
            )?;
            drop(store);
            let mut changed = identity();
            apply(&mut changed);
            let store = Store::open(&file, &changed)?;
            let rows: i64 = store
                .conn
                .query_row("SELECT count(*) FROM records", [], |r| r.get(0))?;
            assert_eq!(rows, 0);
            let mut expected = pairs(&changed);
            expected.sort();
            assert_eq!(meta(&store.conn)?, expected);
        }
        Ok(())
    }

    #[test]
    fn an_unchanged_identity_keeps_the_cache() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("index.db");
        let store = Store::open(&file, &identity())?;
        store.conn.execute(
            "INSERT INTO records (path, id, id_fold, facts) VALUES ('a.md', 'a', 'a', '')",
            [],
        )?;
        drop(store);
        let store = Store::open(&file, &identity())?;
        let rows: i64 = store
            .conn
            .query_row("SELECT count(*) FROM records", [], |r| r.get(0))?;
        assert_eq!(rows, 1);
        Ok(())
    }

    #[test]
    fn a_damaged_cache_is_rebuilt() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("index.db");
        std::fs::write(
            &file,
            "not a database, but long enough to be read as a header",
        )?;
        std::fs::write(dir.path().join("index.db-wal"), "stray")?;
        let store = Store::open(&file, &identity())?;
        assert_complete(&store)
    }

    #[test]
    fn a_cache_with_missing_tables_is_rebuilt() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("index.db");
        let store = Store::open(&file, &identity())?;
        for table in TABLES.iter().filter(|t| **t != "meta") {
            store.conn.execute(&format!("DROP TABLE {table}"), [])?;
        }
        drop(store);
        let store = Store::open(&file, &identity())?;
        assert_complete(&store)
    }

    #[test]
    fn a_cache_that_cannot_be_created_is_reported() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, "a file")?;
        let result = Store::open(&blocker.join("index.db"), &identity());
        assert!(matches!(result, Err(IndexError::Cache { .. })));
        Ok(())
    }

    #[test]
    fn a_busy_cache_is_reported() {
        let error = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
            None,
        );
        let path = std::path::Path::new("x.db");
        assert!(matches!(
            map_error(error, path),
            IndexError::Busy { path: p } if p == path
        ));
    }
}
