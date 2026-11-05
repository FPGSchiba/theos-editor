//! Bringing the index in line with the folder or with given changes (spec §5.9, steps 1 to 3
//! and 5): compare, read what changed, parse and check it, replace its rows, commit.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io::ErrorKind;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::check::check_record;
use crate::finding::{Finding, Severity};
use crate::index::store::map_error;
use crate::index::{Index, IndexError, resolve, rows, write};
use crate::kinds::KindRegistry;
use crate::walk::{Walked, WalkedPath, is_record_path, mtime_ns, walk};

/// A file whose time lies within this many nanoseconds before it was hashed is racy.
const RACY_WINDOW_NS: i64 = 2_000_000_000;

/// One change to apply to the index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    /// A record with the given bytes.
    Content {
        /// The project-relative path with `/`.
        path: String,
        /// The bytes of the file.
        bytes: Vec<u8>,
    },
    /// A record that is gone.
    Removed {
        /// The project-relative path with `/`.
        path: String,
    },
}

impl Change {
    /// A record with the given text.
    #[must_use]
    pub fn text(path: impl Into<String>, text: impl Into<String>) -> Change {
        Change::Content {
            path: path.into(),
            bytes: text.into().into_bytes(),
        }
    }

    /// The path the change is about.
    #[must_use]
    pub fn path(&self) -> &str {
        match self {
            Change::Content { path, .. } | Change::Removed { path } => path,
        }
    }
}

/// What a sync, update or apply did; every list is sorted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Update {
    /// The files read from disk.
    pub read: Vec<String>,
    /// The records whose rows were replaced.
    pub reparsed: Vec<String>,
    /// The records that were removed.
    pub removed: Vec<String>,
    /// The records whose links were recomputed without being re-parsed.
    pub relinked: Vec<String>,
}

impl Update {
    fn sorted(mut self) -> Update {
        for list in [
            &mut self.read,
            &mut self.reparsed,
            &mut self.removed,
            &mut self.relinked,
        ] {
            list.sort();
            list.dedup();
        }
        self
    }
}

/// Where the bytes of a change come from: the three changes of spec §5.9.
pub(crate) enum Source {
    /// Read the file from disk.
    Disk,
    /// Use the given bytes.
    Given(Vec<u8>),
    /// The file is gone.
    Gone,
}

/// The `files` row of a path.
struct FileRow {
    size: Option<i64>,
    mtime_ns: Option<i64>,
    hash: String,
    racy: bool,
}

/// Size and time of a file, as the walk or a `stat` saw them.
#[derive(Clone, Copy)]
struct Stamp {
    size: u64,
    mtime_ns: i64,
}

/// The work of one transaction.
struct Syncer<'a> {
    conn: &'a Connection,
    registry: &'a KindRegistry,
    root: &'a Path,
    update: Update,
}

impl Index {
    /// Walks the project and brings the index in line with it: files whose size or time differ,
    /// and racy ones, are read; those whose bytes differ are parsed again; paths the walk no
    /// longer sees are removed. One transaction.
    ///
    /// # Errors
    ///
    /// Fails when the walk fails or SQLite fails; nothing is changed then.
    pub fn sync(&mut self) -> Result<Update, IndexError> {
        let walked = walk(&self.root)?;
        self.run(|work| work.sync(&walked))
    }

    /// Does what [`Index::sync`] does for the given project-relative paths only. A path that is
    /// not a record is ignored; one that no longer exists is removed.
    ///
    /// # Errors
    ///
    /// Fails when SQLite fails; nothing is changed then.
    pub fn update(&mut self, paths: &[&str]) -> Result<Update, IndexError> {
        self.run(|work| {
            for path in paths {
                work.change(path, Source::Disk)?;
            }
            Ok(())
        })
    }

    /// Commits the given changes. A content whose bytes equal the stored ones changes nothing;
    /// otherwise the record is parsed again, with no file times, so the next sync reads the file.
    /// A path that is not a record is ignored.
    ///
    /// # Errors
    ///
    /// Fails when SQLite fails; nothing is changed then.
    pub fn apply(&mut self, changes: &[Change]) -> Result<Update, IndexError> {
        self.run(|work| {
            for change in changes {
                match change {
                    Change::Content { path, bytes } => {
                        work.change(path, Source::Given(bytes.clone()))?;
                    }
                    Change::Removed { path } => work.change(path, Source::Gone)?,
                }
            }
            Ok(())
        })
    }

    /// Runs `work` in one `BEGIN IMMEDIATE` transaction and commits it.
    fn run(
        &mut self,
        work: impl FnOnce(&mut Syncer<'_>) -> Result<(), IndexError>,
    ) -> Result<Update, IndexError> {
        let path = self.store.path.clone();
        let busy = |error| match &path {
            Some(path) => map_error(error, path),
            None => IndexError::Sqlite(error),
        };
        let tx = self
            .store
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(busy)?;
        let mut syncer = Syncer {
            conn: &tx,
            registry: &self.registry,
            root: &self.root,
            update: Update::default(),
        };
        work(&mut syncer)?;
        syncer.resolve()?;
        let update = syncer.update.sorted();
        tx.commit()?;
        Ok(update)
    }
}

impl Syncer<'_> {
    /// Step 4: when anything changed, relink every record and recheck every id fold (plan ruling
    /// L26). Every record that was not re-parsed counts as relinked.
    fn resolve(&mut self) -> Result<(), IndexError> {
        if self.update.reparsed.is_empty() && self.update.removed.is_empty() {
            return Ok(());
        }
        let paths = resolve::record_paths(self.conn)?;
        resolve::relink(self.conn, self.registry, &paths)?;
        resolve::recheck_ids(self.conn, &resolve::id_folds(self.conn)?)?;
        self.update.relinked = paths
            .into_iter()
            .filter(|path| !self.update.reparsed.contains(path))
            .collect();
        Ok(())
    }

    /// Steps 1 and 2 for the whole project.
    fn sync(&mut self, walked: &[Walked]) -> Result<(), IndexError> {
        let mut seen = BTreeSet::new();
        for file in walked {
            let stamp = Stamp {
                size: file.size,
                mtime_ns: file.mtime_ns,
            };
            match &file.path {
                WalkedPath::Utf8(path) => {
                    seen.insert(path.clone());
                    self.on_disk(path, stamp)?;
                }
                WalkedPath::NotUtf8(path) => {
                    seen.insert(path.clone());
                    self.not_utf8(path, stamp)?;
                }
            }
        }
        let mut stmt = self.conn.prepare("SELECT path FROM files")?;
        let known = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        drop(stmt);
        for path in known.iter().filter(|path| !seen.contains(*path)) {
            self.remove(path)?;
        }
        Ok(())
    }

    /// One change of `update` or `apply`.
    fn change(&mut self, path: &str, source: Source) -> Result<(), IndexError> {
        if !is_record_path(path) {
            return Ok(());
        }
        match source {
            Source::Gone => self.remove(path),
            Source::Given(bytes) => self.settle(path, &bytes, None),
            Source::Disk => match std::fs::metadata(self.root.join(path)) {
                Ok(meta) if meta.is_file() => self.on_disk(
                    path,
                    Stamp {
                        size: meta.len(),
                        mtime_ns: mtime_ns(&meta),
                    },
                ),
                Ok(_) => self.remove(path),
                Err(error) if error.kind() == ErrorKind::NotFound => self.remove(path),
                Err(error) => self.unreadable(
                    path,
                    Stamp {
                        size: 0,
                        mtime_ns: 0,
                    },
                    &error,
                ),
            },
        }
    }

    /// Step 1 for one file: read it unless its size and time say it is unchanged.
    fn on_disk(&mut self, path: &str, stamp: Stamp) -> Result<(), IndexError> {
        if let Some(row) = self.file_row(path)?
            && !row.racy
            && row.size == i64::try_from(stamp.size).ok()
            && row.mtime_ns == Some(stamp.mtime_ns)
        {
            return Ok(());
        }
        match std::fs::read(self.root.join(path)) {
            Ok(bytes) => {
                self.update.read.push(path.to_owned());
                self.settle(path, &bytes, Some(stamp))
            }
            Err(error) if error.kind() == ErrorKind::NotFound => self.remove(path),
            Err(error) => self.unreadable(path, stamp, &error),
        }
    }

    /// Step 3 for one record whose `bytes` are known. `stamp` is the file's size and time, or
    /// `None` for given bytes, which leaves no times so that the next sync reads the file.
    fn settle(&mut self, path: &str, bytes: &[u8], stamp: Option<Stamp>) -> Result<(), IndexError> {
        let hash = sha256_hex(bytes);
        let same = self.file_row(path)?.is_some_and(|row| row.hash == hash);
        let racy = stamp.is_none_or(|stamp| is_racy(stamp.mtime_ns, now_ns()));
        if same {
            if stamp.is_some() {
                self.put_file(path, stamp, &hash, racy)?;
            }
            return Ok(());
        }
        let checked = check_record(self.registry, path, bytes);
        write::replace(self.conn, path, &rows::rows(path, checked, self.registry))?;
        self.put_file(path, stamp, &hash, racy)?;
        self.update.reparsed.push(path.to_owned());
        Ok(())
    }

    /// A file name that is not UTF-8: only a `record/path` finding, and a `files` row so that the
    /// next sync leaves it alone while its size and time stay.
    fn not_utf8(&mut self, path: &str, stamp: Stamp) -> Result<(), IndexError> {
        if self.file_row(path)?.is_some_and(|row| {
            row.size == i64::try_from(stamp.size).ok() && row.mtime_ns == Some(stamp.mtime_ns)
        }) {
            return Ok(());
        }
        write::delete_all(self.conn, path)?;
        write::insert_finding(self.conn, &path_finding(path), "record")?;
        self.put_file(path, Some(stamp), "", false)?;
        self.update.reparsed.push(path.to_owned());
        Ok(())
    }

    /// A file that exists but cannot be read: a `record/read` finding at 1:1 and a racy row, so
    /// that the next sync tries again.
    fn unreadable(
        &mut self,
        path: &str,
        stamp: Stamp,
        error: &std::io::Error,
    ) -> Result<(), IndexError> {
        write::delete_all(self.conn, path)?;
        write::insert_finding(self.conn, &read_finding(path, error), "record")?;
        self.put_file(path, Some(stamp), "", true)?;
        self.update.reparsed.push(path.to_owned());
        Ok(())
    }

    /// Step 2 for one path: delete its rows, when it has any.
    fn remove(&mut self, path: &str) -> Result<(), IndexError> {
        if self.file_row(path)?.is_some() {
            write::delete_all(self.conn, path)?;
            self.update.removed.push(path.to_owned());
        }
        Ok(())
    }

    fn file_row(&self, path: &str) -> Result<Option<FileRow>, IndexError> {
        Ok(self
            .conn
            .query_row(
                "SELECT size, mtime_ns, hash, racy FROM files WHERE path = ?1",
                [path],
                |row| {
                    Ok(FileRow {
                        size: row.get(0)?,
                        mtime_ns: row.get(1)?,
                        hash: row.get(2)?,
                        racy: row.get(3)?,
                    })
                },
            )
            .optional()?)
    }

    fn put_file(
        &self,
        path: &str,
        stamp: Option<Stamp>,
        hash: &str,
        racy: bool,
    ) -> Result<(), IndexError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO files (path, size, mtime_ns, hash, racy) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                path,
                stamp.and_then(|stamp| i64::try_from(stamp.size).ok()),
                stamp.map(|stamp| stamp.mtime_ns),
                hash,
                racy
            ],
        )?;
        Ok(())
    }
}

/// Whether a file with this time, hashed at `now_ns`, must be hashed again next time: its time
/// is within the window before the hash, or after it, or unknown (0).
fn is_racy(mtime_ns: i64, now_ns: i64) -> bool {
    mtime_ns == 0 || mtime_ns > now_ns.saturating_sub(RACY_WINDOW_NS)
}

fn now_ns() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_nanos()).ok())
        .unwrap_or(i64::MAX)
}

/// The SHA-256 of `bytes` as lower-case hex.
fn sha256_hex(bytes: &[u8]) -> String {
    let mut hex = String::new();
    for byte in Sha256::digest(bytes) {
        // Writing to a String cannot fail.
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// `record/path`: the file name is not UTF-8, so the record cannot be named. No span or position.
fn path_finding(path: &str) -> Finding {
    Finding {
        rule: "record/path".to_owned(),
        severity: Severity::Error,
        path: path.to_owned(),
        id: None,
        span: None,
        position: None,
        message: "the file name is not valid UTF-8; rename the file".to_owned(),
    }
}

/// `record/read`: the file exists but could not be read; at 1:1 with the OS message.
fn read_finding(path: &str, error: &std::io::Error) -> Finding {
    Finding {
        rule: "record/read".to_owned(),
        severity: Severity::Error,
        path: path.to_owned(),
        id: path
            .rsplit('/')
            .next()
            .map(|name| name.strip_suffix(".md").unwrap_or(name).to_owned()),
        span: Some(crate::text::Span::default()),
        position: Some(crate::text::Position { line: 1, column: 1 }),
        message: format!("the file cannot be read: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_racy_window() {
        let now = 10_000_000_000;
        assert!(is_racy(now, now));
        assert!(is_racy(now + 1, now));
        assert!(is_racy(now - 1_999_999_999, now));
        assert!(!is_racy(now - 2_000_000_000, now));
        assert!(!is_racy(now - 60_000_000_000, now));
        assert!(is_racy(0, now));
    }

    #[test]
    fn the_hash_is_sha256_hex() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn a_read_error_is_a_finding_at_one_one() {
        let error = std::io::Error::new(ErrorKind::PermissionDenied, "denied");
        let finding = read_finding("things/a.md", &error);
        assert_eq!(
            finding.to_string(),
            "things/a.md:1:1: error record/read: the file cannot be read: denied"
        );
        assert_eq!(finding.id.as_deref(), Some("a"));
    }

    #[test]
    fn a_bad_file_name_is_a_finding_without_position() {
        let finding = path_finding("x\u{fffd}.md");
        assert!(finding.position.is_none() && finding.span.is_none() && finding.id.is_none());
        assert_eq!(finding.rule, "record/path");
    }

    #[test]
    fn changes_know_their_path() {
        let removed = Change::Removed {
            path: "a.md".to_owned(),
        };
        assert_eq!(removed.path(), "a.md");
        assert_eq!(Change::text("b.md", "x").path(), "b.md");
    }
}
