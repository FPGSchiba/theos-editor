//! Rule `alias`: `b: *x`. Position: the `Alias` token, at the `*`.

use yaml_rust2::scanner::TokenType;

use super::{Rule, first_token};
use crate::yaml::{Input, YamlError, YamlRule};

pub(super) struct Alias;

impl Rule for Alias {
    fn check(&self, input: &Input<'_>) -> Option<YamlError> {
        first_token(input, YamlRule::Alias, "aliases are not allowed", |t| {
            matches!(t, TokenType::Alias(_))
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::front_error;

    // AC: CE-09.3
    #[test]
    fn aliases_are_rejected() {
        assert_eq!(
            front_error("a: 1\nb: *x\n"),
            Some(("alias", "3:4".to_owned()))
        );
    }

    // AC: CE-09.3
    #[test]
    fn columns_count_unicode_scalar_values() {
        assert_eq!(
            front_error("tags: [Mère, 😀, *x]\n"),
            Some(("alias", "2:17".to_owned()))
        );
    }
}
