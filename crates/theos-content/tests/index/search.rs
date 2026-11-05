//! Full-text search over the `search/` fixture project: ranking, matching text, stemming and
//! plain-text queries.

use theos_content::{Change, Column, Hit, Index, Match, Project, parts};

use crate::common::{TempProject, expected, fixture, keys};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn indexed() -> Result<Index, Box<dyn std::error::Error>> {
    let project = Project::open(&fixture("search"))?;
    let mut index = Index::in_memory(&project, &parts::builtin()?)?;
    index.sync()?;
    Ok(index)
}

fn ids(hits: &[Hit]) -> Vec<&str> {
    hits.iter().map(|h| h.id.as_str()).collect()
}

fn hit<'a>(hits: &'a [Hit], id: &str) -> Result<&'a Hit, Box<dyn std::error::Error>> {
    hits.iter()
        .find(|h| h.id == id)
        .ok_or_else(|| format!("no hit {id}").into())
}

fn rank(hits: &[Hit], id: &str) -> Result<usize, Box<dyn std::error::Error>> {
    ids(hits)
        .iter()
        .position(|h| *h == id)
        .ok_or_else(|| format!("no hit {id}").into())
}

/// The LF text of a fixture file.
fn text_of(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(fixture("search").join(path))?.replace("\r\n", "\n"))
}

/// Checks a match against the file: its position is the span start, its line text the file line.
fn check_place(text: &str, m: &Match) {
    let before = &text[..m.span.start];
    assert_eq!(m.position.line, before.matches('\n').count() + 1);
    assert_eq!(
        m.position.column,
        before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1
    );
    assert_eq!(Some(m.line.as_str()), text.lines().nth(m.position.line - 1));
}

// AC: CE-13.1
#[test]
fn hits_are_ranked_with_their_matching_text() -> TestResult {
    let hits = indexed()?.search("lantern", 20)?;
    assert!(rank(&hits, "synthetic-lantern")? < rank(&hits, "synthetic-lamp")?);
    for hit in &hits {
        let text = text_of(&hit.path)?;
        for m in &hit.matches {
            if hit.id != "synthetic-quoted" {
                assert_eq!(text[m.span.start..m.span.end].to_lowercase(), "lantern");
            }
            check_place(&text, m);
        }
    }
    let lantern = hit(&hits, "synthetic-lantern")?;
    assert!(lantern.matches.iter().any(|m| m.column == Column::Title));
    let lamp = hit(&hits, "synthetic-lamp")?;
    assert_eq!(lamp.matches.len(), 1);
    assert_eq!(lamp.matches[0].column, Column::Body);
    assert_eq!(lamp.matches[0].line, "Synthetic: the lantern glows.");
    assert_eq!(lamp.kind.as_deref(), Some("thing"));
    assert_eq!(lamp.title.as_deref(), Some("Synthetic lamp"));
    assert!(lamp.score > 0.0);
    Ok(())
}

// AC: CE-13.1
#[test]
fn more_matches_rank_higher() -> TestResult {
    let hits = indexed()?.search("lantern", 20)?;
    assert!(rank(&hits, "synthetic-beacon")? < rank(&hits, "synthetic-lamp")?);
    assert_eq!(hit(&hits, "synthetic-beacon")?.matches.len(), 3);
    Ok(())
}

// AC: CE-13.1
#[test]
fn stems_accents_ids_and_phrases() -> TestResult {
    let index = indexed()?;
    let glow = index.search("glow", 20)?;
    assert!(ids(&glow).contains(&"synthetic-lamp"));
    assert!(ids(&glow).contains(&"synthetic-lantern"));
    for word in ["cafe", "mere"] {
        assert!(ids(&index.search(word, 20)?).contains(&"synthetic-cafe"));
    }
    let id = index.search("T-951", 20)?;
    let truth = hit(&id, "T-950")?;
    assert_eq!(truth.matches.len(), 2);
    assert!(truth.matches.iter().all(|m| m.column == Column::Body));
    assert_eq!(
        ids(&index.search("\"red stone\"", 20)?),
        ["synthetic-red-stone"]
    );
    let both = index.search("red stone", 20)?;
    assert_eq!(both.len(), 2);
    assert!(ids(&both).contains(&"synthetic-red-stone"));
    assert!(ids(&both).contains(&"synthetic-stone-red"));
    Ok(())
}

// AC: CE-13.1
#[test]
fn query_syntax_is_plain_text() -> TestResult {
    let index = indexed()?;
    for query in [
        "AND",
        "OR",
        "NEAR",
        "lantern*",
        "-lantern",
        "title:lantern",
        "^lantern",
        "\"lantern",
        "lantern\"",
    ] {
        index.search(query, 20)?;
    }
    assert_eq!(
        ids(&index.search("lantern*", 20)?),
        ids(&index.search("lantern", 20)?)
    );
    for query in ["\"\"", "\"   \"", "-", "nosuchword"] {
        assert!(index.search(query, 20)?.is_empty(), "{query}");
    }
    Ok(())
}

// AC: CE-13.1
#[test]
fn control_characters_in_a_query_separate_words() -> TestResult {
    let index = indexed()?;
    for query in ["a\0b", "\"lantern\0\"", "\u{7}", "lantern\u{1b}"] {
        index.search(query, 10)?;
    }
    assert_eq!(
        ids(&index.search("lantern\0", 20)?),
        ids(&index.search("lantern", 20)?)
    );
    assert_eq!(index.search("\0\u{7f}", 20)?.len(), 0);
    Ok(())
}

#[test]
fn a_quoted_value_with_escapes_matches_as_a_whole() -> TestResult {
    let hits = indexed()?.search("lantern", 20)?;
    let quoted = hit(&hits, "synthetic-quoted")?;
    let text = text_of(&quoted.path)?;
    let value = "\"Synthetic \\\"quoted\\\" lantern\"";
    let start = text.find(value).ok_or("value not found")?;
    let title = quoted
        .matches
        .iter()
        .find(|m| m.column == Column::Title)
        .ok_or("no title match")?;
    assert_eq!(
        (title.span.start, title.span.end),
        (start, start + value.len())
    );
    Ok(())
}

#[test]
fn a_noncharacter_keeps_the_rank_without_spans() -> TestResult {
    let hits = indexed()?.search("lantern", 20)?;
    let record = hit(&hits, "synthetic-noncharacter")?;
    assert!(record.matches.iter().all(|m| m.column != Column::Body));
    Ok(())
}

#[test]
fn a_broken_front_matter_is_found_by_its_body() -> TestResult {
    let hits = indexed()?.search("lantern", 20)?;
    assert!(ids(&hits).contains(&"synthetic-broken"));
    Ok(())
}

// AC: CE-13.1
#[test]
fn a_record_of_an_unknown_kind_is_found_by_its_id() -> TestResult {
    let project = TempProject::new()?;
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.apply(&[Change::text(
        "things/synthetic-orphan.md",
        "---\n# synthetic fixture (P03), not canon\nid: synthetic-orphan\nkind: kindd\n---\n\
         Body text only.\n",
    )])?;
    let hits = index.search("synthetic-orphan", 20)?;
    let found = hit(&hits, "synthetic-orphan")?;
    let id = Column::Field {
        key: "id".to_owned(),
        entry: None,
    };
    assert!(found.matches.iter().any(|m| m.column == id), "{found:?}");
    Ok(())
}

#[test]
fn the_search_fixture_has_the_expected_findings() -> TestResult {
    let index = indexed()?;
    assert_eq!(
        keys(&index.findings()?),
        expected(&fixture("search").join("expected.toml"))?
    );
    Ok(())
}

/// The one match of `query` in `column` of the record `id`.
fn one(
    query: &str,
    id: &str,
    pick: impl Fn(&Column) -> bool,
) -> Result<(Match, String), Box<dyn std::error::Error>> {
    let hits = indexed()?.search(query, 20)?;
    let found = hit(&hits, id)?;
    let mut picked = found.matches.iter().filter(|m| pick(&m.column));
    let m = picked.next().ok_or("no match in the column")?.clone();
    assert!(picked.next().is_none(), "more than one match");
    Ok((m, text_of(&found.path)?))
}

// AC: CE-13.1
#[test]
fn a_plain_field_value_matches_word_by_word() -> TestResult {
    let (m, text) = one("prism", "synthetic-fielded", |c| *c != Column::Body)?;
    assert_eq!(
        m.column,
        Column::Field {
            key: "type".to_owned(),
            entry: None
        }
    );
    assert_eq!(&text[m.span.start..m.span.end], "prism");
    assert_eq!((m.position.line, m.position.column), (6, 17));
    assert_eq!(m.line, "type: synthetic prism");
    Ok(())
}

// AC: CE-13.1
#[test]
fn a_quoted_lens_entry_matches_word_by_word_past_the_quote() -> TestResult {
    let (m, text) = one("quartz", "synthetic-speaker", |c| *c != Column::Body)?;
    assert_eq!(
        m.column,
        Column::Field {
            key: "lens".to_owned(),
            entry: Some("stance".to_owned())
        }
    );
    assert_eq!(&text[m.span.start..m.span.end], "quartz");
    assert_eq!((m.position.line, m.position.column), (8, 22));
    assert_eq!(m.line, "  stance: \"synthetic quartz stance\"");
    Ok(())
}

// AC: CE-13.1
#[test]
fn multi_byte_words_count_chars_in_columns() -> TestResult {
    let (title, text) = one("cafe", "synthetic-cafe", |c| *c == Column::Title)?;
    assert_eq!(&text[title.span.start..title.span.end], "Café");
    assert_eq!(title.span.end - title.span.start, 5);
    assert_eq!((title.position.line, title.position.column), (5, 18));
    assert_eq!(title.line, "title: Synthetic Café");
    let (body, text) = one("mere", "synthetic-cafe", |c| *c == Column::Body)?;
    assert_eq!(&text[body.span.start..body.span.end], "Mère");
    assert_eq!(body.span.end - body.span.start, 5);
    assert_eq!((body.position.line, body.position.column), (8, 1));
    assert_eq!(body.line, "Mère");
    Ok(())
}
