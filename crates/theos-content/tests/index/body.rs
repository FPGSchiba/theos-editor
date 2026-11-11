//! Body rules over the `body/` fixture project: one finding per fault, judged per kind.

use theos_content::{Finding, Index, Project, parts};

use crate::common::{TempProject, expected, fixture, keys};

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

/// The body rules whose construct means something only in some kinds, and `body/callout-title`,
/// which judges a note title only where notes mean something.
const GATED: [&str; 11] = [
    "body/beat-heading",
    "body/beat-attribute",
    "body/misplaced-attributes",
    "body/when",
    "body/attribute-count",
    "body/attribute-duplicate",
    "body/attribute-unknown",
    "body/beat-number",
    "body/note-id",
    "body/note-state",
    "body/callout-title",
];

/// Rule, line and column of the findings of a [`GATED`] rule on `path`, sorted.
fn gated(findings: &[Finding], path: &str) -> Vec<(String, usize, usize)> {
    let mut found: Vec<_> = in_file(findings, path)
        .into_iter()
        .filter(|f| GATED.contains(&f.rule.as_str()))
        .map(|f| {
            let (line, column) = f.position.map_or((0, 0), |p| (p.line, p.column));
            (f.rule.clone(), line, column)
        })
        .collect();
    found.sort();
    found
}

fn hits(list: &[(&str, usize, usize)]) -> Vec<(String, usize, usize)> {
    let mut hits: Vec<_> = list
        .iter()
        .map(|(rule, line, column)| ((*rule).to_owned(), *line, *column))
        .collect();
    hits.sort();
    hits
}

#[test]
fn constructs_without_meaning_in_a_kind_give_no_finding() -> TestResult {
    // The topic's body holds every beat and note construct, well-formed and malformed. A topic
    // has neither beats nor notes, so only its malformed voice title (23:12) is judged.
    let findings = indexed()?.findings()?;
    assert_eq!(
        gated(&findings, TOPIC),
        hits(&[("body/callout-title", 23, 12)])
    );
    // The script's notes (a bad state, a repeated id, a malformed title) mean nothing either.
    assert_eq!(gated(&findings, SCRIPT).len(), 8, "only its beat findings");
    assert!(
        in_file(&findings, SCRIPT)
            .iter()
            .all(|f| !f.rule.starts_with("body/note-") && f.rule != "body/callout-title")
    );

    // The same body in a script and in a concept, with front matter of the topic's length so
    // the lines stay: each construct is judged where its kind gives it a meaning.
    let text = std::fs::read_to_string(fixture("body").join(TOPIC))?;
    let (_, body) = text
        .split_once("\n---\n")
        .ok_or("the topic has no front matter")?;
    let project = TempProject::copy_of("body")?;
    let twin_script = "story/scripts/S-951.md";
    let twin_concept = "concepts/synthetic-twin.md";
    let padding = "# the body of lore/topics/synthetic-board-explaining.md\n";
    project.write(
        twin_script,
        &format!(
            "---\n# synthetic fixture (P04), not canon\nid: S-951\nkind: script\n\
             title: Synthetic twin of the topic\nstatus: draft\n{padding}---\n{body}"
        ),
    )?;
    project.write(
        twin_concept,
        &format!(
            "---\n# synthetic fixture (P04), not canon\nid: synthetic-twin\nkind: concept\n\
             title: Synthetic twin of the topic\nfor: [synthetic-place]\n{padding}---\n{body}"
        ),
    )?;
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    let twins = index.findings()?;
    assert_eq!(
        gated(&twins, twin_script),
        hits(&[
            ("body/attribute-duplicate", 13, 1),
            ("body/beat-heading", 15, 1),
            ("body/callout-title", 23, 12),
            ("body/misplaced-attributes", 28, 1),
            ("body/attribute-count", 33, 27),
            ("body/when", 34, 8),
            ("body/attribute-unknown", 35, 1),
            ("body/beat-number", 37, 1),
            ("body/beat-attribute", 38, 1),
        ])
    );
    assert_eq!(
        gated(&twins, twin_concept),
        hits(&[
            ("body/note-state", 17, 16),
            ("body/callout-title", 23, 12),
            ("body/note-id", 43, 11),
            ("body/callout-title", 46, 11),
        ])
    );
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
