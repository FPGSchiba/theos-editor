//! Rule `one-line-list`: a flow list that ends on another line. Position: its `[` token.

use yaml_rust2::scanner::TokenType;

use super::Rule;
use crate::yaml::{Input, YamlError, YamlRule};

pub(super) struct OneLineList;

impl Rule for OneLineList {
    fn check(&self, input: &Input<'_>) -> Option<YamlError> {
        let mut open = Vec::new();
        let mut broken = Vec::new();
        for token in &input.tokens {
            match token.value {
                TokenType::FlowSequenceStart => open.push(token),
                TokenType::FlowSequenceEnd => {
                    if let Some(start) = open.pop()
                        && start.position.line != token.position.line
                    {
                        broken.push(start);
                    }
                }
                _ => {}
            }
        }
        broken
            .into_iter()
            .min_by_key(|start| start.offset)
            .map(|start| input.error(YamlRule::OneLineList, start, "a list stays on one line"))
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::front_error;

    // AC: CE-09.3
    #[test]
    fn lists_stay_on_one_line() {
        let want = Some(("one-line-list", "2:4".to_owned()));
        assert_eq!(front_error("w: [a,\n  b]\n"), want);
        assert_eq!(front_error("w: [a, # c\n  b]\n"), want);
    }
}
