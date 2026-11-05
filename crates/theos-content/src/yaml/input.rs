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
    /// The YAML crate's error, or the reader's early-end error where the scanner ended the stream
    /// before the front matter ended; rule `Syntax` either way.
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
        let mut stream_end = None;
        let mut scanner = Scanner::new(text.chars());
        loop {
            match scanner.next_token() {
                Ok(Some(Token(at, token))) => {
                    if token == TokenType::StreamEnd {
                        stream_end = Some(input.mark((), at));
                    }
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
            .map(|error| input.syntax_error(&error))
            .or_else(|| {
                // The scanner ends the stream early at a U+0000: the rest would be dropped. The
                // position is where the stream ended (the start of the line after the cut, as
                // yaml-rust2 moves its end mark to a new line), not the NUL; `control-character`
                // reports the NUL at its own position and fires first through `Record::parse`.
                stream_end
                    .filter(|end| end.offset < input.span.end)
                    .map(|end| {
                        input.error(YamlRule::Syntax, &end, "the front matter ends early here")
                    })
            });
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

    // AC: CE-09.3
    #[test]
    fn a_stream_that_ends_before_the_front_matter_ends_is_a_syntax_error() {
        let r = record("a: x\0y\nb: 2\n", "");
        let input = Input::new(r.index(), r.front_span());
        // The NUL is at 2:5, but the position is where the scanner's stream ended: the start of
        // the line after the cut. `control-character` reports the NUL itself, and fires first.
        let error = input
            .crate_error
            .map(|e| (e.rule, e.message, e.position.line, e.position.column));
        assert_eq!(
            error,
            Some((
                YamlRule::Syntax,
                "the front matter ends early here".to_owned(),
                3,
                1
            ))
        );
        let whole = record("a: 1\nb: 2\n", "");
        let input = Input::new(whole.index(), whole.front_span());
        assert!(input.crate_error.is_none());
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
