//! Rule `tag`: `c: !t v` (YAML tags, not the `tags` field). Position: the `Tag` token, at the `!`.

use yaml_rust2::scanner::TokenType;

use super::{Rule, first_token};
use crate::yaml::{Input, YamlError, YamlRule};

pub(super) struct Tag;

impl Rule for Tag {
    fn check(&self, input: &Input<'_>) -> Option<YamlError> {
        first_token(input, YamlRule::Tag, "tags are not allowed", |t| {
            matches!(t, TokenType::Tag(..))
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::front_error;

    // AC: CE-09.3
    #[test]
    fn tags_are_rejected() {
        assert_eq!(front_error("c: !t v\n"), Some(("tag", "2:4".to_owned())));
    }
}
