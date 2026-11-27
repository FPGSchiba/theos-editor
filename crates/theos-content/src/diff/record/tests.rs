//! Tests of the field and record diffs. The records are the data model's section 14 examples;
//! changes marked "synthetic" are invented.

use super::*;
use crate::diff::fields::{FieldDelta, ItemDelta};

const SCRIPT: &str = "---\nid: S-004\nkind: script\ntitle: The shrine hums\nstatus: draft\n\
where: [f04-choir-hall]\n---\nThe player's first night under the Fates' shrine.\n\n\
## Beat 1 \u{2014} Arrival\nat:: [[fates-shrine]]\n";

fn parts(diff: RecordDiff) -> (String, String, FieldsDiff, WordDiff, bool) {
    match diff {
        RecordDiff::Record {
            old,
            new,
            fields,
            body,
            layout_only,
        } => (old, new, fields, body, layout_only),
        RecordDiff::Text { .. } => panic!("expected a record diff"),
    }
}

fn changes(fields: FieldsDiff) -> Vec<FieldChange> {
    match fields {
        FieldsDiff::Fields(c) => c,
        FieldsDiff::Text(_) => panic!("expected field changes"),
    }
}

fn line<'a>(text: &'a str, field: &crate::yaml::Field) -> &'a str {
    field.lines.get(text).unwrap_or_default().trim_end()
}

// AC: CE-21.2
#[test]
fn a_changed_scalar_shows_old_and_new() {
    // Synthetic change: a new title.
    let new_text = SCRIPT.replace("The shrine hums", "The shrine sings");
    let (old, new, fields, _, layout_only) = parts(record(Some(SCRIPT), Some(&new_text)));
    let c = changes(fields);
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].key, "title");
    let FieldDelta::Changed {
        old: o,
        new: n,
        items,
    } = &c[0].delta
    else {
        panic!("expected Changed");
    };
    assert_eq!(line(&old, o), "title: The shrine hums");
    assert_eq!(line(&new, n), "title: The shrine sings");
    assert_eq!(*items, None);
    assert!(!layout_only);
}

// AC: CE-21.2
#[test]
fn added_and_removed_fields_follow_the_order_rule() {
    // Synthetic: status removed, `parent` and `number` added.
    let new_text = SCRIPT.replace("status: draft\n", "").replace(
        "kind: script\n",
        "kind: script\nparent: hearth\nnumber: 4\n",
    );
    let c = changes(parts(record(Some(SCRIPT), Some(&new_text))).2);
    let got: Vec<(&str, &str)> = c
        .iter()
        .map(|c| {
            let kind = match c.delta {
                FieldDelta::Added { .. } => "added",
                FieldDelta::Removed { .. } => "removed",
                FieldDelta::Changed { .. } => "changed",
            };
            (c.key.as_str(), kind)
        })
        .collect();
    assert_eq!(
        got,
        vec![
            ("parent", "added"),
            ("number", "added"),
            ("status", "removed")
        ]
    );
}

// AC: CE-21.2
#[test]
fn a_list_field_shows_items_added_and_removed() {
    // Synthetic: one place swapped for another.
    let old_text = SCRIPT.replace("[f04-choir-hall]", "[fates-shrine, f04-choir-hall]");
    let new_text = SCRIPT.replace("[f04-choir-hall]", "[fates-shrine, f05-x]");
    let c = changes(parts(record(Some(&old_text), Some(&new_text))).2);
    let FieldDelta::Changed { items, .. } = &c[0].delta else {
        panic!("expected Changed");
    };
    assert_eq!(
        *items,
        Some(ItemDelta {
            added: vec!["f05-x".into()],
            removed: vec!["f04-choir-hall".into()],
        })
    );
}

// AC: CE-21.2
#[test]
fn a_scalar_turned_list_is_changed_without_items() {
    let new_text = SCRIPT.replace("status: draft", "status: [draft]");
    let c = changes(parts(record(Some(SCRIPT), Some(&new_text))).2);
    let FieldDelta::Changed { items, .. } = &c[0].delta else {
        panic!("expected Changed");
    };
    assert_eq!(*items, None);
}

// AC: CE-21.2
#[test]
fn list_style_quoting_and_comments_are_layout_only() {
    let old_text = SCRIPT.replace("[f04-choir-hall]", "[f04-choir-hall, b]");
    let styled = old_text
        .replace("[f04-choir-hall, b]", "\n  - f04-choir-hall\n  - b")
        .replace("title: The shrine hums", "title: 'The shrine hums'")
        .replace("status: draft", "status: draft # synthetic comment");
    let (_, _, fields, body, layout_only) = parts(record(Some(&old_text), Some(&styled)));
    assert_eq!(changes(fields), vec![]);
    assert!(body.ops.iter().all(|o| matches!(o, WordOp::Equal { .. })));
    assert!(layout_only);
}

// AC: CE-21.2
#[test]
fn a_rewrapped_body_is_layout_only() {
    let new_text = SCRIPT.replace("first night under", "first\nnight\nunder");
    let (_, _, _, _, layout_only) = parts(record(Some(SCRIPT), Some(&new_text)));
    assert!(layout_only);
}

// AC: CE-21.2
#[test]
fn crlf_against_lf_is_no_change_and_returns_lf() {
    let crlf = SCRIPT.replace('\n', "\r\n");
    let (old, new, fields, _, layout_only) = parts(record(Some(&crlf), Some(SCRIPT)));
    assert_eq!(old, SCRIPT);
    assert_eq!(new, SCRIPT);
    assert_eq!(changes(fields), vec![]);
    assert!(layout_only);
}

// AC: CE-21.2
#[test]
fn a_blank_line_is_layout_only() {
    let new_text = SCRIPT.replace("\n\n## Beat", "\n\n\n## Beat");
    assert!(parts(record(Some(SCRIPT), Some(&new_text))).4);
}

// AC: CE-21.2
#[test]
fn a_yaml_error_on_one_side_gives_the_front_matter_words() {
    let broken = SCRIPT.replace("kind: script\n", "kind: script\nbad line without colon\n");
    let (_, new, fields, _, _) = parts(record(Some(SCRIPT), Some(&broken)));
    let FieldsDiff::Text(diff) = fields else {
        panic!("expected a text diff");
    };
    let inserted: Vec<&str> = diff
        .ops
        .iter()
        .filter_map(|op| match op {
            WordOp::Insert { new: s } => s.get(&new),
            _ => None,
        })
        .collect();
    assert_eq!(inserted, vec!["bad line without colon"]);
}

// AC: CE-21.2
#[test]
fn a_text_without_dashes_is_a_text_diff() {
    let got = record(Some("one two\r\n"), Some(SCRIPT));
    let RecordDiff::Text { old, new, words } = got else {
        panic!("expected a text diff");
    };
    assert_eq!(old, "one two\n");
    assert_eq!(new, SCRIPT);
    assert_eq!(words.ops.len(), 2);
}

// AC: CE-21.2
#[test]
fn an_added_record_has_every_field_added_and_every_word_inserted() {
    let (old, new, fields, body, _) = parts(record(None, Some(SCRIPT)));
    assert_eq!(old, "");
    assert_eq!(new, SCRIPT);
    let c = changes(fields);
    assert_eq!(c.len(), 5);
    assert!(
        c.iter()
            .all(|c| matches!(c.delta, FieldDelta::Added { .. }))
    );
    assert_eq!(body.ops.len(), 1);
    assert!(body.ops.iter().all(|o| matches!(o, WordOp::Insert { .. })));
}

// AC: CE-21.2
#[test]
fn a_removed_record_has_every_field_removed_and_every_word_deleted() {
    let (_, new, fields, body, _) = parts(record(Some(SCRIPT), None));
    assert_eq!(new, "");
    let c = changes(fields);
    assert_eq!(c.len(), 5);
    assert!(
        c.iter()
            .all(|c| matches!(c.delta, FieldDelta::Removed { .. }))
    );
    assert_eq!(body.ops.len(), 1);
    assert!(body.ops.iter().all(|o| matches!(o, WordOp::Delete { .. })));
}

// AC: CE-21.2
#[test]
fn body_spans_are_offsets_into_the_whole_record_text() {
    let new_text = SCRIPT.replace("first night", "second night");
    let (old, new, _, body, layout_only) = parts(record(Some(SCRIPT), Some(&new_text)));
    assert!(!layout_only);
    let close = old.find("\n---\n").unwrap_or_default() + 5;
    let Some(WordOp::Equal { old: o, new: n }) = body.ops.first() else {
        panic!("expected a leading Equal");
    };
    assert_eq!((o.start, n.start), (close, close));
    assert_eq!(o.get(&old), Some("The player's"));
    assert!(body.ops.iter().any(|op| matches!(
        op,
        WordOp::Insert { new: s } if s.get(&new) == Some("second")
    )));
}
