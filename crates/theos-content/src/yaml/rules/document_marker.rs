//! Rule `document-marker`: `...`, `%YAML 1.2`. Position: the `DocumentEnd`, an explicit
//! `DocumentStart` or a directive token.

use yaml_rust2::scanner::TokenType;

use super::{Rule, first_token};
use crate::yaml::{Input, YamlError, YamlRule};

pub(super) struct DocumentMarker;

impl Rule for DocumentMarker {
    fn check(&self, input: &Input<'_>) -> Option<YamlError> {
        first_token(
            input,
            YamlRule::DocumentMarker,
            "document markers and directives are not allowed",
            |t| {
                matches!(
                    t,
                    TokenType::DocumentEnd
                        | TokenType::DocumentStart
                        | TokenType::VersionDirective(..)
                        | TokenType::TagDirective(..)
                )
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::front_error;

    #[test]
    fn document_markers_are_rejected() {
        assert_eq!(
            front_error("a: 1\n...\n"),
            Some(("document-marker", "3:1".to_owned()))
        );
        assert_eq!(
            front_error("%YAML 1.2\na: 1\n"),
            Some(("document-marker", "2:1".to_owned()))
        );
    }
}
