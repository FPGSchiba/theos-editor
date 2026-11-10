//! The input files (spec §5.4): `.theos/settings.toml` and `canon/formalism/_source.json` are
//! tracked beside the records, and their subjects run the lore rules on them.

use std::time::{Duration, SystemTime};

use theos_content::lore::{Context, FactError, LoreRule, On, Subject};
use theos_content::rules::RuleClass;
use theos_content::{
    Change, Finding, Index, KindRegistry, Position, Severity, Span, Update, parts,
};

use crate::common::TempProject;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SETTINGS: &str = ".theos/settings.toml";
const SOURCE: &str = "canon/formalism/_source.json";

/// `test/settings`: reports a settings file whose text contains `synthetic-bad`, and one that
/// has no text.
const SETTINGS_RULE: LoreRule = LoreRule {
    id: "test/settings",
    severity: Severity::Warning,
    class: RuleClass::Fixed,
    on: On::Input(SETTINGS),
    run: settings,
};

fn settings(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
    let Subject::Input(input) = subject else {
        return Ok(Vec::new());
    };
    // Reads the file once more, as a rule that reads another input does.
    let Some(input) = ctx.input(&input.path)? else {
        return Ok(Vec::new());
    };
    let message = match &input.text {
        None => "no text",
        Some(text) if text.contains("synthetic-bad") => "bad text",
        Some(_) => return Ok(Vec::new()),
    };
    Ok(vec![subject.finding(
        &SETTINGS_RULE,
        Span::default(),
        Position { line: 1, column: 1 },
        message.to_owned(),
    )])
}

fn registry() -> Result<KindRegistry, Box<dyn std::error::Error>> {
    let mut registry = parts::builtin()?;
    registry.register_rule("test", SETTINGS_RULE)?;
    Ok(registry)
}

fn paths(list: &[&str]) -> Vec<String> {
    list.iter().map(|path| (*path).to_owned()).collect()
}

/// The findings, as `path rule message`; none has an id (L9).
fn said(index: &Index) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut said = Vec::new();
    for finding in index.findings()? {
        assert_eq!(finding.id, None, "{finding}");
        said.push(format!(
            "{} {} {}",
            finding.path, finding.rule, finding.message
        ));
    }
    Ok(said)
}

fn open(project: &TempProject) -> Result<Index, Box<dyn std::error::Error>> {
    Ok(Index::in_memory(&project.project()?, &registry()?)?)
}

#[test]
fn a_changed_settings_file_is_read_at_the_next_sync() -> TestResult {
    let project = TempProject::new()?;
    let mut index = open(&project)?;
    assert_eq!(index.sync()?, Update::default());
    assert_eq!(said(&index)?, Vec::<String>::new());

    project.write(SETTINGS, "name = \"synthetic-bad\"\n")?;
    let update = index.sync()?;
    assert_eq!(update.read, paths(&[SETTINGS]));
    assert_eq!(update.revalidated, paths(&[SETTINGS]));
    assert!(update.reparsed.is_empty() && update.removed.is_empty());
    assert_eq!(
        said(&index)?,
        [format!("{SETTINGS} test/settings bad text")]
    );
    assert!(index.dump()?.contains("[inputs]\n.theos/settings.toml\t"));

    // A cold build over the same folder gives the same index.
    let mut cold = open(&project)?;
    cold.sync()?;
    assert_eq!(cold.dump()?, index.dump()?);

    project.write(SETTINGS, "name = \"synthetic-fine\"\n")?;
    assert_eq!(index.sync()?.read, paths(&[SETTINGS]));
    assert_eq!(said(&index)?, Vec::<String>::new());
    Ok(())
}

#[test]
fn an_unchanged_input_is_not_read_again() -> TestResult {
    let project = TempProject::new()?;
    project.write(SETTINGS, "name = \"synthetic-bad\"\n")?;
    project.write(SOURCE, "{}\n")?;
    let mut index = open(&project)?;
    let update = index.sync()?;
    assert_eq!(update.read, paths(&[SETTINGS, SOURCE]));
    assert_eq!(index.sync()?, Update::default());
    assert_eq!(said(&index)?.len(), 1);
    Ok(())
}

#[test]
fn inputs_are_changes_for_update_apply_and_preview() -> TestResult {
    let project = TempProject::new()?;
    let mut index = open(&project)?;
    index.sync()?;

    // update: from disk.
    project.write(SETTINGS, "name = \"synthetic-bad\"\n")?;
    let update = index.update(&[SETTINGS])?;
    assert_eq!(update.read, paths(&[SETTINGS]));
    assert_eq!(update.revalidated, paths(&[SETTINGS]));
    assert_eq!(said(&index)?.len(), 1);

    // preview: the findings the bytes would leave; the cache stays as it was.
    let good = Change::text(".theos\\settings.toml", "name = \"synthetic-fine\"\n");
    let before = index.dump()?;
    let preview = index.preview(std::slice::from_ref(&good))?;
    assert_eq!(preview.update.revalidated, paths(&[SETTINGS]));
    assert_eq!(preview.findings, Vec::new());
    assert_eq!(index.dump()?, before);
    let bad = Change::text(SETTINGS, "name = \"synthetic-bad again\"\n");
    let preview = index.preview(&[bad])?;
    assert_eq!(preview.findings.len(), 1);
    assert_eq!(preview.findings[0].path, SETTINGS);
    assert_eq!(index.dump()?, before);

    // apply: given bytes, then removed.
    let update = index.apply(&[good])?;
    assert_eq!(update.revalidated, paths(&[SETTINGS]));
    assert_eq!(said(&index)?, Vec::<String>::new());
    let same = Change::text(SETTINGS, "name = \"synthetic-fine\"\n");
    assert_eq!(index.apply(&[same])?, Update::default());
    let update = index.apply(&[Change::Removed {
        path: SETTINGS.to_owned(),
    }])?;
    assert!(update.removed.is_empty() && update.revalidated.is_empty());
    assert!(!index.dump()?.contains(".theos/settings.toml"));
    Ok(())
}

#[test]
fn a_removed_input_loses_its_row_and_findings() -> TestResult {
    let project = TempProject::new()?;
    project.write(SETTINGS, "name = \"synthetic-bad\"\n")?;
    let mut index = open(&project)?;
    index.sync()?;
    assert_eq!(said(&index)?.len(), 1);
    assert!(index.dump()?.contains("[inputs]\n.theos/settings.toml\t"));

    project.remove(SETTINGS)?;
    assert_eq!(index.sync()?, Update::default());
    assert_eq!(said(&index)?, Vec::<String>::new());
    let dump = index.dump()?;
    assert!(!dump.contains(".theos/settings.toml"), "{dump}");
    assert_eq!(index.sync()?, Update::default());
    Ok(())
}

#[test]
fn a_non_utf8_input_is_stored_without_text() -> TestResult {
    let project = TempProject::new()?.with_file(SETTINGS, &[0xff, 0xfe, 0x00])?;
    project.set_time(SETTINGS, SystemTime::now() - Duration::from_secs(60))?;
    let mut index = open(&project)?;
    let update = index.sync()?;
    assert_eq!(update.read, paths(&[SETTINGS]));
    assert_eq!(said(&index)?, [format!("{SETTINGS} test/settings no text")]);
    assert!(index.dump()?.contains("[inputs]\n.theos/settings.toml\t"));
    Ok(())
}

#[test]
fn a_racy_input_is_read_again() -> TestResult {
    let project = TempProject::new()?;
    project.write_fresh(SETTINGS, "name = \"synthetic-bad\"\n")?;
    let mut index = open(&project)?;
    assert_eq!(index.sync()?.read, paths(&[SETTINGS]));
    let stored = project.time(SETTINGS)?;
    // Same size, same time, other bytes: only the racy rule can see it.
    project.write_fresh(SETTINGS, "name = \"synthetic-fine\"\n")?;
    project.set_time(SETTINGS, stored)?;
    let update = index.sync()?;
    assert_eq!(update.read, paths(&[SETTINGS]));
    assert_eq!(update.revalidated, paths(&[SETTINGS]));
    assert_eq!(said(&index)?, Vec::<String>::new());
    // Once its time is old enough it is read one more time, then left alone.
    project.set_time(SETTINGS, SystemTime::now() - Duration::from_secs(60))?;
    assert_eq!(index.sync()?.read, paths(&[SETTINGS]));
    assert_eq!(index.sync()?, Update::default());
    Ok(())
}

#[test]
fn an_unreadable_input_counts_as_absent_like_in_a_cold_build() -> TestResult {
    let project = TempProject::new()?;
    project.write(SETTINGS, "name = \"synthetic-bad\"\n")?;
    let mut index = open(&project)?;
    index.sync()?;
    assert_eq!(said(&index)?.len(), 1);

    // A folder at the path: `fs::read` refuses it on every OS.
    project.remove(SETTINGS)?;
    std::fs::create_dir(project.root().join(SETTINGS))?;
    index.sync()?;
    assert_eq!(said(&index)?, Vec::<String>::new());
    let mut cold = open(&project)?;
    cold.sync()?;
    assert_eq!(index.dump()?, cold.dump()?);
    assert!(!index.dump()?.contains(".theos/settings.toml"));
    Ok(())
}
