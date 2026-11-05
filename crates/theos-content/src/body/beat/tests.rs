use crate::body::{AttributeValue, BlockKind};
use crate::record::Record;
use crate::test_support::record;

fn found(r: &Record) -> Vec<(&'static str, usize)> {
    r.body()
        .malformed()
        .iter()
        .map(|m| (m.kind.as_str(), m.position.line))
        .collect()
}

fn keys(r: &Record, beat: usize) -> Vec<(&str, String)> {
    r.body().beats()[beat]
        .attributes
        .iter()
        .map(|a| (a.key.as_str(), a.value.spelling()))
        .collect()
}

#[test]
fn attribute_shapes_and_kept_data() {
    let r = record(
        "id: a\n",
        "## Beat 1 — A\nat::   [[x]]\nwith:: [[a]], [[b]]\nsynthetic:: free text [[c]]\n\
         Prose after the lines.\n\n\
         ## Beat 2 — B\n\nplain prose\n\n\
         ## Beats\n\n## Beat x — y\n\n\
         ## Beat 4 — D\nsays:: [[g]]\n> [!voice] [[g]] · T1\n> Words.\n",
    );
    assert_eq!(found(&r), []);
    let beats = r.body().beats();
    let numbers: Vec<_> = beats.iter().map(|b| (b.number, b.name.as_str())).collect();
    assert_eq!(numbers, [(1, "A"), (2, "B"), (4, "D")]);
    assert_eq!(
        keys(&r, 0),
        [
            ("at", "[[x]]".to_owned()),
            ("with", "[[a]], [[b]]".to_owned()),
            ("synthetic", "free text [[c]]".to_owned()),
        ]
    );
    let AttributeValue::Raw { links, .. } = &beats[0].attributes[2].value else {
        panic!("a key without a shape keeps its raw value");
    };
    assert_eq!(links.len(), 1);
    assert_eq!(beats[1].attributes, []);
    assert_eq!(beats[2].attributes.len(), 1);
    assert_eq!(keys(&r, 2), [("says", "[[g]]".to_owned())]);

    // A beat ends at the next level-2 heading, ordinary or not.
    assert_eq!(beats[0].span.end, beats[1].heading_span.start);
    let ordinary = r.text().find("## Beats\n");
    assert_eq!(Some(beats[1].span.end), ordinary);
    let blocks = r.body().blocks();
    assert!(
        blocks
            .iter()
            .any(|b| b.kind == BlockKind::BeatAttributes(0))
    );
    assert!(
        blocks
            .iter()
            .any(|b| matches!(b.kind, BlockKind::Callout(_)))
    );
}

#[test]
fn setext_headings_and_repeated_keys() {
    let r = record(
        "id: a\n",
        "Beat 3 — X\n---\nat:: [[a]], [[b]]\nat:: [[c]]\n",
    );
    assert_eq!(found(&r), []);
    let beat = &r.body().beats()[0];
    assert_eq!((beat.number, beat.name.as_str()), (3, "X"));
    assert_eq!(beat.heading_span.get(r.text()), Some("Beat 3 — X\n---"));
    assert_eq!(beat.number_span.get(r.text()), Some("3"));
    assert_eq!(beat.name_span.get(r.text()), Some("X"));
    assert_eq!(
        keys(&r, 0),
        [
            ("at", "[[a]], [[b]]".to_owned()),
            ("at", "[[c]]".to_owned())
        ]
    );
    let AttributeValue::Links(links) = &beat.attributes[0].value else {
        panic!("`at` holds links");
    };
    assert_eq!(links.len(), 2);
}

// AC: CE-10.3
#[test]
fn malformed_beat_attributes_report_their_line() {
    let r = record(
        "id: a\n",
        "## Beat 1 — A\nat:: fates-shrine\nwith:: [[a]] [[b]]\nwhen:: [[x]]\nat::\nat::[[x]]\n",
    );
    assert_eq!(
        found(&r),
        [
            ("beat-attribute", 5),
            ("beat-attribute", 6),
            ("beat-attribute", 7),
            ("beat-attribute", 8),
            ("beat-attribute", 9),
        ]
    );
    assert_eq!(r.body().beats()[0].attributes, []);
}

// AC: CE-10.3
#[test]
fn malformed_beat_headings_report_their_line() {
    let forms = [
        "## Beat 5 - x",
        "## Beat 7 —",
        "## Beat 8 —name",
        "## Beat 9",
        "## Beat 99999999999 — x",
    ];
    let r = record("id: a\n", &(forms.join("\n\n") + "\n"));
    let want: Vec<_> = (0..forms.len())
        .map(|i| ("beat-heading", 4 + 2 * i))
        .collect();
    assert_eq!(found(&r), want);
    assert_eq!(r.body().beats(), []);
    assert!(
        r.body()
            .blocks()
            .iter()
            .all(|b| !matches!(b.kind, BlockKind::BeatHeading(_)))
    );
}

// AC: CE-10.3
#[test]
fn misplaced_attributes_are_reported() {
    let r = record("id: a\n", "## Beat 1 — A\n\nat:: [[x]]\n");
    assert_eq!(found(&r), [("misplaced-attributes", 6)]);
    assert_eq!(r.body().beats()[0].attributes, []);
}

// AC: CE-10.3
#[test]
fn an_attribute_paragraph_after_the_attribute_lines_is_reported() {
    let r = record("id: a\n", "## Beat 1 — A\nat:: [[x]]\n\nwhen:: T1\n");
    assert_eq!(found(&r), [("misplaced-attributes", 7)]);
    assert_eq!(keys(&r, 0), [("at", "[[x]]".to_owned())]);

    let r = record(
        "id: a\n",
        "## Beat 1 — A\nat:: [[x]]\n\nplain prose\n\nwhen:: T1\n",
    );
    assert_eq!(found(&r), []);
}

// AC: CE-10.3
#[test]
fn a_bad_link_in_an_attribute_is_reported_once() {
    let r = record("id: a\n", "## Beat 1 — A\nat:: [[world/tower]]\n");
    assert_eq!(found(&r), [("link-target", 5)]);
    let r = record("id: a\n", "## Beat 1 — A\nat:: [[x]], plain\n");
    assert_eq!(found(&r), [("beat-attribute", 5)]);
}
