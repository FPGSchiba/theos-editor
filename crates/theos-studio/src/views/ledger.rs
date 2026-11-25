//! The JSON views of the reveal ledger and of the Shared Formalism regeneration.

use serde::Serialize;
use theos_content::{BeatRef, LedgerEntry, TruthLedger};

/// The reveal ledger as JSON: one element per truth.
#[derive(Debug, Serialize)]
pub struct LedgerView(Vec<TruthView>);

/// One truth and the clues that reveal it.
#[derive(Debug, Serialize)]
pub struct TruthView {
    id: String,
    path: String,
    title: Option<String>,
    target: Option<String>,
    entries: Vec<EntryView>,
}

/// One clue of a truth.
#[derive(Debug, Serialize)]
pub struct EntryView {
    state: String,
    floor: Option<u32>,
    place: Option<String>,
    gate: Option<String>,
    clue: String,
    clue_path: String,
    placed_by: Vec<PlacedByView>,
}

/// A beat that reveals a clue.
#[derive(Debug, Serialize)]
pub struct PlacedByView {
    script: String,
    beat: u32,
}

/// The paths a Shared Formalism regeneration wrote, as JSON.
#[derive(Debug, Serialize)]
pub struct FormalismView {
    written: Vec<String>,
}

impl FormalismView {
    /// The view of the project-relative paths that were written.
    #[must_use]
    pub fn new(written: &[String]) -> FormalismView {
        FormalismView {
            written: written.to_vec(),
        }
    }
}

impl From<&[TruthLedger]> for LedgerView {
    fn from(ledger: &[TruthLedger]) -> Self {
        LedgerView(ledger.iter().map(TruthView::from).collect())
    }
}

impl From<&TruthLedger> for TruthView {
    fn from(truth: &TruthLedger) -> Self {
        TruthView {
            id: truth.id.clone(),
            path: truth.path.clone(),
            title: truth.title.clone(),
            target: truth.target.clone(),
            entries: truth.entries.iter().map(EntryView::from).collect(),
        }
    }
}

impl From<&LedgerEntry> for EntryView {
    fn from(entry: &LedgerEntry) -> Self {
        EntryView {
            state: entry.state.clone(),
            floor: entry.floor,
            place: entry.place.clone(),
            gate: entry.gate.clone(),
            clue: entry.clue.clone(),
            clue_path: entry.clue_path.clone(),
            placed_by: entry.placed_by.iter().map(PlacedByView::from).collect(),
        }
    }
}

impl From<&BeatRef> for PlacedByView {
    fn from(beat: &BeatRef) -> Self {
        PlacedByView {
            script: beat.script.clone(),
            beat: beat.beat,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_truth_has_every_key_and_absent_values_are_null() -> Result<(), serde_json::Error> {
        let truth = TruthLedger {
            id: "T-1".to_owned(),
            path: "lore/truths/T-1.md".to_owned(),
            title: None,
            target: None,
            entries: vec![LedgerEntry {
                state: "hinted".to_owned(),
                floor: None,
                place: None,
                gate: None,
                clue: "C-1".to_owned(),
                clue_path: "lore/clues/C-1.md".to_owned(),
                placed_by: vec![BeatRef {
                    script: "S-1".to_owned(),
                    beat: 2,
                }],
            }],
        };
        let value = serde_json::to_value(LedgerView::from(&[truth][..]))?;
        assert_eq!(
            value,
            json!([{
                "id": "T-1", "path": "lore/truths/T-1.md", "title": null, "target": null,
                "entries": [{
                    "state": "hinted", "floor": null, "place": null, "gate": null,
                    "clue": "C-1", "clue_path": "lore/clues/C-1.md",
                    "placed_by": [{ "script": "S-1", "beat": 2 }]
                }]
            }])
        );
        Ok(())
    }

    #[test]
    fn the_formalism_view_lists_the_written_paths() -> Result<(), serde_json::Error> {
        let value = serde_json::to_value(FormalismView::new(&["a.md".to_owned()]))?;
        assert_eq!(value, json!({ "written": ["a.md"] }));
        Ok(())
    }
}
