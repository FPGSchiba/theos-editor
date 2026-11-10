//! The facts a lore rule reads: what the record step stored of a record, the links that name an
//! id, and the input files (spec §5.4, §5.5).

use crate::index::Slot;
use crate::text::{Position, Span};

/// What the store holds of one record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordFacts {
    /// The project-relative path with `/`.
    pub path: String,
    /// The id (the file stem).
    pub id: String,
    /// The kind, when the front matter names a registered one.
    pub kind: Option<String>,
    /// The SHA-256 of the file, as lower-case hex.
    pub hash: String,
    /// The front-matter values in file order: fields, map entries, lists and list items.
    pub values: Vec<FactValue>,
    /// The links of valid front-matter values and of the body, in file order.
    pub links: Vec<FactLink>,
    /// The beats, for a kind with beats.
    pub beats: Vec<FactBeat>,
    /// The notes, for a kind with notes.
    pub notes: Vec<FactNote>,
    /// The in-world text, in file order.
    pub inworld: Vec<Segment>,
}

impl RecordFacts {
    /// The valid top-level scalar of `field`.
    #[must_use]
    pub fn text(&self, field: &str) -> Option<&FactValue> {
        self.values
            .iter()
            .find(|value| value.is_top(field) && value.valid && value.text.is_some())
    }

    /// The valid items of the list `field`, in order.
    #[must_use]
    pub fn items(&self, field: &str) -> Vec<&FactValue> {
        self.values
            .iter()
            .filter(|value| {
                value.field == field && value.entry.is_none() && value.item.is_some() && value.valid
            })
            .collect()
    }

    /// The span and position of the top-level value of `field`, valid or not.
    #[must_use]
    pub fn value_at(&self, field: &str) -> Option<(Span, Position)> {
        self.values
            .iter()
            .find(|value| value.is_top(field))
            .map(|value| (value.span, value.position))
    }

    /// The links in `slot`, in file order.
    pub fn links_in<'a>(&'a self, slot: &'a Slot) -> impl Iterator<Item = &'a FactLink> {
        self.links.iter().filter(move |link| link.slot == *slot)
    }
}

/// One front-matter value: a field, a map entry, a list or a list item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FactValue {
    /// The front-matter key.
    pub field: String,
    /// The map entry's key, for a value inside a map.
    pub entry: Option<String>,
    /// The index of a list item.
    pub item: Option<usize>,
    /// The scalar's text; none for a list or a map.
    pub text: Option<String>,
    /// The bytes of the value in the record.
    pub span: Span,
    /// Line and column of the span's start.
    pub position: Position,
    /// False when a `schema/value` finding lies on the value.
    pub valid: bool,
}

impl FactValue {
    /// Whether this is the top-level value of `field`: no entry and no item.
    fn is_top(&self, field: &str) -> bool {
        self.field == field && self.entry.is_none() && self.item.is_none()
    }
}

/// One link of a record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FactLink {
    /// Where the link sits.
    pub slot: Slot,
    /// The number of the beat the link sits in, for a beat attribute.
    pub beat: Option<u32>,
    /// The id it names.
    pub target: String,
    /// The text after `#`.
    pub fragment: Option<String>,
    /// The scalar in the front matter, or the whole `[[…]]` in the body.
    pub span: Span,
    /// Line and column of the span's start.
    pub position: Position,
}

/// One beat of a record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FactBeat {
    /// The beat number.
    pub number: u32,
    /// The first `when::` token, as written.
    pub when: Option<String>,
}

/// One note of a record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FactNote {
    /// The note id as written.
    pub id: String,
    /// `open`, `dropped`, `resolved`, or the unknown word as written.
    pub state: String,
    /// The bytes of the note id.
    pub span: Span,
    /// Line and column of the span's start.
    pub position: Position,
}

/// A span of in-world text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    /// The bytes of the segment.
    pub span: Span,
    /// Line and column of the span's start.
    pub position: Position,
    /// The record's text in the span.
    pub text: String,
}

/// A link that names an id, seen from the id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InLink {
    /// The id of the record the link sits in.
    pub source_id: String,
    /// The path of that record.
    pub source_path: String,
    /// Where the link sits.
    pub slot: Slot,
    /// The number of the beat the link sits in, for a beat attribute.
    pub beat: Option<u32>,
    /// The scalar in the front matter, or the whole `[[…]]` in the body.
    pub span: Span,
    /// Line and column of the span's start.
    pub position: Position,
}

/// An input file of the project, such as `.theos/settings.toml`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputFacts {
    /// The project-relative path with `/`.
    pub path: String,
    /// The SHA-256 of the file, as lower-case hex.
    pub hash: String,
    /// The text; none when the file is not UTF-8.
    pub text: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(field: &str, entry: Option<&str>, item: Option<usize>, valid: bool) -> FactValue {
        FactValue {
            field: field.to_owned(),
            entry: entry.map(str::to_owned),
            item,
            text: Some(format!("{field}-{item:?}")),
            span: Span {
                start: item.unwrap_or_default(),
                end: 9,
            },
            position: Position { line: 2, column: 1 },
            valid,
        }
    }

    fn link(slot: Slot, target: &str) -> FactLink {
        FactLink {
            slot,
            beat: None,
            target: target.to_owned(),
            fragment: None,
            span: Span::default(),
            position: Position { line: 1, column: 1 },
        }
    }

    fn facts(values: Vec<FactValue>, links: Vec<FactLink>) -> RecordFacts {
        RecordFacts {
            path: "things/a.md".to_owned(),
            id: "a".to_owned(),
            kind: Some("thing".to_owned()),
            hash: String::new(),
            values,
            links,
            beats: Vec::new(),
            notes: Vec::new(),
            inworld: Vec::new(),
        }
    }

    #[test]
    fn text_is_the_valid_top_level_scalar() {
        let mut list = value("where", None, None, true);
        list.text = None;
        let record = facts(
            vec![
                value("title", None, None, false),
                value("lens", Some("tier"), None, true),
                list,
                value("type", None, None, true),
            ],
            Vec::new(),
        );
        assert_eq!(record.text("title"), None, "an invalid value is no text");
        assert_eq!(record.text("lens"), None, "a map entry is not top-level");
        assert_eq!(record.text("where"), None, "a list has no text");
        assert_eq!(
            record.text("type").and_then(|v| v.text.as_deref()),
            Some("type-None")
        );
        assert_eq!(record.text("nope"), None);
    }

    #[test]
    fn items_are_the_valid_items_of_a_list_in_order() {
        let record = facts(
            vec![
                value("where", None, None, true),
                value("where", None, Some(0), true),
                value("where", None, Some(1), false),
                value("where", None, Some(2), true),
                value("lens", Some("rejects"), Some(0), true),
            ],
            Vec::new(),
        );
        let items: Vec<Option<usize>> = record.items("where").iter().map(|v| v.item).collect();
        assert_eq!(items, [Some(0), Some(2)]);
        assert!(
            record.items("lens").is_empty(),
            "lens items sit in an entry"
        );
    }

    #[test]
    fn value_at_finds_the_top_level_node_valid_or_not() {
        let record = facts(
            vec![
                value("where", None, Some(3), true),
                value("where", None, None, false),
            ],
            Vec::new(),
        );
        assert_eq!(
            record.value_at("where"),
            Some((Span { start: 0, end: 9 }, Position { line: 2, column: 1 }))
        );
        assert_eq!(record.value_at("title"), None);
    }

    #[test]
    fn links_in_keeps_the_links_of_one_slot() {
        let parent = Slot::Field("parent".to_owned());
        let record = facts(
            Vec::new(),
            vec![
                link(parent.clone(), "x"),
                link(Slot::Mention, "y"),
                link(parent.clone(), "z"),
            ],
        );
        let targets: Vec<&str> = record
            .links_in(&parent)
            .map(|link| link.target.as_str())
            .collect();
        assert_eq!(targets, ["x", "z"]);
        assert_eq!(record.links_in(&Slot::VoiceSpeaker).count(), 0);
    }
}
