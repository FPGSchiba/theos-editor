//! The reveal ledger (spec §5.7): built from clue records by the lore step into the cache.

use std::collections::BTreeSet;

use theos_content::{BeatRef, Index, LedgerEntry, TruthLedger, parts};

use crate::common::{TempProject, fixture};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SUSPECTED: &str = "lore/clues/C-951.md";
const T_950: &str = "lore/truths/T-950.md";

#[derive(serde::Deserialize)]
struct Expected {
    truth: Vec<ExpectedTruth>,
}

#[derive(serde::Deserialize)]
struct ExpectedTruth {
    id: String,
    path: String,
    title: Option<String>,
    target: Option<String>,
    #[serde(default)]
    entry: Vec<ExpectedEntry>,
}

#[derive(serde::Deserialize)]
struct ExpectedEntry {
    state: String,
    floor: Option<u32>,
    place: Option<String>,
    gate: Option<String>,
    clue: String,
    clue_path: String,
    placed_by: Vec<ExpectedBeat>,
}

#[derive(serde::Deserialize)]
struct ExpectedBeat {
    script: String,
    beat: u32,
}

/// The `expected-ledger.toml` of the fixture project `lore`, as `ledger()` returns it.
fn expected() -> Result<Vec<TruthLedger>, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(fixture("lore").join("expected-ledger.toml"))?;
    let parsed: Expected = toml::from_str(&text)?;
    Ok(parsed
        .truth
        .into_iter()
        .map(|truth| TruthLedger {
            id: truth.id,
            path: truth.path,
            title: truth.title,
            target: truth.target,
            entries: truth
                .entry
                .into_iter()
                .map(|entry| LedgerEntry {
                    state: entry.state,
                    floor: entry.floor,
                    place: entry.place,
                    gate: entry.gate,
                    clue: entry.clue,
                    clue_path: entry.clue_path,
                    placed_by: entry
                        .placed_by
                        .into_iter()
                        .map(|beat| BeatRef {
                            script: beat.script,
                            beat: beat.beat,
                        })
                        .collect(),
                })
                .collect(),
        })
        .collect())
}

/// Sets the time of every file under `rel` of `project` 60 s back, so that none is racy.
fn settle(project: &TempProject, rel: &str) -> std::io::Result<()> {
    for entry in std::fs::read_dir(project.root().join(rel))? {
        let entry = entry?;
        let name = format!("{rel}/{}", entry.file_name().to_string_lossy());
        if entry.file_type()?.is_dir() {
            settle(project, &name)?;
        } else {
            let back = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
            project.set_time(&name, back)?;
        }
    }
    Ok(())
}

/// The lines of the `[ledger]` and `[ledger_beats]` sections of a dump.
fn ledger_lines(dump: &str) -> Vec<&str> {
    let mut in_ledger = false;
    let mut lines = Vec::new();
    for line in dump.lines() {
        if line.starts_with('[') {
            in_ledger = line.starts_with("[ledger");
        } else if in_ledger {
            lines.push(line);
        }
    }
    lines
}

fn open(
    project: &TempProject,
) -> Result<(Index, theos_content::Update), Box<dyn std::error::Error>> {
    Ok(Index::open(&project.project()?, &parts::builtin()?)?)
}

// AC: CE-15.3
#[test]
fn the_ledger_is_built_from_clue_records() -> TestResult {
    let project = TempProject::copy_of("lore")?;
    let (index, _) = open(&project)?;
    assert_eq!(index.ledger()?, expected()?);
    assert_eq!(index.ledger_of("T-950")?, expected()?[..1]);
    assert_eq!(index.ledger_of("T-999")?.len(), 0);
    Ok(())
}

// AC: CE-15.3
#[test]
fn the_ledger_comes_from_the_cache() -> TestResult {
    let project = TempProject::copy_of("lore")?;
    settle(&project, ".")?;
    let (first, _) = open(&project)?;
    let ledger = first.ledger()?;
    drop(first);
    let (second, update) = open(&project)?;
    assert!(
        update.read.is_empty(),
        "the second open read {:?}",
        update.read
    );
    assert_eq!(second.ledger()?, ledger);
    Ok(())
}

#[test]
fn a_clue_change_updates_its_truths_ledger() -> TestResult {
    let project = TempProject::copy_of("lore")?;
    let (mut index, _) = open(&project)?;
    let text = std::fs::read_to_string(project.root().join(SUSPECTED))?;
    project.write(SUSPECTED, &text.replace("f21-synthetic", "f57-synthetic"))?;
    let update = index.sync()?;
    let ledger = index.ledger_of("T-950")?;
    let entry = ledger
        .first()
        .and_then(|truth| truth.entries.iter().find(|entry| entry.clue == "C-951"))
        .ok_or("C-951 is not in the ledger")?;
    assert_eq!(entry.floor, Some(57));
    assert_eq!(entry.place.as_deref(), Some("f57-synthetic"));
    assert!(
        update.revalidated.contains(&T_950.to_owned()),
        "revalidated: {:?}",
        update.revalidated
    );
    let other = index.ledger_of("T-951")?;
    assert!(other.iter().all(|truth| truth.entries.is_empty()));
    assert!(
        !update
            .revalidated
            .contains(&"lore/truths/T-951.md".to_owned()),
        "a truth without that clue is not re-run"
    );
    Ok(())
}

#[test]
fn a_clue_without_a_floor_is_listed_without_one() -> TestResult {
    let project = TempProject::copy_of("lore")?;
    let (mut index, _) = open(&project)?;
    let text = std::fs::read_to_string(project.root().join(SUSPECTED))?;
    project.write(
        SUSPECTED,
        &text.replace("f21-synthetic", "synthetic-nowhere"),
    )?;
    index.sync()?;
    let ledger = index.ledger_of("T-950")?;
    let entry = ledger
        .first()
        .and_then(|truth| truth.entries.iter().find(|entry| entry.clue == "C-951"))
        .ok_or("C-951 is not in the ledger")?;
    assert_eq!((entry.floor, entry.place.as_deref()), (None, None));
    assert_eq!(entry.gate.as_deref(), Some("T2"));
    assert_eq!(entry.state, "suspected");
    Ok(())
}

#[test]
fn a_removed_clue_and_a_removed_truth_leave_no_rows() -> TestResult {
    let project = TempProject::copy_of("lore")?;
    let (mut index, _) = open(&project)?;
    project.remove(SUSPECTED)?;
    index.sync()?;
    let clues: BTreeSet<String> = index
        .ledger_of("T-950")?
        .iter()
        .flat_map(|truth| truth.entries.iter().map(|entry| entry.clue.clone()))
        .collect();
    assert_eq!(
        clues,
        BTreeSet::from(["C-950".to_owned(), "C-952".to_owned()])
    );
    project.remove(T_950)?;
    index.sync()?;
    assert_eq!(index.ledger_of("T-950")?.len(), 0);
    let dump = index.dump()?;
    let lines = ledger_lines(&dump);
    assert!(
        lines.is_empty(),
        "ledger rows of a removed truth remain: {lines:?}"
    );
    Ok(())
}
