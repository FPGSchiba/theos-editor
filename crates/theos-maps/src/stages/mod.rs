//! The built-in stages. A new stage is one module here plus one line in [`register`].

use crate::stage::Registry;

pub mod terrain;

/// Adds every built-in stage to `registry`, one line per stage.
pub fn register(registry: &mut Registry) {
    registry.add(Box::new(terrain::grid::GridStage));
    registry.add(Box::new(terrain::tectonics::Tectonics));
}

#[cfg(test)]
mod tests {
    use crate::stage::Registry;

    /// The smallest parameter table that parses, for stages with required parameters.
    fn minimal_params(_id: &str) -> toml::Table {
        // Every stage so far has defaults for all parameters; a stage with required ones
        // adds a `match _id` here.
        let text = "";
        toml::from_str(text).unwrap()
    }

    fn is_metric_name(name: &str) -> bool {
        let mut chars = name.chars();
        chars.next().is_some_and(|c| c.is_ascii_lowercase())
            && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    }

    #[test]
    fn every_metric_name_of_every_stage_is_snake_case() {
        let registry = Registry::builtin();
        for id in registry.ids() {
            let stage = registry.get(id).unwrap();
            let params = stage.params(&minimal_params(id)).unwrap();
            for name in stage.metric_names(params.as_ref()) {
                assert!(is_metric_name(name), "stage {id}: metric {name:?}");
            }
        }
        assert!(!is_metric_name("") && !is_metric_name("Area") && !is_metric_name("a-b"));
    }
}
