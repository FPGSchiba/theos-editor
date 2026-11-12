//! The input files of the lore step (spec §5.4): files that are not records, whose subjects the
//! lore rules check. The index stores each one's size, time, hash and text, so that a sync reads
//! it only when it changed and a rule reads it from the store.

use rusqlite::{Connection, OptionalExtension};

use crate::lore::InputFacts;

/// The input files, project-relative with `/`.
pub(crate) const INPUTS: [&str; 2] = [".theos/settings.toml", "canon/formalism/_source.json"];

/// Whether `path` is an input file.
pub(crate) fn is_input(path: &str) -> bool {
    INPUTS.contains(&path)
}

/// The `inputs` row of a path, without its text.
pub(super) struct InputRow {
    pub(super) size: Option<i64>,
    pub(super) mtime_ns: Option<i64>,
    pub(super) hash: String,
    pub(super) racy: bool,
}

/// The `inputs` row of `path`, none when the project has no such input.
pub(super) fn row(conn: &Connection, path: &str) -> rusqlite::Result<Option<InputRow>> {
    conn.prepare_cached("SELECT size, mtime_ns, hash, racy FROM inputs WHERE path = ?1")?
        .query_row([path], |row| {
            Ok(InputRow {
                size: row.get(0)?,
                mtime_ns: row.get(1)?,
                hash: row.get(2)?,
                racy: row.get(3)?,
            })
        })
        .optional()
}

/// Writes the `inputs` row of `path`; `text` is none when the bytes are not UTF-8 (L17).
pub(super) fn put(
    conn: &Connection,
    path: &str,
    stamp: (Option<i64>, Option<i64>),
    hash: &str,
    text: Option<&str>,
    racy: bool,
) -> rusqlite::Result<()> {
    conn.prepare_cached(
        "INSERT OR REPLACE INTO inputs (path, size, mtime_ns, racy, hash, text) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?
    .execute(rusqlite::params![path, stamp.0, stamp.1, racy, hash, text])?;
    Ok(())
}

/// Replaces the size, time and racy flag of `path` and keeps its hash and text.
pub(super) fn restamp(
    conn: &Connection,
    path: &str,
    stamp: (Option<i64>, Option<i64>),
    racy: bool,
) -> rusqlite::Result<()> {
    conn.prepare_cached("UPDATE inputs SET size = ?2, mtime_ns = ?3, racy = ?4 WHERE path = ?1")?
        .execute(rusqlite::params![path, stamp.0, stamp.1, racy])?;
    Ok(())
}

/// Deletes everything the index holds for the input `path`: its row, its lore findings and its
/// reads.
pub(super) fn delete(conn: &Connection, path: &str) -> rusqlite::Result<()> {
    for sql in [
        "DELETE FROM findings WHERE path = ?1",
        "DELETE FROM deps WHERE subject = ?1",
        "DELETE FROM inputs WHERE path = ?1",
    ] {
        conn.prepare_cached(sql)?.execute([path])?;
    }
    if path == crate::settings::SETTINGS {
        super::configured::clear(conn)?;
    }
    Ok(())
}

/// The facts of the input `path`.
pub(super) fn facts(conn: &Connection, path: &str) -> rusqlite::Result<Option<InputFacts>> {
    conn.prepare_cached("SELECT path, hash, text FROM inputs WHERE path = ?1")?
        .query_row([path], fact)
        .optional()
}

/// The facts of every input.
pub(super) fn all_facts(conn: &Connection) -> rusqlite::Result<Vec<InputFacts>> {
    conn.prepare("SELECT path, hash, text FROM inputs")?
        .query_map([], fact)?
        .collect()
}

fn fact(row: &rusqlite::Row<'_>) -> rusqlite::Result<InputFacts> {
    Ok(InputFacts {
        path: row.get(0)?,
        hash: row.get(1)?,
        text: row.get(2)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_two_files_are_inputs() {
        assert!(is_input(".theos/settings.toml"));
        assert!(is_input("canon/formalism/_source.json"));
        assert!(!is_input("things/a.md"));
        assert!(!is_input(".theos/cache/index.db"));
    }
}
