//! Renaming ids in one record as span edits: the id field, front-matter links in their own
//! style, and body links.

use std::collections::BTreeSet;

use theos_content::index::LinkRef;
use theos_content::rename::{IdMap, RenameError, edits};
use theos_content::{Change, Index, Record, Span, apply_edits, parts};

use crate::common::TempProject;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const PATH: &str = "lore/clues/C-960.md";

fn map(pairs: &[(&str, &str)]) -> IdMap {
    pairs
        .iter()
        .map(|(old, new)| ((*old).to_owned(), (*new).to_owned()))
        .collect()
}

/// The record `text` indexed at [`PATH`], with its links.
fn indexed(text: &str) -> Result<(Record, Vec<LinkRef>), Box<dyn std::error::Error>> {
    let temp = TempProject::new()?;
    let project = temp.project()?;
    let mut index = Index::in_memory(&project, &parts::builtin()?)?;
    index.apply(&[Change::text(PATH, text)])?;
    Ok((Record::parse(text)?, index.links_from(PATH)?))
}

/// `text` after renaming `pairs`.
fn renamed(text: &str, pairs: &[(&str, &str)]) -> Result<String, Box<dyn std::error::Error>> {
    let (record, links) = indexed(text)?;
    let edits = edits(&record, &links, &map(pairs))?;
    Ok(apply_edits(text, &edits)?)
}

fn clue(id_line: &str, rest: &str) -> String {
    format!(
        "---\n# synthetic fixture (P06), not canon\n{id_line}\nkind: clue\ntitle: T-960\n{rest}---\nBody.\n"
    )
}

#[test]
fn renames_the_id_field_in_its_style() -> TestResult {
    // AC: CE-20.3
    for (old, new) in [
        ("id: T-960", "id: T-970"),
        ("id: 'T-960'", "id: 'T-970'"),
        ("id: \"T-960\"", "id: \"T-970\""),
    ] {
        let got = renamed(&clue(old, ""), &[("T-960", "T-970")])?;
        assert_eq!(got, clue(new, ""), "{old}");
    }
    Ok(())
}

#[test]
fn renames_a_field_link_and_keeps_its_quoting() -> TestResult {
    // AC: CE-20.3
    let renames = [("T-960", "T-970")];
    for (old, new) in [
        ("reveals: \"T-960\"\n", "reveals: \"T-970\"\n"),
        ("reveals: T-960\n", "reveals: T-970\n"),
        ("reveals: 'T-960'\n", "reveals: 'T-970'\n"),
    ] {
        let got = renamed(&clue("id: C-960", old), &renames)?;
        assert_eq!(got, clue("id: C-960", new), "{old}");
    }
    // A new id that needs quotes in a plain place is double-quoted.
    let got = renamed(&clue("id: C-960", "reveals: T-960\n"), &[("T-960", "a: b")])?;
    assert_eq!(got, clue("id: C-960", "reveals: \"a: b\"\n"));
    Ok(())
}

#[test]
fn renames_list_items_and_keeps_comments() -> TestResult {
    // AC: CE-20.3
    let renames = [("f04-synthetic", "f04-renamed")];
    let got = renamed(
        &clue("id: C-960", "where: [f04-synthetic, f05-synthetic]\n"),
        &renames,
    )?;
    assert_eq!(
        got,
        clue("id: C-960", "where: [f04-renamed, f05-synthetic]\n")
    );
    let got = renamed(
        &clue(
            "id: C-960",
            "where:\n  - f04-synthetic  # synthetic comment\n  - f05-synthetic\n",
        ),
        &renames,
    )?;
    assert_eq!(
        got,
        clue(
            "id: C-960",
            "where:\n  - f04-renamed  # synthetic comment\n  - f05-synthetic\n"
        )
    );
    Ok(())
}

#[test]
fn renames_lens_entries() -> TestResult {
    // AC: CE-20.3
    let text = |a: &str, b: &str| {
        format!(
            "---\n# synthetic fixture (P06), not canon\nid: synthetic-npc\nkind: npc\ntitle: N\nlens:\n  unaware_of: [{a}]\n  rejects: ['{b}']\n---\nBody.\n"
        )
    };
    let got = renamed(
        &text("T-960", "B-960"),
        &[("T-960", "T-970"), ("B-960", "B-970")],
    )?;
    assert_eq!(got, text("T-970", "B-970"));
    Ok(())
}

#[test]
fn renames_body_links_and_keeps_fragment_and_shown_text() -> TestResult {
    // AC: CE-20.3
    let body = |a: &str, b: &str| {
        format!(
            "---\n# synthetic fixture (P06), not canon\nid: S-960\nkind: script\ntitle: S\nstatus: draft\n---\n\
See [[{a}#2|the truth]].\n\n[[{a}]] and [[{a}]] and [[{b}]]\n\n\
## Beat 1 — Opening\nat:: [[{b}]]\n\n\
> [!voice] [[{b}]] · [[{a}]]\n> Words.\n"
        )
    };
    let got = renamed(
        &body("T-960", "f04-synthetic"),
        &[("T-960", "T-970"), ("f04-synthetic", "f04-renamed")],
    )?;
    assert_eq!(got, body("T-970", "f04-renamed"));

    let note = |a: &str, b: &str| {
        format!(
            "---\n# synthetic fixture (P06), not canon\nid: synthetic-concept\nkind: concept\ntitle: C\n---\n\
Brief.\n\n## Notes\n\n> [!note] n1 · [[{a}]] · open\n> Text.\n\n> [!note] n2 · resolved by [[{b}]]\n> Text.\n"
        )
    };
    let got = renamed(&note("c-v1", "c-v2"), &[("c-v1", "c-v7"), ("c-v2", "c-v8")])?;
    assert_eq!(got, note("c-v7", "c-v8"));
    Ok(())
}

#[test]
fn leaves_text_that_only_looks_like_an_id() -> TestResult {
    // AC: CE-20.3
    let text = clue(
        "id: C-960",
        "reveals: T-961\nwhere: [T-9600, f01-synthetic]\n",
    );
    let (record, links) = indexed(&text)?;
    let edits = edits(&record, &links, &map(&[("T-960", "T-970")]))?;
    assert!(edits.is_empty(), "{edits:?}");

    let text = clue("id: C-960", "reveals: T-960\n");
    let (record, links) = indexed(&text)?;
    let edits = edits_of(&record, &links, &[("T-960", "T-970")])?;
    let touched: BTreeSet<_> = edits.iter().map(|e| e.span).collect();
    assert_eq!(touched.len(), 1);
    let span = *touched.iter().next().ok_or("no edit")?;
    assert_eq!(&text[span.start..span.end], "T-960");
    let got = apply_edits(&text, &edits)?;
    assert_eq!(got[..span.start], text[..span.start]);
    assert_eq!(got[span.start + 5..], text[span.end..]);
    assert!(got.contains("title: T-960"), "the title is not an id");
    Ok(())
}

fn edits_of(
    record: &Record,
    links: &[LinkRef],
    pairs: &[(&str, &str)],
) -> Result<Vec<theos_content::TextEdit>, RenameError> {
    edits(record, links, &map(pairs))
}

#[test]
fn a_span_the_record_does_not_have_is_an_error() -> TestResult {
    // AC: CE-20.3
    let text = clue("id: C-960", "reveals: T-960\n");
    let (record, mut links) = indexed(&text)?;
    for link in &mut links {
        link.span = Span::new(link.span.start + 1, link.span.end + 1);
    }
    let error = edits_of(&record, &links, &[("T-960", "T-970")]);
    assert!(
        matches!(error, Err(RenameError::NoScalar { .. })),
        "{error:?}"
    );

    let text = "---\nid: S-960\nkind: script\ntitle: S\nstatus: draft\n---\nSee [[T-960]].\n";
    let (record, mut links) = indexed(text)?;
    for link in &mut links {
        link.span = Span::new(link.span.start + 1, link.span.end + 1);
    }
    let error = edits_of(&record, &links, &[("T-960", "T-970")]);
    assert!(
        matches!(error, Err(RenameError::NoLink { .. })),
        "{error:?}"
    );
    Ok(())
}
