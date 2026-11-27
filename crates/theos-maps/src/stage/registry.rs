//! The stages a run can use.

use super::Stage;

/// The stages recipes can name, in registration order. Ids are unique: the built-in list is
/// checked by a test, and [`Registry::get`] finds the first stage of an id.
#[derive(Default)]
pub struct Registry {
    stages: Vec<Box<dyn Stage>>,
}

impl Registry {
    /// Every built-in stage, as [`crate::stages::register`] lists them.
    #[must_use]
    pub fn builtin() -> Self {
        let mut registry = Self::empty();
        crate::stages::register(&mut registry);
        registry
    }

    /// A registry without stages.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Adds `stage` after the others.
    pub fn add(&mut self, stage: Box<dyn Stage>) {
        self.stages.push(stage);
    }

    /// The stage `id`.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&dyn Stage> {
        self.stages
            .iter()
            .find(|stage| stage.id() == id)
            .map(AsRef::as_ref)
    }

    /// The ids, in registration order.
    pub fn ids(&self) -> impl Iterator<Item = &'static str> {
        self.stages.iter().map(|stage| stage.id())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::Registry;
    use crate::stage::Stage;
    use crate::stage::test_support::{Probe, Step};

    fn is_snake_case(id: &str) -> bool {
        let mut chars = id.chars();
        chars.next().is_some_and(|c| c.is_ascii_lowercase())
            && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    }

    #[test]
    fn the_builtin_registry_has_unique_snake_case_ids() {
        let registry = Registry::builtin();
        let ids: Vec<&str> = registry.ids().collect();
        for id in &ids {
            assert!(is_snake_case(id), "stage id {id:?} is not snake case");
        }
        let unique: BTreeSet<&str> = ids.iter().copied().collect();
        assert_eq!(unique.len(), ids.len(), "duplicate stage ids in {ids:?}");
    }

    #[test]
    fn the_snake_case_check_refuses_other_ids() {
        assert!(is_snake_case("landscape_evolution") && is_snake_case("a1"));
        assert!(!is_snake_case("") && !is_snake_case("1a") && !is_snake_case("Grid"));
        assert!(!is_snake_case("_a") && !is_snake_case("a-b"));
    }

    #[test]
    fn stages_are_found_by_id_in_registration_order() {
        let mut registry = Registry::empty();
        assert_eq!(registry.ids().count(), 0);
        registry.add(Box::new(Step));
        registry.add(Box::new(Probe));
        assert_eq!(registry.ids().collect::<Vec<_>>(), ["step", "probe"]);
        assert_eq!(registry.get("probe").map(Stage::id), Some("probe"));
        assert!(registry.get("nope").is_none());
    }
}
