//! Rule `key`: a key that is not a plain scalar or does not match `^[a-z][a-z0-9_]*$`.

use yaml_rust2::Event;
use yaml_rust2::scanner::TScalarStyle;

use super::Rule;
use super::nodes::first_node;
use crate::yaml::{Input, YamlError, YamlRule};

pub(super) struct Key;

/// Whether `name` matches `^[a-z][a-z0-9_]*$`.
pub(crate) fn is_key_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

impl Rule for Key {
    fn check(&self, input: &Input<'_>) -> Option<YamlError> {
        let message = "keys are plain and match [a-z][a-z0-9_]*";
        first_node(input, YamlRule::Key, message, |node| {
            node.key_of.is_some()
                && match &node.event.value {
                    Event::Scalar(name, style, ..) => {
                        *style != TScalarStyle::Plain || !is_key_name(name)
                    }
                    // A key that is no scalar (`[a]: 1`).
                    _ => true,
                }
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::front_error;

    // AC: CE-09.3
    #[test]
    fn keys_follow_the_key_rule() {
        let want = Some(("key", "2:1".to_owned()));
        assert_eq!(front_error("Key: 1\n"), want);
        assert_eq!(front_error("k-2: 3\n"), want);
        assert_eq!(front_error("\"a\": 1\n"), want);
        assert_eq!(front_error("m:\n  B: 1\n"), Some(("key", "3:3".to_owned())));
    }

    // AC: CE-09.3
    #[test]
    fn a_key_must_be_a_scalar() {
        assert_eq!(front_error("[a]: 1\n"), Some(("key", "2:1".to_owned())));
        assert_eq!(
            front_error("m:\n  [a, b]: 1\n"),
            Some(("key", "3:3".to_owned()))
        );
    }
}
