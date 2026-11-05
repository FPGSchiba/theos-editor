//! Rule `layout`: the spellings P01's reader refuses that YAML allows. A top-level map that is
//! indented (position: its first key), whitespace between a key and its `:` (position: the first
//! whitespace character), and a block item whose scalar is not on the line of its `-` (position:
//! the `-`). A value on the line after its key stays allowed.

use yaml_rust2::scanner::TokenType;

use super::{Rule, error_at};
use crate::yaml::{Input, YamlError, YamlRule};

pub(super) struct Layout;

impl Rule for Layout {
    fn check(&self, input: &Input<'_>) -> Option<YamlError> {
        [
            indented_map(input),
            space_before_colon(input),
            item_on_next_line(input),
        ]
        .into_iter()
        .flatten()
        .min_by_key(|error| error.span.start)
    }
}

/// The first key of the front matter starts after column 1. The scanner puts the map's own
/// start token at the first `:`, so the key is the `Key` token that follows it.
fn indented_map(input: &Input<'_>) -> Option<YamlError> {
    let at = input
        .tokens
        .iter()
        .position(|token| token.value == TokenType::BlockMappingStart)?;
    let start = input
        .tokens
        .get(at + 1)
        .filter(|token| token.value == TokenType::Key)?;
    (start.position.column != 1).then(|| {
        input.error(
            YamlRule::Layout,
            start,
            "the keys of the front matter start at the first column",
        )
    })
}

/// A `:` with a space or tab right before it.
fn space_before_colon(input: &Input<'_>) -> Option<YamlError> {
    let source = input.index.text();
    input
        .tokens
        .iter()
        .filter(|token| token.value == TokenType::Value)
        .find_map(|colon| {
            let before = source.get(input.span.start..colon.offset)?;
            let blanks = before.len() - before.trim_end_matches([' ', '\t']).len();
            (blanks > 0).then(|| {
                error_at(
                    input,
                    YamlRule::Layout,
                    colon.offset - blanks,
                    "a key is followed by its colon, without a space",
                )
            })?
        })
}

/// A block item whose scalar starts on a later line than its `-`.
fn item_on_next_line(input: &Input<'_>) -> Option<YamlError> {
    input.tokens.windows(2).find_map(|pair| {
        let [entry, next] = pair else {
            return None;
        };
        (entry.value == TokenType::BlockEntry
            && matches!(next.value, TokenType::Scalar(..))
            && next.position.line != entry.position.line)
            .then(|| {
                // The dash is the first character of its line after the indentation.
                let line = input.index.line_span(entry.position.line)?;
                let text = line.get(input.index.text())?;
                let indent = text.len() - text.trim_start_matches(' ').len();
                error_at(
                    input,
                    YamlRule::Layout,
                    line.start + indent,
                    "the value of a list item is on the line of its dash",
                )
            })?
    })
}

#[cfg(test)]
mod tests {
    use crate::test_support::front_error;

    // AC: CE-09.3
    #[test]
    fn an_indented_map_is_rejected() {
        assert_eq!(
            front_error("  a: 1\n  b: 2\n"),
            Some(("layout", "2:3".to_owned()))
        );
    }

    // AC: CE-09.3
    #[test]
    fn a_space_before_the_colon_is_rejected() {
        assert_eq!(front_error("a  : 1\n"), Some(("layout", "2:2".to_owned())));
        assert_eq!(
            front_error("m:\n  e\t: 1\n"),
            Some(("layout", "3:4".to_owned()))
        );
    }

    // AC: CE-09.3
    #[test]
    fn an_item_on_the_next_line_is_rejected() {
        assert_eq!(
            front_error("l:\n  -\n    a\n"),
            Some(("layout", "3:3".to_owned()))
        );
    }

    #[test]
    fn allowed_layouts_pass() {
        assert_eq!(front_error("# c\na: 1\n"), None);
        assert_eq!(front_error("title:\n  The Choir-Hall\n"), None);
        assert_eq!(front_error("a: \"x : y\"\nb: 'p  : q'\nu: x:y\n"), None);
        assert_eq!(front_error("l:\n  - a\n  - b\n"), None);
    }
}
