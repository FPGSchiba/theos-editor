//! Binary references, checked on request (spec §5.13).
//!
//! A binary reference is a valid value of a `BinaryRefs` field: `path@sha256:<hex>`, the path
//! relative to the binaries folder. The folder is per machine and outside the project, so these
//! findings are never stored with the others and the hash memo is not part of `dump()`.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::finding::{Finding, Severity};
use crate::index::query::offset;
use crate::index::store::map_error;
use crate::index::{Index, IndexError};
use crate::kinds::FieldType;
use crate::rules::{RuleClass, RuleInfo};
use crate::text::{Position, Span};
use crate::walk::mtime_ns;

const MISSING: &str = "binary/missing";
const HASH: &str = "binary/hash";

/// The rules of this module: both are fixed warnings.
pub(crate) const RULES: [RuleInfo; 2] = [
    RuleInfo {
        id: MISSING,
        severity: Severity::Warning,
        class: RuleClass::Fixed,
    },
    RuleInfo {
        id: HASH,
        severity: Severity::Warning,
        class: RuleClass::Fixed,
    },
];

/// How many bytes a file is read at a time while it is hashed (L13).
const CHUNK: usize = 64 * 1024;

/// What [`Index::binary_findings`] found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BinaryReport {
    /// The warnings, sorted by path, line, column and rule.
    pub findings: Vec<Finding>,
    /// The reference paths whose files were hashed in this call: those whose size or time
    /// differ from the memo. A path appears once.
    pub hashed: Vec<String>,
}

/// One binary reference, with where it sits.
struct Reference {
    path: String,
    id: String,
    span: Span,
    position: Position,
    /// The path before the last `@`.
    file: String,
    /// The hex digest after `sha256:`.
    hash: String,
}

impl Index {
    /// Checks the binary references of every record against the files under `binaries_root`.
    ///
    /// A file is stat-ed and hashed, streaming, only when its size or time differ from the memo
    /// kept in the cache for this folder; a path that leaves the folder, also through a link, is
    /// never read. Findings are `binary/missing` and `binary/hash`, both warnings, at the
    /// reference value. They are not stored with the other findings.
    ///
    /// The hashing holds no transaction: the new memo rows are written afterwards in one short
    /// one.
    ///
    /// # Errors
    ///
    /// Fails when SQLite fails or another process holds the cache.
    pub fn binary_findings(&mut self, binaries_root: &Path) -> Result<BinaryReport, IndexError> {
        let cache = self.store.path.clone();
        let busy = |error| match &cache {
            Some(path) => map_error(error, path),
            None => IndexError::Sqlite(error),
        };
        let fields: Vec<(&str, &str)> = self
            .registry
            .kinds()
            .flat_map(|kind| {
                kind.fields
                    .iter()
                    .filter(|field| field.ty == FieldType::BinaryRefs)
                    .map(|field| (kind.name, field.key))
            })
            .collect();
        let mut references = Vec::new();
        for (kind, field) in fields {
            references.extend(references_of(&self.store.conn, kind, field)?);
        }
        let root_key = binaries_root.to_string_lossy().into_owned();
        let mut files = Files {
            root: binaries_root,
            root_key: &root_key,
            canonical_root: std::fs::canonicalize(binaries_root).ok(),
            known: HashMap::new(),
            hashed: Vec::new(),
            fresh: Vec::new(),
        };
        let mut findings = Vec::new();
        for reference in &references {
            if let Some(found) = files.judge(&self.store.conn, reference)? {
                findings.push(found);
            }
        }
        if !files.fresh.is_empty() {
            // A lost memo row only means one more hash later.
            let tx = self
                .store
                .conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(busy)?;
            for row in &files.fresh {
                tx.execute(
                    "INSERT OR REPLACE INTO binary_memo (root, path, size, mtime_ns, hash) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![root_key, row.path, row.size, row.time, row.hash],
                )?;
            }
            tx.commit().map_err(busy)?;
        }
        findings.sort_by(|a, b| {
            let at = |f: &Finding| f.position.map_or((0, 0), |p| (p.line, p.column));
            (&a.path, at(a), &a.rule).cmp(&(&b.path, at(b), &b.rule))
        });
        Ok(BinaryReport {
            findings,
            hashed: files.hashed,
        })
    }
}

/// The valid entries of `field` in the records of `kind`.
fn references_of(conn: &Connection, kind: &str, field: &str) -> Result<Vec<Reference>, IndexError> {
    let mut stmt = conn.prepare_cached(
        "SELECT f.path, r.id, f.text, f.span_start, f.span_end, f.line, f.col \
         FROM field_values f JOIN records r ON r.path = f.path \
         WHERE r.kind = ?1 AND f.field = ?2 AND f.valid = 1 AND f.entry IS NOT NULL \
           AND f.text IS NOT NULL \
         ORDER BY f.path, f.span_start",
    )?;
    let rows = stmt.query_map([kind, field], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            Span {
                start: offset(row, 3)?.unwrap_or(0),
                end: offset(row, 4)?.unwrap_or(0),
            },
            Position {
                line: offset(row, 5)?.unwrap_or(0),
                column: offset(row, 6)?.unwrap_or(0),
            },
        ))
    })?;
    let mut found = Vec::new();
    for row in rows {
        let (path, id, text, span, position) = row?;
        // P03's type check guarantees `path@sha256:<hex>` for a valid value.
        let Some((file, digest)) = text.rsplit_once('@') else {
            continue;
        };
        found.push(Reference {
            path,
            id,
            span,
            position,
            file: file.to_owned(),
            hash: digest.strip_prefix("sha256:").unwrap_or(digest).to_owned(),
        });
    }
    Ok(found)
}

/// A memo row to write once the hashing is done.
struct MemoRow {
    path: String,
    size: i64,
    time: i64,
    hash: String,
}

/// The files one call looked at.
struct Files<'a> {
    root: &'a Path,
    root_key: &'a str,
    /// The binaries folder with links resolved; `None` when it cannot be.
    canonical_root: Option<PathBuf>,
    /// What each file looked at in this call came to: its hash, or the message of the
    /// `binary/missing` finding it gets.
    known: HashMap<String, Result<String, String>>,
    hashed: Vec<String>,
    fresh: Vec<MemoRow>,
}

impl Files<'_> {
    /// The finding for `reference`, or `None` when its file is there with the right hash.
    fn judge(
        &mut self,
        conn: &Connection,
        reference: &Reference,
    ) -> Result<Option<Finding>, IndexError> {
        let file = reference.file.as_str();
        if !self.known.contains_key(file) {
            let outcome = match inside(file) {
                Some(segments) => self.hash_of(conn, file, &segments)?,
                None => Err(leaves(file)),
            };
            self.known.insert(file.to_owned(), outcome);
        }
        Ok(match self.known.get(file) {
            Some(Ok(actual)) if *actual == reference.hash => None,
            Some(Ok(actual)) => Some(finding(
                reference,
                HASH,
                format!(
                    "the SHA-256 of `{file}` is {actual}, the reference says {}",
                    reference.hash
                ),
            )),
            Some(Err(message)) => Some(finding(reference, MISSING, message.clone())),
            None => None,
        })
    }

    /// The hash of the file at `segments` under the folder, from the memo when its size and
    /// time are unchanged; `Err` is the message of the `binary/missing` finding.
    fn hash_of(
        &mut self,
        conn: &Connection,
        file: &str,
        segments: &[&str],
    ) -> Result<Result<String, String>, IndexError> {
        debug_assert!(conn.is_autocommit(), "hashing must not hold a transaction");
        let mut full = self.root.to_path_buf();
        for segment in segments {
            if !is_one_normal_component(segment) {
                return Ok(Err(leaves(file)));
            }
            full.push(segment);
        }
        // Resolve links first and use only the resolved path from here on, so a link swapped in
        // later is never followed: the containment check, the stat and the read all see `real`.
        let real = match std::fs::canonicalize(&full) {
            Ok(real) => real,
            Err(error) => return Ok(Err(io_message(file, &error))),
        };
        if !self
            .canonical_root
            .as_ref()
            .is_some_and(|root| real.starts_with(root))
        {
            return Ok(Err(leaves(file)));
        }
        let meta = match std::fs::metadata(&real) {
            Ok(meta) => meta,
            Err(error) => return Ok(Err(io_message(file, &error))),
        };
        if !meta.is_file() {
            return Ok(Err(format!("`{file}` is a folder, not a file")));
        }
        let size = i64::try_from(meta.len()).unwrap_or(i64::MAX);
        let time = mtime_ns(&meta);
        let memo: Option<(i64, i64, String)> = conn
            .query_row(
                "SELECT size, mtime_ns, hash FROM binary_memo WHERE root = ?1 AND path = ?2",
                [self.root_key, file],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        if let Some((memo_size, memo_time, hash)) = memo
            && memo_size == size
            && memo_time == time
        {
            return Ok(Ok(hash));
        }
        let hash = match hash_file(&real) {
            Ok(hash) => hash,
            Err(error) => return Ok(Err(format!("`{file}` cannot be read: {error}"))),
        };
        self.fresh.push(MemoRow {
            path: file.to_owned(),
            size,
            time,
            hash: hash.clone(),
        });
        self.hashed.push(file.to_owned());
        Ok(Ok(hash))
    }
}

/// The message of the `binary/missing` finding for a file that cannot be reached.
fn io_message(file: &str, error: &std::io::Error) -> String {
    if error.kind() == std::io::ErrorKind::NotFound {
        format!("there is no file `{file}` in the binaries folder")
    } else {
        format!("`{file}` cannot be read: {error}")
    }
}

/// The message for a path that must not be read.
fn leaves(file: &str) -> String {
    format!("`{file}` leaves the binaries folder, so it is not read")
}

/// Whether `segment` is exactly one plain name for the platform's path parser.
fn is_one_normal_component(segment: &str) -> bool {
    let mut parts = Path::new(segment).components();
    matches!(
        (parts.next(), parts.next()),
        (Some(std::path::Component::Normal(_)), None)
    )
}

fn finding(reference: &Reference, rule: &str, message: String) -> Finding {
    Finding {
        rule: rule.to_owned(),
        severity: Severity::Warning,
        path: reference.path.clone(),
        id: Some(reference.id.clone()),
        span: Some(reference.span),
        position: Some(reference.position),
        message,
    }
}

/// The segments of `path` when it stays inside the binaries folder, `None` when it leaves it.
///
/// The path is split at `/` and `\` on every OS (L15, amended). It leaves the folder when it is
/// empty, starts with a separator, has a `..` segment, or has a `:` in any segment (a drive or
/// stream name on Windows).
fn inside(path: &str) -> Option<Vec<&str>> {
    let segments: Vec<&str> = path.split(['/', '\\']).collect();
    let first = segments.first().copied().unwrap_or("");
    if first.is_empty() || segments.iter().any(|s| *s == ".." || s.contains(':')) {
        return None;
    }
    Some(segments.into_iter().filter(|s| !s.is_empty()).collect())
}

/// The SHA-256 of the file at `path` as lower-case hex, read in [`CHUNK`]s.
fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; CHUNK];
    loop {
        match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => hasher.update(&buffer[..n]),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    let mut hex = String::new();
    for byte in hasher.finalize() {
        // Writing to a String cannot fail.
        let _ = write!(hex, "{byte:02x}");
    }
    Ok(hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    // AC: CE-15.4
    #[test]
    fn a_path_leaves_the_folder_the_same_way_on_every_os() {
        for leaving in [
            "",
            "../x.bin",
            "/x.bin",
            "\\x.bin",
            "C:/x.bin",
            "C:\\x.bin",
            "\\\\server\\x.bin",
            "a/../../x.bin",
            "a\\..\\x.bin",
            "..",
            "a/C:x.bin",
            "a/b:c.bin",
            "a\\b:c.bin",
        ] {
            assert_eq!(inside(leaving), None, "{leaving:?}");
        }
        assert_eq!(inside("boards/ok.bin"), Some(vec!["boards", "ok.bin"]));
        assert_eq!(inside("a\\b/c.bin"), Some(vec!["a", "b", "c.bin"]));
    }

    #[test]
    fn an_io_failure_is_not_reported_as_leaving_or_missing() {
        let denied = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        let message = io_message("a.bin", &denied);
        assert!(message.starts_with("`a.bin` cannot be read: "), "{message}");
        let gone = std::io::Error::from(std::io::ErrorKind::NotFound);
        assert_eq!(
            io_message("a.bin", &gone),
            "there is no file `a.bin` in the binaries folder"
        );
    }

    #[test]
    fn a_segment_is_one_plain_name() {
        assert!(is_one_normal_component("ok.bin"));
        assert!(!is_one_normal_component(".."));
        assert!(!is_one_normal_component("a/b"));
        #[cfg(windows)]
        assert!(!is_one_normal_component("C:x.bin"));
    }

    #[test]
    fn a_file_is_hashed_across_chunks() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let bytes = vec![7_u8; CHUNK * 2 + 5];
        let file = dir.path().join("big.bin");
        std::fs::write(&file, &bytes)?;
        assert_eq!(hash_file(&file)?, crate::index::sha256_hex(&bytes));
        assert!(hash_file(&dir.path().join("none.bin")).is_err());
        Ok(())
    }

    #[test]
    #[should_panic(expected = "hashing must not hold a transaction")]
    fn hashing_inside_a_transaction_is_refused() {
        let conn = Connection::open_in_memory().expect("open");
        conn.execute_batch("BEGIN").expect("begin");
        let mut files = Files {
            root: Path::new("."),
            root_key: ".",
            canonical_root: None,
            known: HashMap::new(),
            hashed: Vec::new(),
            fresh: Vec::new(),
        };
        let _ = files.hash_of(&conn, "a.bin", &["a.bin"]);
    }
}
