//! The facts of the lore step from the store (spec §5.5, L8): [`ById`] fetches them as rules
//! ask, [`Bulk`] loads all of them with one query per table. Both answer every question alike:
//! rows of one path keep the order they were written in (their rowid), records come by path, and
//! links that name an id by source path.
//!
//! Input files come from the `inputs` table (spec §5.4).

use std::collections::HashMap;

use rusqlite::types::Type;
use rusqlite::{Connection, Row};

use crate::index::inputs;
use crate::index::query::offset;
use crate::index::slots::Slot;
use crate::lore::{
    FactBeat, FactError, FactLink, FactNote, FactSource, FactValue, InLink, InputFacts,
    RecordFacts, Segment,
};
use crate::text::{Position, Span};

/// The records with their file hash. Every row of a record has a `files` row (see `write`).
const RECORDS: &str = "SELECT records.path, records.id, records.kind, files.hash FROM records \
     JOIN files ON files.path = records.path";

/// The queries of the rows of a record, each with `path` first.
const VALUES: &str = "SELECT path, field, entry, item, text, span_start, span_end, line, col, \
     valid FROM field_values";
const LINKS: &str = "SELECT path, source_id, slot, beat, target, fragment, span_start, span_end, \
     line, col FROM links";
const BEATS: &str = "SELECT path, number, when_token FROM beats";
const NOTES: &str = "SELECT path, id, state, span_start, span_end, line, col FROM notes";
const INWORLD: &str = "SELECT path, span_start, span_end, line, col, text FROM inworld";

impl crate::index::Index {
    /// A lore context over the store that fetches facts by id, for tests.
    #[cfg(test)]
    pub(crate) fn lore_context(&self) -> crate::lore::Context<'_> {
        crate::lore::Context::new(Box::new(ById::new(&self.store.conn)), &self.registry)
    }
}

/// The facts of the store, fetched by id as rules ask; the context keeps each answer.
pub(crate) struct ById<'c> {
    conn: &'c Connection,
}

impl<'c> ById<'c> {
    /// A source over `conn`.
    pub(crate) fn new(conn: &'c Connection) -> ById<'c> {
        ById { conn }
    }

    /// The rows of `path` that `sql` selects, in the order they were written.
    fn of_path<T>(
        &self,
        sql: &str,
        path: &str,
        read: fn(&Row<'_>) -> rusqlite::Result<T>,
    ) -> rusqlite::Result<Vec<T>> {
        let mut stmt = self
            .conn
            .prepare_cached(&format!("{sql} WHERE path = ?1 ORDER BY rowid"))?;
        stmt.query_map([path], read)?.collect()
    }
}

impl FactSource for ById<'_> {
    fn records(&mut self, id: &str) -> Result<Vec<RecordFacts>, FactError> {
        let mut stmt = self.conn.prepare_cached(&format!(
            "{RECORDS} WHERE records.id = ?1 ORDER BY records.path"
        ))?;
        let heads = stmt.query_map([id], head)?.collect::<Result<Vec<_>, _>>()?;
        let mut found = Vec::with_capacity(heads.len());
        for mut record in heads {
            record.values = self.of_path(VALUES, &record.path, value)?;
            record.links = self
                .of_path(LINKS, &record.path, link)?
                .into_iter()
                .flatten()
                .map(|(_, link)| link)
                .collect();
            record.beats = self.of_path(BEATS, &record.path, beat)?;
            record.notes = self.of_path(NOTES, &record.path, note)?;
            record.inworld = self.of_path(INWORLD, &record.path, segment)?;
            found.push(record);
        }
        Ok(found)
    }

    fn linking_to(&mut self, id: &str) -> Result<Vec<InLink>, FactError> {
        let mut stmt = self
            .conn
            .prepare_cached(&format!("{LINKS} WHERE target = ?1 ORDER BY path, rowid"))?;
        let rows = stmt.query_map([id], |row| Ok((row.get::<_, String>(0)?, link(row)?)))?;
        let mut found = Vec::new();
        for row in rows {
            if let (path, Some((source_id, link))) = row? {
                found.push(in_link(path, source_id, &link));
            }
        }
        Ok(found)
    }

    fn input(&mut self, path: &str) -> Result<Option<InputFacts>, FactError> {
        Ok(inputs::facts(self.conn, path)?)
    }
}

/// Every fact of the store, loaded at once: a cold build makes no query per read (CE-12.1).
pub(crate) struct Bulk {
    /// The records by id, each list ordered by path.
    records: HashMap<String, Vec<RecordFacts>>,
    /// The links by the id they name, ordered by source path.
    inbound: HashMap<String, Vec<InLink>>,
    /// The input files by path.
    inputs: HashMap<String, InputFacts>,
}

impl Bulk {
    /// Loads every fact of the store on `conn`.
    pub(crate) fn load(conn: &Connection) -> Result<Bulk, FactError> {
        let mut by_path: HashMap<String, RecordFacts> = HashMap::new();
        let mut stmt = conn.prepare(RECORDS)?;
        for record in stmt.query_map([], head)? {
            let record = record?;
            by_path.insert(record.path.clone(), record);
        }
        for (path, value) in all(conn, VALUES, value)? {
            if let Some(record) = by_path.get_mut(&path) {
                record.values.push(value);
            }
        }
        let mut inbound: HashMap<String, Vec<InLink>> = HashMap::new();
        for (path, link) in all(conn, LINKS, link)? {
            let Some((source_id, link)) = link else {
                continue;
            };
            inbound
                .entry(link.target.clone())
                .or_default()
                .push(in_link(path.clone(), source_id, &link));
            if let Some(record) = by_path.get_mut(&path) {
                record.links.push(link);
            }
        }
        for (path, beat) in all(conn, BEATS, beat)? {
            if let Some(record) = by_path.get_mut(&path) {
                record.beats.push(beat);
            }
        }
        for (path, note) in all(conn, NOTES, note)? {
            if let Some(record) = by_path.get_mut(&path) {
                record.notes.push(note);
            }
        }
        for (path, segment) in all(conn, INWORLD, segment)? {
            if let Some(record) = by_path.get_mut(&path) {
                record.inworld.push(segment);
            }
        }
        // Stable sorts: the links of one source keep the order they were written in.
        for links in inbound.values_mut() {
            links.sort_by(|a, b| a.source_path.cmp(&b.source_path));
        }
        let mut records: HashMap<String, Vec<RecordFacts>> = HashMap::new();
        for record in by_path.into_values() {
            records.entry(record.id.clone()).or_default().push(record);
        }
        for list in records.values_mut() {
            list.sort_by(|a, b| a.path.cmp(&b.path));
        }
        let inputs = inputs::all_facts(conn)?
            .into_iter()
            .map(|input| (input.path.clone(), input))
            .collect();
        Ok(Bulk {
            records,
            inbound,
            inputs,
        })
    }
}

impl FactSource for Bulk {
    fn records(&mut self, id: &str) -> Result<Vec<RecordFacts>, FactError> {
        Ok(self.records.get(id).cloned().unwrap_or_default())
    }

    fn linking_to(&mut self, id: &str) -> Result<Vec<InLink>, FactError> {
        Ok(self.inbound.get(id).cloned().unwrap_or_default())
    }

    fn input(&mut self, path: &str) -> Result<Option<InputFacts>, FactError> {
        Ok(self.inputs.get(path).cloned())
    }
}

/// Every row that `sql` selects, with its path, in the order the rows were written.
fn all<T>(
    conn: &Connection,
    sql: &str,
    read: fn(&Row<'_>) -> rusqlite::Result<T>,
) -> rusqlite::Result<Vec<(String, T)>> {
    let mut stmt = conn.prepare(&format!("{sql} ORDER BY rowid"))?;
    stmt.query_map([], |row| Ok((row.get(0)?, read(row)?)))?
        .collect()
}

/// A [`RECORDS`] row: a record without its rows yet.
fn head(row: &Row<'_>) -> rusqlite::Result<RecordFacts> {
    Ok(RecordFacts {
        path: row.get(0)?,
        id: row.get(1)?,
        kind: row.get(2)?,
        hash: row.get(3)?,
        values: Vec::new(),
        links: Vec::new(),
        beats: Vec::new(),
        notes: Vec::new(),
        inworld: Vec::new(),
    })
}

/// A [`VALUES`] row.
fn value(row: &Row<'_>) -> rusqlite::Result<FactValue> {
    let (span, position) = at(row, 5)?;
    Ok(FactValue {
        field: row.get(1)?,
        entry: row.get(2)?,
        item: offset(row, 3)?,
        text: row.get(4)?,
        span,
        position,
        valid: row.get(9)?,
    })
}

/// A [`LINKS`] row: the source id and the link, or none for a slot text this build does not
/// write.
fn link(row: &Row<'_>) -> rusqlite::Result<Option<(String, FactLink)>> {
    let slot: String = row.get(2)?;
    let Some(slot) = Slot::from_db(&slot) else {
        return Ok(None);
    };
    let (span, position) = at(row, 6)?;
    let link = FactLink {
        slot,
        beat: row.get(3)?,
        target: row.get(4)?,
        fragment: row.get(5)?,
        span,
        position,
    };
    Ok(Some((row.get(1)?, link)))
}

/// The link of the record at `path` with the id `source_id`, seen from the id it names.
fn in_link(path: String, source_id: String, link: &FactLink) -> InLink {
    InLink {
        source_id,
        source_path: path,
        slot: link.slot.clone(),
        beat: link.beat,
        span: link.span,
        position: link.position,
    }
}

/// A [`BEATS`] row.
fn beat(row: &Row<'_>) -> rusqlite::Result<FactBeat> {
    Ok(FactBeat {
        number: row.get(1)?,
        when: row.get(2)?,
    })
}

/// A [`NOTES`] row.
fn note(row: &Row<'_>) -> rusqlite::Result<FactNote> {
    let (span, position) = at(row, 3)?;
    Ok(FactNote {
        id: row.get(1)?,
        state: row.get(2)?,
        span,
        position,
    })
}

/// An [`INWORLD`] row.
fn segment(row: &Row<'_>) -> rusqlite::Result<Segment> {
    let (span, position) = at(row, 1)?;
    Ok(Segment {
        span,
        position,
        text: row.get(5)?,
    })
}

/// The span and position in the four columns from `first`: start, end, line and column.
fn at(row: &Row<'_>, first: usize) -> rusqlite::Result<(Span, Position)> {
    Ok((
        Span {
            start: count(row, first)?,
            end: count(row, first + 1)?,
        },
        Position {
            line: count(row, first + 2)?,
            column: count(row, first + 3)?,
        },
    ))
}

/// A `NOT NULL` offset, line or column.
fn count(row: &Row<'_>, column: usize) -> rusqlite::Result<usize> {
    offset(row, column)?.ok_or(rusqlite::Error::InvalidColumnType(
        column,
        "offset".to_owned(),
        Type::Null,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::indexed;

    const MARK: &str = "# synthetic fixture (P04), not canon\n";

    /// Records with every kind of fact: values and list items, links in fields, beats and the
    /// body, beats with `when::`, notes, in-world text, a duplicated id and a broken file.
    fn files() -> Vec<(&'static str, String)> {
        vec![
            (
                "lore/clues/C-950.md",
                format!(
                    "---\n{MARK}id: C-950\nkind: clue\ntitle: Synthetic clue A\nstatus: draft\n\
                     reveals: T-950\nmoves_to: hinted\nwhere: [f04-synthetic, f21-synthetic]\n\
                     gate: T1\nvector: synthetic vector\n---\nSynthetic clue text.\n"
                ),
            ),
            (
                "story/scripts/S-950.md",
                format!(
                    "---\n{MARK}id: S-950\nkind: script\ntitle: Synthetic script\n---\n\
                     ## Beat 1 — Start\nat:: [[f04-synthetic]]\nwhen:: T2\nreveals:: [[C-950]]\n\n\
                     > [!voice] [[synthetic-speaker]] · T1\n> Synthetic spoken words.\n\n\
                     ## Beat 2 — Next\nat:: [[f21-synthetic]]\n"
                ),
            ),
            (
                "concepts/synthetic-concept.md",
                format!(
                    "---\n{MARK}id: synthetic-concept\nkind: concept\ntitle: Synthetic concept\n\
                     for: [C-950]\n---\n> [!note] n1 · [[synthetic-concept]] · open\n> text\n"
                ),
            ),
            (
                "things/synthetic-twin.md",
                format!(
                    "---\n{MARK}id: synthetic-twin\nkind: thing\ntitle: Synthetic twin\n\
                     type: synthetic\n---\nSee [[C-950]].\n"
                ),
            ),
            (
                "dialogue/synthetic-twin.md",
                format!(
                    "---\n{MARK}id: synthetic-twin\nkind: thing\ntitle: Synthetic twin B\n\
                     type: synthetic\n---\nSee [[C-950]] and [[S-950#2]].\n"
                ),
            ),
            (
                "things/synthetic-broken.md",
                "synthetic text without a fence\n".to_owned(),
            ),
        ]
    }

    fn ids_and_targets(conn: &Connection) -> rusqlite::Result<(Vec<String>, Vec<String>)> {
        let list = |sql: &str| -> rusqlite::Result<Vec<String>> {
            conn.prepare(sql)?
                .query_map([], |row| row.get(0))?
                .collect()
        };
        Ok((
            list("SELECT DISTINCT id FROM records ORDER BY id")?,
            list("SELECT DISTINCT target FROM links ORDER BY target")?,
        ))
    }

    #[test]
    fn bulk_and_by_id_give_the_same_facts() -> Result<(), Box<dyn std::error::Error>> {
        let files = files();
        let files: Vec<(&str, &str)> = files.iter().map(|(p, t)| (*p, t.as_str())).collect();
        let (_dir, index) = indexed(&files)?;
        let conn = &index.store.conn;
        let mut bulk = Bulk::load(conn)?;
        let mut by_id = ById::new(conn);
        let (mut ids, mut targets) = ids_and_targets(conn)?;
        assert!(ids.len() >= 5 && targets.len() >= 4, "{ids:?} {targets:?}");
        ids.push("synthetic-nowhere".to_owned());
        targets.push("synthetic-nowhere".to_owned());
        for id in &ids {
            assert_eq!(bulk.records(id)?, by_id.records(id)?, "rec:{id}");
        }
        for id in &targets {
            assert_eq!(bulk.linking_to(id)?, by_id.linking_to(id)?, "in:{id}");
        }
        assert_eq!(bulk.input(".theos/settings.toml")?, None);
        assert_eq!(by_id.input(".theos/settings.toml")?, None);
        conn.execute(
            "INSERT INTO inputs (path, racy, hash, text) VALUES ('.theos/settings.toml', 0, 'h', 't')",
            [],
        )?;
        let mut bulk = Bulk::load(conn)?;
        let mut by_id = ById::new(conn);
        let stored = Some(InputFacts {
            path: ".theos/settings.toml".to_owned(),
            hash: "h".to_owned(),
            text: Some("t".to_owned()),
        });
        assert_eq!(bulk.input(".theos/settings.toml")?, stored);
        assert_eq!(by_id.input(".theos/settings.toml")?, stored);
        Ok(())
    }

    #[test]
    fn facts_hold_what_the_record_step_stored() -> Result<(), Box<dyn std::error::Error>> {
        let files = files();
        let files: Vec<(&str, &str)> = files.iter().map(|(p, t)| (*p, t.as_str())).collect();
        let (_dir, index) = indexed(&files)?;
        let mut source = ById::new(&index.store.conn);

        let script = source.records("S-950")?;
        let [script] = script.as_slice() else {
            return Err(format!("one script: {script:?}").into());
        };
        assert_eq!(script.kind.as_deref(), Some("script"));
        assert_eq!(script.hash.len(), 64);
        let beats: Vec<(u32, Option<&str>)> = script
            .beats
            .iter()
            .map(|beat| (beat.number, beat.when.as_deref()))
            .collect();
        assert_eq!(beats, [(1, Some("T2")), (2, None)]);
        let reveals = Slot::Beat("reveals".to_owned());
        let revealed: Vec<(&str, Option<u32>)> = script
            .links_in(&reveals)
            .map(|link| (link.target.as_str(), link.beat))
            .collect();
        assert_eq!(revealed, [("C-950", Some(1))]);
        assert_eq!(script.inworld.len(), 1);
        assert!(script.inworld[0].text.contains("Synthetic spoken words."));

        let clue = source.records("C-950")?;
        let where_items: Vec<Option<&str>> = clue
            .iter()
            .flat_map(|clue| clue.items("where"))
            .map(|item| item.text.as_deref())
            .collect();
        assert_eq!(where_items, [Some("f04-synthetic"), Some("f21-synthetic")]);

        let concept = source.records("synthetic-concept")?;
        let notes: Vec<(&str, &str)> = concept
            .iter()
            .flat_map(|concept| &concept.notes)
            .map(|note| (note.id.as_str(), note.state.as_str()))
            .collect();
        assert_eq!(notes, [("n1", "open")]);

        let twins: Vec<String> = source
            .records("synthetic-twin")?
            .into_iter()
            .map(|twin| twin.path)
            .collect();
        assert_eq!(
            twins,
            ["dialogue/synthetic-twin.md", "things/synthetic-twin.md"]
        );

        let broken = source.records("synthetic-broken")?;
        assert!(
            broken
                .iter()
                .all(|r| r.kind.is_none() && r.values.is_empty())
        );
        assert_eq!(broken.len(), 1);

        let inbound: Vec<String> = source
            .linking_to("C-950")?
            .iter()
            .map(|link| {
                format!(
                    "{} {} {} {:?}",
                    link.source_path,
                    link.source_id,
                    link.slot.as_db(),
                    link.beat
                )
            })
            .collect();
        assert_eq!(
            inbound,
            [
                "concepts/synthetic-concept.md synthetic-concept field:for None",
                "dialogue/synthetic-twin.md synthetic-twin mention None",
                "story/scripts/S-950.md S-950 beat:reveals Some(1)",
                "things/synthetic-twin.md synthetic-twin mention None",
            ]
        );
        Ok(())
    }
}
