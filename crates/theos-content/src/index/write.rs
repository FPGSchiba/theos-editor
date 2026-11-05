//! Replacing the rows of one path: delete everything the path has, then insert its new rows.

use rusqlite::{Connection, params};

use crate::finding::Finding;
use crate::index::rows::{RecordRows, Shape};
use crate::yaml::ScalarStyle;

/// The tables that hold the rows of a record, keyed by `path`.
const TABLES: [&str; 6] = [
    "search",
    "findings",
    "beats",
    "links",
    "field_values",
    "records",
];

/// Deletes every row of `path`, its `files` row included.
pub(crate) fn delete_all(conn: &Connection, path: &str) -> rusqlite::Result<()> {
    delete_rows(conn, path)?;
    conn.execute("DELETE FROM files WHERE path = ?1", [path])?;
    Ok(())
}

/// Replaces the rows of `path` (not its `files` row) with `rows`.
pub(crate) fn replace(conn: &Connection, path: &str, rows: &RecordRows) -> rusqlite::Result<()> {
    delete_rows(conn, path)?;
    insert(conn, path, rows)
}

fn delete_rows(conn: &Connection, path: &str) -> rusqlite::Result<()> {
    for table in TABLES {
        conn.execute(&format!("DELETE FROM {table} WHERE path = ?1"), [path])?;
    }
    Ok(())
}

fn insert(conn: &Connection, path: &str, rows: &RecordRows) -> rusqlite::Result<()> {
    if let Some(row) = &rows.record {
        conn.execute(
            "INSERT INTO records (path, id, id_fold, kind, title, status, front, body_start, \
             body_line, facts) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                row.path,
                row.id,
                row.id.to_lowercase(),
                row.kind,
                row.title,
                row.status,
                row.front,
                opt(row.body_start),
                opt(row.body_line),
                row.facts
            ],
        )?;
    }
    for value in &rows.values {
        conn.execute(
            "INSERT INTO field_values (path, field, entry, item, shape, text, style, span_start, \
             span_end, line, col, valid, search_line) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
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
            ],
        )?;
    }
    let source_id = rows.record.as_ref().map_or("", |row| row.id.as_str());
    for link in &rows.links {
        conn.execute(
            "INSERT INTO links (path, source_id, slot, beat, target, fragment, span_start, \
             span_end, line, col) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
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
            ],
        )?;
    }
    for beat in &rows.beats {
        conn.execute(
            "INSERT INTO beats (path, number, name, span_start, span_end, line) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                path,
                beat.number,
                beat.name,
                int(beat.span.start),
                int(beat.span.end),
                int(beat.line)
            ],
        )?;
    }
    for finding in &rows.findings {
        insert_finding(conn, finding, "record")?;
    }
    if let Some(search) = &rows.search {
        conn.execute(
            "INSERT INTO search (path, title, fields, body) VALUES (?1, ?2, ?3, ?4)",
            params![path, search.title, search.fields, search.body],
        )?;
    }
    Ok(())
}

/// Inserts one finding, made by the check `step` (`record`, `links` or `ids`).
pub(crate) fn insert_finding(
    conn: &Connection,
    finding: &Finding,
    step: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO findings (path, id, rule, severity, span_start, span_end, line, col, \
         message, step) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
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
        ],
    )?;
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
