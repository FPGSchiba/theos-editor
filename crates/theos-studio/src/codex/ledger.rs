//! `codex::ledger_check`: the reveal ledger with marks, and the `lore/` findings on it.

use std::collections::HashSet;

use theos_content::{BeatRef, LedgerEntry, TruthLedger};

use super::lookup::Lookup;

use crate::check::counts;
use crate::views::check::FindingView;
use crate::views::project::{LedgerCheckView, MarkedBeatView, MarkedEntryView, MarkedTruthView};
use crate::{StudioError, Workspace};

/// The ledger of every truth, or of the truth `id`, each with its marks, and the findings whose
/// rule starts with `lore/` on the paths of those truths and of their clues.
///
/// # Errors
/// [`StudioError::UnknownRecord`] when no record has the `id`; [`StudioError::InvalidFilter`]
/// when the `id` names records but none is a truth; [`StudioError::Index`] when the index fails.
pub fn ledger_check(ws: &Workspace, id: Option<&str>) -> Result<LedgerCheckView, StudioError> {
    let ledger: Vec<TruthLedger> = match id {
        Some(id) => {
            let found = ws.index().ledger_of(id)?;
            if found.is_empty() {
                return Err(not_a_truth(ws, id)?);
            }
            found
        }
        None => ws.index().ledger()?,
    };
    let lookup = Lookup::load(ws)?;
    let paths: HashSet<&str> = ledger
        .iter()
        .flat_map(|truth| {
            std::iter::once(truth.path.as_str())
                .chain(truth.entries.iter().map(|entry| entry.clue_path.as_str()))
        })
        .collect();
    let findings: Vec<_> = ws
        .index()
        .findings()?
        .into_iter()
        .filter(|finding| {
            finding.rule.starts_with("lore/") && paths.contains(finding.path.as_str())
        })
        .collect();
    let (errors, warnings) = counts(&findings);
    Ok(LedgerCheckView {
        truths: ledger
            .iter()
            .map(|truth| marked_truth(truth, &lookup))
            .collect(),
        findings: findings.iter().map(FindingView::from).collect(),
        errors,
        warnings,
    })
}

/// The error for an `id` with no ledger: unknown when no record has it, else not a truth.
fn not_a_truth(ws: &Workspace, id: &str) -> Result<StudioError, StudioError> {
    let records = ws.index().record(id)?;
    Ok(match records.first() {
        None => StudioError::UnknownRecord {
            what: id.to_owned(),
        },
        Some(record) => StudioError::InvalidFilter {
            message: format!(
                "`{id}` is not a truth (kind {})",
                record.kind.as_deref().unwrap_or("unknown")
            ),
        },
    })
}

/// The view of `truth`: the marks of its path, and of each clue's path, place and script.
///
/// Each builder destructures its source exhaustively, so a field added to the ledger types in
/// `theos-content` is a compile error here until the view carries it.
pub(super) fn marked_truth(truth: &TruthLedger, lookup: &Lookup) -> MarkedTruthView {
    let TruthLedger {
        id,
        path,
        title,
        target,
        entries,
    } = truth;
    MarkedTruthView {
        id: id.clone(),
        path: path.clone(),
        title: title.clone(),
        target: target.clone(),
        entries: entries
            .iter()
            .map(|entry| marked_entry(entry, lookup))
            .collect(),
        mark: lookup.marks.of_path(path),
    }
}

fn marked_entry(entry: &LedgerEntry, lookup: &Lookup) -> MarkedEntryView {
    let LedgerEntry {
        state,
        floor,
        place,
        gate,
        clue,
        clue_path,
        placed_by,
    } = entry;
    MarkedEntryView {
        state: state.clone(),
        floor: *floor,
        place: place.as_deref().map(|id| lookup.target(id)),
        gate: gate.clone(),
        clue: clue.clone(),
        clue_path: clue_path.clone(),
        placed_by: placed_by
            .iter()
            .map(|beat| marked_beat(beat, lookup))
            .collect(),
        mark: lookup.marks.of_path(clue_path),
    }
}

fn marked_beat(beat: &BeatRef, lookup: &Lookup) -> MarkedBeatView {
    let BeatRef { script, beat } = beat;
    MarkedBeatView {
        script: lookup.target(script),
        beat: *beat,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TestResult, codex_workspace};

    // AC: CE-18.1
    // AC: CE-18.3
    #[test]
    fn ledger_check_marks_and_findings() -> TestResult {
        let (dir, workspace) = codex_workspace()?;
        let view = serde_json::to_value(ledger_check(&workspace, None)?)?;
        let truths = view["truths"].as_array().ok_or("no truths")?;
        let of = |id: &str| truths.iter().find(|t| t["id"] == id).ok_or("missing truth");
        assert_eq!(of("T-911")?["never"], true);
        assert_eq!(of("T-912")?["never"], false);
        let clues: Vec<&str> = of("T-912")?["entries"]
            .as_array()
            .ok_or("no entries")?
            .iter()
            .filter_map(|entry| entry["clue"].as_str())
            .collect();
        assert!(clues.contains(&"C-031"));
        assert_eq!(view["errors"], 0);

        std::fs::write(
            dir.path().join("lore/clues/C-934.md"),
            "---\n# synthetic fixture (P48), not canon\nid: C-934\nkind: clue\ntitle: Synthetic clue\nstatus: draft\nreveals: T-911\nmoves_to: hinted\nwhere: [fates-shrine]\ngate: T1\nvector: synthetic\n---\nText\n",
        )?;
        let workspace = Workspace::open(dir.path())?;
        let view = ledger_check(&workspace, None)?;
        let findings = serde_json::to_value(&view.findings)?;
        assert!(
            findings
                .as_array()
                .ok_or("no findings")?
                .iter()
                .any(|f| f["rule"] == "lore/never-confirm"),
            "{findings}"
        );
        assert!(view.errors >= 1);
        Ok(())
    }

    // AC: CE-18.3
    #[test]
    fn ledger_entries_name_marked_places_and_scripts() -> TestResult {
        let (_dir, workspace) = codex_workspace()?;
        let view = serde_json::to_value(ledger_check(&workspace, Some("T-912"))?)?;
        let entry = view["truths"][0]["entries"]
            .as_array()
            .and_then(|all| all.iter().find(|e| e["clue"] == "C-031"))
            .ok_or("no entry")?;
        assert_eq!(entry["place"]["id"], "f04-synthetic");
        assert_eq!(entry["place"]["exists"], true);
        assert_eq!(entry["place"]["never"], false);
        assert_eq!(entry["place"]["locked"], serde_json::json!([]));
        assert_eq!(entry["placed_by"][0]["script"]["id"], "S-004");
        assert_eq!(
            entry["placed_by"][0]["script"]["locked"],
            serde_json::json!([])
        );
        assert_eq!(entry["placed_by"][0]["beat"], 2);
        Ok(())
    }

    // AC: CE-18.1
    #[test]
    fn ledger_check_one_truth() -> TestResult {
        let (_dir, workspace) = codex_workspace()?;
        assert_eq!(ledger_check(&workspace, Some("T-912"))?.truths.len(), 1);
        assert!(matches!(
            ledger_check(&workspace, Some("no-such-record")),
            Err(StudioError::UnknownRecord { .. })
        ));
        let error = ledger_check(&workspace, Some("B-911"))
            .err()
            .ok_or("B-911")?;
        assert!(
            matches!(error, StudioError::InvalidFilter { .. }),
            "{error}"
        );
        assert!(
            error
                .to_string()
                .contains("`B-911` is not a truth (kind belief)"),
            "{error}"
        );
        Ok(())
    }

    // AC: CE-18.3
    #[test]
    fn ledger_clues_carry_their_marks() -> TestResult {
        let (dir, _) = codex_workspace()?;
        std::fs::write(
            dir.path().join("lore/clues/C-935.md"),
            "---
# synthetic fixture (P48), not canon
id: C-935
kind: clue
title: Synthetic clue
status: canon
reveals: T-912
moves_to: hinted
where: [fates-shrine]
gate: T1
vector: synthetic
---
Text
",
        )?;
        let workspace = Workspace::open(dir.path())?;
        let view = serde_json::to_value(ledger_check(&workspace, Some("T-912"))?)?;
        let entries = view["truths"][0]["entries"]
            .as_array()
            .ok_or("no entries")?;
        let entry = |clue: &str| entries.iter().find(|e| e["clue"] == clue).ok_or("no entry");
        assert_eq!(entry("C-935")?["locked"], serde_json::json!(["canon"]));
        assert_eq!(entry("C-935")?["never"], false);
        assert_eq!(entry("C-031")?["locked"], serde_json::json!([]));
        Ok(())
    }
}
