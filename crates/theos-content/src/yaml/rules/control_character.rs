//! Rule `control-character`: a control character other than a tab anywhere in the front matter
//! (scalars, comments, blank lines). Position: that character. The check reads the characters of
//! the span, because `yaml-rust2` treats U+0000 as the end of the stream and drops the rest.

use super::{Rule, error_at};
use crate::yaml::scalar::is_forbidden_control;
use crate::yaml::{Input, YamlError, YamlRule};

pub(super) struct ControlCharacter;

impl Rule for ControlCharacter {
    fn check(&self, input: &Input<'_>) -> Option<YamlError> {
        let text = input.span.get(input.index.text())?;
        let (at, _) = text
            .char_indices()
            .find(|&(_, c)| c != '\n' && is_forbidden_control(c))?;
        error_at(
            input,
            YamlRule::ControlCharacter,
            input.span.start + at,
            "the front matter cannot hold a control character other than a tab",
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::front_error;

    // AC: CE-09.3
    #[test]
    fn control_characters_are_rejected_at_the_character() {
        for c in [
            '\u{0}', '\u{1}', '\u{7}', '\u{b}', '\u{c}', '\u{1b}', '\u{7f}', '\u{9f}',
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

    // AC: CE-09.3
    #[test]
    fn a_nul_is_rejected_wherever_it_stands() {
        let want = |p: &str| Some(("control-character", p.to_owned()));
        assert_eq!(front_error("a: x\0y\nb: 2\n"), want("2:5"));
        assert_eq!(front_error("a: 1\n# \0\nb: 2\n"), want("3:3"));
        assert_eq!(front_error("a: 1\n\0\nb: 2\n"), want("3:1"));
    }

    #[test]
    fn a_tab_and_other_text_pass() {
        assert_eq!(front_error("a: x\ty\n"), None);
        assert_eq!(front_error("a: \"é😀\"\n"), None);
    }
}
