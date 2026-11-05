//! The rules, one module each, and the registry that runs them.

mod alias;
mod anchor;
mod block_scalar;
mod complex_key;
mod document_marker;
mod duplicate_key;
mod empty_value;
mod flow_map;
mod key;
mod list_in_list;
mod list_of_maps;
mod nested_map;
mod nodes;
mod one_line_list;
mod one_line_scalar;
mod tab_indent;
mod tag;
mod top_level_map;

use yaml_rust2::scanner::TokenType;

use super::input::Marked;
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
    &block_scalar::BlockScalar,
    &one_line_scalar::OneLineScalar,
    &list_of_maps::ListOfMaps,
    &nested_map::NestedMap,
    &list_in_list::ListInList,
    &one_line_list::OneLineList,
    &top_level_map::TopLevelMap,
    &tab_indent::TabIndent,
    &duplicate_key::DuplicateKey,
    &key::Key,
    &empty_value::EmptyValue,
];

/// The earliest violation of any rule; the crate's own error only when no rule fires, because a
/// rule names the problem better (a tab fails the crate a line early, as a plain scalar goes on).
pub(crate) fn first_error(input: &Input<'_>) -> Option<YamlError> {
    RULES
        .iter()
        .filter_map(|rule| rule.check(input))
        .min_by_key(|error| error.span.start)
        .or_else(|| input.crate_error.clone())
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

/// The last `:` or `-` token that starts before `offset`.
fn last_indicator<'a>(input: &'a Input<'_>, offset: usize) -> Option<&'a Marked<TokenType>> {
    input
        .tokens
        .iter()
        .filter(|token| matches!(token.value, TokenType::Value | TokenType::BlockEntry))
        .take_while(|token| token.offset < offset)
        .last()
}

/// An error of `rule` at `offset` of the record, spanning its first character.
fn error_at(input: &Input<'_>, rule: YamlRule, offset: usize, message: &str) -> Option<YamlError> {
    let position = input.index.position(offset).ok()?;
    let at = Marked {
        value: (),
        offset,
        position,
    };
    Some(input.error(rule, &at, message))
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
