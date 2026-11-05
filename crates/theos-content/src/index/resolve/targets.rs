//! Loading what the rules read: the link rows of the sources and the records they name.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{Connection, OptionalExtension};

use super::{SourceRecord, TargetInfo, Targets};
use crate::index::IndexError;
use crate::index::rows::LinkRow;
use crate::index::slots::Slot;
use crate::kinds::KindRegistry;
use crate::text::{Position, Span};

/// The records at `paths`, with their links. A path without a `records` row is left out.
pub(super) fn load_sources(
    conn: &Connection,
    paths: &BTreeSet<String>,
) -> Result<Vec<SourceRecord>, IndexError> {
    let mut sources = Vec::new();
    for path in paths {
        let record = conn
            .prepare_cached("SELECT id, kind FROM records WHERE path = ?1")?
            .query_row([path], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
            })
            .optional()?;
        let Some((id, kind)) = record else { continue };
        sources.push(SourceRecord {
            path: path.clone(),
            id,
            kind,
            kind_at: kind_at(conn, path)?,
            links: links(conn, path)?,
        });
    }
    Ok(sources)
}

/// A stored count, offset, line or column; the store never holds a negative one.
fn offset(value: i64) -> usize {
    usize::try_from(value).unwrap_or_default()
}

fn kind_at(conn: &Connection, path: &str) -> Result<Option<(Span, Position)>, IndexError> {
    Ok(conn
        .prepare_cached(
            "SELECT span_start, span_end, line, col FROM field_values \
             WHERE path = ?1 AND field = 'kind' AND entry IS NULL AND item IS NULL",
        )?
        .query_row([path], |row| {
            Ok((
                Span {
                    start: offset(row.get(0)?),
                    end: offset(row.get(1)?),
                },
                Position {
                    line: offset(row.get(2)?),
                    column: offset(row.get(3)?),
                },
            ))
        })
        .optional()?)
}

fn links(conn: &Connection, path: &str) -> Result<Vec<LinkRow>, IndexError> {
    let mut stmt = conn.prepare_cached(
        "SELECT slot, beat, target, fragment, span_start, span_end, line, col FROM links \
         WHERE path = ?1 ORDER BY span_start",
    )?;
    let rows = stmt.query_map([path], |row| {
        Ok((
            row.get::<_, String>(0)?,
            LinkRow {
                slot: Slot::Mention,
                beat: row.get(1)?,
                target: row.get(2)?,
                fragment: row.get(3)?,
                span: Span {
                    start: offset(row.get(4)?),
                    end: offset(row.get(5)?),
                },
                position: Position {
                    line: offset(row.get(6)?),
                    column: offset(row.get(7)?),
                },
            },
        ))
    })?;
    let mut found = Vec::new();
    for row in rows {
        let (slot, mut link) = row?;
        // A slot text this build does not know cannot have been written by it.
        if let Some(slot) = Slot::from_db(&slot) {
            link.slot = slot;
            found.push(link);
        }
    }
    Ok(found)
}

/// The records that the links of `sources` name, by id.
pub(super) fn load_targets(
    conn: &Connection,
    registry: &KindRegistry,
    sources: &[SourceRecord],
) -> Result<Targets, IndexError> {
    let ids: BTreeSet<&str> = sources
        .iter()
        .flat_map(|source| source.links.iter().map(|link| link.target.as_str()))
        .collect();
    let narrow_fields = registry.narrow_fields();
    let mut targets = Targets::new();
    for id in ids {
        let mut stmt =
            conn.prepare_cached("SELECT path, kind FROM records WHERE id = ?1 ORDER BY path")?;
        let records = stmt
            .query_map([id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut infos = Vec::new();
        for (path, kind) in records {
            let narrow = narrow_values(conn, registry, &narrow_fields, &path, kind.as_deref())?;
            let beats = beat_numbers(conn, &path)?;
            infos.push(TargetInfo {
                path,
                kind,
                narrow,
                beats,
            });
        }
        targets.insert(id.to_owned(), infos);
    }
    Ok(targets)
}

/// The valid top-level scalars of the narrowing fields that the kind has.
fn narrow_values(
    conn: &Connection,
    registry: &KindRegistry,
    narrow_fields: &BTreeSet<&'static str>,
    path: &str,
    kind: Option<&str>,
) -> Result<BTreeMap<String, String>, IndexError> {
    let Some(spec) = kind.and_then(|kind| registry.kind(kind)) else {
        return Ok(BTreeMap::new());
    };
    let mut stmt = conn.prepare_cached(
        "SELECT field, text FROM field_values WHERE path = ?1 AND entry IS NULL \
         AND item IS NULL AND shape = 'scalar' AND valid = 1 AND text IS NOT NULL",
    )?;
    let rows = stmt
        .query_map([path], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows
        .into_iter()
        .filter(|(field, _)| narrow_fields.contains(field.as_str()) && spec.field(field).is_some())
        .collect())
}

fn beat_numbers(conn: &Connection, path: &str) -> Result<BTreeSet<u32>, IndexError> {
    let mut stmt = conn.prepare_cached("SELECT number FROM beats WHERE path = ?1")?;
    let numbers = stmt
        .query_map([path], |row| row.get::<_, u32>(0))?
        .collect::<Result<_, _>>()?;
    Ok(numbers)
}
