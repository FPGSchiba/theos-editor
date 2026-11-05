//! The one pass of `yaml-rust2` over a front matter that every rule reads.

use yaml_rust2::Event;
use yaml_rust2::parser::{MarkedEventReceiver, Parser};
use yaml_rust2::scanner::{Marker, ScanError, Scanner, Token, TokenType};

use super::error::{YamlError, YamlRule};
use crate::text::{Position, Span, TextIndex};

/// A token or event with its place in the record.
pub(crate) struct Marked<T> {
    /// What the crate delivered.
    pub value: T,
    /// Byte offset in the record text.
    pub offset: usize,
    /// Line and column in the record.
    pub position: Position,
}

/// Tokens and events of the front matter, in record coordinates.
pub(crate) struct Input<'a> {
    /// The record's text index.
    pub index: &'a TextIndex,
    /// The front matter in record coordinates.
    pub span: Span,
    /// The scanner's tokens, up to its first error.
    pub tokens: Vec<Marked<TokenType>>,
    /// The parser's events, as delivered to `MarkedEventReceiver`.
    pub events: Vec<Marked<Event>>,
    /// The crate's own error, rule `Syntax`.
    pub crate_error: Option<YamlError>,
}

struct Collector(Vec<(Event, Marker)>);

impl MarkedEventReceiver for Collector {
    fn on_event(&mut self, ev: Event, mark: Marker) {
        self.0.push((ev, mark));
    }
}

impl<'a> Input<'a> {
    /// Scans and parses the front matter `span` of `index` once.
    pub fn new(index: &'a TextIndex, span: Span) -> Input<'a> {
        let text = span.get(index.text()).unwrap_or_default();
        let mut input = Input {
            index,
            span,
            tokens: Vec::new(),
            events: Vec::new(),
            crate_error: None,
        };
        let mut scan_error = None;
        let mut scanner = Scanner::new(text.chars());
        loop {
            match scanner.next_token() {
                Ok(Some(Token(at, token))) => {
                    let marked = input.mark(token, at);
                    input.tokens.push(marked);
                }
                Ok(None) => break,
                Err(error) => {
                    scan_error = Some(error);
                    break;
                }
            }
        }
        let mut collector = Collector(Vec::new());
        let parse_error = Parser::new_from_str(text).load(&mut collector, false).err();
        for (event, marker) in collector.0 {
            let marked = input.mark(event, marker);
            input.events.push(marked);
        }
        input.crate_error = scan_error
            .or(parse_error)
            .map(|error| input.syntax_error(&error));
        input
    }

    /// Whether the sequence whose start event is at `offset` is a flow list (`[a, b]`): the
    /// scanner has a `[` token there, while a block list's event sits at its first `-`.
    pub fn is_flow_sequence(&self, offset: usize) -> bool {
        self.tokens
            .iter()
            .any(|token| token.value == TokenType::FlowSequenceStart && token.offset == offset)
    }

    /// An error of `rule` at `at`, spanning its first character.
    pub fn error<T>(&self, rule: YamlRule, at: &Marked<T>, message: &str) -> YamlError {
        YamlError {
            rule,
            message: message.to_owned(),
            span: self.char_span(at.offset),
            position: at.position,
        }
    }

    fn syntax_error(&self, error: &ScanError) -> YamlError {
        let at = self.mark((), *error.marker());
        self.error(YamlRule::Syntax, &at, error.info())
    }

    fn mark<T>(&self, value: T, marker: Marker) -> Marked<T> {
        let first = self
            .index
            .position(self.span.start)
            .map_or(1, |position| position.line);
        let position = Position {
            line: first + marker.line().saturating_sub(1),
            column: marker.col() + 1,
        };
        match self.index.offset(position) {
            Ok(offset) if offset <= self.span.end => Marked {
                value,
                offset,
                position,
            },
            _ => {
                let position = self
                    .index
                    .position(self.span.end)
                    .unwrap_or(Position { line: 1, column: 1 });
                Marked {
                    value,
                    offset: self.span.end,
                    position,
                }
            }
        }
    }

    fn char_span(&self, offset: usize) -> Span {
        let width = self
            .index
            .text()
            .get(offset..self.span.end)
            .and_then(|rest| rest.chars().next())
            .map_or(0, char::len_utf8);
        Span::new(offset, offset + width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::record;

    fn is_kind(event: &Event) -> bool {
        matches!(event, Event::Scalar(value, ..) if value == "kind")
    }

    // The unit of `Marker::index()` is characters, not bytes (spec §4.2); offsets never use it.
    #[test]
    fn yaml_rust2_markers_count_characters() {
        let text = "title: Mère Ilse ö 😀 x\nkind: npc";
        let mut collector = Collector(Vec::new());
        assert!(
            Parser::new_from_str(text)
                .load(&mut collector, false)
                .is_ok()
        );
        let marker = collector
            .0
            .iter()
            .find(|(event, _)| is_kind(event))
            .map(|(_, marker)| *marker);
        let Some(marker) = marker else {
            panic!("the kind key is an event");
        };
        assert_eq!((marker.index(), marker.line(), marker.col()), (23, 2, 0));

        let r = record(&format!("{text}\n"), "");
        let input = Input::new(r.index(), r.front_span());
        let key = input.events.iter().find(|e| is_kind(&e.value));
        let Some(key) = key else {
            panic!("the kind key is an event");
        };
        assert_eq!(Some(key.offset), r.text().find("kind"));
        assert_eq!(key.position, Position { line: 3, column: 1 });
    }
}
