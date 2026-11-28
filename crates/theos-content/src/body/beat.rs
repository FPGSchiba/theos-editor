//! Beats (data model §9.4): `## Beat <n> — <name>` headings and their `key:: value` lines.
//!
//! The headings and paragraphs come from the shared events; this module reads the heading text
//! and the lines of the attribute paragraph one line at a time, and takes value links from the
//! links already found.

use pulldown_cmark::{Event, HeadingLevel, Tag, TagEnd};

use super::callout::Token;
use super::events::Events;
use super::links::{Link, with_stand_ins};
use super::malformed::{Malformed, MalformedKind, covers_unclosed, link_faults};
use crate::text::{Span, TextIndex};

#[cfg(test)]
mod tests;

const HEADING_START: &str = "Beat ";
const HEADING_DASH: &str = " — ";
const SEPARATOR: &str = ", ";

/// A beat of a script: a `## Beat <n> — <name>` heading and what follows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Beat {
    /// The number `n` of the heading.
    pub number: u32,
    /// The bytes of the digits.
    pub number_span: Span,
    /// The name after the dash.
    pub name: String,
    /// The bytes of the name.
    pub name_span: Span,
    /// The bytes of the heading without its final line break (both lines for a setext heading).
    pub heading_span: Span,
    /// From the start of the heading to the next level-2 heading or the end of the body.
    pub span: Span,
    /// The attribute lines directly after the heading that parse, in order.
    pub attributes: Vec<Attribute>,
}

/// One `key:: value` line of a beat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribute {
    /// The key as written.
    pub key: String,
    /// The bytes of the key.
    pub key_span: Span,
    /// The value, shaped by the key.
    pub value: AttributeValue,
    /// The bytes of the value, trailing spaces excluded.
    pub value_span: Span,
    /// The bytes of the whole line without its line break.
    pub line_span: Span,
}

/// The value of an [`Attribute`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttributeValue {
    /// Wikilinks separated by `, `.
    Links(Vec<Link>),
    /// One bare word.
    Token(Token),
    /// The value of a key without a shape: its text and the links inside it.
    Raw {
        /// The text as written.
        text: String,
        /// The wikilinks inside the text.
        links: Vec<Link>,
    },
}

impl AttributeValue {
    /// The value as written: links joined by `, `, the token, or the raw text.
    #[must_use]
    pub fn spelling(&self) -> String {
        match self {
            AttributeValue::Links(links) => links
                .iter()
                .map(Link::spelling)
                .collect::<Vec<_>>()
                .join(SEPARATOR),
            AttributeValue::Token(token) => token.text.clone(),
            AttributeValue::Raw { text, .. } => text.clone(),
        }
    }
}

/// The shape a key's value must have.
#[derive(Clone, Copy)]
enum Shape {
    Links,
    Bare,
}

/// The one table of attribute shapes; keys not listed keep a raw value.
const SHAPES: &[(&str, Shape)] = &[
    ("at", Shape::Links),
    ("with", Shape::Links),
    ("says", Shape::Links),
    ("reveals", Shape::Links),
    ("when", Shape::Bare),
];

/// A beat heading read from the events.
enum Heading {
    /// Not a beat heading: an ordinary heading.
    Ordinary,
    /// Starts like a beat heading but does not read `Beat <n> — <name>`.
    Malformed,
    /// A beat heading; the span is filled in by the caller.
    Beat(Beat),
}

/// Reads the beats of a body and reports the malformed headings and attributes.
pub(crate) fn scan(
    index: &TextIndex,
    events: &Events<'_>,
    links: &[Link],
    reported: &[Malformed],
    body_end: usize,
) -> (Vec<Beat>, Vec<Malformed>) {
    let starts = block_starts(events);
    let mut beats: Vec<Beat> = Vec::new();
    let mut bad = Vec::new();
    let mut open: Option<usize> = None;
    for (block, &at) in starts.iter().enumerate() {
        let (event, item_span) = &events.items[at];
        if !matches!(
            event,
            Event::Start(Tag::Heading {
                level: HeadingLevel::H2,
                ..
            })
        ) {
            continue;
        }
        if let Some(previous) = open.take() {
            beats[previous].span.end = item_span.start;
        }
        let line = trim_break(events.source, *item_span);
        match read_heading(events, at, line, body_end) {
            Heading::Ordinary => {}
            Heading::Malformed => bad.push(Malformed::new(
                index,
                MalformedKind::BeatHeading,
                "a beat heading reads `Beat <n> — <name>`",
                line,
            )),
            Heading::Beat(mut beat) => {
                // The paragraphs right after the heading, up to the first other block.
                let mut paragraphs =
                    starts[block + 1..]
                        .iter()
                        .map_while(|&i| match &events.items[i] {
                            (Event::Start(Tag::Paragraph), paragraph) => Some(*paragraph),
                            _ => None,
                        });
                // Paragraphs that a blank line separates from the heading or from its attribute
                // lines: attribute lines there are not read, so they are reported.
                let mut detached: Vec<Span> = Vec::new();
                match paragraphs.next() {
                    Some(first) if first.start == item_span.end => {
                        read_attributes(index, events, links, reported, first, &mut beat, &mut bad);
                        if first_line_is_attribute(events.source, first) {
                            detached.extend(paragraphs);
                        }
                    }
                    Some(first) => {
                        detached.push(first);
                        detached.extend(paragraphs);
                    }
                    None => {}
                }
                for paragraph in detached
                    .into_iter()
                    .take_while(|p| first_line_is_attribute(events.source, *p))
                {
                    bad.push(Malformed::new(
                        index,
                        MalformedKind::MisplacedAttributes,
                        "attribute lines directly follow the beat heading, without a blank line",
                        first_line(events.source, paragraph).0,
                    ));
                }
                open = Some(beats.len());
                beats.push(beat);
            }
        }
    }
    (beats, bad)
}

/// The item index at which each top-level block starts; the position is the block index.
fn block_starts(events: &Events<'_>) -> Vec<usize> {
    let mut starts = Vec::new();
    for (i, &block) in events.block_of.iter().enumerate() {
        if starts.len() == block {
            starts.push(i);
        }
    }
    starts
}

/// `span` without a final line break.
fn trim_break(source: &str, span: Span) -> Span {
    let text = span.get(source).unwrap_or("");
    Span::new(
        span.start,
        span.start + text.trim_end_matches(['\n', '\r']).len(),
    )
}

/// Reads the heading opened at item `open`; `line` is its trimmed span.
fn read_heading(events: &Events<'_>, open: usize, line: Span, body_end: usize) -> Heading {
    let Some(text_span) = inline_span(events, open) else {
        return Heading::Ordinary;
    };
    let Some(text) = text_span.get(events.source) else {
        return Heading::Ordinary;
    };
    let Some(rest) = text.strip_prefix(HEADING_START) else {
        return Heading::Ordinary;
    };
    if !rest.starts_with(|c: char| c.is_ascii_digit()) {
        return Heading::Ordinary;
    }
    let digits = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    let name = rest[digits..]
        .strip_prefix(HEADING_DASH)
        .filter(|n| !n.is_empty());
    let (Ok(number), Some(name)) = (rest[..digits].parse::<u32>(), name) else {
        return Heading::Malformed;
    };
    let number_start = text_span.start + HEADING_START.len();
    let name_start = text_span.end - name.len();
    Heading::Beat(Beat {
        number,
        number_span: Span::new(number_start, number_start + digits),
        name: name.to_owned(),
        name_span: Span::new(name_start, text_span.end),
        heading_span: line,
        span: Span::new(line.start, body_end),
        attributes: Vec::new(),
    })
}

/// From the first to the last inline event of the heading opened at item `open`.
fn inline_span(events: &Events<'_>, open: usize) -> Option<Span> {
    let rest = &events.items[open + 1..];
    let close = rest
        .iter()
        .position(|(event, _)| matches!(event, Event::End(TagEnd::Heading(_))))?;
    let inner = &rest[..close];
    Some(Span::new(inner.first()?.1.start, inner.last()?.1.end))
}

/// The first line of `span` without its break: its bytes and its text.
fn first_line(source: &str, span: Span) -> (Span, &str) {
    let text = span.get(source).unwrap_or("");
    let line = text.split('\n').next().unwrap_or("").trim_end_matches('\r');
    (Span::new(span.start, span.start + line.len()), line)
}

/// Splits `key::` off the start of `line`: the key and the offset after the colons.
fn split_key(line: &str) -> Option<(&str, usize)> {
    let end = line.find("::")?;
    let key = &line[..end];
    let mut chars = key.chars();
    let valid = chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    valid.then_some((key, end + 2))
}

/// Whether `line` starts with a `key::` of the attribute form.
pub(crate) fn is_attribute_line(line: &str) -> bool {
    split_key(line).is_some()
}

fn first_line_is_attribute(source: &str, paragraph: Span) -> bool {
    split_key(first_line(source, paragraph).1).is_some()
}

/// Reads the leading attribute lines of the paragraph; the run ends at the first other line.
fn read_attributes(
    index: &TextIndex,
    events: &Events<'_>,
    links: &[Link],
    reported: &[Malformed],
    paragraph: Span,
    beat: &mut Beat,
    bad: &mut Vec<Malformed>,
) {
    let text = paragraph.get(events.source).unwrap_or("");
    let mut offset = paragraph.start;
    for raw in text.split_inclusive('\n') {
        let start = offset;
        offset += raw.len();
        let line = raw.trim_end_matches(['\n', '\r']);
        let Some((key, after)) = split_key(line) else {
            break;
        };
        let line_span = Span::new(start, start + line.len());
        let rest = &line[after..];
        let spaces = rest.len() - rest.trim_start_matches(' ').len();
        let value = rest[spaces..].trim_end();
        let value_start = start + after + spaces;
        let value_span = Span::new(value_start, value_start + value.len());
        let parsed = if spaces == 0 || value.is_empty() {
            Err("`key::` is followed by a space and a value")
        } else {
            read_value(events.source, key, value, value_span, links)
        };
        match parsed {
            Ok(value) => beat.attributes.push(Attribute {
                key: key.to_owned(),
                key_span: Span::new(start, start + key.len()),
                value,
                value_span,
                line_span,
            }),
            Err(message) => {
                if !is_only_a_link_fault(events.source, key, value, value_span, links, reported) {
                    bad.push(Malformed::new(
                        index,
                        MalformedKind::BeatAttribute,
                        format!("attribute `{key}`: {message}"),
                        line_span,
                    ));
                }
            }
        }
    }
}

/// Whether the value is wrong only because of links that are already reported: the shape reads
/// right once every reported link or embed counts as a well-formed link. An unclosed `[[` in the
/// value is the one finding of it.
fn is_only_a_link_fault(
    source: &str,
    key: &str,
    value: &str,
    span: Span,
    links: &[Link],
    reported: &[Malformed],
) -> bool {
    if covers_unclosed(reported, span) {
        return true;
    }
    let faults = link_faults(reported, span);
    if faults.is_empty() {
        return false;
    }
    let inside: Vec<Link> = links
        .iter()
        .filter(|l| span.start <= l.span.start && l.span.end <= span.end)
        .cloned()
        .collect();
    read_value(source, key, value, span, &with_stand_ins(&inside, &faults)).is_ok()
}

/// Shapes `value` by its key.
fn read_value(
    source: &str,
    key: &str,
    value: &str,
    span: Span,
    links: &[Link],
) -> Result<AttributeValue, &'static str> {
    let inside: Vec<Link> = links
        .iter()
        .filter(|l| span.start <= l.span.start && l.span.end <= span.end)
        .cloned()
        .collect();
    let shape = SHAPES
        .iter()
        .find(|(name, _)| *name == key)
        .map(|&(_, s)| s);
    match shape {
        None => Ok(AttributeValue::Raw {
            text: value.to_owned(),
            links: inside,
        }),
        Some(Shape::Links) if separated(source, &inside, span) => Ok(AttributeValue::Links(inside)),
        Some(Shape::Links) => Err("expected wikilinks separated by `, `"),
        Some(Shape::Bare) if inside.is_empty() && is_word(value) => {
            Ok(AttributeValue::Token(Token {
                text: value.to_owned(),
                span,
            }))
        }
        Some(Shape::Bare) => Err("expected one bare word"),
    }
}

/// Whether `links` cover `value` exactly, each pair separated by `, `.
fn separated(source: &str, links: &[Link], value: Span) -> bool {
    let (Some(first), Some(last)) = (links.first(), links.last()) else {
        return false;
    };
    first.span.start == value.start
        && last.span.end == value.end
        && links
            .windows(2)
            .all(|pair| source.get(pair[0].span.end..pair[1].span.start) == Some(SEPARATOR))
}

fn is_word(text: &str) -> bool {
    !text.is_empty()
        && !text
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '[' | ']' | '·'))
}
