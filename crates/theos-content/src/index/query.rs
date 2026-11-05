//! The basic queries of the index. Each one reads the store; nothing is cached.

use rusqlite::types::Type;
use rusqlite::{Connection, Params, Row};

use crate::finding::{Finding, Severity};
use crate::index::slots::Slot;
use crate::index::{Index, IndexError};
use crate::text::{Position, Span};

/// What the index knows of a record at a glance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordSummary {
    /// The id (the file stem).
    pub id: String,
    /// The project-relative path with `/`.
    pub path: String,
    /// The kind, when the front matter names a registered one.
    pub kind: Option<String>,
    /// The text of the `title` scalar.
    pub title: Option<String>,
    /// The text of the `status` scalar.
    pub status: Option<String>,
}

/// One link of a record, with the record it sits in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkRef {
    /// The id of the record the link sits in.
    pub source_id: String,
    /// The path of that record.
    pub source_path: String,
    /// The place the link sits in.
    pub slot: Slot,
    /// The id the link names.
    pub target: String,
    /// The text after `#` in a body link, such as a beat number.
    pub fragment: Option<String>,
    /// The scalar in the front matter, or the whole `[[...]]` in the body.
    pub span: Span,
    /// Line and column of the span start.
    pub position: Position,
}

const LINK_COLUMNS: &str = "SELECT source_id, path, slot, target, fragment, span_start, span_end,                             line, col FROM links";

const FINDING_COLUMNS: &str =
    "SELECT path, id, rule, severity, span_start, span_end, line, col, message FROM findings";

/// Findings without a position come first; `message` and `span_start` make the order total.
const FINDING_ORDER: &str = "ORDER BY path, line, col, rule, message, span_start";

const RECORD_COLUMNS: &str = "SELECT id, path, kind, title, status FROM records";

impl Index {
    /// Every finding, ordered by path, line, column and rule.
    ///
    /// # Errors
    ///
    /// Fails when SQLite fails.
    pub fn findings(&self) -> Result<Vec<Finding>, IndexError> {
        self.finding_rows(&format!("{FINDING_COLUMNS} {FINDING_ORDER}"), [])
    }

    /// The findings of one path, in the same order.
    ///
    /// # Errors
    ///
    /// Fails when SQLite fails.
    pub fn findings_for(&self, path: &str) -> Result<Vec<Finding>, IndexError> {
        self.finding_rows(
            &format!("{FINDING_COLUMNS} WHERE path = ?1 {FINDING_ORDER}"),
            [path],
        )
    }

    /// Every record, ordered by path.
    ///
    /// # Errors
    ///
    /// Fails when SQLite fails.
    pub fn records(&self) -> Result<Vec<RecordSummary>, IndexError> {
        self.summaries(&format!("{RECORD_COLUMNS} ORDER BY path"), [])
    }

    /// The records with this id: several when the id is duplicated.
    ///
    /// # Errors
    ///
    /// Fails when SQLite fails.
    pub fn record(&self, id: &str) -> Result<Vec<RecordSummary>, IndexError> {
        self.summaries(
            &format!("{RECORD_COLUMNS} WHERE id = ?1 ORDER BY path"),
            [id],
        )
    }

    /// The records of a kind, ordered by path.
    ///
    /// # Errors
    ///
    /// Fails when SQLite fails.
    pub fn records_of_kind(&self, kind: &str) -> Result<Vec<RecordSummary>, IndexError> {
        self.summaries(
            &format!("{RECORD_COLUMNS} WHERE kind = ?1 ORDER BY path"),
            [kind],
        )
    }

    /// The links whose target is `id`, ordered by source path and position.
    ///
    /// # Errors
    ///
    /// Fails when SQLite fails.
    pub fn backlinks(&self, id: &str) -> Result<Vec<LinkRef>, IndexError> {
        self.link_rows(
            &format!("{LINK_COLUMNS} WHERE target = ?1 ORDER BY path, span_start"),
            [id],
        )
    }

    /// The backlinks of `id` in the slots `parent`, `concept` and `map`: the records filed under
    /// it.
    ///
    /// # Errors
    ///
    /// Fails when SQLite fails.
    pub fn children(&self, id: &str) -> Result<Vec<LinkRef>, IndexError> {
        self.link_rows(
            &format!(
                "{LINK_COLUMNS} WHERE target = ?1                  AND slot IN ('field:parent', 'field:concept', 'field:map')                  ORDER BY path, span_start"
            ),
            [id],
        )
    }

    /// The links of the record at `path`, in file order.
    ///
    /// # Errors
    ///
    /// Fails when SQLite fails.
    pub fn links_from(&self, path: &str) -> Result<Vec<LinkRef>, IndexError> {
        self.link_rows(
            &format!("{LINK_COLUMNS} WHERE path = ?1 ORDER BY span_start"),
            [path],
        )
    }

    fn link_rows(&self, sql: &str, params: impl Params) -> Result<Vec<LinkRef>, IndexError> {
        let mut stmt = self.store.conn.prepare(sql)?;
        let rows = stmt.query_map(params, link)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    fn summaries(&self, sql: &str, params: impl Params) -> Result<Vec<RecordSummary>, IndexError> {
        let mut stmt = self.store.conn.prepare(sql)?;
        let rows = stmt.query_map(params, |row| {
            Ok(RecordSummary {
                id: row.get(0)?,
                path: row.get(1)?,
                kind: row.get(2)?,
                title: row.get(3)?,
                status: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    fn finding_rows(&self, sql: &str, params: impl Params) -> Result<Vec<Finding>, IndexError> {
        finding_rows_in(&self.store.conn, sql, params)
    }
}

/// The findings of one path on `conn`, in the order of [`Index::findings_for`].
pub(super) fn findings_in(conn: &Connection, path: &str) -> Result<Vec<Finding>, IndexError> {
    finding_rows_in(
        conn,
        &format!("{FINDING_COLUMNS} WHERE path = ?1 {FINDING_ORDER}"),
        [path],
    )
}

fn finding_rows_in(
    conn: &Connection,
    sql: &str,
    params: impl Params,
) -> Result<Vec<Finding>, IndexError> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params, finding)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// One `findings` row, in the column order of [`FINDING_COLUMNS`].
fn finding(row: &Row<'_>) -> rusqlite::Result<Finding> {
    let severity: String = row.get(3)?;
    let severity = Severity::parse(&severity).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            Type::Text,
            format!("unknown severity `{severity}`").into(),
        )
    })?;
    let span = match (offset(row, 4)?, offset(row, 5)?) {
        (Some(start), Some(end)) => Some(Span { start, end }),
        _ => None,
    };
    let position = match (offset(row, 6)?, offset(row, 7)?) {
        (Some(line), Some(column)) => Some(Position { line, column }),
        _ => None,
    };
    Ok(Finding {
        rule: row.get(2)?,
        severity,
        path: row.get(0)?,
        id: row.get(1)?,
        span,
        position,
        message: row.get(8)?,
    })
}

/// One `links` row, in the column order of [`LINK_COLUMNS`].
fn link(row: &Row<'_>) -> rusqlite::Result<LinkRef> {
    let slot: String = row.get(2)?;
    let slot = Slot::from_db(&slot).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            Type::Text,
            format!("unknown slot `{slot}`").into(),
        )
    })?;
    let need = |column| {
        offset(row, column)?.ok_or(rusqlite::Error::InvalidColumnType(
            column,
            "offset".to_owned(),
            Type::Null,
        ))
    };
    Ok(LinkRef {
        source_id: row.get(0)?,
        source_path: row.get(1)?,
        slot,
        target: row.get(3)?,
        fragment: row.get(4)?,
        span: Span {
            start: need(5)?,
            end: need(6)?,
        },
        position: Position {
            line: need(7)?,
            column: need(8)?,
        },
    })
}

/// A nullable integer column as an offset, line or column.
pub(super) fn offset(row: &Row<'_>, column: usize) -> rusqlite::Result<Option<usize>> {
    let Some(value) = row.get::<_, Option<i64>>(column)? else {
        return Ok(None);
    };
    usize::try_from(value)
        .map(Some)
        .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(column, value))
}
