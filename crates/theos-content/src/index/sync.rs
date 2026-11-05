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
use crate::index::identity::Identity;
use crate::index::store::{self, map_error};
use crate::index::{Index, IndexError, query, resolve, rows, write};
use crate::kinds::KindRegistry;
use crate::walk::{Walked, WalkedPath, is_ignored, is_listed, is_record_path, mtime_ns, walk};

/// A file whose time lies within this many nanoseconds before it was hashed is racy.
const RACY_WINDOW_NS: i64 = 2_000_000_000;

/// One change to apply to the index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    /// A record with the given bytes.
    Content {
        /// The project-relative path; a `\` is read as `/`.
        path: String,
        /// The bytes of the file.
        bytes: Vec<u8>,
    },
    /// A record that is gone.
    Removed {
        /// The project-relative path; a `\` is read as `/`.
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

/// What a sync, update or apply did; every list is sorted. A path is in at most one of
/// `reparsed` and `removed`: when one call changes a path more than once, its last change
/// decides.
///
/// When another build has rebuilt the cache file, a call first rebuilds it for this build (see
/// [`Index`]); the update reports only the call's own work.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Update {
    /// The files read from disk, also one that a later change of the same call removed.
    pub read: Vec<String>,
    /// The records whose rows were replaced and that still exist.
    pub reparsed: Vec<String>,
    /// The records that were removed and are gone.
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

/// What [`Index::preview`] found: the update the changes would make and the findings it would
/// leave on every re-parsed or relinked record and on every record given as content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preview {
    /// What an apply of the changes would do.
    pub update: Update,
    /// The findings of every re-parsed or relinked record and of every record given as content,
    /// also one whose bytes equal the stored ones, ordered like [`Index::findings`].
    pub findings: Vec<Finding>,
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
    /// The ids whose link facts changed, old and new (spec §5.9 step 4).
    affected: BTreeSet<String>,
    /// The id folds of every record whose rows were replaced or deleted, old and new: replacing
    /// the rows deletes the record's duplicate-id findings, so they are recomputed even when the
    /// link facts stayed.
    rechecked: BTreeSet<String>,
}

impl Index {
    /// Walks the project and brings the index in line with it: files whose size or time differ,
    /// and racy ones, are read; those whose bytes differ are parsed again; paths the walk no
    /// longer sees are removed. One transaction.
    ///
    /// A cache file that another theos build keeps rebuilding is rebuilt at most three times per
    /// call; then the call fails with [`IndexError::Contended`].
    ///
    /// # Errors
    ///
    /// Fails when the walk fails, SQLite fails or the cache is contended; nothing is changed then.
    pub fn sync(&mut self) -> Result<Update, IndexError> {
        let walked = walk(&self.root)?;
        self.run(|work| work.sync(&walked))
    }

    /// Does what [`Index::sync`] does for the given project-relative paths only. A path that is
    /// not a record is ignored; one that no longer exists is removed. So is one that the walk
    /// would not list: a path that a `.gitignore` hides, or one spelled unlike the file on disk
    /// (another case, a symbolic link). A `\` in a path is read as `/`, so paths joined on
    /// Windows work.
    ///
    /// A cache file that another theos build keeps rebuilding is rebuilt at most three times per
    /// call; then the call fails with [`IndexError::Contended`].
    ///
    /// # Errors
    ///
    /// Fails when SQLite fails or the cache is contended; nothing is changed then.
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
    /// A path that is not a record is ignored; one that a `.gitignore` hides is no record either,
    /// and what the index holds for it is removed. A `\` in a path is read as `/`, so paths
    /// joined on Windows work; the index stores the path with `/`.
    ///
    /// A cache file that another theos build keeps rebuilding is rebuilt at most three times per
    /// call; then the call fails with [`IndexError::Contended`].
    ///
    /// # Errors
    ///
    /// Fails when SQLite fails or the cache is contended; nothing is changed then.
    pub fn apply(&mut self, changes: &[Change]) -> Result<Update, IndexError> {
        self.run(|work| apply_changes(work, changes))
    }

    /// Does what [`Index::apply`] does, reads the findings of every re-parsed or relinked record
    /// and of every record given as content, and rolls everything back: the cache is not
    /// changed. The findings are those [`Index::apply`] would leave, ordered like
    /// [`Index::findings`]; a content equal to the stored bytes is not parsed again, and its
    /// stored findings are the ones it would leave. A `\` in a path is read as `/`, as in
    /// [`Index::apply`].
    ///
    /// A cache file that another theos build keeps rebuilding is rebuilt at most three times per
    /// call; then the call fails with [`IndexError::Contended`].
    ///
    /// # Errors
    ///
    /// Fails when SQLite fails or the cache is contended.
    pub fn preview(&mut self, changes: &[Change]) -> Result<Preview, IndexError> {
        let given = changes.iter().filter_map(|change| match change {
            Change::Content { path, .. } => Some(slashed(path)),
            Change::Removed { .. } => None,
        });
        let given: BTreeSet<String> = given.filter(|path| is_record_path(path)).collect();
        let (update, findings) = self.transact(
            |work| apply_changes(work, changes),
            |conn, update| {
                let mut paths: BTreeSet<&String> =
                    update.reparsed.iter().chain(&update.relinked).collect();
                paths.extend(&given);
                let mut findings = Vec::new();
                for path in paths {
                    findings.extend(query::findings_in(conn, path)?);
                }
                Ok(findings)
            },
            false,
        )?;
        Ok(Preview { update, findings })
    }

    /// Runs `work` in one `BEGIN IMMEDIATE` transaction and commits it.
    fn run(
        &mut self,
        work: impl FnOnce(&mut Syncer<'_>) -> Result<(), IndexError>,
    ) -> Result<Update, IndexError> {
        self.transact(work, |_, _| Ok(()), true)
            .map(|(update, ())| update)
    }

    /// Runs `work` in one `BEGIN IMMEDIATE` transaction, lets `read` look at the result, and
    /// then commits or, when `commit` is false, rolls back.
    ///
    /// A cache file whose `meta` is another build's (one that opened the same file since) is
    /// first rebuilt for this build and synced with the folder, in a transaction of its own that
    /// is committed also for a preview; then `work` runs in a new one. The update reports `work`
    /// only. After [`MAX_REBUILDS`] such rebuilds in one call the cache is still not this
    /// build's: another build keeps rebuilding it, and the call fails with
    /// [`IndexError::Contended`]. An in-memory cache belongs to this index alone and is not
    /// checked.
    fn transact<T>(
        &mut self,
        work: impl FnOnce(&mut Syncer<'_>) -> Result<(), IndexError>,
        read: impl FnOnce(&Connection, &Update) -> Result<T, IndexError>,
        commit: bool,
    ) -> Result<(Update, T), IndexError> {
        let path = self.store.path.clone();
        let busy = |error| match &path {
            Some(path) => map_error(error, path),
            None => IndexError::Sqlite(error),
        };
        let mut rebuilds = 0;
        loop {
            let tx = self
                .store
                .conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(busy)?;
            if let Some(file) = &path
                && !store::holds(&tx, &self.identity)?
            {
                if !may_rebuild(rebuilds) {
                    return Err(IndexError::Contended { path: file.clone() });
                }
                rebuilds += 1;
                tracing::warn!(
                    path = %file.display(),
                    "another build rebuilt the index cache; rebuilding it for this one"
                );
                rebuild(&tx, &self.registry, &self.root, &self.identity)?;
                tx.commit()?;
                // Checked again under the new write lock: another build may have rebuilt the
                // cache once more in between.
                continue;
            }
            let mut syncer = Syncer::new(&tx, &self.registry, &self.root);
            work(&mut syncer)?;
            syncer.resolve()?;
            let update = syncer.update.sorted();
            let read = read(&tx, &update)?;
            if commit {
                tx.commit()?;
            } else {
                tx.rollback()?;
            }
            return Ok((update, read));
        }
    }
}

/// How many times one write rebuilds a cache that another build took over again and again.
const MAX_REBUILDS: u32 = 3;

/// Whether a write that has rebuilt the cache `rebuilds` times in this call may rebuild it once
/// more, or must give up with [`IndexError::Contended`].
fn may_rebuild(rebuilds: u32) -> bool {
    rebuilds < MAX_REBUILDS
}

/// Rebuilds the cache in `tx` for the build of `identity`: the empty schema with its `meta`,
/// then every record of the folder at `root` (spec §5.8, identity).
fn rebuild(
    tx: &Connection,
    registry: &KindRegistry,
    root: &Path,
    identity: &Identity,
) -> Result<(), IndexError> {
    store::rebuild(tx, identity)?;
    let walked = walk(root)?;
    let mut syncer = Syncer::new(tx, registry, root);
    syncer.sync(&walked)?;
    syncer.resolve()
}

/// Step 3 for each given change.
fn apply_changes(work: &mut Syncer<'_>, changes: &[Change]) -> Result<(), IndexError> {
    for change in changes {
        match change {
            Change::Content { path, bytes } => work.change(path, Source::Given(bytes.clone()))?,
            Change::Removed { path } => work.change(path, Source::Gone)?,
        }
    }
    Ok(())
}

impl<'a> Syncer<'a> {
    /// The work of one transaction on `conn`, before any change.
    fn new(conn: &'a Connection, registry: &'a KindRegistry, root: &'a Path) -> Syncer<'a> {
        Syncer {
            conn,
            registry,
            root,
            update: Update::default(),
            affected: BTreeSet::new(),
            rechecked: BTreeSet::new(),
        }
    }

    /// Step 4: relink the re-parsed records and the records that link to an affected id
    /// (CE-12.2), and recheck the id folds of the re-parsed and removed records. `relinked` lists
    /// those that were relinked without being re-parsed.
    fn resolve(&mut self) -> Result<(), IndexError> {
        let untouched = self.affected.is_empty() && self.rechecked.is_empty();
        if untouched && self.update.reparsed.is_empty() {
            return Ok(());
        }
        let mut sources: BTreeSet<String> = self.update.reparsed.iter().cloned().collect();
        let mut stmt = self
            .conn
            .prepare_cached("SELECT DISTINCT path FROM links WHERE target = ?1")?;
        for id in &self.affected {
            for path in stmt.query_map([id], |row| row.get::<_, String>(0))? {
                sources.insert(path?);
            }
        }
        drop(stmt);
        for path in &self.update.removed {
            sources.remove(path);
        }
        resolve::relink(self.conn, self.registry, &sources)?;
        resolve::recheck_ids(self.conn, &self.rechecked)?;
        self.update.relinked = sources
            .into_iter()
            .filter(|path| !self.update.reparsed.contains(path))
            .collect();
        Ok(())
    }

    /// The id and link facts that the rows of `path` hold, none when it has no record row.
    fn old_facts(&self, path: &str) -> Result<Option<(String, String)>, IndexError> {
        Ok(self
            .conn
            .prepare_cached("SELECT id, facts FROM records WHERE path = ?1")?
            .query_row([path], |row| Ok((row.get(0)?, row.get(1)?)))
            .optional()?)
    }

    /// Step 2 of the affected set: before the rows of a path are replaced or deleted, note the
    /// old and `new` id folds for the recheck, and compare the `old` id and facts, which the rows
    /// hold, with the `new` ones; either is none when there is no record. A record that appears
    /// or disappears counts as changed.
    fn touch(&mut self, old: Option<(String, String)>, new: Option<(&str, &str)>) {
        self.rechecked
            .extend(old.iter().map(|(id, _)| resolve::id_fold(id)));
        self.rechecked
            .extend(new.iter().map(|(id, _)| resolve::id_fold(id)));
        if old.as_ref().map(|(_, facts)| facts.as_str()) == new.map(|(_, facts)| facts) {
            return;
        }
        self.affected.extend(old.map(|(id, _)| id));
        self.affected.extend(new.map(|(id, _)| id.to_owned()));
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

    /// One change of `update` or `apply`, at `path` with `\` read as `/`. A path that is no
    /// record by its name is ignored. One that the walk would not list, since a `.gitignore`
    /// hides it or, for a path read from disk, no file of that spelling is listed, is no record
    /// of the project either: the index holds nothing for it, so whatever it held is removed.
    fn change(&mut self, path: &str, source: Source) -> Result<(), IndexError> {
        let path = slashed(path);
        let path = path.as_str();
        if !is_record_path(path) {
            return Ok(());
        }
        match source {
            Source::Gone => self.remove(path),
            _ if is_ignored(self.root, path) => self.remove(path),
            Source::Given(bytes) => {
                let known = self.file_row(path)?;
                self.settle(path, &bytes, None, known.as_ref())
            }
            Source::Disk if !is_listed(self.root, path) => self.remove(path),
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
        let known = self.file_row(path)?;
        if let Some(row) = &known
            && !row.racy
            && row.size == i64::try_from(stamp.size).ok()
            && row.mtime_ns == Some(stamp.mtime_ns)
        {
            return Ok(());
        }
        match std::fs::read(self.root.join(path)) {
            Ok(bytes) => {
                self.update.read.push(path.to_owned());
                self.settle(path, &bytes, Some(stamp), known.as_ref())
            }
            Err(error) if error.kind() == ErrorKind::NotFound => self.remove(path),
            Err(error) => self.unreadable(path, stamp, &error),
        }
    }

    /// Step 3 for one record whose `bytes` are known. `stamp` is the file's size and time, or
    /// `None` for given bytes, which leaves no times so that the next sync reads the file.
    /// `known` is the path's `files` row, as the caller read it.
    fn settle(
        &mut self,
        path: &str,
        bytes: &[u8],
        stamp: Option<Stamp>,
        known: Option<&FileRow>,
    ) -> Result<(), IndexError> {
        let hash = sha256_hex(bytes);
        let same = known.is_some_and(|row| row.hash == hash);
        let racy = stamp.is_none_or(|stamp| is_racy(stamp.mtime_ns, now_ns()));
        if same {
            if stamp.is_some() {
                self.put_file(path, stamp, &hash, racy)?;
            }
            return Ok(());
        }
        let checked = check_record(self.registry, path, bytes);
        let new = rows::rows(path, checked, self.registry);
        let facts = new
            .record
            .as_ref()
            .map(|row| (row.id.as_str(), row.facts.as_str()));
        // A path without a `files` row has no rows (see `write`): nothing to look up or delete,
        // so a cold build only inserts.
        if known.is_some() {
            let old = self.old_facts(path)?;
            self.touch(old, facts);
            write::replace(self.conn, path, &new)?;
        } else {
            self.touch(None, facts);
            write::insert(self.conn, path, &new)?;
        }
        self.put_file(path, stamp, &hash, racy)?;
        self.reparsed(path);
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
        let old = self.old_facts(path)?;
        self.touch(old, None);
        write::delete_all(self.conn, path)?;
        write::insert_finding(self.conn, &path_finding(path), "record")?;
        self.put_file(path, Some(stamp), "", false)?;
        self.reparsed(path);
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
        let old = self.old_facts(path)?;
        self.touch(old, None);
        write::delete_all(self.conn, path)?;
        write::insert_finding(self.conn, &read_finding(path, error), "record")?;
        self.put_file(path, Some(stamp), "", true)?;
        self.reparsed(path);
        Ok(())
    }

    /// Step 2 for one path: delete its rows, when it has any. A change of it earlier in the same
    /// call no longer counts as a re-parse: the path is gone.
    fn remove(&mut self, path: &str) -> Result<(), IndexError> {
        if self.file_row(path)?.is_some() {
            let old = self.old_facts(path)?;
            self.touch(old, None);
            write::delete_all(self.conn, path)?;
            self.update.reparsed.retain(|other| other != path);
            self.update.removed.push(path.to_owned());
        }
        Ok(())
    }

    /// Notes that the rows of `path` were replaced. A removal of it earlier in the same call no
    /// longer counts: the path exists again, and step 4 must relink it.
    fn reparsed(&mut self, path: &str) {
        self.update.removed.retain(|other| other != path);
        self.update.reparsed.push(path.to_owned());
    }

    fn file_row(&self, path: &str) -> Result<Option<FileRow>, IndexError> {
        Ok(self
            .conn
            .prepare_cached("SELECT size, mtime_ns, hash, racy FROM files WHERE path = ?1")?
            .query_row([path], |row| {
                Ok(FileRow {
                    size: row.get(0)?,
                    mtime_ns: row.get(1)?,
                    hash: row.get(2)?,
                    racy: row.get(3)?,
                })
            })
            .optional()?)
    }

    fn put_file(
        &self,
        path: &str,
        stamp: Option<Stamp>,
        hash: &str,
        racy: bool,
    ) -> Result<(), IndexError> {
        let mut stmt = self.conn.prepare_cached(
            "INSERT OR REPLACE INTO files (path, size, mtime_ns, hash, racy) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        stmt.execute(rusqlite::params![
            path,
            stamp.and_then(|stamp| i64::try_from(stamp.size).ok()),
            stamp.map(|stamp| stamp.mtime_ns),
            hash,
            racy
        ])?;
        Ok(())
    }
}

/// A given project-relative path with every `\` replaced by `/`, so that a path joined by
/// `std::path` on Windows names the same record. No record path holds a `\`.
fn slashed(path: &str) -> String {
    path.replace('\\', "/")
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

    // AC: CE-12.4
    #[test]
    fn a_write_rebuilds_a_foreign_cache_at_most_three_times() {
        assert!((0..MAX_REBUILDS).all(may_rebuild));
        assert_eq!(MAX_REBUILDS, 3);
        assert!(!may_rebuild(3));
        assert!(!may_rebuild(4));
        let message = IndexError::Contended {
            path: "x/cache.db".into(),
        }
        .to_string();
        assert!(message.contains("x/cache.db") && message.contains("close one of the theos"));
    }

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
