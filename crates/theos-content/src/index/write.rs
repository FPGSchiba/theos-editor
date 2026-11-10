//! Replacing the rows of one path: delete everything the path has, then insert its new rows.
//!
//! A path has rows only while it has a `files` row: every write of a path's rows writes its
//! `files` row in the same transaction, and [`delete_all`] deletes that row with the others. A
//! path without a `files` row is new, so its rows are inserted without deleting first.
//!
//! Every statement here runs once per record or row, so each one comes from the connection's
//! statement cache ([`Connection::prepare_cached`]) and is compiled once per connection.

use rusqlite::{Connection, params};

use crate::finding::Finding;
use crate::index::resolve::id_fold;
use crate::index::rows::{RecordRows, Shape};
use crate::yaml::ScalarStyle;

/// The deletes of the rows of a record, one per table that holds them, keyed by its path
/// (`subject` in `deps`: the reads of its lore rules). The `search` row goes by the rowid that
/// the `records` row holds, so before the `records` row: an FTS5 table has no index on `path`,
/// and a delete by `path` would read every search row.
const DELETES: [&str; 9] = [
    "DELETE FROM search WHERE rowid = (SELECT search_rowid FROM records WHERE path = ?1)",
    "DELETE FROM findings WHERE path = ?1",
    "DELETE FROM deps WHERE subject = ?1",
    "DELETE FROM inworld WHERE path = ?1",
    "DELETE FROM notes WHERE path = ?1",
    "DELETE FROM beats WHERE path = ?1",
    "DELETE FROM links WHERE path = ?1",
    "DELETE FROM field_values WHERE path = ?1",
    "DELETE FROM records WHERE path = ?1",
];

/// Deletes every row of `path`, its `files` row included.
pub(crate) fn delete_all(conn: &Connection, path: &str) -> rusqlite::Result<()> {
    delete_rows(conn, path)?;
    conn.prepare_cached("DELETE FROM files WHERE path = ?1")?
        .execute([path])?;
    Ok(())
}

/// Replaces the rows of `path` (not its `files` row) with `rows`.
pub(crate) fn replace(conn: &Connection, path: &str, rows: &RecordRows) -> rusqlite::Result<()> {
    delete_rows(conn, path)?;
    insert(conn, path, rows)
}

fn delete_rows(conn: &Connection, path: &str) -> rusqlite::Result<()> {
    for sql in DELETES {
        conn.prepare_cached(sql)?.execute([path])?;
    }
    Ok(())
}

/// Inserts `rows` for `path`, which has no rows yet: it has no `files` row.
pub(crate) fn insert(conn: &Connection, path: &str, rows: &RecordRows) -> rusqlite::Result<()> {
    // The search row first: the `records` row holds its rowid.
    let search_rowid = match &rows.search {
        Some(search) => {
            let mut stmt = conn.prepare_cached(
                "INSERT INTO search (path, title, fields, body) VALUES (?1, ?2, ?3, ?4)",
            )?;
            Some(stmt.insert(params![path, search.title, search.fields, search.body])?)
        }
        None => None,
    };
    if let Some(row) = &rows.record {
        let mut stmt = conn.prepare_cached(
            "INSERT INTO records (path, id, id_fold, kind, title, status, front, body_start, \
             body_line, facts, search_rowid) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )?;
        stmt.execute(params![
            row.path,
            row.id,
            id_fold(&row.id),
            row.kind,
            row.title,
            row.status,
            row.front,
            opt(row.body_start),
            opt(row.body_line),
            row.facts,
            search_rowid
        ])?;
    }
    let mut stmt = conn.prepare_cached(
        "INSERT INTO field_values (path, field, entry, item, shape, text, style, span_start, \
         span_end, line, col, valid, search_line) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
    )?;
    for value in &rows.values {
        stmt.execute(params![
            path,
            value.field,
            value.entry,
            opt(value.item),
            shape(value.shape),
            value.text,
            value.style.map(style),
            int(value.span.start),
            int(value.span.end),
            int(value.position.line),
            int(value.position.column),
            value.valid,
            opt(value.search_line)
        ])?;
    }
    let source_id = rows.record.as_ref().map_or("", |row| row.id.as_str());
    let mut stmt = conn.prepare_cached(
        "INSERT INTO links (path, source_id, slot, beat, target, fragment, span_start, \
         span_end, line, col) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )?;
    for link in &rows.links {
        stmt.execute(params![
            path,
            source_id,
            link.slot.as_db(),
            link.beat,
            link.target,
            link.fragment,
            int(link.span.start),
            int(link.span.end),
            int(link.position.line),
            int(link.position.column)
        ])?;
    }
    insert_lore_rows(conn, path, rows)?;
    for finding in &rows.findings {
        insert_finding(conn, finding, "record")?;
    }
    Ok(())
}

/// Inserts the beats, notes and in-world segments of `rows` for `path`.
fn insert_lore_rows(conn: &Connection, path: &str, rows: &RecordRows) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare_cached(
        "INSERT INTO beats (path, number, name, span_start, span_end, line, when_token) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    for beat in &rows.beats {
        stmt.execute(params![
            path,
            beat.number,
            beat.name,
            int(beat.span.start),
            int(beat.span.end),
            int(beat.line),
            beat.when
        ])?;
    }
    let mut stmt = conn.prepare_cached(
        "INSERT INTO notes (path, id, number, state, resolved_by, version, span_start, span_end, line, col) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )?;
    for note in &rows.notes {
        stmt.execute(params![
            path,
            note.id,
            note.number,
            note.state,
            note.resolved_by,
            note.version,
            int(note.span.start),
            int(note.span.end),
            int(note.position.line),
            int(note.position.column)
        ])?;
    }
    let mut stmt = conn.prepare_cached(
        "INSERT INTO inworld (path, span_start, span_end, line, col, text) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for segment in &rows.inworld {
        stmt.execute(params![
            path,
            int(segment.span.start),
            int(segment.span.end),
            int(segment.position.line),
            int(segment.position.column),
            segment.text
        ])?;
    }
    Ok(())
}

/// Inserts one finding, made by the check `step` (`record`, `links`, `ids` or `lore`).
pub(crate) fn insert_finding(
    conn: &Connection,
    finding: &Finding,
    step: &str,
) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare_cached(
        "INSERT INTO findings (path, id, rule, severity, span_start, span_end, line, col, \
         message, step) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )?;
    stmt.execute(params![
        finding.path,
        finding.id,
        finding.rule,
        finding.severity.as_str(),
        opt(finding.span.map(|span| span.start)),
        opt(finding.span.map(|span| span.end)),
        opt(finding.position.map(|position| position.line)),
        opt(finding.position.map(|position| position.column)),
        finding.message,
        step
    ])?;
    Ok(())
}

fn shape(shape: Shape) -> &'static str {
    match shape {
        Shape::Scalar => "scalar",
        Shape::List => "list",
        Shape::Map => "map",
    }
}

fn style(style: ScalarStyle) -> &'static str {
    match style {
        ScalarStyle::Plain => "plain",
        ScalarStyle::SingleQuoted => "single",
        ScalarStyle::DoubleQuoted => "double",
    }
}

/// A count or offset as a SQLite integer; values beyond `i64` cannot occur in a file.
fn int(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

fn opt(n: Option<usize>) -> Option<i64> {
    n.map(int)
}
