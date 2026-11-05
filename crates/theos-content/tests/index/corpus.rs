//! P02's corpus: `records/` and `malformed/` as one content project, and the files of `crlf/`
//! and `rejected/` one at a time.

use theos_content::{Finding, Index, parts};

use crate::common::{TempProject, corpus_dir, expected_index, keys};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The findings of a project that holds one file, `bytes` at `rel`.
fn alone(rel: &str, bytes: &[u8]) -> Result<Vec<Finding>, Box<dyn std::error::Error>> {
    let project = TempProject::new()?.with_file(rel, bytes)?;
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    Ok(index.findings()?)
}

/// What two findings must share when they come from twin files: everything but the byte span.
fn shape(findings: &[Finding]) -> Vec<String> {
    findings
        .iter()
        .map(|f| {
            let at = f.position.map_or((0, 0), |p| (p.line, p.column));
            format!(
                "{} {} {} {:?}: {}",
                f.path,
                f.rule,
                f.severity.as_str(),
                at,
                f.message
            )
        })
        .collect()
}

#[derive(serde::Deserialize)]
struct Twin {
    input: String,
    lf: String,
}

#[derive(serde::Deserialize)]
struct Twins {
    twin: Vec<Twin>,
}

#[derive(serde::Deserialize)]
struct Case {
    file: String,
    rule: String,
    line: usize,
    column: usize,
}

#[derive(serde::Deserialize)]
struct Cases {
    case: Vec<Case>,
}

// AC: CE-11.1
#[test]
fn the_corpus_project_reports_exactly_its_findings() -> TestResult {
    let project = TempProject::corpus()?;
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    let expected = expected_index(&corpus_dir().join("expected-index.toml"))?;
    assert_eq!(keys(&index.findings()?), expected.project);
    Ok(())
}

// AC: CE-11.1
#[test]
fn crlf_files_report_their_twins_findings() -> TestResult {
    let crlf = corpus_dir().join("crlf");
    let twins: Twins = toml::from_str(&std::fs::read_to_string(crlf.join("twins.toml"))?)?;
    let expected = expected_index(&corpus_dir().join("expected-index.toml"))?;
    assert_eq!(twins.twin.len(), 6);
    for twin in twins.twin {
        // A twin among the records sits where it sits there; the others are things.
        let rel = match twin.lf.strip_prefix("../records/") {
            Some(own) => own.to_owned(),
            None => format!("things/{}", twin.input),
        };
        let input = alone(&rel, &std::fs::read(crlf.join(&twin.input))?)?;
        let lf = alone(&rel, &std::fs::read(crlf.join(&twin.lf))?)?;
        assert_eq!(shape(&input), shape(&lf), "{}", twin.input);
        if !twin.lf.starts_with("../records/") {
            let listed: Vec<_> = expected
                .crlf
                .iter()
                .filter(|(name, _)| *name == twin.input)
                .map(|(_, key)| key.clone())
                .collect();
            assert_eq!(keys(&input), listed, "{}", twin.input);
        }
    }
    let unlisted = expected.crlf.iter().filter(|(name, _)| {
        !["synthetic-crlf.md", "mixed.md", "lone-cr.md"].contains(&name.as_str())
    });
    assert_eq!(unlisted.count(), 0);
    Ok(())
}

// AC: CE-11.1
#[test]
fn rejected_files_report_only_their_rule() -> TestResult {
    let rejected = corpus_dir().join("rejected");
    let cases: Cases = toml::from_str(&std::fs::read_to_string(rejected.join("expected.toml"))?)?;
    let files = std::fs::read_dir(&rejected)?.count() - 1;
    assert_eq!(cases.case.len(), files, "a rejected file has no case");
    for case in cases.case {
        let group = match case.rule.as_str() {
            "bom" | "opening-line" | "closing-line" => "record",
            _ => "yaml",
        };
        let found = alone(&case.file, &std::fs::read(rejected.join(&case.file))?)?;
        assert_eq!(
            keys(&found),
            vec![(
                case.file.clone(),
                format!("{group}/{}", case.rule),
                "error".to_owned(),
                case.line,
                case.column
            )],
            "{}",
            case.file
        );
    }
    Ok(())
}
