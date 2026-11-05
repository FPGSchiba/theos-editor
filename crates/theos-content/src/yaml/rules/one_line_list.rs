//! Rule `one-line-list`: a flow list that ends on another line (position: its `[`), or a block
//! list as the value of a map entry (position: its first `-`).

use yaml_rust2::Event;
use yaml_rust2::scanner::TokenType;

use super::Rule;
use super::nodes::{Container, nodes};
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
        let split = broken
            .into_iter()
            .map(|start| input.error(YamlRule::OneLineList, start, "a list stays on one line"));
        split
            .chain(block_list_in_entry(input))
            .min_by_key(|error| error.span.start)
    }
}

/// A block list as the value of an entry of a nested map. The event of a flow list sits at a
/// `[` token; the event of a block list sits at its first `-`.
fn block_list_in_entry(input: &Input<'_>) -> Option<YamlError> {
    let message = "a list in a map entry goes on one line";
    nodes(input)
        .into_iter()
        .filter(|node| {
            matches!(node.event.value, Event::SequenceStart(..))
                && node.parents.len() >= 2
                && node.parents.last() == Some(&Container::Map)
        })
        .find(|node| !input.is_flow_sequence(node.event.offset))
        .map(|node| input.error(YamlRule::OneLineList, node.event, message))
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

    // AC: CE-09.3
    #[test]
    fn block_lists_do_not_fit_in_a_map_entry() {
        assert_eq!(
            front_error("m:\n  a:\n    - x\n"),
            Some(("one-line-list", "4:5".to_owned()))
        );
        assert_eq!(front_error("m:\n  a: [x, y]\n"), None);
        assert_eq!(front_error("l:\n  - x\n"), None);
    }
}
