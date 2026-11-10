//! Value checks over concepts, maps and their versions (spec §5.10).

use theos_content::{Finding, Index, Project, Severity, parts};

use crate::common::{TempProject, expected, fixture, keys};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const ALPHA: &str = "concepts/synthetic-alpha.md";
const ALPHA_V1: &str = "concepts/synthetic-alpha/synthetic-alpha-v1.md";
const ALPHA_V2: &str = "concepts/synthetic-alpha/synthetic-alpha-v2.md";
const ORPHAN: &str = "concepts/synthetic-nowhere/synthetic-nowhere-v1.md";

fn indexed(project: &Project) -> Result<Index, Box<dyn std::error::Error>> {
    let mut index = Index::in_memory(project, &parts::builtin()?)?;
    index.sync()?;
    Ok(index)
}

fn of_rule<'a>(findings: &'a [Finding], rule: &str) -> Vec<&'a Finding> {
    findings.iter().filter(|f| f.rule == rule).collect()
}

// AC: CE-14.2
#[test]
fn version_findings_equal_the_expectation() -> TestResult {
    let index = indexed(&Project::open(&fixture("versions"))?)?;
    assert_eq!(
        keys(&index.findings()?),
        expected(&fixture("versions").join("expected.toml"))?
    );
    Ok(())
}

// AC: CE-15.5
#[test]
fn addresses_naming_a_missing_note_is_an_error() -> TestResult {
    let findings = indexed(&Project::open(&fixture("versions"))?)?.findings()?;
    let in_v1: Vec<_> = findings
        .iter()
        .filter(|f| f.rule == "version/addresses" && f.path == ALPHA_V1)
        .collect();
    assert_eq!(in_v1.len(), 1, "n1 exists and gives nothing, n9 does not");
    let finding = in_v1[0];
    assert_eq!(finding.severity, Severity::Error);
    let text = std::fs::read_to_string(fixture("versions").join(ALPHA_V1))?;
    let at = text.find("n9").ok_or("n9 not in the fixture")?;
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
fn a_version_whose_parent_dangles_is_not_judged() -> TestResult {
    let findings = indexed(&Project::open(&fixture("versions"))?)?.findings()?;
    let orphan: Vec<&str> = findings
        .iter()
        .filter(|f| f.path == ORPHAN)
        .map(|f| f.rule.as_str())
        .collect();
    assert_eq!(orphan, ["link/dangling"], "its note n9 is not judged");
    assert_eq!(of_rule(&findings, "version/addresses").len(), 3);
    Ok(())
}

// AC: CE-14.3
#[test]
fn removing_a_note_revalidates_the_versions_that_address_it() -> TestResult {
    let temp = TempProject::copy_of("versions")?;
    let mut index = indexed(&temp.project()?)?;
    assert_eq!(index.findings_for(ALPHA_V2)?, Vec::<Finding>::new());

    let concept = std::fs::read_to_string(temp.root().join(ALPHA))?;
    let at = concept
        .find("> [!note] n2")
        .ok_or("n2 not in the fixture")?;
    temp.write(ALPHA, concept[..at].trim_end())?;
    let update = index.sync()?;
    assert_eq!(update.reparsed, [ALPHA]);
    assert!(
        update.revalidated.iter().any(|path| path == ALPHA_V2),
        "{:?}",
        update.revalidated
    );
    let findings = index.findings_for(ALPHA_V2)?;
    assert_eq!(of_rule(&findings, "version/addresses").len(), 1);
    let cold = indexed(&temp.project()?)?;
    assert_eq!(index.dump()?, cold.dump()?);
    Ok(())
}
