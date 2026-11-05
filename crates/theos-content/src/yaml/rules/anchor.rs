//! Rule `anchor`: `a: &x 1`. Position: the `Anchor` token, at the `&`.

use yaml_rust2::scanner::TokenType;

use super::{Rule, first_token};
use crate::yaml::{Input, YamlError, YamlRule};

pub(super) struct Anchor;

impl Rule for Anchor {
    fn check(&self, input: &Input<'_>) -> Option<YamlError> {
        first_token(input, YamlRule::Anchor, "anchors are not allowed", |t| {
            matches!(t, TokenType::Anchor(_))
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::front_error;

    // AC: CE-09.3
    #[test]
    fn anchors_are_rejected() {
        assert_eq!(front_error("a: &x 1\n"), Some(("anchor", "2:4".to_owned())));
    }
}
