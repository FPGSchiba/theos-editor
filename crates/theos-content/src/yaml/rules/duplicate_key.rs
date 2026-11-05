//! Rule `duplicate-key`: a key twice in one map. Position: the second occurrence.

use std::collections::HashSet;

use yaml_rust2::Event;

use super::Rule;
use super::nodes::nodes;
use crate::yaml::{Input, YamlError, YamlRule};

pub(super) struct DuplicateKey;

impl Rule for DuplicateKey {
    fn check(&self, input: &Input<'_>) -> Option<YamlError> {
        let mut seen = HashSet::new();
        nodes(input).into_iter().find_map(|node| {
            let map = node.key_of?;
            let Event::Scalar(name, ..) = &node.event.value else {
                return None;
            };
            let first_time = seen.insert((map, name.as_str()));
            (!first_time)
                .then(|| input.error(YamlRule::DuplicateKey, node.at, "the key occurs twice"))
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::front_error;

    // AC: CE-09.3
    #[test]
    fn keys_are_unique_per_map() {
        assert_eq!(
            front_error("l: 1\nl: 2\n"),
            Some(("duplicate-key", "3:1".to_owned()))
        );
        assert_eq!(
            front_error("m:\n  a: 1\n  a: 2\n"),
            Some(("duplicate-key", "4:3".to_owned()))
        );
        assert_eq!(front_error("a: 1\nm:\n  a: 2\n"), None);
    }
}
