//! The built-in stages. A new stage is one module here plus one line in [`register`].

use crate::stage::Registry;

pub mod evolution;
pub mod terrain;

/// Adds every built-in stage to `registry`, one line per stage.
pub fn register(registry: &mut Registry) {
    registry.add(Box::new(terrain::grid::GridStage));
    registry.add(Box::new(terrain::tectonics::Tectonics));
    registry.add(Box::new(terrain::geology::Geology));
    registry.add(Box::new(terrain::climate::ClimateModel));
    registry.add(Box::new(evolution::Evolution::new()));
    registry.add(Box::new(terrain::hydrology::Hydrology));
    registry.add(Box::new(terrain::ecology::Ecology));
    registry.add(Box::new(terrain::elements::Elements));
    registry.add(Box::new(terrain::settlements::Settlements));
    registry.add(Box::new(terrain::pois::Pois));
    registry.add(Box::new(terrain::detuning::Detuning));
    registry.add(Box::new(terrain::resources::Resources));
    registry.add(Box::new(terrain::roads::Roads));
    registry.add(Box::new(terrain::sites::Sites));
    registry.add(Box::new(terrain::travel::Travel));
}

#[cfg(test)]
mod tests {
    use crate::stage::Registry;

    /// The smallest parameter table that parses, for stages with required parameters.
    fn minimal_params(id: &str) -> toml::Table {
        let text = match id {
            "hydrology" => "river_q = 0.15\nbig_river_q = 2.0",
            _ => "",
        };
        toml::from_str(text).unwrap()
    }

    fn is_metric_name(name: &str) -> bool {
        let mut chars = name.chars();
        chars.next().is_some_and(|c| c.is_ascii_lowercase())
            && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    }

    // Final review round 3, class B (and finding 8): a number no `validate` bounds is refused by
    // `stage::check_numbers` in every stage, naming its path, before anything runs. Each case was
    // accepted before: a province count that sizes an allocation of terabytes, rock weights whose
    // total is +inf, a Gaussian radius beyond `usize`, overflowing products and quotients.
    #[test]
    fn numbers_out_of_bounds_are_refused_by_every_stage() {
        let registry = Registry::builtin();
        for (id, text, path) in [
            ("geology", "provinces = 10000000", "provinces"),
            (
                "geology",
                "[rock_mix]\ngranite = 1e308\nshale = 1e308",
                "rock_mix.granite",
            ),
            ("geology", "plutons = [2000000, 2000000]", "plutons[0]"),
            ("elements", "wet_sigma_m = 1e300", "wet_sigma_m"),
            ("tectonics", "warp = 1e308", "warp"),
            ("climate_model", "orographic_mm = 1.7e308", "orographic_mm"),
            ("climate_model", "nm = 1e140", "nm"),
            ("roads", "ford_cost = 1e308", "ford_cost"),
            ("travel", "off_path_factor = 1e-308", "off_path_factor"),
            ("detuning", "drift_amplitude = 1e308", "drift_amplitude"),
            (
                "landscape_evolution",
                "initial_relief_m = 1e308",
                "initial_relief_m",
            ),
            ("landscape_evolution", "steps = 2000000", "steps"),
            (
                "hydrology",
                "river_q = 0.15\nbig_river_q = 2.0\nwidth_a = 1e308",
                "width_a",
            ),
        ] {
            let stage = registry.get(id).unwrap();
            let err = stage.params(&toml::from_str(text).unwrap()).unwrap_err();
            assert_eq!(
                (err.stage.as_str(), err.parameter.as_deref()),
                (id, Some(path)),
                "{text}: {err}"
            );
        }
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
