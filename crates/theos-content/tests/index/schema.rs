//! The `schema/` fixture project: one record per rule of spec §5.5 and per field type of §5.4,
//! valid and invalid.

use theos_content::{Change, Finding, Index, Project, parts};

use crate::common::{expected, fixture, keys};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Bytes that are not UTF-8: the 0xE9 of a Latin-1 "café".
const LATIN_1: &[u8] = b"---\nid: synthetic-latin-1\nkind: thing\ntitle: caf\xe9\n---\n";

fn indexed() -> Result<Index, Box<dyn std::error::Error>> {
    let project = Project::open(&fixture("schema"))?;
    let mut index = Index::in_memory(&project, &parts::builtin()?)?;
    index.sync()?;
    Ok(index)
}

fn at(findings: &[Finding], path: &str) -> Vec<(String, String)> {
    findings
        .iter()
        .filter(|f| f.path == path)
        .map(|f| (f.rule.clone(), f.severity.as_str().to_owned()))
        .collect()
}

// AC: CE-11.1
#[test]
fn schema_fixtures_report_exactly_their_findings() -> TestResult {
    let mut index = indexed()?;
    index.apply(&[Change::Content {
        path: "things/synthetic-latin-1.md".to_owned(),
        bytes: LATIN_1.to_vec(),
    }])?;
    let mut want = expected(&fixture("schema").join("expected.toml"))?;
    want.push((
        "things/synthetic-latin-1.md".to_owned(),
        "record/utf8".to_owned(),
        "error".to_owned(),
        4,
        11,
    ));
    let mut got = keys(&index.findings()?);
    got.sort();
    want.sort();
    assert_eq!(got, want);
    Ok(())
}

// AC: CE-11.4
#[test]
fn unknown_fields_warn() -> TestResult {
    let index = indexed()?;
    let findings = index.findings()?;
    assert_eq!(
        at(&findings, "things/synthetic-unknown.md"),
        [("schema/unknown-field".to_owned(), "warning".to_owned())]
    );
    assert_eq!(
        at(&findings, "people/synthetic-folk-unknown.md"),
        [("schema/unknown-entry".to_owned(), "warning".to_owned())]
    );
    let field = keys(&findings)
        .into_iter()
        .find(|k| k.0 == "things/synthetic-unknown.md");
    assert_eq!(
        field.map(|k| (k.3, k.4)),
        Some((7, 1)),
        "at the key `colour`"
    );
    let entry = keys(&findings)
        .into_iter()
        .find(|k| k.0 == "people/synthetic-folk-unknown.md");
    assert_eq!(entry.map(|k| (k.3, k.4)), Some((8, 3)), "at the key `mood`");
    Ok(())
}
