//! Rule `complex-key`: `? a` / `: b`. Position: the `Key` token, at the `?`. A simple key's `Key`
//! token sits where its scalar starts; a complex key's sits at the `?`, before the scalar.

use yaml_rust2::scanner::TokenType;

use super::Rule;
use crate::yaml::{Input, YamlError, YamlRule};

pub(super) struct ComplexKey;

impl Rule for ComplexKey {
    fn check(&self, input: &Input<'_>) -> Option<YamlError> {
        input
            .tokens
            .windows(2)
            .find(|pair| {
                matches!(pair[0].value, TokenType::Key) && pair[0].position != pair[1].position
            })
            .map(|pair| {
                input.error(
                    YamlRule::ComplexKey,
                    &pair[0],
                    "complex keys are not allowed",
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::front_error;

    #[test]
    fn complex_keys_are_rejected() {
        assert_eq!(
            front_error("? a\n: b\n"),
            Some(("complex-key", "2:1".to_owned()))
        );
    }
}
