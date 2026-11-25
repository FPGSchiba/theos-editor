//! `Index::paths` and `Index::snapshot` (spec §5.7): the findings of a whole changed state, read
//! with the severities that state configures, and rolled back.

use theos_content::{Change, Finding, Index, Severity, parts};

use crate::common::TempProject;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SETTINGS: &str = ".theos/settings.toml";
const TRUTH: &str = "lore/truths/T-950.md";
const THREE_CLUE: &str = "lore/three-clue";

fn open(project: &TempProject) -> Result<Index, Box<dyn std::error::Error>> {
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    Ok(index)
}

fn of_rule<'a>(findings: &'a [Finding], rule: &str) -> Vec<&'a Finding> {
    findings.iter().filter(|f| f.rule == rule).collect()
}

#[test]
fn paths_lists_records_and_input_files() -> TestResult {
    let project = TempProject::copy_of("settings")?;
    let index = open(&project)?;
    let paths = index.paths()?;
    let mut want: Vec<String> = index.records()?.into_iter().map(|r| r.path).collect();
    want.push(SETTINGS.to_owned());
    want.sort();
    assert_eq!(paths, want);
    assert!(paths.contains(&SETTINGS.to_owned()));
    assert!(paths.contains(&"lore/truths/T-950.md".to_owned()));
    assert!(!paths.contains(&"theos.toml".to_owned()));
    Ok(())
}

// AC: CE-17.2
#[test]
fn a_snapshot_holds_every_finding_of_the_changed_state() -> TestResult {
    let project = TempProject::copy_of("lore")?;
    let mut index = open(&project)?;

    // With no change it is every finding of the index.
    assert_eq!(index.snapshot(&[])?, index.findings()?);

    // Lowering a truth's target makes an untouched clue overshoot.
    let text = std::fs::read_to_string(project.root().join(TRUTH))?;
    assert!(text.contains("target: confirmed"));
    let lowered = text.replace("target: confirmed", "target: hinted");
    let before = index.findings()?;
    assert!(
        !before
            .iter()
            .any(|f| f.rule == "lore/no-overshoot" && f.path == "lore/clues/C-952.md")
    );
    let snapshot = index.snapshot(&[Change::text(TRUTH, lowered)])?;
    let shown = of_rule(&snapshot, "lore/no-overshoot");
    assert!(
        shown.iter().any(|f| f.path == "lore/clues/C-952.md"),
        "{shown:?}"
    );

    // It holds findings of paths the change did not touch.
    let untouched = snapshot
        .iter()
        .filter(|f| f.path != TRUTH)
        .filter(|f| before.contains(f))
        .count();
    assert!(untouched > 0);
    assert!(untouched >= before.iter().filter(|f| f.path != TRUTH).count());
    Ok(())
}

#[test]
fn a_settings_change_applies_to_untouched_paths() -> TestResult {
    let project = TempProject::copy_of("lore")?;
    let mut index = open(&project)?;
    let before = index.findings()?;
    let warned = of_rule(&before, THREE_CLUE);
    assert!(warned.len() > 1, "{warned:?}");
    assert!(warned.iter().all(|f| f.severity == Severity::Warning));

    let settings = "[validate.severity]\n\"lore/three-clue\" = \"error\"\n";
    let snapshot = index.snapshot(&[Change::text(SETTINGS, settings)])?;
    let raised = of_rule(&snapshot, THREE_CLUE);
    assert_eq!(raised.len(), warned.len());
    assert!(raised.iter().all(|f| f.severity == Severity::Error));
    Ok(())
}

#[test]
fn a_staged_guardrail_lowering_is_an_error() -> TestResult {
    let project = TempProject::copy_of("lore")?;
    let mut index = open(&project)?;
    let settings = "[validate.severity]\n\"lore/never-confirm\" = \"warning\"\n";
    let snapshot = index.snapshot(&[Change::text(SETTINGS, settings)])?;
    let found = of_rule(&snapshot, "settings/guardrail");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].severity, Severity::Error);
    assert_eq!(found[0].path, SETTINGS);
    assert!(
        of_rule(&snapshot, "lore/never-confirm")
            .iter()
            .all(|f| f.severity == Severity::Error)
    );
    Ok(())
}

#[test]
fn a_snapshot_leaves_the_cache_unchanged() -> TestResult {
    let project = TempProject::copy_of("lore")?;
    let mut index = open(&project)?;
    let dump = index.dump()?;
    let text = std::fs::read_to_string(project.root().join(TRUTH))?;
    index.snapshot(&[
        Change::text(TRUTH, text.replace("target: confirmed", "target: hinted")),
        Change::text(
            SETTINGS,
            "[validate.severity]\n\"lore/three-clue\" = \"error\"\n",
        ),
    ])?;
    assert_eq!(index.dump()?, dump);
    Ok(())
}
