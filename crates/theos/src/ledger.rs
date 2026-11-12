//! `theos ledger`: print the reveal ledger of every truth (spec §5.15).

use std::fmt::Write as _;
use std::io::Write;
use std::path::Path;

use anyhow::Context;
use theos_content::{BeatRef, Index, Project, TruthLedger, parts};

/// Opens the project at `folder`, syncs the index and writes the ledger to `out`.
///
/// # Errors
///
/// Fails when the project or the index cannot be opened, or when `out` cannot be written.
pub(crate) fn run(folder: &Path, out: &mut impl Write) -> anyhow::Result<()> {
    let project = Project::open(folder)?;
    let registry = parts::builtin()?;
    let (index, _update) = Index::open(&project, &registry)?;
    let ledger = index.ledger()?;
    out.write_all(format(&ledger).as_bytes())
        .context("cannot write the output")
}

/// The ledger as text: one block per truth, separated by one empty line.
pub(crate) fn format(ledger: &[TruthLedger]) -> String {
    ledger.iter().map(truth).collect::<Vec<_>>().join("\n")
}

/// One truth: its line, one line per clue, and the beats that place its clues.
fn truth(truth: &TruthLedger) -> String {
    let title = truth.title.as_deref().unwrap_or("(no title)");
    let target = truth.target.as_deref().unwrap_or("?");
    let mut text = format!("{}  {title}  (target {target})\n", truth.id);
    if truth.entries.is_empty() {
        text.push_str("  no clue\n");
    }
    for entry in &truth.entries {
        let floor = entry
            .floor
            .map_or_else(|| "F?".to_owned(), |floor| format!("F{floor:02}"));
        let gate = entry.gate.as_deref().unwrap_or("T?");
        let _ = writeln!(
            text,
            "  {:<9}  {floor} · {gate} · {}",
            entry.state, entry.clue
        );
    }
    let mut beats: Vec<&BeatRef> = truth
        .entries
        .iter()
        .flat_map(|entry| &entry.placed_by)
        .collect();
    beats.sort_by(|a, b| (&a.script, a.beat).cmp(&(&b.script, b.beat)));
    beats.dedup();
    if !beats.is_empty() {
        let placed: Vec<String> = beats
            .iter()
            .map(|beat| format!("{}#{}", beat.script, beat.beat))
            .collect();
        let _ = writeln!(text, "  placed by  {}", placed.join(" · "));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use theos_content::LedgerEntry;

    fn entry(state: &str, floor: Option<u32>, gate: Option<&str>, clue: &str) -> LedgerEntry {
        LedgerEntry {
            state: state.to_owned(),
            floor,
            place: None,
            gate: gate.map(str::to_owned),
            clue: clue.to_owned(),
            clue_path: format!("lore/clues/{clue}.md"),
            placed_by: Vec::new(),
        }
    }

    fn beat(script: &str, beat: u32) -> BeatRef {
        BeatRef {
            script: script.to_owned(),
            beat,
        }
    }

    fn truth_of(id: &str, title: Option<&str>, entries: Vec<LedgerEntry>) -> TruthLedger {
        TruthLedger {
            id: id.to_owned(),
            path: format!("lore/truths/{id}.md"),
            title: title.map(str::to_owned),
            target: Some("confirmed".to_owned()),
            entries,
        }
    }

    #[test]
    fn format_lays_out_states_floors_gates_and_beats() {
        let mut first = entry("hinted", Some(4), Some("T1"), "C-940");
        first.placed_by = vec![beat("S-940", 3), beat("S-940", 2)];
        let mut second = entry("confirmed", Some(100), Some("T2"), "C-942");
        second.placed_by = vec![beat("S-940", 2)];
        let ledger = [
            truth_of("T-940", Some("Statement A"), vec![first, second]),
            truth_of("T-941", None, vec![entry("suspected", None, None, "C-941")]),
            truth_of("T-942", Some("Statement C"), Vec::new()),
        ];
        let expected = "T-940  Statement A  (target confirmed)
  hinted     F04 · T1 · C-940
  confirmed  F100 · T2 · C-942
  placed by  S-940#2 · S-940#3

T-941  (no title)  (target confirmed)
  suspected  F? · T? · C-941

T-942  Statement C  (target confirmed)
  no clue
";
        assert_eq!(format(&ledger), expected);
    }

    #[test]
    fn format_of_no_truth_is_empty() {
        assert_eq!(format(&[]), "");
    }
}
