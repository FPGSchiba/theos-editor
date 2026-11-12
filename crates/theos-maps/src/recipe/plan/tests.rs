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
        "renders = [\"omega\"]\n[constraints]\n\"m.p\" = [1.0, 2.0]\n[render.base]\nk = 1\n",
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
    assert_eq!(planned.constraints["m.p"], [1.0, 2.0]);
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

#[test]
fn a_recipe_input_is_unknown() {
    let err = refusal("[inputs]\nseed_hint = 3\n", &["a"]);
    let (_, _, parameter, message) = fields(&err);
    assert_eq!(parameter, Some("seed_hint"));
    assert_eq!(message, "unknown recipe input `seed_hint`");
}

#[test]
fn a_request_input_is_unknown() {
    let mut inputs = no_inputs();
    inputs.insert("tint".to_owned(), serde_json::json!(1));
    let err = plan(&recipe("", &["a"]), &registry(), &inputs).unwrap_err();
    assert_eq!(fields(&err).2, Some("tint"));
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
    ] {
        let loaded = parse(&text("", &[]).replace(from, to), RecipeOrigin::Builtin).unwrap();
        let err = plan(&loaded, &registry(), &no_inputs()).unwrap_err();
        assert!(fields(&err).3.contains(word), "{err}");
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
