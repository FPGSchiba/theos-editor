//! The reveal ledger in the cache (spec §5.7): the lore step stores the rows of each truth, and
//! [`Index::ledger`] and [`Index::ledger_of`] read them back.

use rusqlite::{Connection, params};

use crate::index::{Index, IndexError};
use crate::lore::availability::Tier;
use crate::lore::ledger::Ledger;

/// A truth and its ledger.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TruthLedger {
    /// The id of the truth.
    pub id: String,
    /// The path of the truth.
    pub path: String,
    /// The text of its `title`.
    pub title: Option<String>,
    /// The text of its valid `target`.
    pub target: Option<String>,
    /// One entry per clue that reveals it, hinted first, then suspected, then confirmed, each by
    /// floor (unknown last) and clue id. The first entry of a state is its summary.
    pub entries: Vec<LedgerEntry>,
}

/// One clue of a truth.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerEntry {
    /// `hinted`, `suspected` or `confirmed`: what the clue does to the truth.
    pub state: String,
    /// The lowest floor of the clue's places; none when no place gives one.
    pub floor: Option<u32>,
    /// The place that gives the floor: a floor, or a band that counts as its first floor.
    pub place: Option<String>,
    /// The tier of the clue's `gate`.
    pub gate: Option<String>,
    /// The id of the clue.
    pub clue: String,
    /// The path of the clue.
    pub clue_path: String,
    /// The beats that reveal the clue, by script id and beat.
    pub placed_by: Vec<BeatRef>,
}

/// A beat of a script.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BeatRef {
    /// The id of the script.
    pub script: String,
    /// The beat number.
    pub beat: u32,
}

/// Replaces the rows of the truth at `path` with `ledger`.
pub(crate) fn store(conn: &Connection, path: &str, ledger: &Ledger) -> rusqlite::Result<()> {
    clear(conn, path)?;
    let mut rows = conn.prepare_cached(
        "INSERT INTO ledger (path, clue, clue_path, state, floor, place, gate) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    let mut beats = conn.prepare_cached(
        "INSERT INTO ledger_beats (path, clue_path, script, beat) VALUES (?1, ?2, ?3, ?4)",
    )?;
    for row in &ledger.rows {
        let floor = row.availability.floor.as_ref();
        rows.execute(params![
            path,
            row.clue,
            row.clue_path,
            row.state.as_str(),
            floor.map(|floor| floor.number),
            floor.map(|floor| floor.place.as_str()),
            row.availability.gate.map(Tier::as_str),
        ])?;
        for (script, beat) in &row.placed_by {
            beats.execute(params![path, row.clue_path, script, beat])?;
        }
    }
    Ok(())
}

/// Deletes the ledger rows of the truth at `path`.
pub(crate) fn clear(conn: &Connection, path: &str) -> rusqlite::Result<()> {
    for sql in [
        "DELETE FROM ledger WHERE path = ?1",
        "DELETE FROM ledger_beats WHERE path = ?1",
    ] {
        conn.prepare_cached(sql)?.execute([path])?;
    }
    Ok(())
}

const TRUTHS: &str = "SELECT r.id, r.path, r.title, \
     (SELECT v.text FROM field_values v WHERE v.path = r.path AND v.field = 'target' \
        AND v.entry IS NULL AND v.item IS NULL AND v.valid = 1) \
     FROM records r WHERE r.kind = 'truth'";

impl Index {
    /// The ledger of every truth, ordered by id, then path.
    ///
    /// # Errors
    ///
    /// Fails when SQLite fails.
    pub fn ledger(&self) -> Result<Vec<TruthLedger>, IndexError> {
        self.truth_ledgers(&format!("{TRUTHS} ORDER BY r.id, r.path"), [])
    }

    /// The ledger of the truth with the id `id`: one per record with that id, ordered by path;
    /// none when no truth has it.
    ///
    /// # Errors
    ///
    /// Fails when SQLite fails.
    pub fn ledger_of(&self, id: &str) -> Result<Vec<TruthLedger>, IndexError> {
        self.truth_ledgers(&format!("{TRUTHS} AND r.id = ?1 ORDER BY r.path"), [id])
    }

    fn truth_ledgers(
        &self,
        sql: &str,
        args: impl rusqlite::Params,
    ) -> Result<Vec<TruthLedger>, IndexError> {
        let conn = &self.store.conn;
        let mut truths = conn
            .prepare(sql)?
            .query_map(args, |row| {
                Ok(TruthLedger {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    title: row.get(2)?,
                    target: row.get(3)?,
                    entries: Vec::new(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        for truth in &mut truths {
            truth.entries = entries(conn, &truth.path)?;
        }
        Ok(truths)
    }
}

/// The entries of the truth at `path`, in the order of spec §5.7.
fn entries(conn: &Connection, path: &str) -> rusqlite::Result<Vec<LedgerEntry>> {
    let mut stmt = conn.prepare_cached(
        "SELECT state, floor, place, gate, clue, clue_path FROM ledger WHERE path = ?1 \
         ORDER BY CASE state WHEN 'hinted' THEN 0 WHEN 'suspected' THEN 1 ELSE 2 END, \
         floor IS NULL, floor, clue, clue_path",
    )?;
    let mut beats = conn.prepare_cached(
        "SELECT script, beat FROM ledger_beats WHERE path = ?1 AND clue_path = ?2 \
         ORDER BY script, beat",
    )?;
    let rows = stmt
        .query_map([path], |row| {
            Ok(LedgerEntry {
                state: row.get(0)?,
                floor: row.get(1)?,
                place: row.get(2)?,
                gate: row.get(3)?,
                clue: row.get(4)?,
                clue_path: row.get(5)?,
                placed_by: Vec::new(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut out = Vec::with_capacity(rows.len());
    for mut entry in rows {
        entry.placed_by = beats
            .query_map([path, entry.clue_path.as_str()], |row| {
                Ok(BeatRef {
                    script: row.get(0)?,
                    beat: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        out.push(entry);
    }
    Ok(out)
}
