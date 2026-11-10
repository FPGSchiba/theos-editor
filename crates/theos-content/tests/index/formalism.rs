//! The Shared Formalism mirror (spec §5.14) over the `formalism/` fixture project: its
//! `_source.json` has 18 equations and no records; the tests generate the records into a
//! temporary copy.

use theos_content::formalism::{self, FormalismError, Regenerated};
use theos_content::{Finding, Index, parts};

use crate::common::{TempProject, expected, fixture, keys};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const E7: &str = "canon/formalism/E7.md";
const SOURCE: &str = "canon/formalism/_source.json";

/// The bytes the fixture's E7 must have.
const E7_TEXT: &str = "---\n# generated from canon/formalism/_source.json by theos; a hand edit fails the check\nid: E7\nkind: equation\ntitle: Synthetic equation 7\nsource: https://example.invalid/synthetic-formalism#E7\ngenerated: true\n---\n$x_{7} = 7$\n";

/// A copy of the fixture with its 18 records generated.
fn generated() -> Result<TempProject, Box<dyn std::error::Error>> {
    let project = TempProject::copy_of("formalism")?;
    formalism::regenerate(project.root())?;
    Ok(project)
}

/// The findings of a fresh index over `project`.
fn findings(project: &TempProject) -> Result<Vec<Finding>, Box<dyn std::error::Error>> {
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    Ok(index.findings()?)
}

fn of_rule(findings: &[Finding], rule: &str) -> Vec<Finding> {
    findings
        .iter()
        .filter(|finding| finding.rule == rule)
        .cloned()
        .collect()
}

fn read(project: &TempProject, rel: &str) -> Result<String, std::io::Error> {
    std::fs::read_to_string(project.root().join(rel))
}

/// The source of the fixture, as the index would parse it.
fn fixture_source() -> Result<formalism::Source, Box<dyn std::error::Error>> {
    Ok(formalism::parse(&std::fs::read_to_string(
        fixture("formalism").join(SOURCE),
    )?)?)
}

// AC: CE-16.1
#[test]
fn the_source_generates_e1_to_e18() -> TestResult {
    let files = formalism::render(&fixture_source()?)?;
    assert_eq!(files.len(), 18);
    let paths: Vec<&str> = files.iter().map(|(path, _)| path.as_str()).collect();
    assert_eq!(paths[0], "canon/formalism/E1.md");
    assert_eq!(paths[17], "canon/formalism/E18.md");
    let e7 = files.iter().find(|(path, _)| path == E7).map(|(_, t)| t);
    assert_eq!(e7.map(String::as_str), Some(E7_TEXT));

    let project = TempProject::copy_of("formalism")?;
    let done = formalism::regenerate(project.root())?;
    assert_eq!(done.written.len(), 18);
    assert_eq!(read(&project, E7)?, E7_TEXT);
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    assert_eq!(index.records_of_kind("equation")?.len(), 18);
    assert_eq!(
        keys(&index.findings()?),
        expected(&fixture("formalism").join("expected.toml"))?
    );
    Ok(())
}

// AC: CE-16.1
#[test]
fn a_bracket_pair_in_latex_is_verbatim() -> TestResult {
    let project = generated()?;
    assert!(read(&project, "canon/formalism/E3.md")?.contains("$\\left[[a, b]\\right]$"));
    let found = findings(&project)?;
    assert!(
        found.iter().all(|f| f.path != "canon/formalism/E3.md"),
        "{found:?}"
    );
    Ok(())
}

// AC: CE-16.2
#[test]
fn a_record_citing_e7_resolves() -> TestResult {
    let project = generated()?;
    let found = findings(&project)?;
    assert!(
        found.iter().all(|f| !f.rule.starts_with("link/")),
        "{found:?}"
    );
    project.remove(E7)?;
    let found = findings(&project)?;
    let dangling = of_rule(&found, "link/dangling");
    assert_eq!(dangling.len(), 1, "{found:?}");
    assert_eq!(dangling[0].path, "lore/truths/T-950.md");
    assert_eq!(of_rule(&found, "formalism/missing").len(), 1);
    Ok(())
}

// AC: CE-16.3
#[test]
fn a_hand_edit_fails_the_check() -> TestResult {
    let original = E7_TEXT;
    let edits = [
        original.replace(
            "title: Synthetic equation 7",
            "title: Synthetic equation seven",
        ),
        original.replace("$x_{7} = 7$", "$x_{7} = 8$"),
        original.replace("$x_{7} = 7$", "$x_{7} = 7$ "),
        original.replace('\n', "\r\n"),
    ];
    for edited in edits {
        let project = generated()?;
        project.write(E7, &edited)?;
        let found = findings(&project)?;
        let hits = of_rule(&found, "formalism/edited");
        assert_eq!(hits.len(), 1, "{edited:?}: {found:?}");
        assert_eq!(hits[0].path, E7);
        assert_eq!(hits[0].id.as_deref(), Some("E7"));
        // The finding sits on the value of `generated`.
        let line = edited
            .lines()
            .position(|l| l == "generated: true")
            .map(|i| i + 1);
        assert_eq!(hits[0].position.map(|p| p.line), line);
        assert_eq!(hits[0].position.map(|p| p.column), Some(12));

        let done = formalism::regenerate(project.root())?;
        assert_eq!(done.written, [E7]);
        assert_eq!(read(&project, E7)?, E7_TEXT);
        assert_eq!(
            of_rule(&findings(&project)?, "formalism/edited"),
            Vec::new()
        );
    }
    Ok(())
}

// AC: CE-16.4
#[test]
fn regenerating_is_idempotent() -> TestResult {
    let project = TempProject::copy_of("formalism")?;
    formalism::regenerate(project.root())?;
    let before: Vec<String> = (1..=18)
        .map(|n| read(&project, &format!("canon/formalism/E{n}.md")))
        .collect::<Result<_, _>>()?;
    let again = formalism::regenerate(project.root())?;
    assert_eq!(again.written, Vec::<String>::new());
    assert_eq!(again.unchanged.len(), 18);
    let after: Vec<String> = (1..=18)
        .map(|n| read(&project, &format!("canon/formalism/E{n}.md")))
        .collect::<Result<_, _>>()?;
    assert_eq!(before, after);

    // Also after a repair.
    project.write(E7, "edited\n")?;
    let repaired = formalism::regenerate(project.root())?;
    assert_eq!(repaired.written, [E7]);
    let last = formalism::regenerate(project.root())?;
    assert_eq!(
        last,
        Regenerated {
            written: Vec::new(),
            unchanged: again.unchanged.clone(),
        }
    );
    let third: Vec<String> = (1..=18)
        .map(|n| read(&project, &format!("canon/formalism/E{n}.md")))
        .collect::<Result<_, _>>()?;
    assert_eq!(before, third);
    Ok(())
}

#[test]
fn a_stray_equation_and_a_missing_one_are_findings() -> TestResult {
    let project = generated()?;
    // A record the source does not list.
    let stray = E7_TEXT.replace("E7", "E99");
    project.write("canon/formalism/E99.md", &stray)?;
    project.remove("canon/formalism/E2.md")?;
    let found = findings(&project)?;
    let strays = of_rule(&found, "formalism/stray");
    assert_eq!(strays.len(), 1, "{found:?}");
    assert_eq!(strays[0].path, "canon/formalism/E99.md");
    assert_eq!(strays[0].id.as_deref(), Some("E99"));
    assert_eq!(strays[0].position.map(|p| (p.line, p.column)), Some((3, 5)));
    let missing = of_rule(&found, "formalism/missing");
    assert_eq!(missing.len(), 1, "{found:?}");
    assert_eq!(missing[0].path, SOURCE);
    assert_eq!(missing[0].id, None);
    assert_eq!(
        missing[0].position.map(|p| (p.line, p.column)),
        Some((1, 1))
    );
    assert!(missing[0].message.contains("E2"), "{}", missing[0].message);
    // `regenerate` never deletes the stray record, and writes the missing one.
    let done = formalism::regenerate(project.root())?;
    assert_eq!(done.written, ["canon/formalism/E2.md"]);
    assert!(project.root().join("canon/formalism/E99.md").exists());
    Ok(())
}

#[test]
fn every_equation_record_is_stray_without_a_source() -> TestResult {
    let project = generated()?;
    project.remove(SOURCE)?;
    let found = findings(&project)?;
    assert_eq!(of_rule(&found, "formalism/stray").len(), 18, "{found:?}");
    assert_eq!(of_rule(&found, "formalism/source"), Vec::new());
    assert_eq!(of_rule(&found, "formalism/missing"), Vec::new());
    assert!(matches!(
        formalism::regenerate(project.root()),
        Err(FormalismError::Missing)
    ));
    Ok(())
}

#[test]
fn a_broken_source_is_a_finding_with_its_position() -> TestResult {
    let project = generated()?;
    // A syntax error: serde_json's line and column.
    project.write(SOURCE, "{\n  \"format\": 1,\n  oops\n}\n")?;
    let found = findings(&project)?;
    let hits = of_rule(&found, "formalism/source");
    assert_eq!(hits.len(), 1, "{found:?}");
    assert_eq!(hits[0].path, SOURCE);
    assert_eq!(hits[0].id, None);
    assert_eq!(hits[0].position.map(|p| (p.line, p.column)), Some((3, 3)));
    // A broken source gives no stray, edited or missing finding.
    assert_eq!(of_rule(&found, "formalism/stray"), Vec::new());
    assert_eq!(of_rule(&found, "formalism/edited"), Vec::new());
    assert_eq!(of_rule(&found, "formalism/missing"), Vec::new());

    // A rule of the format: 1:1, naming the equation.
    let text = std::fs::read_to_string(fixture("formalism").join(SOURCE))?;
    project.write(SOURCE, &text.replace("\"E4\"", "\"E04\""))?;
    let found = findings(&project)?;
    let hits = of_rule(&found, "formalism/source");
    assert_eq!(hits.len(), 1, "{found:?}");
    assert_eq!(hits[0].position.map(|p| (p.line, p.column)), Some((1, 1)));
    assert!(hits[0].message.contains("`E04`"), "{}", hits[0].message);

    // Bytes that are not UTF-8.
    project.write_fresh(SOURCE, "")?;
    std::fs::write(project.root().join(SOURCE), [0xff, 0xfe, 0x00])?;
    let found = findings(&project)?;
    assert_eq!(of_rule(&found, "formalism/source").len(), 1, "{found:?}");
    Ok(())
}

#[test]
fn a_source_change_re_runs_the_checks_of_the_records() -> TestResult {
    let project = generated()?;
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    assert_eq!(of_rule(&index.findings()?, "formalism/edited"), Vec::new());
    let text = std::fs::read_to_string(project.root().join(SOURCE))?;
    project.write(
        SOURCE,
        &text.replace("Synthetic equation 7", "Synthetic equation seven"),
    )?;
    index.sync()?;
    let hits = of_rule(&index.findings()?, "formalism/edited");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, E7);
    Ok(())
}
