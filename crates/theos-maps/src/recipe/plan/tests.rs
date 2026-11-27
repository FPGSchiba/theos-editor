use super::*;
use crate::recipe::RecipeError;
use crate::recipe::test_stages::{recipe, registry, text};
use crate::recipe::{RecipeOrigin, parse};

fn no_inputs() -> serde_json::Map<String, serde_json::Value> {
    serde_json::Map::new()
}

fn refusal(extra: &str, stages: &[&str]) -> RecipeError {
    plan(&recipe(extra, stages), &registry(), &no_inputs()).unwrap_err()
}

fn fields(err: &RecipeError) -> (Option<&str>, Option<&str>, Option<&str>, &str) {
    match err {
        RecipeError::Invalid {
            stage,
            layer,
            parameter,
            message,
            ..
        } => (
            stage.as_deref(),
            layer.as_deref(),
            parameter.as_deref(),
            message,
        ),
        other => panic!("expected Invalid, got {other:?}"),
    }
}

#[test]
fn a_valid_recipe_plans_frame_stages_and_keys() {
    let loaded = recipe(
        "renders = [\"omega\"]\n[constraints]\nlake_pct = [1.0, 2.0]\n[render.base]\nk = 1\n",
        &["a", "b", "a", "c"],
    );
    let planned = plan(&loaded, &registry(), &no_inputs()).unwrap();
    assert_eq!(planned.recipe_id, "t");
    assert_eq!((planned.frame.rows, planned.frame.cols), (4, 8));
    assert!((planned.frame.cell_m - 125.0).abs() < 1e-9);
    let keys: Vec<&str> = planned.stages.iter().map(|s| s.key.as_str()).collect();
    assert_eq!(keys, ["a#0", "b#0", "a#1", "c#0"]);
    assert_eq!(
        planned.stages[1].reads,
        vec![crate::stage::LayerSpec::f64("x")]
    );
    assert_eq!(
        planned.stages[1].writes,
        vec![crate::stage::LayerSpec::f64("y")]
    );
    assert_eq!(planned.stages[1].metric_names.len(), 0);
    assert_eq!(planned.constraints["lake_pct"], [1.0, 2.0]);
    assert_eq!(planned.renders, ["omega"]);
    assert!(planned.render_options.contains_key("base"));
}

#[test]
fn a_stage_before_its_input_fails_naming_stage_and_layer() {
    let err = refusal("", &["b", "a"]);
    let (stage, layer, _, message) = fields(&err);
    assert_eq!((stage, layer), (Some("b"), Some("x")));
    assert_eq!(
        message,
        "stage `b` reads `x`, which no earlier stage writes"
    );
}

#[test]
fn an_optional_write_does_not_satisfy_a_required_read() {
    let err = refusal("", &["o", "b"]);
    let (stage, layer, _, _) = fields(&err);
    assert_eq!((stage, layer), (Some("b"), Some("x")));
}

#[test]
fn an_optional_read_may_be_missing() {
    assert!(plan(&recipe("", &["c"]), &registry(), &no_inputs()).is_ok());
}

#[test]
fn an_unknown_stage_fails_naming_it() {
    let err = refusal("", &["a", "nope"]);
    let (stage, _, _, message) = fields(&err);
    assert_eq!(stage, Some("nope"));
    assert!(message.contains("unknown stage"), "{message}");
}

#[test]
fn a_stage_without_an_id_fails() {
    let loaded = parse(
        &format!("{}\n[[stage]]\nn = 1\n", text("", &[])),
        RecipeOrigin::Builtin,
    )
    .unwrap();
    let err = plan(&loaded, &registry(), &no_inputs()).unwrap_err();
    assert!(fields(&err).3.contains("no `id`"), "{err}");
}

#[test]
fn an_unknown_parameter_fails_naming_it() {
    let loaded = parse(
        &format!("{}\n[[stage]]\nid = \"a\"\nbogus = 1\n", text("", &[])),
        RecipeOrigin::Builtin,
    )
    .unwrap();
    let err = plan(&loaded, &registry(), &no_inputs()).unwrap_err();
    let (stage, _, parameter, message) = fields(&err);
    assert_eq!((stage, parameter), (Some("a"), Some("bogus")));
    assert!(message.contains("unknown parameter"), "{message}");
}

#[test]
fn check_order_failures_name_the_stage_and_step() {
    let err = refusal("", &["ord"]);
    let (stage, _, parameter, message) = fields(&err);
    assert_eq!((stage, parameter), (Some("ord"), Some("process[0]")));
    assert!(message.contains("needs `x`"), "{message}");
    assert!(plan(&recipe("", &["a", "ord"]), &registry(), &no_inputs()).is_ok());
}

#[test]
fn a_stage_of_another_kind_is_refused() {
    use crate::stage::{LayerSpec, MapKind, StageContext, StageError, StageReport, TypedStage};
    // A stage listing no kind applies to no map.
    struct Never;
    impl TypedStage for Never {
        type Params = crate::recipe::test_stages::Knob;
        const ID: &'static str = "never";
        const SUMMARY: &'static str = "Applies to nothing.";
        const KINDS: &'static [MapKind] = &[];
        fn reads(&self, _: &Self::Params) -> Vec<LayerSpec> {
            vec![]
        }
        fn writes(&self, _: &Self::Params) -> Vec<LayerSpec> {
            vec![]
        }
        fn run(
            &self,
            _: &mut StageContext<'_>,
            _: &Self::Params,
        ) -> Result<StageReport, StageError> {
            Ok(StageReport::default())
        }
    }
    let mut registry = registry();
    registry.add(Box::new(Never));
    let err = plan(&recipe("", &["never"]), &registry, &no_inputs()).unwrap_err();
    assert!(fields(&err).3.contains("does not apply"), "{err}");
}

// AC: CE-28.3
#[test]
fn an_undeclared_input_is_refused_before_the_run() {
    let mut inputs = no_inputs();
    inputs.insert("floor".to_owned(), serde_json::json!(2));
    let err = plan(&recipe("", &["a"]), &registry(), &inputs).unwrap_err();
    let (_, _, parameter, message) = fields(&err);
    assert_eq!(parameter, Some("floor"));
    assert_eq!(message, "unknown recipe input `floor`");
}

// AC: CE-28.3
#[test]
fn an_input_of_the_wrong_type_is_refused_before_the_run() {
    let loaded = recipe(
        "[inputs]
floor = { type = \"number\", min = 1, max = 10, integer = true }
",
        &["a"],
    );
    for bad in [
        serde_json::json!("two"),
        serde_json::json!(11),
        serde_json::json!(2.5),
    ] {
        let mut inputs = no_inputs();
        inputs.insert("floor".to_owned(), bad.clone());
        let err = plan(&loaded, &registry(), &inputs).unwrap_err();
        assert_eq!(fields(&err).2, Some("floor"), "{bad}");
    }
    let mut inputs = no_inputs();
    inputs.insert("floor".to_owned(), serde_json::json!(3));
    assert!(plan(&loaded, &registry(), &inputs).is_ok());
}

fn parse_refusal(text: &str) -> String {
    parse(text, RecipeOrigin::Builtin).unwrap_err().to_string()
}

#[test]
fn two_feature_inputs_are_refused() {
    let extra = "[inputs]
a = { type = \"feature\", parent_types = [\"town\"] }
                 b = { type = \"feature\", parent_types = [\"town\"] }
";
    assert!(parse_refusal(&text(extra, &[])).contains("only one `feature` input"));
}

#[test]
fn cameras_inputs_are_refused_as_unsupported() {
    let extra = "[inputs]
cams = { type = \"cameras\" }
";
    assert!(parse_refusal(&text(extra, &[])).contains("unsupported"));
}

fn settlement_text(frame: &str) -> String {
    format!(
        "id = \"s\"
title = \"S\"
kind = \"settlement\"
{frame}"
    )
}

#[test]
fn a_settlement_recipe_with_grid_is_refused() {
    let loaded = parse(
        &settlement_text(
            "grid = [4, 8]
",
        ),
        RecipeOrigin::Builtin,
    )
    .unwrap();
    let err = plan(&loaded, &registry(), &no_inputs()).unwrap_err();
    assert_eq!(fields(&err).2, Some("grid"));
    let loaded = parse(
        &settlement_text(
            "size_km = [1.0, 1.0]
",
        ),
        RecipeOrigin::Builtin,
    )
    .unwrap();
    let err = plan(&loaded, &registry(), &no_inputs()).unwrap_err();
    assert_eq!(fields(&err).2, Some("size_km"));
}

#[test]
fn a_terrain_recipe_without_grid_is_refused() {
    let without_grid = "id = \"t\"
title = \"T\"
kind = \"terrain\"
size_km = [1.0, 0.5]
";
    let loaded = parse(without_grid, RecipeOrigin::Builtin).unwrap();
    let err = plan(&loaded, &registry(), &no_inputs()).unwrap_err();
    assert_eq!(fields(&err).2, Some("grid"));
    let without_size = "id = \"t\"
title = \"T\"
kind = \"terrain\"
grid = [4, 8]
";
    let loaded = parse(without_size, RecipeOrigin::Builtin).unwrap();
    let err = plan(&loaded, &registry(), &no_inputs()).unwrap_err();
    assert_eq!(fields(&err).2, Some("size_km"));
}

#[test]
fn non_square_cells_are_refused() {
    let loaded = parse(
        &text("", &[]).replace("size_km = [1.0, 0.5]", "size_km = [1.0, 0.6]"),
        RecipeOrigin::Builtin,
    )
    .unwrap();
    let err = plan(&loaded, &registry(), &no_inputs()).unwrap_err();
    assert!(fields(&err).3.contains("square"), "{err}");
}

#[test]
fn degenerate_frames_are_refused() {
    for (from, to, word) in [
        ("grid = [4, 8]", "grid = [1, 8]", "at least 2"),
        ("grid = [4, 8]", "grid = [4, 1]", "at least 2"),
        ("size_km = [1.0, 0.5]", "size_km = [0.0, 0.0]", "positive"),
        ("size_km = [1.0, 0.5]", "size_km = [-1.0, 0.5]", "positive"),
        (
            "size_km = [1.0, 0.5]",
            "size_km = [inf, 0.5]",
            "positive and finite",
        ),
        // finite in km, but the cell edge in metres is not
        (
            "size_km = [1.0, 0.5]",
            "size_km = [1e306, 5e305]",
            "positive and finite",
        ),
    ] {
        let loaded = parse(&text("", &[]).replace(from, to), RecipeOrigin::Builtin).unwrap();
        let err = plan(&loaded, &registry(), &no_inputs()).unwrap_err();
        assert!(fields(&err).3.contains(word), "{err}");
    }
}

// Final review round 3, finding 1: `rows * cols` overflowed in `MapFrame::cells` (a panic in a
// debug build, 0 cells in a release build for [2^32, 2^32]) after `plan` had accepted the grid.
#[test]
fn a_grid_whose_cell_count_overflows_is_refused() {
    for (grid, size) in [
        ("[3000000000, 7000000000]", "[7000000.0, 3000000.0]"),
        ("[4294967296, 4294967296]", "[4294967296.0, 4294967296.0]"),
    ] {
        let edited = text("", &[])
            .replace("grid = [4, 8]", &format!("grid = {grid}"))
            .replace("size_km = [1.0, 0.5]", &format!("size_km = {size}"));
        let loaded = parse(&edited, RecipeOrigin::Builtin).unwrap();
        let err = plan(&loaded, &registry(), &no_inputs()).unwrap_err();
        let (stage, layer, parameter, message) = fields(&err);
        assert_eq!(
            (stage, layer, parameter),
            (None, None, Some("grid")),
            "{err}"
        );
        assert!(message.contains("cell count"), "{message}");
    }
}

// Final review round 3, class B: a cell edge of 0 (the size underflows over 2048 columns, and
// the square check compares 0 with 0), of nanometres or of 1e160 m planned.
#[test]
fn a_cell_edge_out_of_range_is_refused() {
    for (grid, size) in [
        ("[2048, 2048]", "[5e-324, 5e-324]"),
        ("[4, 8]", "[8e-9, 4e-9]"),
        ("[4, 8]", "[8e160, 4e160]"),
        ("[4, 8]", "[0.00007, 0.000035]"),
        ("[4, 8]", "[808.0, 404.0]"),
    ] {
        let edited = text("", &[])
            .replace("grid = [4, 8]", &format!("grid = {grid}"))
            .replace("size_km = [1.0, 0.5]", &format!("size_km = {size}"));
        let loaded = parse(&edited, RecipeOrigin::Builtin).unwrap();
        let err = plan(&loaded, &registry(), &no_inputs()).unwrap_err();
        let (_, _, parameter, message) = fields(&err);
        assert_eq!(parameter, Some("size_km"), "{err}");
        assert!(
            message.contains("the edge must be from 0.01 m to 100000 m"),
            "{message}"
        );
    }
    for size in ["[0.0000801, 0.00004005]", "[800.0, 400.0]"] {
        let edited = text("", &[]).replace("size_km = [1.0, 0.5]", &format!("size_km = {size}"));
        let loaded = parse(&edited, RecipeOrigin::Builtin).unwrap();
        let planned = plan(&loaded, &registry(), &no_inputs()).unwrap();
        assert!((0.01..=100_000.0).contains(&planned.frame.cell_m), "{size}");
    }
}

#[test]
fn an_unknown_extra_render_is_refused() {
    let err = refusal("renders = [\"sparkle\"]\n", &["a"]);
    let (_, _, parameter, message) = fields(&err);
    assert_eq!(parameter, Some("renders"));
    assert!(message.contains("sparkle"), "{message}");
}

#[test]
fn render_options_may_name_a_standard_or_extra_render() {
    for id in ["inworld", "omega"] {
        let extra = format!(
            "[render.{id}]
k = 1
"
        );
        assert!(plan(&recipe(&extra, &["a"]), &registry(), &no_inputs()).is_ok());
    }
}

#[test]
fn render_options_of_an_unknown_render_are_refused() {
    let err = refusal("[render.sparkle]\nk = 1\n", &["a"]);
    assert_eq!(fields(&err).2, Some("render.sparkle"));
}

#[test]
fn errors_display_what_they_name() {
    let err = refusal("", &["b"]);
    assert_eq!(
        err.to_string(),
        "recipe `t`, stage `b`, layer `x`: stage `b` reads `x`, which no earlier stage writes"
    );
}

#[test]
fn a_read_of_another_layer_kind_fails_naming_both_kinds() {
    let err = refusal("", &["u", "b"]);
    let (stage, layer, _, message) = fields(&err);
    assert_eq!((stage, layer), (Some("b"), Some("x")));
    assert!(
        message.contains("an f64 raster") && message.contains("a u8 raster"),
        "{message}"
    );
}

#[test]
fn the_later_write_decides_the_kind() {
    assert!(plan(&recipe("", &["u", "a", "b"]), &registry(), &no_inputs()).is_ok());
    let err = refusal("", &["a", "u", "b"]);
    assert_eq!(fields(&err).0, Some("b"));
}

#[test]
fn recipe_ids_are_folder_safe() {
    for bad in ["", "-x", "A", "a b", "a_b", "a/b", "a.b"] {
        let loaded = parse(
            &text("", &[]).replace("id = \"t\"", &format!("id = \"{bad}\"")),
            RecipeOrigin::Builtin,
        )
        .unwrap();
        let err = plan(&loaded, &registry(), &no_inputs()).unwrap_err();
        assert_eq!(fields(&err).2, Some("id"), "{bad:?}");
    }
    for good in ["t", "0a", "a-b-1"] {
        let loaded = parse(
            &text("", &[]).replace("id = \"t\"", &format!("id = \"{good}\"")),
            RecipeOrigin::Builtin,
        )
        .unwrap();
        assert!(plan(&loaded, &registry(), &no_inputs()).is_ok(), "{good}");
    }
}

#[test]
fn constraint_bounds_must_be_finite_and_ordered() {
    for range in ["[2.0, 1.0]", "[nan, 1.0]", "[0.0, inf]"] {
        let extra = format!(
            "[constraints]
lake_pct = {range}
"
        );
        let err = refusal(&extra, &["a"]);
        assert_eq!(fields(&err).2, Some("lake_pct"), "{range}");
    }
}

#[test]
fn a_kind_is_printed_in_the_recipe_spelling() {
    assert_eq!(crate::stage::MapKind::Terrain.to_string(), "terrain");
}
