//! The nodes of the document with their place in the nesting, for the shape and key rules.

use yaml_rust2::Event;

use crate::yaml::Input;
use crate::yaml::input::Marked;

/// The kind of a container that holds a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Container {
    /// A block or flow map.
    Map,
    /// A block or flow list.
    Seq,
}

/// A scalar, alias or container start of the document.
pub(super) struct Node<'a> {
    /// The event of the node.
    pub event: &'a Marked<Event>,
    /// Where to report the node: its event, or the first key for a block map, whose start
    /// event sits at the first `:`.
    pub at: &'a Marked<Event>,
    /// The containers around the node, outermost first.
    pub parents: Vec<Container>,
    /// The index (in `Input::events`) of the map the node is a key of, if it is one.
    pub key_of: Option<usize>,
}

struct Frame {
    kind: Container,
    event_index: usize,
    children: usize,
}

/// Every scalar, alias and container start of `input`, in document order.
pub(super) fn nodes<'a>(input: &'a Input<'_>) -> Vec<Node<'a>> {
    let mut stack: Vec<Frame> = Vec::new();
    let mut nodes = Vec::new();
    for (event_index, event) in input.events.iter().enumerate() {
        let container = match event.value {
            Event::MappingStart(..) => Some(Container::Map),
            Event::SequenceStart(..) => Some(Container::Seq),
            Event::MappingEnd | Event::SequenceEnd => {
                stack.pop();
                continue;
            }
            Event::Scalar(..) | Event::Alias(_) => None,
            _ => continue,
        };
        let key_of = stack.last_mut().and_then(|frame| {
            frame.children += 1;
            (frame.kind == Container::Map && frame.children % 2 == 1).then_some(frame.event_index)
        });
        let at = match event.value {
            Event::MappingStart(..) => input.events.get(event_index + 1).unwrap_or(event),
            _ => event,
        };
        nodes.push(Node {
            event,
            at,
            parents: stack.iter().map(|frame| frame.kind).collect(),
            key_of,
        });
        if let Some(kind) = container {
            stack.push(Frame {
                kind,
                event_index,
                children: 0,
            });
        }
    }
    nodes
}

/// The first node that `is_forbidden` accepts, as an error of `rule`.
pub(super) fn first_node(
    input: &Input<'_>,
    rule: crate::yaml::YamlRule,
    message: &str,
    is_forbidden: impl Fn(&Node<'_>) -> bool,
) -> Option<crate::yaml::YamlError> {
    nodes(input)
        .iter()
        .find(|node| is_forbidden(node))
        .map(|node| input.error(rule, node.at, message))
}
