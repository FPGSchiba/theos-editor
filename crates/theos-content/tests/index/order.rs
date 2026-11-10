//! The order rules `lore/reveal-order`, `lore/requires` and `lore/three-clue` (spec §5.8) over
//! the `lore/` fixture project.

use theos_content::{Finding, Index, Severity, parts};

use crate::common::{TempProject, expected, fixture, keys};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const ORDER: &str = "lore/reveal-order";
const REQUIRES: &str = "lore/requires";
const THREE: &str = "lore/three-clue";

/// Records of the fixture that no order rule objects to.
const PASSING: [&str; 15] = [
    "lore/clues/C-950.md",
    "lore/clues/C-951.md",
    "lore/clues/C-952.md",
    "lore/clues/C-953.md",
    "lore/clues/C-954.md",
    "lore/clues/C-961.md",
    "lore/clues/C-968.md",
    "lore/clues/C-969.md",
    "lore/clues/C-970.md",
    "lore/clues/C-971.md",
    "lore/clues/C-972.md",
    "lore/truths/T-950.md",
    "lore/truths/T-956.md",
    "lore/truths/T-957.md",
    "lore/truths/T-964.md",
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

/// Checks the findings of `rule` against `expected.toml` and that no passing record has one.
fn check(rule: &str) -> Result<Vec<Finding>, Box<dyn std::error::Error>> {
    let project = TempProject::copy_of("lore")?;
    let findings = found(&project, rule)?;
    let all = expected(&fixture("lore").join("expected.toml"))?;
    let wanted: Vec<_> = all.into_iter().filter(|key| key.1 == rule).collect();
    assert_eq!(keys(&findings), wanted);
    for finding in &findings {
        assert!(
            !PASSING.contains(&finding.path.as_str()),
            "{finding} sits on a passing record"
        );
    }
    Ok(findings)
}

fn messages(findings: &[Finding], path: &str) -> Vec<String> {
    findings
        .iter()
        .filter(|finding| finding.path == path)
        .map(|finding| finding.message.clone())
        .collect()
}

fn edit(project: &TempProject, rel: &str, from: &str, to: &str) -> TestResult {
    let text = std::fs::read_to_string(project.root().join(rel))?;
    assert!(text.contains(from), "{rel} has no `{from}`");
    project.write(rel, &text.replace(from, to))?;
    Ok(())
}

// AC: CE-15.1
#[test]
fn reveal_order_passes_and_fails() -> TestResult {
    let findings = check(ORDER)?;
    assert!(findings.iter().all(|f| f.severity == Severity::Error));
    assert_eq!(
        messages(&findings, "lore/clues/C-960.md"),
        [
            "C-960 comes after C-961, which cannot be found no later than C-960 (C-961: floor 21, gate T2; C-960: floor 4, gate T1). Move one of them or change `after`."
        ]
    );
    assert_eq!(
        messages(&findings, "lore/clues/C-962.md"),
        ["C-962 is on a cycle of `after`: it would come after itself, through C-963."]
    );
    assert_eq!(
        messages(&findings, "lore/clues/C-963.md"),
        ["C-963 is on a cycle of `after`: it would come after itself, through C-962."]
    );
    Ok(())
}

// AC: CE-15.1
#[test]
fn requires_passes_and_fails() -> TestResult {
    let findings = check(REQUIRES)?;
    assert!(findings.iter().all(|f| f.severity == Severity::Error));
    assert_eq!(
        messages(&findings, "lore/truths/T-959.md"),
        ["T-959 requires T-965, whose target is hinted: T-965 can never be confirmed."]
    );
    assert_eq!(
        messages(&findings, "lore/truths/T-960.md"),
        [
            "T-960 requires T-966, but no clue that confirms T-966 can be found no later than C-990, which brings T-960 to its target."
        ]
    );
    assert_eq!(
        messages(&findings, "lore/truths/T-961.md"),
        ["T-961 is on a cycle of `requires`: it requires itself, through T-962."]
    );
    assert_eq!(
        messages(&findings, "lore/truths/T-962.md"),
        ["T-962 is on a cycle of `requires`: it requires itself, through T-961."]
    );
    Ok(())
}

// AC: CE-15.1
#[test]
fn three_clue_passes_and_fails() -> TestResult {
    let findings = check(THREE)?;
    assert!(findings.iter().all(|f| f.severity == Severity::Warning));
    let tail = "The three-clue rule asks for three.";
    assert_eq!(
        messages(&findings, "lore/truths/T-963.md"),
        [format!("Only one clue moves T-963 to confirmed. {tail}")]
    );
    assert_eq!(
        messages(&findings, "lore/truths/T-951.md"),
        [format!("No clue moves T-951 to hinted. {tail}")]
    );
    let project = TempProject::copy_of("lore")?;
    edit(
        &project,
        "lore/clues/C-954.md",
        "reveals: T-950",
        "reveals: T-951",
    )?;
    let after = found(&project, THREE)?;
    assert_eq!(
        messages(&after, "lore/truths/T-950.md"),
        [format!("Only two clues move T-950 to confirmed. {tail}")]
    );
    Ok(())
}

#[test]
fn states_out_of_order_give_one_finding() -> TestResult {
    let project = TempProject::copy_of("lore")?;
    let findings = found(&project, ORDER)?;
    let truth: Vec<_> = findings
        .iter()
        .filter(|f| f.message.contains("T-958"))
        .collect();
    assert_eq!(truth.len(), 1, "{truth:?}");
    assert_eq!(truth[0].path, "lore/clues/C-982.md");
    assert!(
        truth[0].message.starts_with(
            "C-982 confirms T-958, but no clue that raises suspicion of it can be found no later than C-982"
        ),
        "{}",
        truth[0].message
    );
    // A failing middle state: only its clue gets the finding.
    edit(
        &project,
        "lore/clues/C-982.md",
        "f21-synthetic",
        "f57-synthetic",
    )?;
    edit(
        &project,
        "lore/clues/C-981.md",
        "f50-synthetic",
        "f21-synthetic",
    )?;
    edit(
        &project,
        "lore/clues/C-980.md",
        "synthetic-hall",
        "f50-synthetic",
    )?;
    let findings = found(&project, ORDER)?;
    let truth: Vec<_> = findings
        .iter()
        .filter(|f| f.message.contains("T-958"))
        .map(|f| f.path.as_str())
        .collect();
    assert_eq!(truth, ["lore/clues/C-981.md"]);
    Ok(())
}

#[test]
fn order_rules_skip_unknown_floors() -> TestResult {
    let project = TempProject::copy_of("lore")?;
    // C-961 and C-981 lose their places' floors; C-990 and T-960's partner too.
    for (rel, from) in [
        ("lore/clues/C-961.md", "where: [f21-synthetic]"),
        ("lore/clues/C-981.md", "where: [f50-synthetic]"),
        ("lore/clues/C-991.md", "where: [f57-synthetic]"),
    ] {
        edit(&project, rel, from, "where: [tower]")?;
    }
    let all = |rule| found(&project, rule);
    assert!(
        all(ORDER)?
            .iter()
            .all(|f| !f.message.contains("C-961") && !f.message.contains("T-958")),
        "{:?}",
        all(ORDER)?
    );
    assert!(
        all(REQUIRES)?
            .iter()
            .all(|f| f.path != "lore/truths/T-960.md"),
        "{:?}",
        all(REQUIRES)?
    );
    Ok(())
}
