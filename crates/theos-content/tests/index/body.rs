//! Body rules over the `body/` fixture project: one finding per fault, judged per kind.

use theos_content::{Finding, Index, Project, parts};

use crate::common::{expected, fixture, keys};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SCRIPT: &str = "story/scripts/S-950.md";
const CONCEPT: &str = "concepts/synthetic-board.md";
const TOPIC: &str = "lore/topics/synthetic-board-explaining.md";
const EQUATION: &str = "canon/formalism/E950.md";

fn indexed() -> Result<Index, Box<dyn std::error::Error>> {
    let project = Project::open(&fixture("body"))?;
    let mut index = Index::in_memory(&project, &parts::builtin()?)?;
    index.sync()?;
    Ok(index)
}

fn in_file<'a>(findings: &'a [Finding], path: &str) -> Vec<&'a Finding> {
    findings.iter().filter(|f| f.path == path).collect()
}

// AC: CE-14.2
#[test]
fn body_findings_carry_rule_severity_path_and_position() -> TestResult {
    let findings = indexed()?.findings()?;
    assert_eq!(
        keys(&findings),
        expected(&fixture("body").join("expected.toml"))?
    );
    Ok(())
}

// AC: CE-15.5
#[test]
fn an_unknown_note_state_is_an_error() -> TestResult {
    let findings = indexed()?.findings()?;
    let states: Vec<_> = findings
        .iter()
        .filter(|f| f.rule == "body/note-state")
        .collect();
    assert_eq!(states.len(), 1);
    let finding = states[0];
    assert_eq!(finding.path, CONCEPT);
    assert_eq!(finding.severity, theos_content::Severity::Error);
    let text = std::fs::read_to_string(fixture("body").join(CONCEPT))?;
    let at = text.find("closed").ok_or("closed not in the fixture")?;
    let line = text[..at].matches('\n').count() + 1;
    let column = text[..at]
        .rsplit('\n')
        .next()
        .map_or(0, |l| l.chars().count())
        + 1;
    let position = finding.position.ok_or("no position")?;
    assert_eq!((position.line, position.column), (line, column));
    Ok(())
}

#[test]
fn constructs_without_meaning_in_a_kind_give_no_finding() -> TestResult {
    let findings = indexed()?.findings()?;
    let topic = in_file(&findings, TOPIC);
    assert!(
        !topic.is_empty(),
        "the topic keeps its link and voice findings"
    );
    for finding in topic {
        let beat_or_note = finding.rule.starts_with("body/beat-")
            || finding.rule.starts_with("body/attribute-")
            || finding.rule.starts_with("body/note-")
            || finding.rule == "body/when"
            || finding.rule == "body/misplaced-attributes";
        assert!(!beat_or_note, "{}", finding.rule);
    }
    let script = in_file(&findings, SCRIPT);
    assert!(script.iter().all(|f| !f.rule.starts_with("body/note-")));
    Ok(())
}

#[test]
fn a_verbatim_body_is_not_judged() -> TestResult {
    let index = indexed()?;
    let findings = index.findings()?;
    let rules: Vec<_> = in_file(&findings, EQUATION)
        .iter()
        .map(|f| f.rule.as_str())
        .collect();
    // No body finding: the fixture has no `_source.json`, so the record is stray (R-P4).
    assert_eq!(rules, ["formalism/stray"]);
    let targets: Vec<_> = index
        .links_from(EQUATION)?
        .into_iter()
        .map(|l| l.target)
        .collect();
    assert_eq!(targets, Vec::<String>::new());
    Ok(())
}
