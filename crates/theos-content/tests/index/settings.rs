//! The validation settings (spec §5.12) over the `settings/` fixture project: its
//! `.theos/settings.toml` has a row of the spec's table each, and the configured severities are
//! applied when findings are read.

use theos_content::{Change, Finding, Index, Severity, parts};

use crate::common::{TempProject, expected, fixture, keys};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SETTINGS: &str = ".theos/settings.toml";
const THREE_CLUE: &str = "lore/three-clue";
const NEVER: &str = "lore/never-confirm";
const OVERSHOOT: &str = "lore/no-overshoot";

fn open(project: &TempProject) -> Result<Index, Box<dyn std::error::Error>> {
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    Ok(index)
}

fn of_rule(index: &Index, rule: &str) -> Result<Vec<Finding>, Box<dyn std::error::Error>> {
    Ok(index
        .findings()?
        .into_iter()
        .filter(|finding| finding.rule == rule)
        .collect())
}

/// The severity of the only finding of `rule`.
fn severity_of(index: &Index, rule: &str) -> Result<Severity, Box<dyn std::error::Error>> {
    let found = of_rule(index, rule)?;
    assert_eq!(found.len(), 1, "{found:?}");
    Ok(found[0].severity)
}

/// The severities of every finding of `rule`.
fn severities_of(index: &Index, rule: &str) -> Result<Vec<Severity>, Box<dyn std::error::Error>> {
    Ok(of_rule(index, rule)?
        .iter()
        .map(|finding| finding.severity)
        .collect())
}

/// The settings file of the fixture without the line that holds `entry`.
fn without(entry: &str) -> Result<String, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(fixture("settings").join(SETTINGS))?;
    assert!(text.contains(entry), "no `{entry}` in the settings");
    let kept: Vec<&str> = text.lines().filter(|line| !line.contains(entry)).collect();
    Ok(kept.join("\n") + "\n")
}

// AC: CE-14.1
#[test]
fn a_setting_that_lowers_a_guardrail_is_rejected_with_a_message() -> TestResult {
    let project = TempProject::copy_of("settings")?;
    let index = open(&project)?;
    let found = of_rule(&index, "settings/guardrail")?;
    let said: Vec<String> = found.iter().map(ToString::to_string).collect();
    assert_eq!(
        said,
        [
            ".theos/settings.toml:3:1: error settings/guardrail: lore/never-confirm is a \
             guardrail: it always blocks, and no setting lowers it.",
            ".theos/settings.toml:4:1: error settings/guardrail: lore/no-overshoot is a \
             guardrail: it always blocks, and no setting lowers it.",
            ".theos/settings.toml:5:1: error settings/guardrail: lore/inworld-vocab is a \
             guardrail: it always blocks, and no setting lowers it.",
        ]
    );
    assert!(found.iter().all(|finding| finding.id.is_none()));
    // The entries had no effect: the guardrails they named stay errors.
    assert_eq!(severity_of(&index, NEVER)?, Severity::Error);
    assert_eq!(severity_of(&index, OVERSHOOT)?, Severity::Error);
    assert_eq!(
        severities_of(&index, "lore/inworld-vocab")?,
        [Severity::Error; 2]
    );
    Ok(())
}

// AC: CE-14.1
#[test]
fn an_order_rule_setting_changes_severity_on_read() -> TestResult {
    let project = TempProject::copy_of("settings")?;
    let mut index = open(&project)?;
    assert_eq!(severities_of(&index, THREE_CLUE)?, [Severity::Error; 2]);
    let path = "lore/truths/T-950.md";
    assert_eq!(index.findings_for(path)?[0].severity, Severity::Error);

    // A preview reads with the configured severities too.
    let text = std::fs::read_to_string(project.root().join(path))?;
    let preview = index.preview(&[Change::text(path, text.replace("one clue", "a clue"))])?;
    let previewed: Vec<Severity> = preview
        .findings
        .iter()
        .filter(|finding| finding.rule == THREE_CLUE)
        .map(|finding| finding.severity)
        .collect();
    assert_eq!(previewed, [Severity::Error]);

    // The store keeps the default severity and the configuration beside it.
    let dump = index.dump()?;
    assert!(
        dump.contains(&format!("{path}\tT-950\t{THREE_CLUE}\twarning\t")),
        "{dump}"
    );
    assert!(dump.contains(&format!("[configured]\n{THREE_CLUE}\terror\n")));

    // Removing the entry makes it a warning again, and no record is parsed again.
    let update = index.apply(&[Change::text(SETTINGS, &without("\"lore/three-clue\"")?)])?;
    assert!(update.reparsed.is_empty(), "{update:?}");
    assert_eq!(severities_of(&index, THREE_CLUE)?, [Severity::Warning; 2]);
    assert!(!index.dump()?.contains(&format!("{THREE_CLUE}\terror")));
    Ok(())
}

// AC: CE-15.2
#[test]
fn an_allowlist_entry_without_a_reason_is_an_error() -> TestResult {
    let project = TempProject::copy_of("settings")?;
    let index = open(&project)?;
    let found = of_rule(&index, "settings/allow-reason")?;
    assert_eq!(
        keys(&found),
        [
            (
                SETTINGS.to_owned(),
                "settings/allow-reason".to_owned(),
                "error".to_owned(),
                14,
                3
            ),
            (
                SETTINGS.to_owned(),
                "settings/allow-reason".to_owned(),
                "error".to_owned(),
                15,
                3
            ),
        ],
        "the entry without `reason` and the one with `reason = \"  \"`"
    );
    Ok(())
}

#[test]
fn settings_findings_equal_the_expectation() -> TestResult {
    let project = TempProject::copy_of("settings")?;
    let index = open(&project)?;
    let wanted = expected(&fixture("settings").join("expected.toml"))?;
    assert_eq!(keys(&index.findings()?), wanted);
    Ok(())
}

#[test]
fn a_crlf_settings_file_gives_the_same_findings_and_positions() -> TestResult {
    // A Windows checkout without `.gitattributes` writes the file with CRLF.
    let project = TempProject::copy_of("settings")?;
    let text = std::fs::read_to_string(project.root().join(SETTINGS))?;
    assert!(!text.contains('\r'), "the fixture is LF");
    project.write(SETTINGS, &text.replace('\n', "\r\n"))?;
    let index = open(&project)?;
    // Every settings finding at its line and column, and the entries that work (the order
    // rule's severity, the project's word) still have their effect.
    let wanted = expected(&fixture("settings").join("expected.toml"))?;
    assert_eq!(keys(&index.findings()?), wanted);
    let messages: Vec<String> = of_rule(&index, "settings/guardrail")?
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        messages[0],
        ".theos/settings.toml:3:1: error settings/guardrail: lore/never-confirm is a guardrail: \
         it always blocks, and no setting lowers it."
    );
    assert_eq!(severities_of(&index, THREE_CLUE)?, [Severity::Error; 2]);
    Ok(())
}

#[test]
fn a_broken_settings_file_falls_back_to_defaults() -> TestResult {
    let project = TempProject::copy_of("settings")?;
    let mut index = open(&project)?;
    let good = std::fs::read_to_string(project.root().join(SETTINGS))?;

    project.write(
        SETTINGS,
        "[validate.severity\n\"lore/three-clue\" = \"error\"\n",
    )?;
    index.sync()?;
    let broken = of_rule(&index, "settings/toml")?;
    assert_eq!(broken.len(), 1, "{broken:?}");
    assert_eq!(broken[0].path, SETTINGS);
    // `[validate.severity` ends at the line break: the parse error sits at 1:19.
    assert_eq!(
        broken[0].position.map(|p| (p.line, p.column)),
        Some((1, 19))
    );
    // No other settings finding, the configuration is empty, the guardrail is an error.
    assert_eq!(
        index
            .findings()?
            .iter()
            .filter(|finding| finding.rule.starts_with("settings/"))
            .count(),
        1
    );
    assert_eq!(severities_of(&index, THREE_CLUE)?, [Severity::Warning; 2]);
    assert_eq!(severity_of(&index, NEVER)?, Severity::Error);
    assert_eq!(severity_of(&index, OVERSHOOT)?, Severity::Error);
    assert!(index.dump()?.contains("[configured]\n[search]"));

    // A file that is not text is one finding at 1:1.
    std::fs::write(project.root().join(SETTINGS), [0xff, 0xfe, 0x00])?;
    index.sync()?;
    let binary = of_rule(&index, "settings/toml")?;
    assert_eq!(keys(&binary).len(), 1);
    assert_eq!(binary[0].position.map(|p| (p.line, p.column)), Some((1, 1)));

    // The fixed file restores everything, as a cold build over it would have it.
    project.write(SETTINGS, &good)?;
    index.sync()?;
    assert_eq!(severities_of(&index, THREE_CLUE)?, [Severity::Error; 2]);
    let wanted = expected(&fixture("settings").join("expected.toml"))?;
    assert_eq!(keys(&index.findings()?), wanted);
    assert_eq!(open(&project)?.dump()?, index.dump()?);
    Ok(())
}

#[test]
fn without_a_settings_file_the_defaults_apply() -> TestResult {
    let project = TempProject::copy_of("settings")?;
    let mut index = open(&project)?;
    project.remove(SETTINGS)?;
    index.sync()?;
    for index in [&index, &open(&project)?] {
        let all = index.findings()?;
        assert!(
            all.iter()
                .all(|finding| !finding.rule.starts_with("settings/")),
            "{all:?}"
        );
        assert_eq!(severities_of(index, THREE_CLUE)?, [Severity::Warning; 2]);
        assert_eq!(severity_of(index, NEVER)?, Severity::Error);
        assert!(index.dump()?.contains("[configured]\n[search]"));
    }
    assert_eq!(open(&project)?.dump()?, index.dump()?);
    Ok(())
}
