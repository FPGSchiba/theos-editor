//! Rule `top-level-map`: the front matter is a scalar or a list. Position: the first node.

use yaml_rust2::Event;

use super::Rule;
use super::nodes::nodes;
use crate::yaml::{Input, YamlError, YamlRule};

pub(super) struct TopLevelMap;

impl Rule for TopLevelMap {
    fn check(&self, input: &Input<'_>) -> Option<YamlError> {
        let first = nodes(input).into_iter().next()?;
        if matches!(first.event.value, Event::MappingStart(..)) {
            return None;
        }
        Some(input.error(
            YamlRule::TopLevelMap,
            first.at,
            "the front matter must be a map",
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::front_error;

    // AC: CE-09.3
    #[test]
    fn the_front_matter_is_a_map() {
        let want = Some(("top-level-map", "2:1".to_owned()));
        assert_eq!(front_error("just text\n"), want);
        assert_eq!(front_error("- a\n"), want);
    }
}
