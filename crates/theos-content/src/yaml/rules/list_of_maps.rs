//! Rule `list-of-maps`: `- a: 1`. Position: the inner `MappingStart` event.

use yaml_rust2::Event;

use super::Rule;
use super::nodes::{Container, first_node};
use crate::yaml::{Input, YamlError, YamlRule};

pub(super) struct ListOfMaps;

impl Rule for ListOfMaps {
    fn check(&self, input: &Input<'_>) -> Option<YamlError> {
        let message = "lists of maps are not allowed";
        first_node(input, YamlRule::ListOfMaps, message, |node| {
            matches!(node.event.value, Event::MappingStart(..))
                && node.parents.last() == Some(&Container::Seq)
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::front_error;

    // AC: CE-09.3
    #[test]
    fn lists_of_maps_are_rejected() {
        assert_eq!(
            front_error("g:\n  - a: 1\n"),
            Some(("list-of-maps", "3:5".to_owned()))
        );
    }
}
