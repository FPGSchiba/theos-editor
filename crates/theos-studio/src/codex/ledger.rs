//! `codex::ledger_check`: the reveal ledger with marks, and the `lore/` findings on it.

use std::collections::HashSet;

use theos_content::TruthLedger;

use crate::check::counts;
use crate::views::check::FindingView;
use crate::views::ledger::TruthView;
use crate::views::project::{LedgerCheckView, MarkedTruthView};
use crate::{Marks, StudioError, Workspace};

/// The ledger of every truth, or of the truth `id`, each with its marks, and the findings whose
/// rule starts with `lore/` on the paths of those truths and of their clues.
///
/// # Errors
/// [`StudioError::UnknownRecord`] when `id` is not a truth; [`StudioError::Index`] when the
/// index fails.
pub fn ledger_check(ws: &Workspace, id: Option<&str>) -> Result<LedgerCheckView, StudioError> {
    let ledger: Vec<TruthLedger> = match id {
        Some(id) => {
            let found = ws.index().ledger_of(id)?;
            if found.is_empty() {
                return Err(StudioError::UnknownRecord {
                    what: id.to_owned(),
                });
            }
            found
        }
        None => ws.index().ledger()?,
    };
    let marks = Marks::load(ws.index())?;
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
            .map(|truth| MarkedTruthView {
                truth: TruthView::from(truth),
                mark: marks.of_path(&truth.path),
            })
            .collect(),
        findings: findings.iter().map(FindingView::from).collect(),
        errors,
        warnings,
    })
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
        assert_eq!(of("T-911")?["mark"]["never"], true);
        assert_eq!(of("T-912")?["mark"]["never"], false);
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

    // AC: CE-18.1
    #[test]
    fn ledger_check_one_truth() -> TestResult {
        let (_dir, workspace) = codex_workspace()?;
        assert_eq!(ledger_check(&workspace, Some("T-912"))?.truths.len(), 1);
        assert!(matches!(
            ledger_check(&workspace, Some("B-911")),
            Err(StudioError::UnknownRecord { .. })
        ));
        Ok(())
    }
}
