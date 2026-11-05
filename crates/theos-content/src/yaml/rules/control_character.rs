//! Rule `control-character`: a scalar (key or value, any style) that holds a control character
//! other than a tab. Position: that character. A scalar another rule already rejects for its
//! spelling (a line break, an escape) is left to that rule, so a fault is reported once.

use yaml_rust2::Event;

use super::{Rule, error_at};
use crate::yaml::scalar::{ScalarStyle, is_forbidden_control, spelled_end};
use crate::yaml::{Input, YamlError, YamlRule};

pub(super) struct ControlCharacter;

impl Rule for ControlCharacter {
    fn check(&self, input: &Input<'_>) -> Option<YamlError> {
        let source = input.index.text();
        input.events.iter().find_map(|event| {
            let Event::Scalar(text, style, ..) = &event.value else {
                return None;
            };
            let style = ScalarStyle::from_crate(*style)?;
            let end = spelled_end(source, event.offset, style, text).ok()?;
            let spelled = source.get(event.offset..end)?;
            let (at, _) = spelled
                .char_indices()
                .find(|(_, c)| is_forbidden_control(*c))?;
            error_at(
                input,
                YamlRule::ControlCharacter,
                event.offset + at,
                "a value cannot hold a control character other than a tab",
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::front_error;

    // AC: CE-09.3
    #[test]
    fn control_characters_are_rejected_at_the_character() {
        for c in [
            '\u{1}', '\u{7}', '\u{b}', '\u{c}', '\u{1b}', '\u{7f}', '\u{9f}',
        ] {
            let want = Some(("control-character", "2:5".to_owned()));
            assert_eq!(front_error(&format!("a: x{c}y\n")), want, "plain {c:?}");
            assert_eq!(
                front_error(&format!("b: 'p{c}q'\n")),
                Some(("control-character", "2:6".to_owned())),
                "single {c:?}"
            );
            assert_eq!(
                front_error(&format!("b: \"p{c}q\"\n")),
                Some(("control-character", "2:6".to_owned())),
                "double {c:?}"
            );
            assert_eq!(
                front_error(&format!("l: [a, x{c}y]\n")),
                Some(("control-character", "2:9".to_owned())),
                "item {c:?}"
            );
            assert_eq!(
                front_error(&format!("l:\n  - x{c}y\n")),
                Some(("control-character", "3:6".to_owned())),
                "block item {c:?}"
            );
        }
    }

    #[test]
    fn a_tab_and_other_text_pass() {
        assert_eq!(front_error("a: x\ty\n"), None);
        assert_eq!(front_error("a: \"é😀\"\n"), None);
    }
}
