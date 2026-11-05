//! The `rejected/` corpus: every file fails with the rule, line and column of its case.

use std::error::Error;
use std::fs;

use theos_content::{ReadError, Record, decode};

use crate::common::{cases, corpus, md_files};

fn failure(bytes: &[u8]) -> Option<(String, usize, usize)> {
    match decode(bytes) {
        Err(ReadError::Utf8 { position }) => {
            Some(("utf8".to_owned(), position.line, position.column))
        }
        Err(_) => None,
        Ok(text) => match Record::parse(text) {
            Err(error) => {
                let at = error.position();
                Some((error.rule().to_owned(), at.line, at.column))
            }
            Ok(record) => record.front_matter_error().map(|error| {
                let rule = error.rule.as_str().to_owned();
                (rule, error.position.line, error.position.column)
            }),
        },
    }
}

// AC: CE-09.3
#[test]
fn rejected_files_fail_with_rule_line_and_column() -> Result<(), Box<dyn Error>> {
    let dir = corpus().join("rejected");
    let cases = cases(&dir.join("expected.toml"))?;
    let files = md_files(&dir)?;
    assert!(!files.is_empty(), "the rejected corpus is not empty");
    assert_eq!(files.len(), cases.len(), "one case per file");
    for file in files {
        let name = file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let matching: Vec<_> = cases.iter().filter(|case| case.file == name).collect();
        assert_eq!(matching.len(), 1, "exactly one case for {name}");
        let case = matching[0];
        let got = failure(&fs::read(&file)?);
        let want = (
            case.rule.clone().unwrap_or_default(),
            case.line,
            case.column,
        );
        assert_eq!(got, Some(want), "{name}");
    }
    Ok(())
}

#[test]
fn invalid_utf8_is_rejected_with_its_position() {
    let bytes = b"---\n# synthetic fixture (P02), not canon\nid: \xff\n---\n";
    assert_eq!(failure(bytes), Some(("utf8".to_owned(), 3, 5)));
}
