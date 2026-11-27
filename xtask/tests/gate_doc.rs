//! `docs/gate.md` says what `gate/code-paths.toml` does: the subsection on pull requests that
//! change no code lists the same globs, in the same order, and explains every check the rule
//! skips. Read through `xtask::markdown`, so a change of layout does not matter, only of content.

use std::error::Error;
use std::fs;
use std::path::Path;

use xtask::code_paths::CodePaths;
use xtask::markdown;
use xtask::verdict::{Check, GATE_ORDER};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

/// The heading of the subsection of §1 that holds the lists and the table.
const HEADING: &str = "Pull requests that change no code";

fn root() -> TestResult<&'static Path> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| "xtask has no parent folder".into())
}

/// The body of the subsection, as written in `docs/gate.md`.
fn subsection(gate_md: &str) -> TestResult<&str> {
    markdown::sections(gate_md, 3)
        .into_iter()
        .find(|section| section.heading.text == HEADING)
        .map(|section| section.body)
        .ok_or_else(|| format!("docs/gate.md has no subsection \"{HEADING}\"").into())
}

// AC: CE-08C.4
#[test]
fn gate_md_lists_the_code_paths_of_gate_toml() -> TestResult {
    let root = root()?;
    let gate_md = fs::read_to_string(root.join("docs/gate.md"))?;
    let listed: Vec<String> = markdown::list_items(subsection(&gate_md)?)
        .into_iter()
        .map(|item| item.text)
        .collect();
    let paths = CodePaths::load(root)?;
    let expected: Vec<String> = paths
        .non_code()
        .iter()
        .chain(paths.always_code())
        .cloned()
        .collect();
    assert_eq!(
        listed, expected,
        "the list items of the subsection must be non_code, then always_code, as in the file"
    );
    Ok(())
}

// AC: CE-08C.4
#[test]
fn gate_md_explains_every_check_a_docs_change_skips() -> TestResult {
    let gate_md = fs::read_to_string(root()?.join("docs/gate.md"))?;
    let tables = markdown::tables(subsection(&gate_md)?);
    let [table] = tables.as_slice() else {
        return Err(format!("expected one table, found {}", tables.len()).into());
    };
    let explained: Vec<&str> = table
        .rows
        .iter()
        .filter_map(|row| row.first())
        .flat_map(|cell| cell.split(','))
        .map(str::trim)
        .collect();
    for check in GATE_ORDER.into_iter().filter(|c| *c != Check::CommitMsg) {
        assert!(
            explained.contains(&check.name()),
            "the table has no row for `{}`; it has {explained:?}",
            check.name()
        );
    }
    for row in &table.rows {
        assert!(
            row.iter().all(|cell| !cell.is_empty()),
            "a row has an empty cell: {row:?}"
        );
    }
    Ok(())
}
