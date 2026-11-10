//! The story rules for beats (spec §5.9) over the `story/` fixture project.

use theos_content::{Finding, Index, Project, parts};

use crate::common::{TempProject, expected, fixture, keys};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The findings of `project`.
fn story_findings(project: &Project) -> Result<Vec<Finding>, Box<dyn std::error::Error>> {
    let mut index = Index::in_memory(project, &parts::builtin()?)?;
    index.sync()?;
    Ok(index.findings()?)
}

fn messages(findings: &[Finding], path: &str, rule: &str) -> Vec<String> {
    findings
        .iter()
        .filter(|finding| finding.path == path && finding.rule == rule)
        .map(|finding| finding.message.clone())
        .collect()
}

// AC: CE-14.2
#[test]
fn story_findings_equal_the_expectation() -> TestResult {
    let project = Project::open(&fixture("story"))?;
    let findings = story_findings(&project)?;
    let wanted = expected(&fixture("story").join("expected.toml"))?;
    assert_eq!(keys(&findings), wanted);
    Ok(())
}

// AC: CE-14.2
#[test]
fn a_belief_rejected_by_one_of_two_holders_passes() -> TestResult {
    let findings = story_findings(&Project::open(&fixture("story"))?)?;
    let on_script = messages(&findings, "story/scripts/S-953.md", "story/says-rejected");
    // Beat 1 (folk rejects B-950, free does not) passes; beat 2 (both reject) and the elder's
    // own B-952 fail; the elder's lens replaces the folk's, so B-950 passes there.
    assert_eq!(on_script.len(), 2, "{on_script:?}");
    assert!(on_script[0].starts_with("Beat 2 has synthetic-folk, synthetic-guild say B-950"));
    assert!(on_script[1].starts_with("Beat 3 has synthetic-elder say B-952, but its lens"));
    Ok(())
}

// AC: CE-14.2
#[test]
fn beats_without_at_or_when_skip_those_checks() -> TestResult {
    let findings = story_findings(&Project::open(&fixture("story"))?)?;
    let story: Vec<&Finding> = findings
        .iter()
        .filter(|finding| finding.rule.starts_with("story/"))
        .collect();
    // A script without `where` (S-951), a beat without `at::` (S-950 beat 3, S-954 beat 5) and a
    // beat without `when::` (S-955 beat 4) have no finding.
    assert!(story.iter().all(|f| f.path != "story/scripts/S-951.md"));
    assert!(story.iter().all(|f| f.path != "story/scripts/S-952.md"));
    assert_eq!(
        messages(&findings, "story/scripts/S-954.md", "story/reveal-where").len(),
        1
    );
    assert_eq!(
        messages(&findings, "story/scripts/S-955.md", "story/reveal-gate").len(),
        1
    );
    Ok(())
}

// AC: CE-14.2
#[test]
fn a_clue_with_two_records_fails_when_any_breaks_the_rule() -> TestResult {
    let project = TempProject::copy_of("story")?;
    // A second C-951 with a gate above beat 5's `when::` of S-955.
    project.write(
        "lore/clues/other/C-951.md",
        "---\n# synthetic fixture (P04), not canon\nid: C-951\nkind: clue\ntitle: Synthetic twin\nstatus: draft\nreveals: T-950\nmoves_to: hinted\nwhere: [synthetic-yard]\ngate: T3\nvector: synthetic\n---\nSynthetic text.\n",
    )?;
    let findings = story_findings(&project.project()?)?;
    let gate = messages(&findings, "story/scripts/S-955.md", "story/reveal-gate");
    assert_eq!(gate.len(), 2, "{gate:?}");
    assert!(gate[1].contains("the gate of C-951 is T3"), "{gate:?}");
    Ok(())
}
