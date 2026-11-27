//! `codex::read`: one record with its text, links, lens and marks.

use std::collections::HashMap;

use theos_content::{EffectiveLens, LensData, RecordSummary, read_record};

use super::ledger::marked_truth;
use super::lookup::Lookup;

use crate::views::check::FindingView;
use crate::views::codex::{
    BeliefView, LensFieldView, LensValueView, LensView, OverrideView, ReadView, RecordRef,
    RecordView,
};
use crate::{StudioError, Workspace};

/// What to read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadTarget {
    /// Every record with this id (several when the id is duplicated).
    Id(String),
    /// The record at this project-relative path; a backslash counts as `/`.
    Path(String),
}

/// The record `target` names, with its text, links, backlinks, children, findings, lens and
/// ledger. The caller syncs the index first.
///
/// # Errors
/// [`StudioError::UnknownRecord`] when no record matches; [`StudioError::Read`] when a file
/// cannot be read; [`StudioError::Index`] when the index fails.
pub fn read(ws: &Workspace, target: &ReadTarget) -> Result<ReadView, StudioError> {
    let lookup = Lookup::load(ws)?;
    let (summaries, what) = match target {
        ReadTarget::Id(id) => (ws.index().record(id)?, id),
        ReadTarget::Path(path) => (
            lookup
                .by_path
                .get(&path.replace('\\', "/"))
                .cloned()
                .into_iter()
                .collect(),
            path,
        ),
    };
    if summaries.is_empty() {
        return Err(StudioError::UnknownRecord { what: what.clone() });
    }
    let records = summaries
        .iter()
        .map(|summary| record_view(ws, &lookup, summary))
        .collect::<Result<_, _>>()?;
    Ok(ReadView { records })
}

fn record_view(
    ws: &Workspace,
    lookup: &Lookup,
    summary: &RecordSummary,
) -> Result<RecordView, StudioError> {
    let index = ws.index();
    let file = ws.project().root().join(&summary.path);
    let text = read_record(&file)
        .map_err(|source| StudioError::Read { path: file, source })?
        .text()
        .to_owned();
    let links = index.links_from(&summary.path)?;
    let backlinks = index.backlinks(&summary.id)?;
    let children = index.children(&summary.id)?;
    let ledger = index
        .ledger_of(&summary.id)?
        .iter()
        .find(|truth| truth.path == summary.path)
        .map(|truth| marked_truth(truth, lookup));
    Ok(RecordView {
        record: RecordRef::from_summary(summary, &lookup.marks),
        text,
        links: links.iter().map(|l| lookup.link(l)).collect(),
        backlinks: backlinks.iter().map(|l| lookup.link(l)).collect(),
        children: children
            .iter()
            .map(|l| lookup.of_path(&l.source_path, &l.source_id, None))
            .collect(),
        findings: index
            .findings_for(&summary.path)?
            .iter()
            .map(FindingView::from)
            .collect(),
        lens: index
            .lens(&summary.id)?
            .map(|lens| lens_view(ws, lookup, &lens))
            .transpose()?,
        ledger,
    })
}

fn lens_view(
    ws: &Workspace,
    lookup: &Lookup,
    lens: &EffectiveLens,
) -> Result<LensView, StudioError> {
    let distorts = texts(ws, "distorts")?;
    let accuracy = texts(ws, "accuracy")?;
    Ok(LensView {
        chain: lens.chain.iter().map(|l| lookup.of_layer(l)).collect(),
        fields: lens
            .fields
            .iter()
            .map(|field| LensFieldView {
                field: field.field.clone(),
                value: value_view(lookup, &field.value),
                from: lookup.of_layer(&field.origin.layer),
                overrides: field
                    .overrides
                    .iter()
                    .map(|o| OverrideView {
                        value: value_view(lookup, &o.value),
                        from: lookup.of_layer(&o.origin.layer),
                    })
                    .collect(),
            })
            .collect(),
        beliefs: lens
            .beliefs
            .iter()
            .map(|held| BeliefView {
                belief: lookup.of_path(&held.path, &held.belief, Some("belief")),
                through: lookup.of_layer(&held.through),
                distorts: distorts.get(&held.path).map(|id| lookup.target(id)),
                accuracy: accuracy.get(&held.path).cloned(),
            })
            .collect(),
    })
}

/// The valid scalar texts of `field` by path.
fn texts(ws: &Workspace, field: &str) -> Result<HashMap<String, String>, StudioError> {
    Ok(ws
        .index()
        .field_texts(field)?
        .into_iter()
        .map(|row| (row.path, row.text))
        .collect())
}

fn value_view(lookup: &Lookup, data: &LensData) -> LensValueView {
    match data {
        LensData::Text(text) => LensValueView::Text(text.clone()),
        LensData::Links(ids) => {
            LensValueView::Links(ids.iter().map(|id| lookup.target(id)).collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TestResult, codex_workspace};

    fn id(value: &str) -> ReadTarget {
        ReadTarget::Id(value.to_owned())
    }

    fn only(view: ReadView) -> Result<RecordView, Box<dyn std::error::Error>> {
        let mut records = view.records;
        if records.len() == 1 {
            Ok(records.remove(0))
        } else {
            Err(format!("{} records", records.len()).into())
        }
    }

    // AC: CE-18.2
    #[test]
    fn read_mother_ilse_beliefs() -> TestResult {
        let (_dir, ws) = codex_workspace()?;
        let record = only(read(&ws, &id("mother-ilse"))?)?;
        assert!(record.text.contains("role: keeper of the Fates' shrine"));
        let lens = record.lens.ok_or("no lens")?;
        let chain: Vec<&str> = lens.chain.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(chain, ["mother-ilse", "fates-cult", "hearth-folk"]);
        assert_eq!(lens.beliefs.len(), 2);
        // The lens lists the holder's own beliefs first.
        let find = |id: &str| lens.beliefs.iter().find(|b| b.belief.id == id);
        let a = find("B-911").ok_or("B-911")?;
        assert_eq!(a.belief.id, "B-911");
        assert_eq!(a.belief.title.as_deref(), Some("Synthetic belief A"));
        assert_eq!(a.through.id, "fates-cult");
        assert_eq!(a.distorts.as_ref().ok_or("distorts")?.record.id, "T-912");
        assert_eq!(a.accuracy.as_deref(), Some("partial"));
        let b = find("B-912").ok_or("B-912")?;
        assert_eq!(b.belief.id, "B-912");
        assert_eq!(b.through.id, "mother-ilse");
        let distorts = b.distorts.as_ref().ok_or("distorts")?;
        assert_eq!(distorts.record.id, "T-911");
        assert!(distorts.record.mark.never);
        assert_eq!(b.accuracy.as_deref(), Some("false"));
        let json = serde_json::to_value(&lens)?;
        let beliefs = json["beliefs"].as_array().ok_or("beliefs")?;
        let b912 = beliefs.iter().find(|b| b["belief"]["id"] == "B-912");
        assert_eq!(b912.ok_or("B-912")?["distorts"]["never"], true);
        Ok(())
    }

    // AC: CE-18.3
    #[test]
    fn read_marks_never_everywhere() -> TestResult {
        let (_dir, ws) = codex_workspace()?;
        let truth = only(read(&ws, &id("T-911"))?)?;
        assert!(truth.record.mark.never);
        assert!(truth.text.contains("Synthetic statement A"));
        let back = truth
            .backlinks
            .iter()
            .find(|l| l.source.id == "B-912")
            .ok_or("no backlink from B-912")?;
        assert_eq!(back.slot, "field:distorts");
        let belief = only(read(&ws, &id("B-912"))?)?;
        let link = belief
            .links
            .iter()
            .find(|l| l.target.record.id == "T-911")
            .ok_or("no link to T-911")?;
        assert!(link.target.record.mark.never);
        assert!(link.target.exists);
        let json = serde_json::to_value(&belief)?;
        assert!(
            json["links"]
                .as_array()
                .ok_or("links")?
                .iter()
                .any(|l| l["target"]["never"] == true)
        );
        Ok(())
    }

    // AC: CE-18.3
    #[test]
    fn lens_links_carry_marks() -> TestResult {
        let (dir, ws) = codex_workspace()?;
        let fields =
            |record: &RecordView| -> Result<serde_json::Value, Box<dyn std::error::Error>> {
                let lens = serde_json::to_value(record.lens.as_ref().ok_or("no lens")?)?;
                lens["fields"]
                    .as_array()
                    .and_then(|all| all.iter().find(|f| f["field"] == "unaware_of"))
                    .cloned()
                    .ok_or_else(|| "no unaware_of".into())
            };
        // The fixture: fates-cult names the never truth, mother-ilse replaces it.
        let field = fields(&only(read(&ws, &id("mother-ilse"))?)?)?;
        assert_eq!(field["value"][0]["id"], "T-912");
        assert_eq!(field["value"][0]["never"], false);
        let replaced = &field["overrides"][0]["value"][0];
        assert_eq!(replaced["id"], "T-911");
        assert_eq!(replaced["never"], true);
        assert_eq!(replaced["locked"], serde_json::json!(["never"]));
        assert_eq!(replaced["exists"], true);

        // The nearest layer's own value names the never truth, and a missing id is bare.
        let path = dir
            .path()
            .join("people/hearth-folk/fates-cult/mother-ilse.md");
        let text = std::fs::read_to_string(&path)?;
        std::fs::write(&path, text.replace("[T-912]", "[T-911, T-000]"))?;
        let ws = Workspace::open(dir.path())?;
        let field = fields(&only(read(&ws, &id("mother-ilse"))?)?)?;
        assert_eq!(field["value"][0]["id"], "T-911");
        assert_eq!(field["value"][0]["never"], true);
        assert_eq!(field["value"][1]["id"], "T-000");
        assert_eq!(field["value"][1]["exists"], false);
        Ok(())
    }

    // AC: CE-18.1
    #[test]
    fn read_by_a_backslash_path() -> TestResult {
        let (_dir, ws) = codex_workspace()?;
        let record = only(read(
            &ws,
            &ReadTarget::Path(r"lore\beliefs\B-911.md".to_owned()),
        )?)?;
        assert_eq!(record.record.id, "B-911");
        assert_eq!(record.record.path.as_deref(), Some("lore/beliefs/B-911.md"));
        Ok(())
    }

    // AC: CE-18.3
    #[test]
    fn read_locked_records_return_text() -> TestResult {
        let (dir, ws) = codex_workspace()?;
        for (target, path, reason) in [
            (
                id("fates-cult"),
                "people/hearth-folk/fates-cult.md",
                "canon",
            ),
            (id("E91"), "canon/formalism/E91.md", "formalism"),
            (
                ReadTarget::Path("things/dup-911.md".to_owned()),
                "things/dup-911.md",
                "canon",
            ),
            (id("T-911"), "lore/truths/T-911.md", "never"),
        ] {
            let record = only(read(&ws, &target)?)?;
            // The full text of the file, as written (LF).
            let file = std::fs::read_to_string(dir.path().join(path))?.replace("\r\n", "\n");
            assert_eq!(record.text, file, "{target:?}");
            let json = serde_json::to_value(&record)?;
            let locked = json["locked"].as_array().ok_or("locked")?;
            assert!(locked.iter().any(|r| r == reason), "{target:?} {locked:?}");
        }
        Ok(())
    }

    // AC: CE-18.1
    #[test]
    fn read_missing_link_target() -> TestResult {
        let (_dir, ws) = codex_workspace()?;
        let script = only(read(&ws, &id("S-004"))?)?;
        let link = script
            .links
            .iter()
            .find(|l| l.target.record.id == "ghost-911")
            .ok_or("no ghost link")?;
        assert!(!link.target.exists);
        let json = serde_json::to_value(&link.target)?;
        assert_eq!(json["path"], serde_json::Value::Null);
        assert_eq!(json["never"], false);
        assert_eq!(json["exists"], false);
        assert!(link.line >= 1);
        Ok(())
    }

    // AC: CE-18.1
    #[test]
    fn read_duplicated_id_returns_both() -> TestResult {
        let (_dir, ws) = codex_workspace()?;
        let view = read(&ws, &id("dup-911"))?;
        assert_eq!(view.records.len(), 2);
        Ok(())
    }

    // AC: CE-18.1
    #[test]
    fn read_truth_has_ledger() -> TestResult {
        let (_dir, ws) = codex_workspace()?;
        let truth = only(read(&ws, &id("T-912"))?)?;
        let json = serde_json::to_value(truth.ledger.ok_or("no ledger")?)?;
        assert!(
            json["entries"]
                .as_array()
                .ok_or("entries")?
                .iter()
                .any(|e| e["clue"] == "C-031")
        );
        Ok(())
    }

    // AC: CE-18.1
    #[test]
    fn read_unknown_is_error() -> TestResult {
        let (_dir, ws) = codex_workspace()?;
        for target in [id("nope"), ReadTarget::Path("lore/nope.md".to_owned())] {
            let error = read(&ws, &target).err().ok_or("accepted")?;
            assert!(
                matches!(error, StudioError::UnknownRecord { .. }),
                "{error:?}"
            );
        }
        Ok(())
    }
}
