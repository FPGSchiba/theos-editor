use std::collections::BTreeSet;
use std::fmt::Write as _;

use super::test_support::{ScaleParams, test_evolution};
use super::{Evolution, EvolutionParams};
use crate::error::RunError;
use crate::progress::CancelToken;
use crate::recipe::{RecipeError, RecipeOrigin, parse, plan};
use crate::runner::{RunOptions, execute};
use crate::stage::{LayerSpec, ParamError, Registry, Stage, StageParams};
use crate::stages::terrain::grid::GridStage;

fn table(text: &str) -> toml::Table {
    toml::from_str(text).unwrap()
}

/// The parameters of `stage` for `toml`, parsed and validated.
fn params(stage: &Evolution, toml: &str) -> Result<Box<dyn StageParams>, ParamError> {
    Stage::params(stage, &table(toml))
}

/// The `[[process]]` tables of the ids, in order.
fn listing(ids: &[&str]) -> String {
    let mut text = String::new();
    for id in ids {
        let _ = writeln!(text, "[[process]]\nid = \"{id}\"");
    }
    text
}

fn evolution(params: &dyn StageParams) -> &EvolutionParams {
    params.as_any().downcast_ref().unwrap()
}

#[test]
fn evolution_reads_and_writes_follow_its_processes() {
    let stage = test_evolution();
    let p = params(&stage, &listing(&["uplift", "route", "erode"])).unwrap();
    assert_eq!(
        Stage::reads(&stage, p.as_ref()),
        [
            LayerSpec::mask("fixed"),
            LayerSpec::f64("uplift"),
            LayerSpec::f64("sag_rate"),
            LayerSpec::f64("kd"),
            LayerSpec::f64("omega").optional(),
        ],
        "height comes from the initial surface, receivers from `route`"
    );
    assert_eq!(
        Stage::writes(&stage, p.as_ref()),
        [
            LayerSpec::f64("height"),
            LayerSpec::f64("erosion"),
            LayerSpec::f64("deposition"),
            LayerSpec::u32("receivers"),
            LayerSpec::f64("drainage_area"),
        ]
    );
    let swapped = params(&stage, &listing(&["erode", "route"])).unwrap();
    assert!(
        Stage::reads(&stage, swapped.as_ref()).contains(&LayerSpec::u32("receivers")),
        "a read no earlier process writes is the stage's read"
    );
    let none = params(&stage, "").unwrap();
    assert_eq!(
        Stage::reads(&stage, none.as_ref()),
        [LayerSpec::mask("fixed")]
    );
    assert_eq!(Stage::writes(&stage, none.as_ref()).len(), 3);
}

#[test]
fn each_layer_is_declared_once_and_a_required_read_wins() {
    let stage = test_evolution();
    let p = params(&stage, &listing(&["route", "erode", "tint", "uplift"])).unwrap();
    let reads = Stage::reads(&stage, p.as_ref());
    assert_eq!(
        reads,
        [
            LayerSpec::mask("fixed"),
            LayerSpec::f64("kd"),
            LayerSpec::f64("omega"),
            LayerSpec::f64("uplift"),
            LayerSpec::f64("sag_rate"),
        ],
        "`fixed` once; `omega` required because `tint` needs it"
    );
    let p = params(&stage, &listing(&["route", "route"])).unwrap();
    let writes = Stage::writes(&stage, p.as_ref());
    assert_eq!(writes.iter().filter(|w| w.name == "receivers").count(), 1);
}

#[test]
fn the_builtin_evolution_with_uplift_declares_its_layers() {
    let stage = Evolution::new();
    let p = params(&stage, &listing(&["uplift"])).unwrap();
    assert_eq!(
        Stage::reads(&stage, p.as_ref()),
        [
            LayerSpec::mask("fixed"),
            LayerSpec::f64("uplift"),
            LayerSpec::f64("sag_rate"),
        ]
    );
    assert_eq!(
        Stage::writes(&stage, p.as_ref()),
        [
            LayerSpec::f64("height"),
            LayerSpec::f64("erosion"),
            LayerSpec::f64("deposition"),
        ]
    );
    assert_eq!(Stage::id(&stage), "landscape_evolution");
    assert!(Evolution::default().processes().get("uplift").is_some());
}

#[test]
fn a_process_before_its_input_fails_naming_the_process() {
    let stage = test_evolution();
    let available = BTreeSet::from(["fixed", "kd", "uplift", "sag_rate"]);
    let good = params(&stage, &listing(&["route", "erode"])).unwrap();
    assert_eq!(
        Stage::check_order(&stage, good.as_ref(), &available),
        Ok(())
    );
    let bad = params(&stage, &listing(&["uplift", "erode", "route"])).unwrap();
    let err = Stage::check_order(&stage, bad.as_ref(), &available).unwrap_err();
    assert_eq!(err.stage, "landscape_evolution");
    assert_eq!(err.parameter.as_deref(), Some("process[1]"));
    assert!(
        err.message.contains("`erode`") && err.message.contains("`receivers`"),
        "{err}"
    );
    let missing = BTreeSet::from(["fixed", "uplift", "sag_rate"]);
    let err = Stage::check_order(&stage, good.as_ref(), &missing).unwrap_err();
    assert!(err.message.contains("`kd`"), "{err}");
    let misread = params(&stage, &listing(&["misread"])).unwrap();
    let err = Stage::check_order(&stage, misread.as_ref(), &available).unwrap_err();
    assert_eq!(err.parameter.as_deref(), Some("process[0]"));
    assert!(
        err.message.contains("`height`") && err.message.contains("a u8 raster"),
        "{err}"
    );
}

#[test]
fn processes_declaring_a_layer_as_different_kinds_are_refused() {
    let stage = test_evolution();
    let available = BTreeSet::from(["fixed", "kd", "receivers"]);
    for (ids, at, words) in [
        (
            ["route", "floaty"],
            "process[1]",
            [
                "process `floaty` reads `receivers` as an f64 raster",
                "process `route` (process[0]) writes it as a u32 raster",
            ],
        ),
        (
            ["floaty", "route"],
            "process[1]",
            [
                "process `route` writes `receivers` as a u32 raster",
                "process `floaty` (process[0]) reads it as an f64 raster",
            ],
        ),
        (
            ["route", "rewrite"],
            "process[1]",
            [
                "process `rewrite` writes `receivers` as an f64 raster",
                "process `route` (process[0]) writes it as a u32 raster",
            ],
        ),
        (
            ["erode", "floaty"],
            "process[1]",
            [
                "process `floaty` reads `receivers` as an f64 raster",
                "process `erode` (process[0]) reads it as a u32 raster",
            ],
        ),
        (
            ["misread", "route"],
            "process[0]",
            [
                "process `misread` reads `height` as a u8 raster",
                "the initial surface writes it as an f64 raster",
            ],
        ),
    ] {
        let p = params(&stage, &listing(&ids)).unwrap();
        let err = Stage::check_order(&stage, p.as_ref(), &available).unwrap_err();
        assert_eq!(err.stage, "landscape_evolution");
        assert_eq!(err.parameter.as_deref(), Some(at), "{ids:?}: {err}");
        for word in words {
            assert!(err.message.contains(word), "{ids:?}: {err}");
        }
    }
}

#[test]
fn the_plan_refuses_a_process_before_its_input() {
    let mut registry = Registry::empty();
    registry.add(Box::new(GridStage));
    registry.add(Box::new(test_evolution()));
    let text = format!(
        "id = \"t\"\ntitle = \"T\"\nkind = \"terrain\"\ngrid = [8, 8]\nsize_km = [1.0, 1.0]\n\
         [[stage]]\nid = \"grid\"\n[[stage]]\nid = \"landscape_evolution\"\n{}",
        listing(&["erode", "route"]).replace("[[process]]", "[[stage.process]]")
    );
    let recipe = parse(&text, RecipeOrigin::Builtin).unwrap();
    let err = plan(
        &recipe,
        &registry,
        &serde_json::Map::new(),
        crate::recipe::Planning::Standalone,
    )
    .unwrap_err();
    match err {
        RecipeError::Invalid {
            stage,
            parameter,
            message,
            ..
        } => {
            assert_eq!(stage.as_deref(), Some("landscape_evolution"));
            assert_eq!(parameter.as_deref(), Some("process[0]"));
            assert!(message.contains("`erode`") && message.contains("`receivers`"));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn processes_parse_their_parameters_and_unknown_ones_fail() {
    let stage = test_evolution();
    let p = params(
        &stage,
        "[[process]]\nid = \"scale\"\nfactor = 2.5\nevery = 3\n[[process]]\nid = \"scale\"\n",
    )
    .unwrap();
    let planned = &evolution(p.as_ref()).planned;
    assert_eq!(planned.len(), 2);
    assert_eq!(
        (planned[0].id, planned[0].key.as_str(), planned[0].every),
        ("scale", "scale#0", Some(3))
    );
    assert_eq!(
        (planned[1].key.as_str(), planned[1].every),
        ("scale#1", None)
    );
    let factor = |k: usize| {
        let scale: &ScaleParams = planned[k].params.as_any().downcast_ref().unwrap();
        scale.factor
    };
    assert!((factor(0) - 2.5).abs() < 1e-12 && (factor(1) - 1.0).abs() < 1e-12);

    for (toml, parameter, words) in [
        (
            "[[process]]\nid = \"nope\"",
            "process[0]",
            "unknown process `nope`",
        ),
        ("[[process]]\nfactor = 2.0", "process[0]", "`id`"),
        ("[[process]]\nid = 7", "process[0]", "`id`"),
        (
            "[[process]]\nid = \"uplift\"\n[[process]]\nid = \"uplift\"\nbogus = 1",
            "process[1].bogus",
            "process `uplift`: unknown parameter",
        ),
        (
            "[[process]]\nid = \"scale\"\nfactor = \"x\"",
            "process[0].factor",
            "process `scale`: invalid type",
        ),
        (
            "[[process]]\nid = \"scale\"\nevery = 0",
            "process[0].every",
            "at least 1",
        ),
        (
            "[[process]]\nid = \"scale\"\nevery = -2",
            "process[0].every",
            "at least 1",
        ),
        (
            "[[process]]\nid = \"scale\"\nevery = 1.5",
            "process[0].every",
            "at least 1",
        ),
    ] {
        let err = params(&stage, toml).unwrap_err();
        assert_eq!(err.stage, "landscape_evolution", "{toml}");
        assert_eq!(err.parameter.as_deref(), Some(parameter), "{toml}: {err}");
        assert!(err.message.contains(words), "{toml}: {err}");
    }
    let err = params(&stage, "[[process]]\nid = \"nope\"").unwrap_err();
    assert!(err.message.contains("known: uplift, log"), "{err}");
    let err = Stage::params(&Evolution::new(), &table("planned = 1")).unwrap_err();
    assert_eq!(err.parameter.as_deref(), Some("planned"), "{err}");
}

#[test]
fn bad_loop_values_are_refused() {
    let stage = test_evolution();
    for (toml, parameter) in [
        ("steps = 0", "steps"),
        ("dt_yr = 0.0", "dt_yr"),
        ("dt_yr = nan", "dt_yr"),
        ("check_every = 0", "check_every"),
        ("initial_relief_m = -1.0", "initial_relief_m"),
        ("initial_relief_m = inf", "initial_relief_m"),
    ] {
        let err = params(&stage, toml).unwrap_err();
        assert_eq!(err.stage, "landscape_evolution");
        assert_eq!(err.parameter.as_deref(), Some(parameter), "{toml}: {err}");
    }
    let p = params(&stage, "").unwrap();
    let p = evolution(p.as_ref());
    assert_eq!((p.steps, p.check_every), (240, 10));
    assert!((p.dt_yr - 2.5e4).abs() < 1e-9 && (p.initial_relief_m - 40.0).abs() < 1e-12);
    assert!(p.process.is_empty() && p.planned.is_empty());
}

#[test]
fn a_panicking_process_fails_the_run_as_a_stage_panic() {
    let mut registry = Registry::empty();
    registry.add(Box::new(GridStage));
    registry.add(Box::new(test_evolution()));
    let text = "id = \"t\"\ntitle = \"T\"\nkind = \"terrain\"\ngrid = [8, 8]\n\
                size_km = [1.0, 1.0]\n[[stage]]\nid = \"grid\"\n\
                [[stage]]\nid = \"landscape_evolution\"\nsteps = 3\n\
                [[stage.process]]\nid = \"boom\"\n";
    let recipe = parse(text, RecipeOrigin::Builtin).unwrap();
    let plan = plan(
        &recipe,
        &registry,
        &serde_json::Map::new(),
        crate::recipe::Planning::Standalone,
    )
    .unwrap();
    let result = execute(
        &plan,
        1,
        &registry,
        &RunOptions::default(),
        None,
        &mut |_| {},
        &CancelToken::new(),
    );
    match result {
        Err(RunError::StagePanicked { stage, message }) => {
            assert_eq!(stage, "landscape_evolution");
            assert!(message.contains("boom in a process"), "{message}");
        }
        other => panic!("{:?}", other.map(|_| ())),
    }
}

/// The 384 x 384 budget fixture (Task 27) plans with the builtin registry; it only plans.
#[test]
fn evolution_384_plans() {
    let text = include_str!("../../../tests/fixtures/recipes/evolution-384.toml");
    let loaded = parse(text, RecipeOrigin::Builtin).unwrap();
    let planned = plan(
        &loaded,
        &Registry::builtin(),
        &serde_json::Map::new(),
        crate::recipe::Planning::Standalone,
    )
    .unwrap();
    assert_eq!(planned.frame.rows, 384);
    assert_eq!(planned.frame.cols, 384);
    let ids: Vec<&str> = planned.stages.iter().map(|s| s.id).collect();
    assert_eq!(
        ids,
        [
            "grid",
            "tectonics",
            "geology",
            "climate_model",
            "landscape_evolution"
        ]
    );
}
