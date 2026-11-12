//! The reveal ledger of a truth (spec §5.7): one row per clue that reveals it, with the clue's
//! availability and the beats that place it. The lore step stores the rows of every truth.

use std::collections::BTreeSet;

use crate::index::Slot;
use crate::lore::availability::{Availability, State, of_clue};
use crate::lore::reveals::{clues_of, moves_to};
use crate::lore::{Context, FactError};

/// One clue of a truth.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerRow {
    /// The id of the clue.
    pub clue: String,
    /// The path of the clue.
    pub clue_path: String,
    /// What the clue does to the truth (`moves_to`).
    pub state: State,
    /// The floor and the gate of the clue.
    pub availability: Availability,
    /// The beats that reveal the clue as (script id, beat number), by script id then beat.
    pub placed_by: Vec<(String, u32)>,
}

/// The rows of a truth in the order of spec §5.7: by state (hinted, suspected, confirmed), then
/// by floor with unknown floors last, then by clue id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ledger {
    /// The rows.
    pub rows: Vec<LedgerRow>,
}

/// The ledger of the truth `truth_id`: its clues are the records of kind `clue` whose `reveals`
/// link names it and whose `moves_to` is valid. Reads `in:<truth>`, `in:<clue>` and `rec:<clue>`
/// of each clue, and what [`of_clue`] reads.
///
/// # Errors
///
/// Fails when the store fails.
pub fn of_truth(ctx: &mut Context<'_>, truth_id: &str) -> Result<Ledger, FactError> {
    let mut rows = Vec::new();
    for clue in clues_of(ctx, truth_id)? {
        let Some(state) = moves_to(&clue) else {
            continue;
        };
        let availability = of_clue(ctx, &clue)?;
        let beat = Slot::Beat("reveals".to_owned());
        let placed_by: BTreeSet<(String, u32)> = ctx
            .linking_to(&clue.id)?
            .into_iter()
            .filter(|link| link.slot == beat)
            .filter_map(|link| Some((link.source_id, link.beat?)))
            .collect();
        rows.push(LedgerRow {
            clue: clue.id.clone(),
            clue_path: clue.path.clone(),
            state,
            availability,
            placed_by: placed_by.into_iter().collect(),
        });
    }
    rows.sort_by(|a, b| {
        let floor = |row: &LedgerRow| row.availability.floor.as_ref().map(|floor| floor.number);
        let unknown = |row: &LedgerRow| floor(row).is_none();
        (a.state, unknown(a), floor(a), &a.clue, &a.clue_path).cmp(&(
            b.state,
            unknown(b),
            floor(b),
            &b.clue,
            &b.clue_path,
        ))
    });
    Ok(Ledger { rows })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::with_context;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn clue(id: &str, to: &str, place: &str) -> (String, String) {
        (
            format!("lore/clues/{id}.md"),
            format!(
                "---\nid: {id}\nkind: clue\ntitle: t\nstatus: draft\nreveals: T-950\nmoves_to: {to}\nwhere: [{place}]\ngate: T1\nvector: v\n---\n"
            ),
        )
    }

    #[test]
    fn rows_come_in_state_floor_and_clue_order() -> TestResult {
        let mut owned = vec![
            (
                "world/synthetic-low.md".to_owned(),
                "---\nid: synthetic-low\nkind: band\ntitle: low\nparent: tower\nfloors: [3, 5]\n---\n".to_owned(),
            ),
            (
                "lore/truths/T-950.md".to_owned(),
                "---\nid: T-950\nkind: truth\ntitle: t\nstatus: draft\ntarget: confirmed\n---\n".to_owned(),
            ),
            (
                "story/scripts/S-950.md".to_owned(),
                "---\nid: S-950\nkind: script\ntitle: s\nstatus: draft\n---\n## Beat 2 — x\nreveals:: [[C-954]]\n\n## Beat 1 — y\nreveals:: [[C-954]]\n".to_owned(),
            ),
        ];
        for (id, to, place) in [
            ("C-953", "confirmed", "synthetic-low"),
            ("C-952", "hinted", "nowhere"),
            ("C-951", "hinted", "synthetic-low"),
            ("C-954", "hinted", "synthetic-low"),
            ("C-955", "bogus", "synthetic-low"),
        ] {
            owned.push(clue(id, to, place));
        }
        let files: Vec<(&str, &str)> = owned
            .iter()
            .map(|(p, t)| (p.as_str(), t.as_str()))
            .collect();
        with_context(&files, |ctx| {
            let ledger = of_truth(ctx, "T-950")?;
            let ids: Vec<&str> = ledger.rows.iter().map(|row| row.clue.as_str()).collect();
            assert_eq!(ids, ["C-951", "C-954", "C-952", "C-953"]);
            let placed = &ledger.rows[1].placed_by;
            assert_eq!(placed, &[("S-950".to_owned(), 1), ("S-950".to_owned(), 2)]);
            Ok::<_, FactError>(())
        })??;
        Ok(())
    }
}
