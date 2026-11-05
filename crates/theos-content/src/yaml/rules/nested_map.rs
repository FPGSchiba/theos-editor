//! Rule `nested-map`: a map inside a map inside the top map. Position: the inner `MappingStart`.

use yaml_rust2::Event;

use super::Rule;
use super::nodes::{Container, first_node};
use crate::yaml::{Input, YamlError, YamlRule};

pub(super) struct NestedMap;

impl Rule for NestedMap {
    fn check(&self, input: &Input<'_>) -> Option<YamlError> {
        let message = "maps nest one level deep at most";
        first_node(input, YamlRule::NestedMap, message, |node| {
            matches!(node.event.value, Event::MappingStart(..))
                && node.parents.last() == Some(&Container::Map)
                && node.parents.len() >= 2
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::front_error;

    // AC: CE-09.3
    #[test]
    fn deeper_nesting_is_rejected() {
        assert_eq!(
            front_error("h:\n  i:\n    j: 1\n"),
            Some(("nested-map", "4:5".to_owned()))
        );
        assert_eq!(
            front_error("g:\n  - a: 1\n"),
            Some(("list-of-maps", "3:5".to_owned()))
        );
        assert_eq!(
            front_error("l:\n  - - a\n"),
            Some(("list-in-list", "3:5".to_owned()))
        );
        assert_eq!(
            front_error("l: [a, [b]]\n"),
            Some(("list-in-list", "2:8".to_owned()))
        );
    }

    #[test]
    fn one_level_of_nesting_is_accepted() {
        assert_eq!(front_error("m:\n  a: 1\n  b: [x, y]\n"), None);
    }
}
