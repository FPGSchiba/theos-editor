//! The order rules `lore/reveal-order`, `lore/requires` and `lore/three-clue` (spec §5.8) over
//! the `lore/` fixture project.

use theos_content::{Finding, Index, Project, Severity, parts};

use crate::common::{TempProject, expected, fixture, keys};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const ORDER: &str = "lore/reveal-order";
const REQUIRES: &str = "lore/requires";
const THREE: &str = "lore/three-clue";

/// Records of the fixture that no order rule objects to.
const PASSING: [&str; 16] = [
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
    "lore/clues/C-993.md",
    "lore/truths/T-950.md",
    "lore/truths/T-956.md",
    "lore/truths/T-957.md",
    "lore/truths/T-964.md",
];

/// The findings of `rule` in `project`.
fn in_project(project: &Project, rule: &str) -> Result<Vec<Finding>, Box<dyn std::error::Error>> {
    let mut index = Index::in_memory(project, &parts::builtin()?)?;
    index.sync()?;
    Ok(index
        .findings()?
        .into_iter()
        .filter(|finding| finding.rule == rule)
        .collect())
}

/// The findings of `rule` in the temporary copy `project`.
fn found(project: &TempProject, rule: &str) -> Result<Vec<Finding>, Box<dyn std::error::Error>> {
    in_project(&project.project()?, rule)
}

/// Checks the findings of `rule` against `expected.toml` and that no passing record has one. The
/// fixture is indexed in place: nothing is written.
fn check(rule: &str) -> Result<Vec<Finding>, Box<dyn std::error::Error>> {
    let findings = in_project(&Project::open(&fixture("lore"))?, rule)?;
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

/// A clue of the truth `truth`, placed at `place` with `gate`.
fn clue(id: &str, truth: &str, moves_to: &str, place: &str, gate: &str) -> String {
    format!(
        "---\n# synthetic fixture (P04), not canon\nid: {id}\nkind: clue\ntitle: Synthetic clue\nstatus: draft\nreveals: {truth}\nmoves_to: {moves_to}\nwhere: [{place}]\ngate: {gate}\nvector: synthetic\n---\nSynthetic text.\n"
    )
}

fn add_clue(
    project: &TempProject,
    id: &str,
    truth: &str,
    moves_to: &str,
    place: &str,
    gate: &str,
) -> TestResult {
    let text = clue(id, truth, moves_to, place, gate);
    project.write(&format!("lore/clues/{id}.md"), &text)?;
    Ok(())
}

/// A floor of the high band.
fn add_floor(project: &TempProject, number: u32) -> TestResult {
    let text = format!(
        "---\n# synthetic fixture (P04), not canon\nid: f{number}-synthetic\nkind: floor\ntitle: Synthetic floor\nstatus: draft\nparent: synthetic-high\nnumber: {number}\n---\nSynthetic text.\n"
    );
    project.write(
        &format!("world/synthetic-high/f{number}-synthetic.md"),
        &text,
    )?;
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
    // The same floor and a higher gate fails; a lower floor and gate (C-961 after C-969) passes.
    assert_eq!(
        messages(&findings, "lore/clues/C-994.md"),
        [
            "C-994 comes after C-993, which cannot be found no later than C-994 (C-993: floor 4, gate T3; C-994: floor 4, gate T1). Move one of them or change `after`."
        ]
    );
    assert_eq!(
        messages(&findings, "lore/clues/C-962.md"),
        [
            "C-962 is on a cycle of `after`: it would come after itself, through C-963. Remove one `after` link of the cycle."
        ]
    );
    assert_eq!(
        messages(&findings, "lore/clues/C-963.md"),
        [
            "C-963 is on a cycle of `after`: it would come after itself, through C-962. Remove one `after` link of the cycle."
        ]
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
        [
            "T-959 requires T-965, whose target is hinted, so T-965 can never be confirmed. Set the target of T-965 to `confirmed` or drop it from `requires`."
        ]
    );
    assert_eq!(
        messages(&findings, "lore/truths/T-960.md"),
        [
            "T-960 requires T-966, but no clue that confirms T-966 can be found no later than C-990 (floor 21, gate T2), which brings T-960 to its target. Move a clue that confirms T-966 earlier or C-990 later."
        ]
    );
    assert_eq!(
        messages(&findings, "lore/truths/T-961.md"),
        [
            "T-961 is on a cycle of `requires`: it requires itself, through T-962. Remove one link of the cycle."
        ]
    );
    assert_eq!(
        messages(&findings, "lore/truths/T-962.md"),
        [
            "T-962 is on a cycle of `requires`: it requires itself, through T-961. Remove one link of the cycle."
        ]
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
fn reveal_order_passes_when_some_clue_of_the_nearest_state_is_early_enough() -> TestResult {
    // C-982 confirms T-958 on F21; the only suspecting clue C-981 sits on F50. A second one on
    // F04 is early enough, so the order holds.
    let project = TempProject::copy_of("lore")?;
    add_clue(
        &project,
        "C-983",
        "T-958",
        "suspected",
        "synthetic-hall",
        "T1",
    )?;
    let findings = found(&project, ORDER)?;
    assert!(
        findings.iter().all(|f| !f.message.contains("T-958")),
        "{findings:?}"
    );
    Ok(())
}

#[test]
fn requires_passes_when_some_confirming_clue_is_early_enough() -> TestResult {
    // T-957, which T-950 requires, is confirmed at (F04, T3) and at (F30, T1). Each case below
    // moves the clue C-952 that confirms T-950.
    let project = TempProject::copy_of("lore")?;
    add_floor(&project, 30)?;
    edit(&project, "lore/clues/C-970.md", "gate: T1", "gate: T3")?;
    edit(
        &project,
        "lore/clues/C-971.md",
        "[synthetic-hall]",
        "[f30-synthetic]",
    )?;
    project.remove("lore/clues/C-972.md")?;
    project.remove("lore/clues/C-953.md")?;
    project.remove("lore/clues/C-954.md")?;
    // Case 1, at (F21, T2): the first clue's floor is early enough but its gate is not, the
    // second clue's gate is but its floor is not: none is no later, one finding.
    edit(
        &project,
        "lore/clues/C-952.md",
        "f57-synthetic",
        "f21-synthetic",
    )?;
    let findings = found(&project, REQUIRES)?;
    let on_t950 = messages(&findings, "lore/truths/T-950.md");
    assert_eq!(on_t950.len(), 1, "{on_t950:?}");
    assert!(
        on_t950[0].contains("than C-952 (floor 21, gate T2)"),
        "{on_t950:?}"
    );
    // Case 2, at (F21, T3): only the first clue is no later (floor 4 <= 21, gate T3 <= T3); the
    // second is not. One early clue is enough: no finding.
    edit(&project, "lore/clues/C-952.md", "gate: T2", "gate: T3")?;
    let findings = found(&project, REQUIRES)?;
    assert_eq!(messages(&findings, "lore/truths/T-950.md").len(), 0);
    // Case 3, at (F57, T3): both clues are no later: no finding.
    edit(
        &project,
        "lore/clues/C-952.md",
        "f21-synthetic",
        "f57-synthetic",
    )?;
    let findings = found(&project, REQUIRES)?;
    assert_eq!(messages(&findings, "lore/truths/T-950.md").len(), 0);
    Ok(())
}

#[test]
fn requires_names_the_first_failing_clue_not_the_first_clue() -> TestResult {
    // T-966, which T-960 requires, is confirmed at (F21, T2) by a new clue and at (F57, T2). The
    // lowest clue that confirms T-960, C-990 at (F21, T2), has a confirming clue no later than it
    // and passes; C-999 at (F30, T1) has none and fails.
    let project = TempProject::copy_of("lore")?;
    add_floor(&project, 30)?;
    add_clue(
        &project,
        "C-998",
        "T-966",
        "confirmed",
        "f21-synthetic",
        "T2",
    )?;
    add_clue(
        &project,
        "C-999",
        "T-960",
        "confirmed",
        "f30-synthetic",
        "T1",
    )?;
    let findings = found(&project, REQUIRES)?;
    let on_t960 = messages(&findings, "lore/truths/T-960.md");
    assert_eq!(on_t960.len(), 1, "{on_t960:?}");
    assert!(
        on_t960[0].contains("than C-999 (floor 30, gate T1)"),
        "{on_t960:?}"
    );
    Ok(())
}

#[test]
fn requires_names_the_lowest_failing_clue_once_per_item() -> TestResult {
    // T-960 requires T-966, confirmed at (F57, T2); every clue that confirms T-960 sits earlier.
    let project = TempProject::copy_of("lore")?;
    add_clue(
        &project,
        "C-995",
        "T-960",
        "confirmed",
        "synthetic-hall",
        "T3",
    )?;
    add_clue(
        &project,
        "C-996",
        "T-960",
        "confirmed",
        "f21-synthetic",
        "T1",
    )?;
    add_clue(
        &project,
        "C-997",
        "T-960",
        "confirmed",
        "f21-synthetic",
        "T2",
    )?;
    for (named, gone) in [
        ("C-995", None),
        ("C-996", Some("C-995")),
        ("C-990", Some("C-996")),
    ] {
        if let Some(gone) = gone {
            project.remove(&format!("lore/clues/{gone}.md"))?;
        }
        let findings = found(&project, REQUIRES)?;
        let on_t960 = messages(&findings, "lore/truths/T-960.md");
        assert_eq!(on_t960.len(), 1, "{on_t960:?}");
        assert!(
            on_t960[0].contains(&format!("than {named} (")),
            "{named}: {on_t960:?}"
        );
    }
    Ok(())
}

/// A twin of T-960 with the given target, next to the original.
fn twin_of_t960(project: &TempProject, target: &str) -> TestResult {
    let text = std::fs::read_to_string(project.root().join("lore/truths/T-960.md"))?;
    let twin = text.replace("target: confirmed", &format!("target: {target}"));
    project.write("lore/truths/synthetic-copy/T-960.md", &twin)?;
    Ok(())
}

#[test]
fn requires_judges_a_truth_by_its_own_target_not_a_twins() -> TestResult {
    let original = "lore/truths/T-960.md";
    let twin = "lore/truths/synthetic-copy/T-960.md";
    // The original has no valid target, its twin does: only the twin is judged.
    let project = TempProject::copy_of("lore")?;
    twin_of_t960(&project, "confirmed")?;
    edit(&project, original, "target: confirmed", "target: maybe")?;
    let findings = found(&project, REQUIRES)?;
    assert_eq!(messages(&findings, original), Vec::<String>::new());
    assert_eq!(messages(&findings, twin).len(), 1);
    // The original is confirmed, its twin hinted with an earlier hinting clue: each is judged by
    // the clues of its own target.
    let project = TempProject::copy_of("lore")?;
    twin_of_t960(&project, "hinted")?;
    add_clue(&project, "C-995", "T-960", "hinted", "synthetic-hall", "T1")?;
    let findings = found(&project, REQUIRES)?;
    let on_original = messages(&findings, original);
    assert_eq!(on_original.len(), 1, "{on_original:?}");
    assert!(on_original[0].contains("than C-990 ("), "{on_original:?}");
    let on_twin = messages(&findings, twin);
    assert!(
        on_twin.iter().any(|m| m.contains("than C-995 (")),
        "{on_twin:?}"
    );
    Ok(())
}

#[test]
fn order_rules_skip_unknown_floors_and_gates() -> TestResult {
    // A place without a floor: the tower itself.
    let project = TempProject::copy_of("lore")?;
    for (rel, from) in [
        ("lore/clues/C-961.md", "where: [f21-synthetic]"),
        ("lore/clues/C-981.md", "where: [f50-synthetic]"),
        ("lore/clues/C-991.md", "where: [f57-synthetic]"),
    ] {
        edit(&project, rel, from, "where: [tower]")?;
    }
    let order = found(&project, ORDER)?;
    let requires = found(&project, REQUIRES)?;
    // C-960 after C-961, C-982 above C-981 and T-960 above C-991 are skipped ...
    assert!(order.iter().all(|f| f.path != "lore/clues/C-960.md"));
    assert!(order.iter().all(|f| !f.message.contains("T-958")));
    assert!(requires.iter().all(|f| f.path != "lore/truths/T-960.md"));
    // ... while the other findings stay.
    assert!(order.iter().any(|f| f.path == "lore/clues/C-962.md"));
    assert!(requires.iter().any(|f| f.path == "lore/truths/T-961.md"));
    // An invalid gate is skipped the same way.
    let project = TempProject::copy_of("lore")?;
    edit(&project, "lore/clues/C-961.md", "gate: T2", "gate: T9")?;
    edit(&project, "lore/clues/C-990.md", "gate: T2", "gate: T9")?;
    let order = found(&project, ORDER)?;
    let requires = found(&project, REQUIRES)?;
    assert!(order.iter().all(|f| f.path != "lore/clues/C-960.md"));
    assert!(requires.iter().all(|f| f.path != "lore/truths/T-960.md"));
    assert!(order.iter().any(|f| f.path == "lore/clues/C-962.md"));
    assert!(requires.iter().any(|f| f.path == "lore/truths/T-961.md"));
    Ok(())
}
