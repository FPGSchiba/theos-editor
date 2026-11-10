//! Turning one checked record into the rows of the index: values, links with their slots,
//! beats, the search row and the link facts (spec §5.5, §5.6, §5.8, §5.9 step 4, §5.11).

use std::collections::HashMap;

use crate::body::{AttributeValue, CalloutTitle, NoteState};
use crate::check::Checked;
use crate::finding::Finding;
use crate::index::slots::Slot;
use crate::inworld::segments;
use crate::kinds::{FieldType, KindRegistry, KindSpec, LENS_ENTRIES, ValueRef};
use crate::record::Record;
use crate::text::{Position, Span};
use crate::yaml::{Entry, EntryValue, Field, List, Scalar, ScalarStyle, Value};

/// Everything the index keeps of one record.
pub(crate) struct RecordRows {
    /// The record's own row; every file that is read has one.
    pub record: Option<RecordRow>,
    /// The front-matter values, in file order.
    pub values: Vec<ValueRow>,
    /// The links of valid front-matter values and of the body, in document order.
    pub links: Vec<LinkRow>,
    /// The beats of a kind with beats.
    pub beats: Vec<BeatRow>,
    /// The notes of a kind with notes.
    pub notes: Vec<NoteRow>,
    /// The in-world segments, see [`crate::inworld`].
    pub inworld: Vec<SegmentRow>,
    /// The findings of the record check.
    pub findings: Vec<Finding>,
    /// The search row; none when the file could not be decoded or split.
    pub search: Option<SearchRow>,
}

/// The row of the `records` table.
pub(crate) struct RecordRow {
    /// The project-relative path with `/`.
    pub path: String,
    /// The id (the file stem).
    pub id: String,
    /// The kind, when the front matter names a registered one.
    pub kind: Option<String>,
    /// The text of the `title` scalar.
    pub title: Option<String>,
    /// The text of the `status` scalar.
    pub status: Option<String>,
    /// The front matter: the lines between the two `---` lines.
    pub front: Option<String>,
    /// The byte offset where the body starts.
    pub body_start: Option<usize>,
    /// The line where the body starts.
    pub body_line: Option<usize>,
    /// The link facts, see [`facts`].
    pub facts: String,
}

/// The shape of a stored value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shape {
    /// One scalar.
    Scalar,
    /// A list.
    List,
    /// A map.
    Map,
}

/// One row of `field_values`: a field, a map entry, a list or a list item.
pub(crate) struct ValueRow {
    /// The front-matter key.
    pub field: String,
    /// The map entry's key, for a value inside a map.
    pub entry: Option<String>,
    /// The index of a list item.
    pub item: Option<usize>,
    /// What the row is.
    pub shape: Shape,
    /// The scalar's text; none for a list or a map.
    pub text: Option<String>,
    /// The scalar's spelling.
    pub style: Option<ScalarStyle>,
    /// The bytes of the value in the record.
    pub span: Span,
    /// Line and column of the span's start.
    pub position: Position,
    /// False when a `schema/value` finding lies on the value.
    pub valid: bool,
    /// The 0-based line of [`SearchRow::fields`] holding this scalar's text.
    pub search_line: Option<usize>,
}

/// One row of `links`.
pub(crate) struct LinkRow {
    /// Where the link sits.
    pub slot: Slot,
    /// The number of the beat the link sits in, for a beat attribute.
    pub beat: Option<u32>,
    /// The id it points at.
    pub target: String,
    /// The text after `#`.
    pub fragment: Option<String>,
    /// The scalar in the front matter, or the whole `[[…]]` in the body.
    pub span: Span,
    /// Line and column of the span's start.
    pub position: Position,
}

/// One row of `beats`.
pub(crate) struct BeatRow {
    /// The beat number.
    pub number: u32,
    /// The name after the dash.
    pub name: String,
    /// The whole beat.
    pub span: Span,
    /// The line of the heading.
    pub line: usize,
    /// The first `when::` token, as written.
    pub when: Option<String>,
}

/// One row of `notes`.
pub(crate) struct NoteRow {
    /// The note id as written.
    pub id: String,
    /// The number in the id.
    pub number: u32,
    /// `open`, `dropped`, `resolved`, or the unknown word as written.
    pub state: String,
    /// The target of the `resolved by` link.
    pub resolved_by: Option<String>,
    /// The target of the version link.
    pub version: Option<String>,
    /// The bytes of the note id.
    pub span: Span,
    /// Line and column of the span's start.
    pub position: Position,
}

/// One row of `inworld`: a span of in-world text.
pub(crate) struct SegmentRow {
    /// The bytes of the segment.
    pub span: Span,
    /// Line and column of the span's start.
    pub position: Position,
    /// The record's text in the span.
    pub text: String,
}

/// One row of `search`.
pub(crate) struct SearchRow {
    /// The `title` text, or empty.
    pub title: String,
    /// The searchable scalars, one per line, in file order.
    pub fields: String,
    /// The body text exactly.
    pub body: String,
}

/// The rows of `checked`, which came from `path`.
pub(crate) fn rows(path: &str, checked: Checked, registry: &KindRegistry) -> RecordRows {
    let Checked {
        id,
        record,
        kind,
        findings,
    } = checked;
    let Some(record) = record else {
        let row = RecordRow {
            path: path.to_owned(),
            facts: facts(&id, None, path, &[], &[]),
            id,
            kind: None,
            title: None,
            status: None,
            front: None,
            body_start: None,
            body_line: None,
        };
        return RecordRows {
            record: Some(row),
            values: Vec::new(),
            links: Vec::new(),
            beats: Vec::new(),
            notes: Vec::new(),
            inworld: Vec::new(),
            findings,
            search: None,
        };
    };
    let spec = kind.and_then(|name| registry.kind(name));
    let mut front = Front::new(&record, spec, &findings);
    if let Ok(fields) = record.front_matter() {
        for field in &fields.fields {
            front.field(field);
        }
    }
    let beats = beat_rows(&record, spec);
    let notes = note_rows(&record, spec);
    let inworld = segments(&record, spec, &front.values);
    let narrow = narrow_values(&front.values, spec, registry);
    let mut links = std::mem::take(&mut front.links);
    links.extend(body_links(&record, spec));
    let body_start = record.body_span().start;
    let title = top_text(&front.values, "title");
    let row = RecordRow {
        path: path.to_owned(),
        facts: facts(&id, kind, path, &beats, &narrow),
        id,
        kind: kind.map(str::to_owned),
        status: top_text(&front.values, "status"),
        title: title.clone(),
        front: record.front_span().get(record.text()).map(str::to_owned),
        body_start: Some(body_start),
        body_line: record.index().position(body_start).ok().map(|p| p.line),
    };
    let search = SearchRow {
        title: title.unwrap_or_default(),
        fields: front.lines.join("\n"),
        body: record
            .body_span()
            .get(record.text())
            .unwrap_or_default()
            .to_owned(),
    };
    RecordRows {
        record: Some(row),
        values: front.values,
        links,
        beats,
        notes,
        inworld,
        findings,
        search: Some(search),
    }
}

/// The text of the top-level scalar `key`.
fn top_text(values: &[ValueRow], key: &str) -> Option<String> {
    values
        .iter()
        .find(|v| v.field == key && v.entry.is_none() && v.item.is_none())
        .and_then(|v| v.text.clone())
}

/// The link facts of a record as one canonical text: its id, kind and path, the sorted beat
/// numbers, and `field=value` for each narrowing field with a valid value, one per line.
/// Strings are written in Rust's debug form, so no value can pass for another line. Two records
/// have equal facts exactly when nothing a link to them depends on differs.
fn facts(
    id: &str,
    kind: Option<&str>,
    path: &str,
    beats: &[BeatRow],
    narrow: &[(&str, &str)],
) -> String {
    let mut numbers: Vec<u32> = beats.iter().map(|beat| beat.number).collect();
    numbers.sort_unstable();
    let numbers: Vec<String> = numbers.iter().map(u32::to_string).collect();
    let mut lines = vec![
        format!("id={id:?}"),
        format!("kind={:?}", kind.unwrap_or_default()),
        format!("path={path:?}"),
        format!("beats={}", numbers.join(",")),
    ];
    lines.extend(
        narrow
            .iter()
            .map(|(field, value)| format!("{field}={value:?}")),
    );
    lines.join("\n")
}

/// The valid scalar values of the fields some target narrows by, for the fields the kind has.
fn narrow_values<'a>(
    values: &'a [ValueRow],
    spec: Option<&KindSpec>,
    registry: &KindRegistry,
) -> Vec<(&'static str, &'a str)> {
    let Some(spec) = spec else {
        return Vec::new();
    };
    registry
        .narrow_fields()
        .into_iter()
        .filter(|field| spec.field(field).is_some())
        .filter_map(|field| {
            let row = values
                .iter()
                .find(|v| v.field == field && v.entry.is_none() && v.item.is_none() && v.valid)?;
            Some((field, row.text.as_deref()?))
        })
        .collect()
}

fn beat_rows(record: &Record, spec: Option<&KindSpec>) -> Vec<BeatRow> {
    if spec.is_none_or(|s| s.body.beats.is_none()) {
        return Vec::new();
    }
    record
        .body()
        .beats()
        .iter()
        .map(|beat| BeatRow {
            number: beat.number,
            name: beat.name.clone(),
            span: beat.span,
            line: position(record, beat.heading_span.start).line,
            when: beat.attributes.iter().find_map(|attribute| {
                match (attribute.key.as_str(), &attribute.value) {
                    ("when", AttributeValue::Token(token)) => Some(token.text.clone()),
                    _ => None,
                }
            }),
        })
        .collect()
}

/// The notes of a kind whose grammar has notes.
fn note_rows(record: &Record, spec: Option<&KindSpec>) -> Vec<NoteRow> {
    if spec.is_none_or(|s| s.body.notes.is_none()) {
        return Vec::new();
    }
    record
        .body()
        .callouts()
        .iter()
        .filter_map(|callout| match &callout.title {
            Some(CalloutTitle::Note(note)) => Some(note),
            _ => None,
        })
        .map(|note| {
            let (state, resolved_by) = match &note.state {
                NoteState::Open => ("open".to_owned(), None),
                NoteState::Dropped => ("dropped".to_owned(), None),
                NoteState::ResolvedBy(link) => ("resolved".to_owned(), Some(link.target.clone())),
                NoteState::Unknown(word) => (word.clone(), None),
            };
            NoteRow {
                id: note.id.text.clone(),
                number: note.number,
                state,
                resolved_by,
                version: note.version.as_ref().map(|link| link.target.clone()),
                span: note.id.span,
                position: position(record, note.id.span.start),
            }
        })
        .collect()
}

fn position(record: &Record, offset: usize) -> Position {
    record
        .index()
        .position(offset)
        .unwrap_or(Position { line: 1, column: 1 })
}

/// Whether two spans share a byte; an empty span counts as the point it sits on.
pub(crate) fn touches(a: Span, b: Span) -> bool {
    if a.is_empty() || b.is_empty() {
        a.start <= b.end && b.start <= a.end
    } else {
        a.start < b.end && b.start < a.end
    }
}

/// The walk over the front matter: value rows, the links of valid values and the search lines.
struct Front<'a> {
    record: &'a Record,
    spec: Option<&'a KindSpec>,
    /// The spans of the `schema/value` findings.
    bad: Vec<Span>,
    values: Vec<ValueRow>,
    links: Vec<LinkRow>,
    lines: Vec<String>,
}

/// What the type of a value says about it: whether its scalars are searched, and the slot of
/// its links.
struct Typed {
    ty: Option<FieldType>,
    search: bool,
    slot: Slot,
}

impl<'a> Front<'a> {
    fn new(record: &'a Record, spec: Option<&'a KindSpec>, findings: &[Finding]) -> Front<'a> {
        Front {
            record,
            spec,
            bad: findings
                .iter()
                .filter(|finding| finding.rule == "schema/value")
                .filter_map(|finding| finding.span)
                .collect(),
            values: Vec::new(),
            links: Vec::new(),
            lines: Vec::new(),
        }
    }

    fn valid(&self, span: Span) -> bool {
        !self.bad.iter().any(|&bad| touches(bad, span))
    }

    fn field(&mut self, field: &Field) {
        let ty = self
            .spec
            .and_then(|spec| spec.field(&field.key))
            .map(|f| f.ty);
        let typed = Typed {
            ty,
            // The title has its own column. The id is searched also when the kind is unknown.
            search: field.key == "id" || (field.key != "title" && is_searched(ty, true)),
            slot: Slot::Field(field.key.clone()),
        };
        match &field.value {
            Value::Scalar(scalar) => self.scalar(&field.key, None, None, scalar, &typed),
            Value::List(list) => self.list(&field.key, None, list, &typed),
            Value::Map(map) => {
                let span = ValueRef::Map(map).span();
                self.node(&field.key, None, Shape::Map, span);
                let is_lens = ty == Some(FieldType::Lens);
                for entry in &map.entries {
                    self.entry(&field.key, entry, is_lens);
                }
            }
        }
    }

    fn entry(&mut self, field: &str, entry: &Entry, is_lens: bool) {
        let known = LENS_ENTRIES
            .iter()
            .find(|known| is_lens && known.key == entry.key);
        let ty = known.map(|known| known.ty);
        let typed = Typed {
            ty,
            search: is_searched(ty, false),
            slot: Slot::LensEntry(entry.key.clone()),
        };
        match &entry.value {
            EntryValue::Scalar(scalar) => {
                self.scalar(field, Some(&entry.key), None, scalar, &typed);
            }
            EntryValue::List(list) => self.list(field, Some(&entry.key), list, &typed),
        }
    }

    /// Pushes the row of a list or a map.
    fn node(&mut self, field: &str, entry: Option<&str>, shape: Shape, span: Span) {
        let valid = self.valid(span);
        self.values.push(ValueRow {
            field: field.to_owned(),
            entry: entry.map(str::to_owned),
            item: None,
            shape,
            text: None,
            style: None,
            span,
            position: position(self.record, span.start),
            valid,
            search_line: None,
        });
    }

    fn list(&mut self, field: &str, entry: Option<&str>, list: &List, typed: &Typed) {
        self.node(field, entry, Shape::List, list.span);
        for (index, item) in list.items.iter().enumerate() {
            self.scalar(field, entry, Some(index), &item.scalar, typed);
        }
    }

    fn scalar(
        &mut self,
        field: &str,
        entry: Option<&str>,
        item: Option<usize>,
        scalar: &Scalar,
        typed: &Typed,
    ) {
        let valid = self.valid(scalar.span);
        let search_line = typed.search.then(|| {
            self.lines.push(scalar.text.clone());
            self.lines.len() - 1
        });
        self.values.push(ValueRow {
            field: field.to_owned(),
            entry: entry.map(str::to_owned),
            item,
            shape: Shape::Scalar,
            text: Some(scalar.text.clone()),
            style: Some(scalar.style),
            span: scalar.span,
            position: scalar.position,
            valid,
            search_line,
        });
        let links_here = matches!(typed.ty, Some(FieldType::Link(_) | FieldType::Links(_)));
        if links_here && valid {
            self.links.push(LinkRow {
                slot: typed.slot.clone(),
                beat: None,
                target: scalar.text.clone(),
                fragment: None,
                span: scalar.span,
                position: scalar.position,
            });
        }
    }
}

/// Whether the scalars of a value of type `ty` go into the search row. A field is searched by
/// its id, texts and links; a lens entry only by its texts and links.
fn is_searched(ty: Option<FieldType>, field: bool) -> bool {
    match ty {
        Some(FieldType::Id) => field,
        Some(FieldType::Text | FieldType::TextList | FieldType::Link(_) | FieldType::Links(_)) => {
            true
        }
        _ => false,
    }
}

/// The links of the body in document order, each with its slot.
fn body_links(record: &Record, spec: Option<&KindSpec>) -> Vec<LinkRow> {
    if spec.is_some_and(|spec| spec.body.verbatim) {
        return Vec::new();
    }
    let body = record.body();
    let mut slots: HashMap<Span, (Slot, Option<u32>)> = HashMap::new();
    if let Some(attributes) = spec.and_then(|spec| spec.body.beats) {
        for beat in body.beats() {
            for attribute in &beat.attributes {
                let listed = attributes.iter().any(|a| a.key == attribute.key);
                if let (true, AttributeValue::Links(links)) = (listed, &attribute.value) {
                    for link in links {
                        slots.insert(
                            link.span,
                            (Slot::Beat(attribute.key.clone()), Some(beat.number)),
                        );
                    }
                }
            }
        }
    }
    let notes = spec.is_some_and(|spec| spec.body.notes.is_some());
    for callout in body.callouts() {
        match &callout.title {
            Some(CalloutTitle::Voice(voice)) => {
                slots.insert(voice.speaker.span, (Slot::VoiceSpeaker, None));
                for belief in &voice.beliefs {
                    slots.insert(belief.span, (Slot::VoiceBelief, None));
                }
            }
            Some(CalloutTitle::Note(note)) if notes => {
                if let Some(version) = &note.version {
                    slots.insert(version.span, (Slot::NoteVersion, None));
                }
                if let NoteState::ResolvedBy(link) = &note.state {
                    slots.insert(link.span, (Slot::NoteResolvedBy, None));
                }
            }
            _ => {}
        }
    }
    body.links()
        .iter()
        .map(|link| {
            let (slot, beat) = slots.remove(&link.span).unwrap_or((Slot::Mention, None));
            LinkRow {
                slot,
                beat,
                target: link.target.clone(),
                fragment: link.fragment.clone(),
                span: link.span,
                position: position(record, link.span.start),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::check_record;
    use crate::index::slots::Slot;
    use crate::parts::builtin;

    type Res = Result<(), Box<dyn std::error::Error>>;

    fn build(path: &str, text: &str) -> Result<RecordRows, Box<dyn std::error::Error>> {
        let registry = builtin()?;
        let checked = check_record(&registry, path, text.as_bytes());
        Ok(rows(path, checked, &registry))
    }

    fn links_of(found: &RecordRows) -> Vec<(String, Option<u32>, &str)> {
        found
            .links
            .iter()
            .map(|link| (link.slot.as_db(), link.beat, link.target.as_str()))
            .collect()
    }

    fn front(kind: &str, rest: &str) -> String {
        format!("---\n# synthetic fixture (P03), not canon\nid: x\nkind: {kind}\n{rest}---\n")
    }

    #[test]
    fn slots_of_every_kind_of_link() -> Res {
        let text = "---\n# synthetic fixture (P03), not canon\nid: S-930\nkind: script\n\
            title: synthetic script\nwhere: [synthetic-place]\n---\n\
            ## Beat 1 — Start\nat:: [[a]]\nwith:: [[b]], [[c]]\nwhen:: T2\nfoo:: [[d]]\n\
            > [!voice] [[e]] · [[B-930]] · T1\n> Words.\n\nprose [[f]]\n\n\
            > [!note] n1 · [[g]] · open\n> text\n";
        let found = build("story/scripts/S-930.md", text)?;
        let slot = |s: &str, beat, target| (s.to_owned(), beat, target);
        assert_eq!(
            links_of(&found),
            [
                slot("field:where", None, "synthetic-place"),
                slot("beat:at", Some(1), "a"),
                slot("beat:with", Some(1), "b"),
                slot("beat:with", Some(1), "c"),
                slot("mention", None, "d"),
                slot("voice:speaker", None, "e"),
                slot("voice:belief", None, "B-930"),
                slot("mention", None, "f"),
                slot("mention", None, "g"),
            ]
        );
        assert_eq!(found.links[1].span.get(text), Some("[[a]]"));
        assert_eq!(found.links[0].span.get(text), Some("synthetic-place"));
        assert_eq!(found.links[1].position.line, 9);
        assert_eq!(found.links[0].position.line, 6);
        assert_eq!(found.beats.len(), 1);
        assert_eq!(found.beats[0].number, 1);
        assert_eq!(found.beats[0].name, "Start");
        assert_eq!(found.beats[0].line, 8);
        assert!(
            found.beats[0]
                .span
                .get(text)
                .is_some_and(|beat| beat.starts_with("## Beat 1"))
        );
        Ok(())
    }

    #[test]
    fn notes_are_slots_only_in_concepts_and_maps() -> Res {
        let body = "> [!note] n1 · [[c-v1]] · resolved by [[c-v2]]\n> text\n";
        let concept = build("concepts/x.md", &format!("{}{body}", front("concept", "")))?;
        assert_eq!(
            links_of(&concept),
            [
                ("note:version".to_owned(), None, "c-v1"),
                ("note:resolved-by".to_owned(), None, "c-v2"),
            ]
        );
        let topic = build("codex/x.md", &format!("{}{body}", front("topic", "")))?;
        assert_eq!(
            links_of(&topic),
            [
                ("mention".to_owned(), None, "c-v1"),
                ("mention".to_owned(), None, "c-v2"),
            ]
        );
        assert!(topic.beats.is_empty());
        Ok(())
    }

    #[test]
    fn a_link_keeps_its_fragment() -> Res {
        let found = build(
            "codex/x.md",
            &format!("{}see [[S-004#3]]\n", front("topic", "")),
        )?;
        assert_eq!(found.links.len(), 1);
        assert_eq!(found.links[0].target, "S-004");
        assert_eq!(found.links[0].fragment.as_deref(), Some("3"));
        Ok(())
    }

    #[test]
    fn links_come_only_from_valid_values() -> Res {
        let npc = build("codex/x.md", &front("npc", "parent: [a]\n"))?;
        assert_eq!(links_of(&npc), []);
        let thing = build("codex/x.md", &front("thing", "where: [a, \"\"]\n"))?;
        assert_eq!(links_of(&thing), [("field:where".to_owned(), None, "a")]);
        let home = build("codex/x.md", &front("npc", "home: h\nparent: p\n"))?;
        assert_eq!(
            links_of(&home),
            [
                ("field:home".to_owned(), None, "h"),
                ("field:parent".to_owned(), None, "p"),
            ]
        );
        let lens = build(
            "codex/x.md",
            &front(
                "npc",
                "lens:\n  unaware_of: [T-930, \"\"]\n  rejects: [B-930]\n",
            ),
        )?;
        assert_eq!(
            links_of(&lens),
            [
                ("lens:unaware_of".to_owned(), None, "T-930"),
                ("lens:rejects".to_owned(), None, "B-930"),
            ]
        );
        Ok(())
    }

    #[test]
    fn values_hold_scalars_lists_and_maps() -> Res {
        let text = front("npc", "lens:\n  tier: T4\n  unaware_of: [T-930, T-931]\n");
        let found = build("codex/x.md", &text)?;
        let shown: Vec<_> = found
            .values
            .iter()
            .filter(|value| value.field == "lens")
            .map(|value| {
                (
                    value.entry.as_deref(),
                    value.item,
                    value.shape,
                    value.valid,
                    value.text.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            shown,
            [
                (None, None, Shape::Map, false, None),
                (Some("tier"), None, Shape::Scalar, false, Some("T4")),
                (Some("unaware_of"), None, Shape::List, true, None),
                (
                    Some("unaware_of"),
                    Some(0),
                    Shape::Scalar,
                    true,
                    Some("T-930")
                ),
                (
                    Some("unaware_of"),
                    Some(1),
                    Shape::Scalar,
                    true,
                    Some("T-931")
                ),
            ]
        );
        let tier = &found.values[found
            .values
            .iter()
            .position(|v| v.entry.as_deref() == Some("tier"))
            .ok_or("tier row")?];
        assert_eq!(tier.span.get(&text), Some("T4"));
        assert_eq!(tier.position.line, 6);
        assert_eq!(tier.style, Some(crate::yaml::ScalarStyle::Plain));
        let kind = found
            .values
            .iter()
            .find(|v| v.field == "kind")
            .ok_or("kind row")?;
        assert!(kind.valid && kind.entry.is_none() && kind.item.is_none());
        Ok(())
    }

    #[test]
    fn search_columns_and_lines() -> Res {
        let text = "---\nid: x\nkind: npc\ntitle: Synthetic Title\nstatus: draft\n\
            tags: [one, two]\nhome: h\nrole: synthetic role\n\
            lens:\n  stance: synthetic stance\n  tier: T1\n  unaware_of: [T-930]\n---\n\
            Body [[m]] text.\n";
        let found = build("codex/x.md", text)?;
        let search = found.search.as_ref().ok_or("search row")?;
        assert_eq!(search.title, "Synthetic Title");
        assert_eq!(
            search.fields,
            "x\none\ntwo\nh\nsynthetic role\nsynthetic stance\nT-930"
        );
        assert_eq!(search.body, "Body [[m]] text.\n");
        let lines: Vec<&str> = search.fields.lines().collect();
        let mut seen = 0;
        for value in &found.values {
            if let Some(line) = value.search_line {
                assert_eq!(value.text.as_deref(), lines.get(line).copied());
                seen += 1;
            }
        }
        assert_eq!(seen, lines.len());
        let record = found.record.as_ref().ok_or("record row")?;
        assert_eq!(record.title.as_deref(), Some("Synthetic Title"));
        assert_eq!(record.status.as_deref(), Some("draft"));
        assert_eq!(record.kind.as_deref(), Some("npc"));
        assert_eq!(record.id, "x");
        assert_eq!(record.path, "codex/x.md");
        assert!(
            record
                .front
                .as_deref()
                .is_some_and(|f| f.starts_with("id: x\n"))
        );
        assert!(
            record
                .front
                .as_deref()
                .is_some_and(|f| f.ends_with("T-930]\n"))
        );
        assert_eq!(record.body_start, Some(text.find("Body").ok_or("body")?));
        assert_eq!(record.body_line, Some(14));
        Ok(())
    }

    #[test]
    fn a_broken_front_matter_keeps_the_body() -> Res {
        let text = "---\nid: x\nkind: npc\ntitle: &a b\n---\nsee [[m]]\n";
        let found = build("codex/x.md", text)?;
        let record = found.record.as_ref().ok_or("record row")?;
        assert_eq!(record.kind, None);
        assert_eq!(record.title, None);
        assert!(record.body_start.is_some());
        assert!(found.values.is_empty());
        assert_eq!(links_of(&found), [("mention".to_owned(), None, "m")]);
        let search = found.search.as_ref().ok_or("search row")?;
        assert_eq!(search.title, "");
        assert_eq!(search.fields, "");
        assert_eq!(search.body, "see [[m]]\n");
        assert!(found.findings.iter().any(|f| f.rule.starts_with("yaml/")));
        Ok(())
    }

    #[test]
    fn an_undecodable_file_has_only_its_finding() -> Res {
        let registry = builtin()?;
        let checked = check_record(&registry, "codex/x.md", &[0xff, 0xfe, b'a']);
        let found = rows("codex/x.md", checked, &registry);
        let record = found.record.as_ref().ok_or("record row")?;
        assert_eq!(record.id, "x");
        assert_eq!(record.kind, None);
        assert_eq!(record.front, None);
        assert_eq!(record.body_start, None);
        assert_eq!(record.body_line, None);
        assert!(found.links.is_empty() && found.values.is_empty() && found.beats.is_empty());
        assert!(found.search.is_none());
        assert_eq!(found.findings.len(), 1);
        assert_eq!(found.findings[0].rule, "record/utf8");
        Ok(())
    }

    fn facts(path: &str, text: &str) -> Result<String, Box<dyn std::error::Error>> {
        let found = build(path, text)?;
        Ok(found.record.ok_or("record row")?.facts)
    }

    #[test]
    fn facts_change_with_link_facts_only() -> Res {
        let script = |title: &str, beats: &str| {
            format!("---\nid: S-930\nkind: script\ntitle: {title}\n---\n{beats}")
        };
        let two = "## Beat 1 — A\n\n## Beat 2 — B\n";
        let base = facts("story/scripts/S-930.md", &script("one", two))?;
        let renamed = facts("story/scripts/S-930.md", &script("two", two))?;
        let fewer = facts("story/scripts/S-930.md", &script("one", "## Beat 1 — A\n"))?;
        let moved = facts("story/other/S-930.md", &script("one", two))?;
        assert_eq!(base, renamed);
        assert_ne!(base, fewer);
        assert_ne!(base, moved);

        let topic =
            |track: &str| format!("---\nid: x\nkind: topic\ntitle: t\ntrack: {track}\n---\n");
        let explaining = facts("codex/x.md", &topic("explaining"))?;
        let in_world = facts("codex/x.md", &topic("in-world"))?;
        let bad = facts("codex/x.md", &topic("nonsense"))?;
        assert_ne!(explaining, in_world);
        assert_eq!(
            bad,
            facts("codex/x.md", "---\nid: x\nkind: topic\ntitle: t\n---\n")?
        );
        Ok(())
    }

    #[test]
    fn notes_rows_come_only_from_kinds_with_notes() -> Res {
        let body = "> [!note] n1 · [[c-v1]] · resolved by [[c-v2]]\n> text\n\n\
            > [!note] n2 · [[c-v1]] · open\n> more\n\n\
            > [!note] n3 · [[c-v1]] · dropped\n> x\n\n\
            > [!note] n4 · [[c-v1]] · maybe\n> y\n";
        let concept = build("concepts/x.md", &format!("{}{body}", front("concept", "")))?;
        let shown: Vec<_> = concept
            .notes
            .iter()
            .map(|n| {
                (
                    n.id.as_str(),
                    n.number,
                    n.state.as_str(),
                    n.resolved_by.as_deref(),
                    n.version.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            shown,
            [
                ("n1", 1, "resolved", Some("c-v2"), Some("c-v1")),
                ("n2", 2, "open", None, Some("c-v1")),
                ("n3", 3, "dropped", None, Some("c-v1")),
                ("n4", 4, "maybe", None, Some("c-v1")),
            ]
        );
        assert_eq!(concept.notes[0].position.line, 6);
        let topic = build("codex/x.md", &format!("{}{body}", front("topic", "")))?;
        assert!(topic.notes.is_empty());
        Ok(())
    }

    #[test]
    fn a_beat_keeps_its_when_token() -> Res {
        let text = "---\nid: S-930\nkind: script\ntitle: t\n---\n\
            ## Beat 1 — A\nwhen:: T2\n\n## Beat 2 — B\nat:: [[a]]\n";
        let found = build("story/scripts/S-930.md", text)?;
        let whens: Vec<_> = found.beats.iter().map(|b| b.when.as_deref()).collect();
        assert_eq!(whens, [Some("T2"), None]);
        Ok(())
    }

    #[test]
    fn only_tracks_narrow_a_target() -> Res {
        let registry = builtin()?;
        let narrow: Vec<&str> = registry.narrow_fields().into_iter().collect();
        assert_eq!(narrow, ["track"]);
        assert!(Slot::Mention.targets(None).is_none());
        Ok(())
    }
}
