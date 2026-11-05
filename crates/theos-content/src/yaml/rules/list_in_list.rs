//! Rule `list-in-list`: `- - a` or `[a, [b]]`. Position: the inner `SequenceStart` event.

use yaml_rust2::Event;

use super::Rule;
use super::nodes::{Container, first_node};
use crate::yaml::{Input, YamlError, YamlRule};

pub(super) struct ListInList;

impl Rule for ListInList {
    fn check(&self, input: &Input<'_>) -> Option<YamlError> {
        let message = "lists inside lists are not allowed";
        first_node(input, YamlRule::ListInList, message, |node| {
            matches!(node.event.value, Event::SequenceStart(..))
                && node.parents.last() == Some(&Container::Seq)
        })
    }
}
