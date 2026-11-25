//! The built-in stages. A new stage is one module here plus one line in [`register`].

use crate::stage::Registry;

pub mod evolution;
pub mod terrain;
#[cfg(test)]
mod unknown_keys;

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

    // Final review round 4, finding 1: the number rule follows the parameter's type, not how the
    // recipe spells the number. A float written as a TOML integer takes the float rule (the
    // stages' own defaults, such as `sag_duration_yr = 6.0e6`, may be written without a point);
    // a whole-number parameter keeps the count bound.
    #[test]
    fn the_number_rule_follows_the_parameter_type() {
        let registry = Registry::builtin();
        for (id, text) in [
            ("tectonics", "sag_duration_yr = 6000000"),
            ("landscape_evolution", "dt_yr = 2000000"),
            (
                "hydrology",
                "river_q = 0.15\nbig_river_q = 2.0\nmin_lake_area_m2 = 2000000",
            ),
            ("hydrology", "river_q = 1\nbig_river_q = 1000000000000"),
        ] {
            let stage = registry.get(id).unwrap();
            let params = stage.params(&toml::from_str(text).unwrap());
            assert!(params.is_ok(), "{id}: {text}: {:?}", params.err());
        }
        for (id, text, path) in [
            ("geology", "provinces = 2000000", "provinces"),
            ("landscape_evolution", "steps = 2000000", "steps"),
            ("pois", "chimes = 2000000", "chimes"),
            ("settlements", "[counts]\nhamlet = 2000000", "counts.hamlet"),
            ("resources", "[counts]\nstone = 2000000", "counts.stone"),
            (
                "landscape_evolution",
                "[[process]]\nid = \"fluvial\"\ng = 1.0\nmax_iterations = 2000000",
                "process[0].max_iterations",
            ),
            (
                "landscape_evolution",
                "[[process]]\nid = \"uplift\"\nevery = 2000000",
                "process[0].every",
            ),
            (
                "tectonics",
                "sag_duration_yr = 2000000000000",
                "sag_duration_yr",
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

    // Final review round 4, class K: the manifest's `timing` holds the stages by id and the steps
    // after the pipeline by name, and the outputs hold the renders by name next to `features`
    // and `package`, so no stage takes a step's name and no render an output's.
    #[test]
    fn no_stage_or_render_takes_a_reserved_name() {
        let steps = ["constraints", "features", "renders", "package"];
        for id in Registry::builtin().ids() {
            assert!(!steps.contains(&id), "stage {id}");
        }
        for render in crate::render::RENDERS {
            assert!(
                !["features", "package"].contains(&render.name),
                "render {}",
                render.name
            );
        }
    }
}
