//! The rules, one module each, and the registry that runs them.

mod alias;
mod anchor;
mod complex_key;
mod document_marker;
mod flow_map;
mod tag;

use yaml_rust2::scanner::TokenType;

use super::{Input, YamlError, YamlRule};

/// One rule of the restricted YAML.
pub(crate) trait Rule {
    /// The first violation of this rule in `input`, in document order.
    fn check(&self, input: &Input<'_>) -> Option<YamlError>;
}

/// Every rule: a new rule is one module plus one line here.
pub(crate) const RULES: &[&dyn Rule] = &[
    &anchor::Anchor,
    &alias::Alias,
    &tag::Tag,
    &flow_map::FlowMap,
    &complex_key::ComplexKey,
    &document_marker::DocumentMarker,
];

/// The earliest violation of any rule or of the crate; a rule beats the crate at the same place.
pub(crate) fn first_error(input: &Input<'_>) -> Option<YamlError> {
    RULES
        .iter()
        .filter_map(|rule| rule.check(input))
        .chain(input.crate_error.clone())
        .min_by_key(|error| error.span.start)
}

/// The first token that `is_forbidden` accepts, as an error of `rule`.
fn first_token(
    input: &Input<'_>,
    rule: YamlRule,
    message: &str,
    is_forbidden: impl Fn(&TokenType) -> bool,
) -> Option<YamlError> {
    input
        .tokens
        .iter()
        .find(|token| is_forbidden(&token.value))
        .map(|token| input.error(rule, token, message))
}

#[cfg(test)]
mod tests {
    use crate::test_support::front_error;

    #[test]
    fn the_first_error_in_document_order_wins() {
        assert_eq!(
            front_error("a: *x\nb: &y 1\n"),
            Some(("alias", "2:4".to_owned()))
        );
        assert_eq!(
            front_error("a: &x 1\nb: @x\n"),
            Some(("anchor", "2:4".to_owned()))
        );
    }

    #[test]
    fn crate_errors_keep_the_crate_position() {
        let (rule, position) = front_error("a: @x\n").unwrap_or_default();
        assert_eq!(rule, "syntax");
        assert_eq!(position, "2:4");
    }
}
