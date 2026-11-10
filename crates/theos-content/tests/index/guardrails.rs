//! The guardrails `lore/never-confirm` and `lore/no-overshoot` (spec §5.8) over the `lore/`
//! fixture project: a clue and the beats that place it.

use theos_content::{Finding, Index, Project, Severity, parts};

use crate::common::{Key, TempProject, expected, fixture, keys};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const NEVER: &str = "lore/never-confirm";
const OVERSHOOT: &str = "lore/no-overshoot";

/// Records of the fixture that no guardrail objects to.
const PASSING: [&str; 5] = [
    "lore/clues/C-950.md",
    "lore/clues/C-951.md",
    "lore/clues/C-952.md",
    "lore/truths/T-950.md",
    "story/scripts/S-950.md",
];

/// The findings of `rule` in the project at `root`.
fn found(project: &TempProject, rule: &str) -> Result<Vec<Finding>, Box<dyn std::error::Error>> {
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    Ok(index
        .findings()?
        .into_iter()
        .filter(|finding| finding.rule == rule)
        .collect())
}

/// The expected findings of `rule`.
fn wanted(rule: &str) -> Result<Vec<Key>, Box<dyn std::error::Error>> {
    let all = expected(&fixture("lore").join("expected.toml"))?;
    Ok(all.into_iter().filter(|key| key.1 == rule).collect())
}

fn edit(project: &TempProject, rel: &str, from: &str, to: &str) -> TestResult {
    let text = std::fs::read_to_string(project.root().join(rel))?;
    assert!(text.contains(from), "{rel} has no `{from}`");
    project.write(rel, &text.replace(from, to))?;
    Ok(())
}

fn check(rule: &str) -> TestResult {
    let project = TempProject::copy_of("lore")?;
    let findings = found(&project, rule)?;
    assert_eq!(keys(&findings), wanted(rule)?);
    for finding in &findings {
        assert!(
            !PASSING.contains(&finding.path.as_str()),
            "{finding} sits on a passing record"
        );
    }
    Ok(())
}

// AC: CE-15.1
#[test]
fn never_confirm_passes_and_fails() -> TestResult {
    check(NEVER)?;
    let project = TempProject::copy_of("lore")?;
    let messages: Vec<String> = found(&project, NEVER)?
        .into_iter()
        .map(|finding| finding.message)
        .collect();
    assert_eq!(
        messages[0],
        "C-955 hints at T-952, whose target is `never`: nothing may reveal it. Let a belief voice it instead."
    );
    assert_eq!(
        messages[2],
        "Beat 2 places C-955, which hints at T-952, whose target is `never`: nothing may reveal it. Let a belief voice it instead."
    );
    Ok(())
}

// AC: CE-15.1
#[test]
fn no_overshoot_passes_and_fails() -> TestResult {
    check(OVERSHOOT)?;
    let project = TempProject::copy_of("lore")?;
    let messages: Vec<String> = found(&project, OVERSHOOT)?
        .into_iter()
        .map(|finding| finding.message)
        .collect();
    assert_eq!(
        messages,
        [
            "C-956 moves T-953 to suspected, past its target hinted.",
            "Beat 3 places C-956, which moves T-953 to suspected, past its target hinted."
        ]
    );
    Ok(())
}

#[test]
fn a_duplicated_truth_id_fires_if_any_breaks_it() -> TestResult {
    let project = TempProject::copy_of("lore")?;
    let on_clue = |findings: &[Finding]| findings.iter().any(|f| f.path == "lore/clues/C-957.md");
    assert!(on_clue(&found(&project, NEVER)?), "one copy is `never`");
    project.remove("lore/truths/T-954.md")?;
    assert!(
        !on_clue(&found(&project, NEVER)?),
        "the remaining copy is `confirmed`"
    );
    // Confirming a `confirmed` truth is no overshoot.
    edit(
        &project,
        "lore/clues/C-957.md",
        "moves_to: hinted",
        "moves_to: confirmed",
    )?;
    assert!(!on_clue(&found(&project, OVERSHOOT)?));
    Ok(())
}

#[test]
fn a_clue_that_reveals_nothing_checkable_is_left_to_the_other_findings() -> TestResult {
    let project = TempProject::copy_of("lore")?;
    for (from, to) in [
        ("reveals: T-952", "reveals: T-999"),
        ("reveals: T-952", "reveals: C-950"),
    ] {
        edit(&project, "lore/clues/C-955.md", from, to)?;
        let findings = found(&project, NEVER)?;
        assert!(
            findings.iter().all(|f| f.path != "lore/clues/C-955.md"),
            "{findings:?}"
        );
        edit(&project, "lore/clues/C-955.md", to, from)?;
    }
    edit(
        &project,
        "lore/truths/T-953.md",
        "target: hinted",
        "target: soon",
    )?;
    let findings = found(&project, OVERSHOOT)?;
    assert_eq!(findings.len(), 0, "{findings:?}");
    Ok(())
}

#[test]
fn guardrail_findings_are_errors() -> TestResult {
    let project = Project::open(&fixture("lore"))?;
    let mut index = Index::in_memory(&project, &parts::builtin()?)?;
    index.sync()?;
    let guardrails: Vec<Finding> = index
        .findings()?
        .into_iter()
        .filter(|f| f.rule == NEVER || f.rule == OVERSHOOT)
        .collect();
    assert_eq!(guardrails.len(), 5);
    assert!(guardrails.iter().all(|f| f.severity == Severity::Error));
    Ok(())
}
