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
        messages[3],
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
            "C-958 moves T-955 to suspected, past its target hinted.",
            "Beat 3 places C-956, which moves T-953 to suspected, past its target hinted."
        ]
    );
    Ok(())
}

#[test]
fn a_duplicated_truth_id_fires_if_any_breaks_it() -> TestResult {
    let project = TempProject::copy_of("lore")?;
    let on = |findings: &[Finding], path: &str| findings.iter().any(|f| f.path == path);
    // T-954: one copy is `never`, the other `confirmed`.
    assert!(on(&found(&project, NEVER)?, "lore/clues/C-957.md"));
    project.remove("lore/truths/T-954.md")?;
    assert!(
        !on(&found(&project, NEVER)?, "lore/clues/C-957.md"),
        "the remaining copy is `confirmed`"
    );
    Ok(())
}

// AC: CE-15.1
#[test]
fn both_guardrails_fire_on_a_clue_whose_truth_id_is_duplicated() -> TestResult {
    // T-955: one copy is `never`, the other `hinted`; C-958 moves it to `suspected`.
    let project = TempProject::copy_of("lore")?;
    let at = |rule: &str| -> Result<Vec<(usize, usize)>, Box<dyn std::error::Error>> {
        let findings = found(&project, rule)?;
        Ok(findings
            .iter()
            .filter(|f| f.path == "lore/clues/C-958.md")
            .filter_map(|f| f.position.map(|p| (p.line, p.column)))
            .collect())
    };
    assert_eq!(at(NEVER)?, [(7, 10)], "never-confirm sits at `reveals`");
    assert_eq!(at(OVERSHOOT)?, [(8, 11)], "no-overshoot sits at `moves_to`");
    // The `never` copy sorts second by path.
    let first = "lore/truths/T-955.md";
    let second = "lore/truths/synthetic-copy/T-955.md";
    edit(&project, first, "target: never", "target: hinted")?;
    edit(&project, second, "target: hinted", "target: never")?;
    assert_eq!(at(NEVER)?, [(7, 10)]);
    assert_eq!(at(OVERSHOOT)?, [(8, 11)]);
    Ok(())
}

/// A clue record `id` that reveals `truth` up to `moves_to`.
fn clue_twin(id: &str, truth: &str, moves_to: &str) -> String {
    format!(
        "---\n# synthetic fixture (P04), not canon\nid: {id}\nkind: clue\ntitle: Synthetic twin\n\
         status: draft\nreveals: {truth}\nmoves_to: {moves_to}\nwhere: [synthetic-hall]\n\
         gate: T1\nvector: synthetic\n---\nSynthetic text.\n"
    )
}

// AC: CE-15.1
#[test]
fn a_beat_is_judged_against_every_clue_record_of_its_id() -> TestResult {
    // Twins of C-955 and C-956 that sort first by path and reveal T-950, whose target is
    // `confirmed`: the beats that place C-955 and C-956 still break the guardrails.
    let project = TempProject::copy_of("lore")?;
    let twins = "lore/clue-twins";
    assert!(*twins < *"lore/clues", "the twins sort before the clues");
    project.write(
        &format!("{twins}/C-955.md"),
        &clue_twin("C-955", "T-950", "hinted"),
    )?;
    project.write(
        &format!("{twins}/C-956.md"),
        &clue_twin("C-956", "T-950", "suspected"),
    )?;
    let on_script = |rule: &str| -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(found(&project, rule)?
            .into_iter()
            .filter(|finding| finding.path == "story/scripts/S-951.md")
            .map(|finding| finding.message)
            .collect())
    };
    assert_eq!(
        on_script(NEVER)?,
        [
            "Beat 2 places C-955, which hints at T-952, whose target is `never`: nothing may reveal it. Let a belief voice it instead."
        ]
    );
    assert_eq!(
        on_script(OVERSHOOT)?,
        ["Beat 3 places C-956, which moves T-953 to suspected, past its target hinted."]
    );
    Ok(())
}

// AC: CE-15.1
#[test]
fn never_confirm_fires_whatever_the_moves_to() -> TestResult {
    for (from, to) in [
        (
            "moves_to: hinted
",
            "moves_to: bogus
",
        ),
        (
            "moves_to: hinted
",
            "",
        ),
    ] {
        let project = TempProject::copy_of("lore")?;
        edit(&project, "lore/clues/C-955.md", from, to)?;
        let findings = found(&project, NEVER)?;
        let clue = findings
            .iter()
            .find(|f| f.path == "lore/clues/C-955.md")
            .ok_or("no finding on C-955")?;
        assert_eq!(
            clue.message,
            "C-955 reveals T-952, whose target is `never`: nothing may reveal it. Let a belief voice it instead."
        );
    }
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
    let on_c956: Vec<_> = findings
        .iter()
        .filter(|f| f.path == "lore/clues/C-956.md" || f.path == "story/scripts/S-951.md")
        .collect();
    assert_eq!(on_c956.len(), 0, "{on_c956:?}");
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
    assert_eq!(guardrails.len(), 7);
    assert!(guardrails.iter().all(|f| f.severity == Severity::Error));
    Ok(())
}

#[test]
fn the_project_has_exactly_the_expected_findings() -> TestResult {
    let project = Project::open(&fixture("lore"))?;
    let mut index = Index::in_memory(&project, &parts::builtin()?)?;
    index.sync()?;
    assert_eq!(
        keys(&index.findings()?),
        expected(&fixture("lore").join("expected.toml"))?
    );
    Ok(())
}
