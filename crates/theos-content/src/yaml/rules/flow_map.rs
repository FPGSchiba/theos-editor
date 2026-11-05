//! Rule `flow-map`: `f: {a: 1}`, `id: {{title}}`. Position: the `FlowMappingStart` token, at the
//! `{`.

use yaml_rust2::scanner::TokenType;

use super::{Rule, first_token};
use crate::yaml::{Input, YamlError, YamlRule};

pub(super) struct FlowMap;

impl Rule for FlowMap {
    fn check(&self, input: &Input<'_>) -> Option<YamlError> {
        first_token(input, YamlRule::FlowMap, "flow maps are not allowed", |t| {
            matches!(t, TokenType::FlowMappingStart)
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::front_error;

    // AC: CE-09.3
    #[test]
    fn flow_maps_are_rejected() {
        assert_eq!(
            front_error("f: {a: 1}\n"),
            Some(("flow-map", "2:4".to_owned()))
        );
        assert_eq!(
            front_error("id: {{title}}\n"),
            Some(("flow-map", "2:5".to_owned()))
        );
    }
}
