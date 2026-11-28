//! Tests of the block tree. Every record here is synthetic; the voice and note shapes follow the
//! examples of data model §9.

use super::{Inline, Node, tree};
use crate::body::Link;
use crate::record::Record;
use crate::test_support::record;

fn text(s: &str) -> Inline {
    Inline::Text(s.to_owned())
}

fn para(inlines: Vec<Inline>) -> Node {
    Node::Paragraph { inlines }
}

fn raw(s: &str) -> Node {
    Node::Raw { text: s.to_owned() }
}

fn link(r: &Record, target: &str) -> Link {
    match r.body().links().iter().find(|l| l.target == target) {
        Some(link) => link.clone(),
        None => panic!("no link to {target}"),
    }
}

// AC: CE-33.1
#[test]
fn paragraph_with_emphasis_and_two_wikilinks() {
    let r = record(
        "id: a\n",
        "Plain *soft* and **hard**, see [[mother-ilse|the mother]] and [[S-004#3]].\n",
    );
    let first = link(&r, "mother-ilse");
    let second = link(&r, "S-004");
    assert_eq!(first.shown.as_deref(), Some("the mother"));
    assert_eq!(second.fragment.as_deref(), Some("3"));
    assert_eq!(
        tree(&r),
        [para(vec![
            text("Plain "),
            Inline::Emphasis(vec![text("soft")]),
            text(" and "),
            Inline::Strong(vec![text("hard")]),
            text(", see "),
            Inline::Link(first),
            text(" and "),
            Inline::Link(second),
            text("."),
        ])]
    );
}

// AC: CE-33.1
#[test]
fn heading_levels_and_lists_nested() {
    let r = record(
        "id: a\n",
        "# One\n\n### Three\n\n- a\n  1. b\n  2. c\n- d\n\n---\n\n```\ncode [[x]]\n```\n",
    );
    assert_eq!(
        tree(&r),
        [
            Node::Heading {
                level: 1,
                inlines: vec![text("One")],
            },
            Node::Heading {
                level: 3,
                inlines: vec![text("Three")],
            },
            Node::List {
                ordered: false,
                items: vec![
                    vec![
                        para(vec![text("a")]),
                        Node::List {
                            ordered: true,
                            items: vec![vec![para(vec![text("b")])], vec![para(vec![text("c")])]],
                        },
                    ],
                    vec![para(vec![text("d")])],
                ],
            },
            Node::Rule,
            Node::Code {
                text: "code [[x]]\n".to_owned(),
            },
        ]
    );
}

// AC: CE-33.1
#[test]
fn voice_block_has_speaker_beliefs_tier_and_caption() {
    let r = record(
        "id: a\n",
        "> [!voice] [[throat-kin]] · [[B-020]] · T2\n\
         > Sing a name the way it was *first* sung\n\
         > and they lean close to hear it.\n\
         > — a binder, teaching the naming-song\n",
    );
    assert_eq!(
        tree(&r),
        [Node::Voice {
            speaker: link(&r, "throat-kin"),
            beliefs: vec![link(&r, "B-020")],
            tier: Some("T2".to_owned()),
            nodes: vec![para(vec![
                text("Sing a name the way it was "),
                Inline::Emphasis(vec![text("first")]),
                text(" sung and they lean close to hear it."),
            ])],
            caption: Some(vec![text("a binder, teaching the naming-song")]),
        }]
    );
}

// AC: CE-33.1
#[test]
fn design_and_note_callouts() {
    let r = record(
        "id: a\n",
        "> [!design] Pacing for [[f04]]\n\
         > Keep this short.\n\n\
         > [!design]\n\
         > Untitled.\n\n\
         > [!note] n3 · [[f04-terrain-v2]] · open\n\
         > Lakes cover a tenth of the floor.\n\
         > — Jann\n\n\
         > [!note] n4 · resolved by [[fates-shrine-v5]]\n\
         > The roof reads as a church.\n\n\
         > [!tip] Plain *title*\n\
         > Body.\n",
    );
    assert_eq!(
        tree(&r),
        [
            Node::Design {
                nodes: vec![
                    para(vec![text("Pacing for "), Inline::Link(link(&r, "f04"))]),
                    para(vec![text("Keep this short.")]),
                ],
            },
            Node::Design {
                nodes: vec![para(vec![text("Untitled.")])],
            },
            Node::Note {
                state: Some("open".to_owned()),
                title: vec![text("n3 · "), Inline::Link(link(&r, "f04-terrain-v2"))],
                nodes: vec![
                    para(vec![text("Lakes cover a tenth of the floor.")]),
                    para(vec![text("— Jann")]),
                ],
            },
            Node::Note {
                state: Some("resolved by [[fates-shrine-v5]]".to_owned()),
                title: vec![text("n4")],
                nodes: vec![para(vec![text("The roof reads as a church.")])],
            },
            Node::Callout {
                name: "tip".to_owned(),
                title: vec![text("Plain "), Inline::Emphasis(vec![text("title")])],
                nodes: vec![para(vec![text("Body.")])],
            },
        ]
    );
}

// AC: CE-33.1
#[test]
fn beat_heading_with_attribute_lines() {
    let r = record(
        "id: a\n",
        "Intro.\n\n\
         ## Beat 3 — The gate\n\
         at:: [[thornstead]]\n\
         says:: nobody\n\
         when:: T2\n\
         Prose on the same paragraph.\n\n\
         More prose.\n\n\
         ## Beat 4 — Next\n\n\
         ## Afterword\n",
    );
    assert_eq!(
        tree(&r),
        [
            para(vec![text("Intro.")]),
            Node::Beat {
                heading: vec![text("Beat 3 — The gate")],
                attributes: vec![
                    ("at".to_owned(), vec![Inline::Link(link(&r, "thornstead"))]),
                    ("when".to_owned(), vec![text("T2")]),
                ],
                nodes: vec![
                    raw("says:: nobody"),
                    para(vec![text("Prose on the same paragraph.")]),
                    para(vec![text("More prose.")]),
                ],
            },
            Node::Beat {
                heading: vec![text("Beat 4 — Next")],
                attributes: vec![],
                nodes: vec![],
            },
            Node::Heading {
                level: 2,
                inlines: vec![text("Afterword")],
            },
        ]
    );
}

// AC: CE-33.1
#[test]
fn malformed_callout_title_is_raw_text() {
    let r = record(
        "id: a\n",
        "> [!voice] T2 · [[throat-kin]]\n\
         > Words.\n\n\
         ## Beat 2 broken\n\n\
         ## Beat 1 — A\n\n\
         at:: [[x]]\n",
    );
    assert_eq!(
        tree(&r),
        [
            Node::Quote {
                nodes: vec![
                    raw("[!voice] T2 · [[throat-kin]]"),
                    para(vec![text("Words.")])
                ],
            },
            raw("## Beat 2 broken"),
            Node::Beat {
                heading: vec![text("Beat 1 — A")],
                attributes: vec![],
                nodes: vec![raw("at:: [[x]]")],
            },
        ]
    );
}

// AC: CE-33.1
#[test]
fn link_inside_inline_code_is_code_not_link() {
    let r = record("id: a\n", "Use `[[a]]` here, [[b c]] and ![[d]].\n");
    assert_eq!(
        tree(&r),
        [para(vec![
            text("Use "),
            Inline::Code("[[a]]".to_owned()),
            text(" here, [[b c]] and ![[d]]."),
        ])]
    );
}

// AC: CE-33.1
#[test]
fn html_and_script_stay_text() {
    let r = record(
        "id: a\n",
        "Inline <b>bold</b> &amp; <script>alert(1)</script> text.\n\n\
         <script>x</script>\n\n\
         <div>\n<b>kept</b>\n</div>\n",
    );
    assert_eq!(
        tree(&r),
        [
            para(vec![text(
                "Inline <b>bold</b> & <script>alert(1)</script> text."
            )]),
            raw("<script>x</script>"),
            raw("<div>\n<b>kept</b>\n</div>"),
        ]
    );
}

// AC: CE-33.1
#[test]
fn external_links_and_images_keep_text_and_url() {
    let r = record(
        "id: a\n",
        "See [the *site*](https://example.org) and ![a map](maps/f04.png).\n",
    );
    assert_eq!(
        tree(&r),
        [para(vec![
            text("See "),
            Inline::External {
                text: "the site".to_owned(),
                url: "https://example.org".to_owned(),
            },
            text(" and "),
            Inline::External {
                text: "a map".to_owned(),
                url: "maps/f04.png".to_owned(),
            },
            text("."),
        ])]
    );
}

// AC: CE-33.1
#[test]
fn hard_breaks_and_nested_quotes() {
    let r = record(
        "id: a\n",
        "one  \ntwo\n\n> outer\n>\n> > [!voice] [[a]]\n> > Inner.\n",
    );
    assert_eq!(
        tree(&r),
        [
            para(vec![text("one"), Inline::Break, text("two")]),
            Node::Quote {
                nodes: vec![
                    para(vec![text("outer")]),
                    Node::Voice {
                        speaker: link(&r, "a"),
                        beliefs: vec![],
                        tier: None,
                        nodes: vec![para(vec![text("Inner.")])],
                        caption: None,
                    },
                ],
            },
        ]
    );
}

// AC: CE-33.1
#[test]
fn crlf_record_gives_the_same_tree_as_lf() {
    let body = "# Title\n\nText with [[a]].\n\n> [!voice] [[a]] · T1\n> Words.\n> — someone\n\n\
                ## Beat 1 — A\nat:: [[a]]\n\n- x\n- y\n";
    let lf = record("id: a\n", body);
    let crlf = match Record::parse(&format!(
        "---\r\nid: a\r\n---\r\n{}",
        body.replace('\n', "\r\n")
    )) {
        Ok(r) => r,
        Err(error) => panic!("{error}"),
    };
    assert!(crlf.was_normalized());
    assert_eq!(tree(&lf).len(), 4);
    assert_eq!(tree(&crlf), tree(&lf));
}

/// Every record file under `dir`, at any depth.
fn fixture_files(dir: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            fixture_files(&path, found);
        } else if path.extension().is_some_and(|e| e == "md") {
            found.push(path);
        }
    }
}

// AC: CE-33.1
#[test]
fn every_fixture_record_gives_a_tree() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut files = Vec::new();
    fixture_files(&root, &mut files);
    assert!(
        files.len() > 100,
        "fixtures not found under {}",
        root.display()
    );
    let mut built = 0;
    for file in files {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        if let Ok(r) = Record::parse(&text) {
            let nodes = tree(&r);
            let blank = r.text()[r.body_span().start..].trim().is_empty();
            assert!(blank || !nodes.is_empty(), "{}: empty tree", file.display());
            built += 1;
        }
    }
    assert!(built > 100);
}
