//! The `malformed/` corpus: every construct is reported with its kind, line and column.

use std::error::Error;
use std::fs;

use theos_content::Record;

use crate::common::{cases, corpus, md_files, project_records};

// AC: CE-10.3
#[test]
fn malformed_files_report_each_construct() -> Result<(), Box<dyn Error>> {
    let dir = corpus().join("malformed");
    let cases = cases(&dir.join("expected.toml"))?;
    let files = md_files(&dir)?;
    assert!(!files.is_empty(), "the malformed corpus is not empty");
    for file in files {
        let name = file
            .strip_prefix(&dir)?
            .to_string_lossy()
            .replace('\\', "/");
        let mut want: Vec<_> = cases
            .iter()
            .filter(|case| case.file == name)
            .map(|case| {
                (
                    case.kind.clone().unwrap_or_default(),
                    case.line,
                    case.column,
                )
            })
            .collect();
        assert!(!want.is_empty(), "{name} has at least one case");
        want.sort();
        let record = Record::parse(&fs::read_to_string(&file)?)?;
        assert!(record.front_matter().is_ok(), "{name}: front matter is Ok");
        let mut got: Vec<_> = record
            .body()
            .malformed()
            .iter()
            .map(|m| {
                (
                    m.kind.as_str().to_owned(),
                    m.position.line,
                    m.position.column,
                )
            })
            .collect();
        got.sort();
        assert_eq!(got, want, "{name}");
    }
    Ok(())
}

#[test]
fn records_have_no_malformed_constructs() -> Result<(), Box<dyn Error>> {
    let dir = corpus().join("records");
    for file in project_records()?
        .into_iter()
        .filter(|f| f.starts_with(&dir))
    {
        let record = Record::parse(&fs::read_to_string(&file)?)?;
        let found = record.body().malformed();
        assert!(found.is_empty(), "{}: {found:?}", file.display());
    }
    Ok(())
}
