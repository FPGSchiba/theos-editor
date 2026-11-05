//! The front matter as a model: fields, values, lists and maps, each with the exact bytes it
//! occupies, built from the events `yaml-rust2` delivers once the subset rules have passed.

use yaml_rust2::Event;

use super::input::Marked;
use super::scalar::{Scalar, ScalarStyle, spelled_end};
use super::{Input, YamlError, YamlRule, first_error};
use crate::text::{Position, Span, TextIndex};

/// The fields of a front matter, in file order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrontMatter {
    /// The front matter in record coordinates (the lines between the `---` lines).
    pub span: Span,
    /// The fields, in the order they are written.
    pub fields: Vec<Field>,
}

/// One `key: value` entry of the top level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    /// The key.
    pub key: String,
    /// The bytes of the key.
    pub key_span: Span,
    /// The value.
    pub value: Value,
    /// From the start of the key's line to the end of the last value line, `\n` included.
    pub lines: Span,
    /// The key line's trailing comment, `#` to the end of the line without the newline.
    pub comment: Option<Span>,
}

/// The value of a field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// A scalar.
    Scalar(Scalar),
    /// A list of scalars.
    List(List),
    /// One level of map.
    Map(Map),
}

/// A list of scalars.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct List {
    /// Written as `[a, b]` or as `- a` lines.
    pub style: ListStyle,
    /// The items, in order.
    pub items: Vec<Item>,
    /// A flow list: `[` to after `]`. A block list: its first item's line start to its last
    /// item's line end (`\n` included).
    pub span: Span,
}

/// How a list is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListStyle {
    /// `[a, b]` on one line.
    Flow,
    /// One `- item` line per item.
    Block,
}

/// An item of a list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// The item's scalar.
    pub scalar: Scalar,
    /// The item's own line, `\n` included; only block items have one.
    pub lines: Option<Span>,
    /// The comment after a block item, without the newline.
    pub comment: Option<Span>,
}

/// A map whose values are scalars or one-line lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Map {
    /// The entries, in order.
    pub entries: Vec<Entry>,
}

/// One `key: value` entry of a map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The key.
    pub key: String,
    /// The bytes of the key.
    pub key_span: Span,
    /// The value.
    pub value: EntryValue,
    /// From the start of the key's line to the end of the last value line, `\n` included.
    pub lines: Span,
    /// The key line's trailing comment, without the newline.
    pub comment: Option<Span>,
}

/// The value of a map entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntryValue {
    /// A scalar.
    Scalar(Scalar),
    /// A list, always [`ListStyle::Flow`].
    List(List),
}

/// A field value as the raw texts of ruling D1, with every span dropped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RawValue {
    /// A scalar's text.
    Text(String),
    /// The texts of a list's items.
    List(Vec<String>),
    /// The entries of a map.
    Map(Vec<(String, RawEntry)>),
}

/// A map entry's value as raw text, with every span dropped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RawEntry {
    /// A scalar's text.
    Text(String),
    /// The texts of a list's items.
    List(Vec<String>),
}

impl List {
    fn texts(&self) -> Vec<String> {
        self.items
            .iter()
            .map(|item| item.scalar.text.clone())
            .collect()
    }
}

impl Value {
    /// The value as raw texts, without spans.
    #[must_use]
    pub fn to_raw(&self) -> RawValue {
        match self {
            Value::Scalar(scalar) => RawValue::Text(scalar.text.clone()),
            Value::List(list) => RawValue::List(list.texts()),
            Value::Map(map) => RawValue::Map(
                map.entries
                    .iter()
                    .map(|entry| (entry.key.clone(), entry.value.to_raw()))
                    .collect(),
            ),
        }
    }
}

impl EntryValue {
    /// The value as raw texts, without spans.
    #[must_use]
    pub fn to_raw(&self) -> RawEntry {
        match self {
            EntryValue::Scalar(scalar) => RawEntry::Text(scalar.text.clone()),
            EntryValue::List(list) => RawEntry::List(list.texts()),
        }
    }
}

impl FrontMatter {
    /// Reads the front matter `span` of `index`.
    ///
    /// # Errors
    ///
    /// The first violation of the restricted YAML (the rules of the restricted YAML).
    pub fn parse(index: &TextIndex, span: Span) -> Result<FrontMatter, YamlError> {
        FrontMatter::from_input(&Input::new(index, span))
    }

    /// Runs the rules over `input`, then builds the model.
    pub(crate) fn from_input(input: &Input<'_>) -> Result<FrontMatter, YamlError> {
        match first_error(input) {
            Some(error) => Err(error),
            None => Builder::new(input).front_matter(),
        }
    }

    /// The field called `key`.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Field> {
        self.fields.iter().find(|field| field.key == key)
    }
}

/// Walks the events of a front matter that passed every rule.
struct Builder<'a, 'b> {
    input: &'b Input<'a>,
    events: std::slice::Iter<'b, Marked<Event>>,
}

/// A value event, read by the builder.
enum Read {
    Scalar(Scalar),
    List(List),
    Map(Map),
}

impl<'a, 'b> Builder<'a, 'b> {
    fn new(input: &'b Input<'a>) -> Self {
        Builder {
            input,
            events: input.events.iter(),
        }
    }

    fn front_matter(mut self) -> Result<FrontMatter, YamlError> {
        let mut fields = Vec::new();
        loop {
            match self.events.next() {
                None => break,
                Some(event) => match event.value {
                    Event::StreamStart
                    | Event::StreamEnd
                    | Event::DocumentStart
                    | Event::DocumentEnd => {}
                    Event::MappingStart(..) => {
                        fields = self.fields()?;
                        break;
                    }
                    _ => return Err(self.unexpected(Some(event))),
                },
            }
        }
        Ok(FrontMatter {
            span: self.input.span,
            fields,
        })
    }

    fn fields(&mut self) -> Result<Vec<Field>, YamlError> {
        let mut fields = Vec::new();
        loop {
            let event = self.take()?;
            match &event.value {
                Event::MappingEnd => return Ok(fields),
                Event::Scalar(key, ..) => fields.push(self.field(event, key)?),
                _ => return Err(self.unexpected(Some(event))),
            }
        }
    }

    fn field(&mut self, key_event: &Marked<Event>, key: &str) -> Result<Field, YamlError> {
        let key_span = self.key_span(key_event, key)?;
        let event = self.take()?;
        let (value, value_end, comment) = match self.read_value(event)? {
            Read::Scalar(scalar) => {
                let comment = self.comment_for(key_span, scalar.span);
                let value_end = scalar.span.end;
                (Value::Scalar(scalar), value_end, comment)
            }
            Read::List(list) if list.style == ListStyle::Flow => {
                let comment = self.comment_for(key_span, list.span);
                let value_end = list.span.end;
                (Value::List(list), value_end, comment)
            }
            Read::List(list) => {
                let value_end = list.span.end;
                (
                    Value::List(list),
                    value_end,
                    self.key_line_comment(key_span),
                )
            }
            Read::Map(map) => {
                let value_end = map.entries.last().map_or(key_span.end, |e| e.lines.end);
                (Value::Map(map), value_end, self.key_line_comment(key_span))
            }
        };
        Ok(Field {
            key: key.to_owned(),
            key_span,
            value,
            lines: self.lines(key_span, value_end),
            comment,
        })
    }

    fn read_value(&mut self, event: &Marked<Event>) -> Result<Read, YamlError> {
        match &event.value {
            Event::Scalar(..) => Ok(Read::Scalar(self.scalar(event)?)),
            Event::SequenceStart(..) => Ok(Read::List(self.list(event)?)),
            Event::MappingStart(..) => Ok(Read::Map(self.map()?)),
            _ => Err(self.unexpected(Some(event))),
        }
    }

    fn map(&mut self) -> Result<Map, YamlError> {
        let mut entries = Vec::new();
        loop {
            let key_event = self.take()?;
            let key = match &key_event.value {
                Event::MappingEnd => return Ok(Map { entries }),
                Event::Scalar(key, ..) => key,
                _ => return Err(self.unexpected(Some(key_event))),
            };
            let key_span = self.key_span(key_event, key)?;
            let event = self.take()?;
            let (value, span) = match self.read_value(event)? {
                Read::Scalar(scalar) => {
                    let span = scalar.span;
                    (EntryValue::Scalar(scalar), span)
                }
                Read::List(list) if list.style == ListStyle::Flow => {
                    let span = list.span;
                    (EntryValue::List(list), span)
                }
                _ => return Err(self.unexpected(Some(event))),
            };
            entries.push(Entry {
                key: key.clone(),
                key_span,
                value,
                lines: self.lines(key_span, span.end),
                comment: self.comment_for(key_span, span),
            });
        }
    }

    fn list(&mut self, start: &Marked<Event>) -> Result<List, YamlError> {
        let is_flow = self.input.is_flow_sequence(start.offset);
        let mut items = Vec::new();
        loop {
            let event = self.take()?;
            match &event.value {
                Event::SequenceEnd => {
                    return Ok(Self::finish_list(is_flow, start, items, event));
                }
                Event::Scalar(..) => {
                    let scalar = self.scalar(event)?;
                    items.push(self.item(is_flow, scalar));
                }
                _ => return Err(self.unexpected(Some(event))),
            }
        }
    }

    fn item(&self, is_flow: bool, scalar: Scalar) -> Item {
        if is_flow {
            return Item {
                scalar,
                lines: None,
                comment: None,
            };
        }
        Item {
            lines: self.line_span(scalar.span.start),
            comment: self.comment_after(scalar.span.end),
            scalar,
        }
    }

    fn finish_list(
        is_flow: bool,
        start: &Marked<Event>,
        items: Vec<Item>,
        end: &Marked<Event>,
    ) -> List {
        let span = if is_flow {
            // the end event sits on the `]`, one byte wide
            Span::new(start.offset, end.offset + 1)
        } else {
            let first = items.first().and_then(|item| item.lines);
            let last = items.last().and_then(|item| item.lines);
            Span::new(
                first.map_or(start.offset, |lines| lines.start),
                last.map_or(end.offset, |lines| lines.end),
            )
        };
        List {
            style: if is_flow {
                ListStyle::Flow
            } else {
                ListStyle::Block
            },
            items,
            span,
        }
    }

    fn scalar(&self, event: &Marked<Event>) -> Result<Scalar, YamlError> {
        let Event::Scalar(text, style, ..) = &event.value else {
            return Err(self.unexpected(Some(event)));
        };
        let style = ScalarStyle::from_crate(*style).ok_or_else(|| self.unexpected(Some(event)))?;
        let end = spelled_end(self.input.index.text(), event.offset, style, text)
            .map_err(|_| self.unexpected(Some(event)))?;
        Ok(Scalar {
            text: text.clone(),
            style,
            span: Span::new(event.offset, end),
            position: event.position,
        })
    }

    fn key_span(&self, event: &Marked<Event>, key: &str) -> Result<Span, YamlError> {
        let span = Span::new(event.offset, event.offset + key.len());
        if span.get(self.input.index.text()) == Some(key) {
            Ok(span)
        } else {
            Err(self.unexpected(Some(event)))
        }
    }

    fn take(&mut self) -> Result<&'b Marked<Event>, YamlError> {
        self.events.next().ok_or_else(|| self.unexpected(None))
    }

    /// A `Syntax` error at `at`, or at the end of the front matter when the events ran out.
    fn unexpected(&self, at: Option<&Marked<Event>>) -> YamlError {
        const MESSAGE: &str = "the front matter has a structure the model cannot read";
        if let Some(event) = at {
            return self.input.error(YamlRule::Syntax, event, MESSAGE);
        }
        let offset = self.input.span.end;
        let position = self
            .input
            .index
            .position(offset)
            .unwrap_or(Position { line: 1, column: 1 });
        let end = Marked {
            value: (),
            offset,
            position,
        };
        self.input.error(YamlRule::Syntax, &end, MESSAGE)
    }

    fn line_of(&self, offset: usize) -> usize {
        self.input
            .index
            .position(offset)
            .map_or(1, |position| position.line)
    }

    fn line_span(&self, offset: usize) -> Option<Span> {
        self.input.index.line_span(self.line_of(offset))
    }

    /// From the start of the key's line to the end of the line of the last value byte before
    /// `last_end`.
    fn lines(&self, key_span: Span, last_end: usize) -> Span {
        let start = self
            .line_span(key_span.start)
            .map_or(key_span.start, |s| s.start);
        // the last value byte may be the tail of a multi-byte character: use that character
        let text = self.input.index.text();
        let mut last = last_end.saturating_sub(1);
        while last > 0 && !text.is_char_boundary(last) {
            last -= 1;
        }
        let end = self.line_span(last).map_or(last_end, |s| s.end);
        Span::new(start, end)
    }

    /// The key line's comment: after the value when it shares the key's line, else after `key:`.
    fn comment_for(&self, key_span: Span, value: Span) -> Option<Span> {
        if self.line_of(value.start) > self.line_of(key_span.start) {
            self.key_line_comment(key_span)
                .or_else(|| self.comment_after(value.end))
        } else {
            self.comment_after(value.end)
        }
    }

    /// The comment after the `:` that follows the key.
    fn key_line_comment(&self, key_span: Span) -> Option<Span> {
        let rest = self.rest_of_front(key_span.end)?;
        let trimmed = rest.trim_start_matches([' ', '\t']);
        let colon = key_span.end + (rest.len() - trimmed.len());
        trimmed
            .starts_with(':')
            .then(|| self.comment_after(colon + 1))
            .flatten()
    }

    /// The `#…` on the rest of the line after `offset`, without the newline.
    fn comment_after(&self, offset: usize) -> Option<Span> {
        let rest = self.rest_of_front(offset)?;
        let trimmed = rest.trim_start_matches([' ', '\t']);
        if !trimmed.starts_with('#') {
            return None;
        }
        let start = offset + (rest.len() - trimmed.len());
        let length = trimmed.find('\n').unwrap_or(trimmed.len());
        Some(Span::new(start, start + length))
    }

    fn rest_of_front(&self, offset: usize) -> Option<&'b str> {
        self.input.index.text().get(offset..self.input.span.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::record;

    const ILSE: &str = "id: mother-ilse\nkind: npc\ntitle: Mother Ilse\nparent: fates-cult\n\
home: fates-shrine\nrole: keeper of the Fates' shrine\nlens:\n  speaks_to: children and newcomers; \
calls the player \"stair-child\"\n";

    const B901: &str = "# synthetic fixture (P10), not canon\nid: B-901\nkind: belief\n\
title: \"Synthetic belief: a double-quoted title\"\nstatus: draft   # a comment after a value\n\
held_by:\n  - throat-kin\n  - hearth-folk\ndistorts: T-012\naccuracy: 'partial'\n\n\
topic: synthetic-voices-in-world\n";

    fn front_of(front: &str) -> Result<(crate::record::Record, FrontMatter), YamlError> {
        let r = record(front, "");
        let model = r.front_matter().map_err(Clone::clone)?.clone();
        Ok((r, model))
    }

    fn field<'a>(model: &'a FrontMatter, key: &str) -> Result<&'a Field, String> {
        model.get(key).ok_or(format!("no field {key}"))
    }

    fn slice(r: &crate::record::Record, span: Span) -> &str {
        span.get(r.text()).unwrap_or("<out of range>")
    }

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn fields_keep_their_spans_and_lines() -> TestResult {
        let (r, model) = front_of(ILSE)?;
        let keys: Vec<&str> = model.fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(
            keys,
            ["id", "kind", "title", "parent", "home", "role", "lens"]
        );
        for f in &model.fields {
            assert_eq!(slice(&r, f.key_span), f.key);
        }
        let Value::Scalar(role) = &field(&model, "role")?.value else {
            return Err("role is a scalar".into());
        };
        assert_eq!(role.text, "keeper of the Fates' shrine");
        assert_eq!(role.style, ScalarStyle::Plain);
        let lens = field(&model, "lens")?;
        assert_eq!(
            slice(&r, lens.lines),
            "lens:\n  speaks_to: children and newcomers; calls the player \"stair-child\"\n"
        );
        let Value::Map(map) = &lens.value else {
            return Err("lens is a map".into());
        };
        assert_eq!(map.entries.len(), 1);
        let entry = &map.entries[0];
        assert_eq!(entry.key, "speaks_to");
        assert_eq!(slice(&r, entry.key_span), "speaks_to");
        assert!(matches!(&entry.value, EntryValue::Scalar(s)
                if s.text == "children and newcomers; calls the player \"stair-child\""));
        assert!(slice(&r, entry.lines).starts_with("  speaks_to:"));
        Ok(())
    }

    #[test]
    fn comments_and_quotes_of_b901() -> TestResult {
        let (r, model) = front_of(B901)?;
        let comment_line = "# synthetic fixture (P10), not canon\n";
        for f in &model.fields {
            assert!(!slice(&r, f.lines).contains(comment_line), "{}", f.key);
        }
        let status = field(&model, "status")?;
        assert_eq!(
            status.comment.map(|c| slice(&r, c)),
            Some("# a comment after a value")
        );
        assert_eq!(
            slice(&r, status.lines),
            "status: draft   # a comment after a value\n"
        );
        let held_by = field(&model, "held_by")?;
        let Value::List(list) = &held_by.value else {
            return Err("held_by is a list".into());
        };
        assert_eq!(list.style, ListStyle::Block);
        assert_eq!(slice(&r, list.span), "  - throat-kin\n  - hearth-folk\n");
        let lines: Vec<&str> = list
            .items
            .iter()
            .filter_map(|i| i.lines.map(|l| slice(&r, l)))
            .collect();
        assert_eq!(lines, ["  - throat-kin\n", "  - hearth-folk\n"]);
        assert_eq!(
            held_by.value.to_raw(),
            RawValue::List(vec!["throat-kin".into(), "hearth-folk".into(),])
        );
        assert_eq!(
            slice(&r, held_by.lines),
            "held_by:\n  - throat-kin\n  - hearth-folk\n"
        );
        let Value::Scalar(accuracy) = &field(&model, "accuracy")?.value else {
            return Err("accuracy is a scalar".into());
        };
        assert_eq!(accuracy.style, ScalarStyle::SingleQuoted);
        assert_eq!(accuracy.text, "partial");
        assert_eq!(slice(&r, accuracy.span), "'partial'");
        let topic = field(&model, "topic")?;
        assert_eq!(slice(&r, topic.lines), "topic: synthetic-voices-in-world\n");
        Ok(())
    }

    #[test]
    fn flow_lists_and_odd_spacing() -> TestResult {
        let (r, model) = front_of("w: ['a', \"b\",]\n")?;
        let Value::List(list) = &field(&model, "w")?.value else {
            return Err("w is a list".into());
        };
        assert_eq!(list.style, ListStyle::Flow);
        assert_eq!(list.items.len(), 2);
        assert_eq!(slice(&r, list.span), "['a', \"b\",]");
        assert!(list.items.iter().all(|i| i.lines.is_none()));

        let (_, model) = front_of("w: []\n")?;
        let Value::List(list) = &field(&model, "w")?.value else {
            return Err("w is a list".into());
        };
        assert_eq!((list.style, list.items.len()), (ListStyle::Flow, 0));

        let (r, model) = front_of("w: [ a ]  # c\n")?;
        let w = field(&model, "w")?;
        assert_eq!(w.comment.map(|c| slice(&r, c)), Some("# c"));
        Ok(())
    }

    #[test]
    fn values_on_the_next_line_and_key_comments() -> TestResult {
        let (r, model) = front_of("title:\n  The Choir-Hall\n")?;
        assert_eq!(
            slice(&r, field(&model, "title")?.lines),
            "title:\n  The Choir-Hall\n"
        );

        let (r, model) = front_of("held_by:   # who\n  - a\n")?;
        let held_by = field(&model, "held_by")?;
        assert_eq!(held_by.comment.map(|c| slice(&r, c)), Some("# who"));

        let (r, model) = front_of("held_by:\n- a\n- b\n")?;
        let Value::List(list) = &field(&model, "held_by")?.value else {
            return Err("held_by is a list".into());
        };
        assert_eq!(list.items.len(), 2);
        assert_eq!(slice(&r, list.span), "- a\n- b\n");

        let (r, model) = front_of("l:\n  - a\n  # note\n  - b\n")?;
        let l = field(&model, "l")?;
        let Value::List(list) = &l.value else {
            return Err("l is a list".into());
        };
        assert_eq!(list.items.len(), 2);
        assert_eq!(slice(&r, l.lines), "l:\n  - a\n  # note\n  - b\n");
        Ok(())
    }

    // AC: CE-09.2
    #[test]
    fn lines_end_after_a_value_that_ends_in_a_multi_byte_character() -> TestResult {
        let (r, model) = front_of("title: A¡\nlens:\n  stance: ö\nnext: 1\n")?;
        assert_eq!(slice(&r, field(&model, "title")?.lines), "title: A¡\n");
        let lens = field(&model, "lens")?;
        assert_eq!(slice(&r, lens.lines), "lens:\n  stance: ö\n");
        let Value::Map(map) = &lens.value else {
            return Err("lens is a map".into());
        };
        assert_eq!(slice(&r, map.entries[0].lines), "  stance: ö\n");
        Ok(())
    }

    #[test]
    fn empty_front_matter_has_no_fields() -> TestResult {
        for front in ["", "# only a comment\n"] {
            let (_, model) = front_of(front)?;
            assert!(model.fields.is_empty(), "{front:?}");
        }
        Ok(())
    }

    #[test]
    fn raw_values_drop_spans() -> TestResult {
        let (_, model) = front_of(&format!("{ILSE}{}", "held_by:\n  - throat-kin\n"))?;
        assert_eq!(
            field(&model, "held_by")?.value.to_raw(),
            RawValue::List(vec!["throat-kin".into()])
        );
        assert_eq!(
            field(&model, "lens")?.value.to_raw(),
            RawValue::Map(vec![(
                "speaks_to".into(),
                RawEntry::Text("children and newcomers; calls the player \"stair-child\"".into())
            )])
        );
        assert_eq!(
            field(&model, "id")?.value.to_raw(),
            RawValue::Text("mother-ilse".into())
        );
        let (_, model) = front_of("m:\n  a: [x, y]\n")?;
        assert_eq!(
            field(&model, "m")?.value.to_raw(),
            RawValue::Map(vec![(
                "a".into(),
                RawEntry::List(vec!["x".into(), "y".into()])
            )])
        );
        Ok(())
    }

    #[test]
    fn record_exposes_the_front_matter() {
        let ok = record("a: 1\n", "");
        assert!(ok.front_matter().is_ok());
        assert_eq!(ok.front_matter_error(), None);

        let bad = record("a: &x 1\n", "");
        let error = bad.front_matter().err().cloned();
        assert_eq!(error.as_ref().map(|e| e.rule), Some(YamlRule::Anchor));
        assert_eq!(bad.front_matter_error(), error.as_ref());
    }
}
