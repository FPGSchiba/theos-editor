//! Rule `empty-value`: a key or list item with no value. Position: one character after the
//! `:` or `-` that has none.

use yaml_rust2::Event;
use yaml_rust2::scanner::TScalarStyle;

use super::{Rule, error_at, last_indicator};
use crate::yaml::{Input, YamlError, YamlRule};

pub(super) struct EmptyValue;

impl Rule for EmptyValue {
    fn check(&self, input: &Input<'_>) -> Option<YamlError> {
        // Where the source has no value the parser invents an empty plain scalar.
        let indicator = input.events.iter().find_map(|event| {
            if !matches!(&event.value, Event::Scalar(text, TScalarStyle::Plain, ..) if text.is_empty()) {
                return None;
            }
            let indicator = last_indicator(input, event.offset)?;
            // Another token in between means the value is not missing but something else
            // (a flow map, say) that its own rule reports.
            let between = input.tokens.iter().any(|token| {
                token.offset > indicator.offset && token.offset < event.offset
            });
            (!between).then_some(indicator)
        })?;
        // A `-` token marks the end of its blanks, so the `-` is the first character of its line.
        let mut offset = indicator.offset;
        if !input.index.text().get(offset..)?.starts_with(':') {
            let line = input.index.line_span(indicator.position.line)?;
            let text = line.get(input.index.text())?;
            offset = line.start + (text.len() - text.trim_start_matches(' ').len());
        }
        error_at(
            input,
            YamlRule::EmptyValue,
            offset + 1,
            "a key or item needs a value",
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::front_error;

    // AC: CE-09.3
    #[test]
    fn empty_values_are_rejected() {
        assert_eq!(front_error("o:\n"), Some(("empty-value", "2:3".to_owned())));
        assert_eq!(
            front_error("o:\np: 1\n"),
            Some(("empty-value", "2:3".to_owned()))
        );
        assert_eq!(
            front_error("l:\n  - \n"),
            Some(("empty-value", "3:4".to_owned()))
        );
        assert_eq!(front_error("e: \"\"\n"), None);
    }

    #[test]
    fn a_tilde_is_a_value() {
        assert_eq!(front_error("t: ~\n"), None);
    }

    #[test]
    fn allowed_forms_pass() {
        let front = "# comment\nid: x\ntitle: Some title # after\n\nl:\n- a\n-   b\n\
                     k:\n    - c\n    - d\ne: []\nt: [a, b,]\nm:\n  a: 1\n  f: [x, y]\n";
        assert_eq!(front_error(front), None);
    }
}
