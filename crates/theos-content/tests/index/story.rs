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

#[test]
fn beats_that_share_a_number_are_left_to_body_beat_number() -> TestResult {
    let project = TempProject::copy_of("story")?;
    // Two beats 2: links are told apart only by beat number, so neither beat's `at::`,
    // `when::` or `reveals::` can be paired with the other's. Beat 1 is judged as usual.
    let path = "story/scripts/S-956.md";
    project.write(
        path,
        "---\n# synthetic fixture (P04), not canon\nid: S-956\nkind: script\n\
         title: Synthetic script S-956\nstatus: draft\nwhere: [f04-synthetic]\n---\n\
         Synthetic opening text.\n\n\
         ## Beat 1 — Below the gate\nreveals:: [[C-950]]\nwhen:: T1\n\n\
         ## Beat 2 — Gate T3\nat:: [[synthetic-hall]]\nreveals:: [[C-952]]\nwhen:: T3\n\n\
         ## Beat 2 — Gate T1\nat:: [[synthetic-yard]]\nreveals:: [[C-951]]\nwhen:: T1\n",
    )?;
    let findings = story_findings(&project.project()?)?;
    let story: Vec<&str> = findings
        .iter()
        .filter(|finding| finding.path == path && finding.rule.starts_with("story/"))
        .map(|finding| finding.message.as_str())
        .collect();
    assert_eq!(
        story,
        [
            "Beat 1 reveals C-950 at T1, but the gate of C-950 is T2. Raise `when::` of the beat to T2 or lower the gate of C-950."
        ]
    );
    assert_eq!(messages(&findings, path, "body/beat-number").len(), 1);
    Ok(())
}

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
