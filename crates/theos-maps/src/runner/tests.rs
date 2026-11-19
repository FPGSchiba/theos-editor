use std::fmt::Write as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::Deserialize;

use super::*;
use crate::grid::Grid;
use crate::recipe::{RecipeOrigin, parse, plan};
use crate::stage::{
    Finding, LayerSpec, MapKind, Registry, StageContext, StageError, StageReport, TypedStage,
};
use crate::world::Layer;

#[derive(Debug, Default, Deserialize)]
struct Knob {}

const KINDS: &[MapKind] = &[MapKind::Terrain];

/// What a test stage does in `run`.
#[derive(Clone, Copy)]
enum Act {
    Write(f64),
    Panic,
    PanicString,
    Fail,
    Cancel,
    Nothing,
    ReportError,
    Update,
}

/// One scripted stage per id; `layer` is the f64 layer it writes (none for `Nothing`).
struct Script {
    reads: Option<&'static str>,
    layer: Option<&'static str>,
    act: Act,
    finding: Option<Finding>,
    check_panics: bool,
    metrics_panic: bool,
    runs: Arc<AtomicUsize>,
}

fn script(layer: Option<&'static str>, act: Act) -> Script {
    Script {
        reads: None,
        layer,
        act,
        finding: None,
        check_panics: false,
        metrics_panic: false,
        runs: Arc::new(AtomicUsize::new(0)),
    }
}

/// Stages cannot take their id from a field through `TypedStage::ID`, so each test id is a
/// type with its own constant that delegates to a `Script`.
macro_rules! scripted {
    ($name:ident, $id:literal) => {
        struct $name(Script);
        impl TypedStage for $name {
            type Params = Knob;
            const ID: &'static str = $id;
            const SUMMARY: &'static str = "A scripted test stage.";
            const KINDS: &'static [MapKind] = KINDS;
            fn reads(&self, _: &Knob) -> Vec<LayerSpec> {
                self.0.reads.map(LayerSpec::f64).into_iter().collect()
            }
            fn writes(&self, _: &Knob) -> Vec<LayerSpec> {
                self.0.layer.map(LayerSpec::f64).into_iter().collect()
            }
            fn metric_names(&self, _: &Knob) -> Vec<&'static str> {
                vec!["cells"]
            }
            fn run(&self, ctx: &mut StageContext<'_>, _: &Knob) -> Result<StageReport, StageError> {
                run_script(&self.0, ctx)
            }
            fn checks(&self, _: &World, _: &Knob, _: &StageReport) -> Vec<Finding> {
                assert!(!self.0.check_panics, "checks blew up");
                self.0.finding.clone().into_iter().collect()
            }
            fn metrics(&self, world: &World, _: &Knob, _: &StageReport) -> Metrics {
                assert!(!self.0.metrics_panic, "metrics blew up");
                let cells = u32::try_from(world.frame().cells()).unwrap_or(0);
                Metrics(BTreeMap::from([("cells".to_owned(), f64::from(cells))]))
            }
        }
    };
}

use crate::stage::Metrics;
use crate::world::World;
use std::collections::BTreeMap;

scripted!(First, "first");
scripted!(Second, "second");
scripted!(Third, "third");
scripted!(Boom, "boom");
scripted!(Bad, "bad");
scripted!(Warn, "warn");
scripted!(Nan, "nan");
scripted!(Count, "count");
scripted!(Upd, "upd");

fn run_script(s: &Script, ctx: &mut StageContext<'_>) -> Result<StageReport, StageError> {
    s.runs.fetch_add(1, Ordering::SeqCst);
    match s.act {
        Act::Write(v) => {
            if let Some(name) = s.layer {
                let (rows, cols) = (ctx.frame().rows, ctx.frame().cols);
                ctx.write(name, Layer::F64(Grid::filled(rows, cols, v)))?;
            }
            Ok(StageReport::default())
        }
        Act::Panic => panic!("boom at cell 7"),
        Act::PanicString => std::panic::panic_any(String::from("owned boom")),
        Act::Fail => Err(ctx.fail("it broke")),
        Act::Cancel => Err(StageError::Cancelled),
        Act::Nothing => Ok(StageReport::default()),
        Act::ReportError => Ok(StageReport {
            findings: vec![Finding::error("found while running")],
            ..StageReport::default()
        }),
        Act::Update => {
            if let Some(name) = s.layer {
                let mut grid = ctx.take_f64(name)?;
                grid.as_mut_slice().iter_mut().for_each(|v| *v += 1.0);
                ctx.write(name, Layer::F64(grid))?;
            }
            Ok(StageReport::default())
        }
    }
}

fn registry(stages: Vec<Box<dyn crate::stage::Stage>>) -> Registry {
    let mut registry = Registry::empty();
    for stage in stages {
        registry.add(stage);
    }
    registry
}

fn recipe_text(grid: usize, stages: &[&str]) -> String {
    let mut text = format!(
        "id = \"t\"\ntitle = \"T\"\nkind = \"terrain\"\ngrid = [{grid}, {grid}]\nsize_km = [1.0, 1.0]\n"
    );
    for stage in stages {
        let _ = write!(text, "[[stage]]\nid = \"{stage}\"\n");
    }
    text
}

fn planned(registry: &Registry, grid: usize, stages: &[&str]) -> Plan {
    let loaded = parse(&recipe_text(grid, stages), RecipeOrigin::Builtin).unwrap();
    plan(&loaded, registry, &serde_json::Map::new()).unwrap()
}

fn go(
    plan: &Plan,
    registry: &Registry,
    options: &RunOptions,
    cancel: &CancelToken,
) -> (Result<Execution, RunError>, Vec<Progress>) {
    let mut events = Vec::new();
    let result = execute(
        plan,
        5,
        registry,
        options,
        &mut |p| events.push(p.clone()),
        cancel,
    );
    (result, events)
}

fn simple() -> (Registry, Plan) {
    let a = script(Some("x"), Act::Write(1.0));
    let mut b = script(Some("y"), Act::Write(2.0));
    b.reads = Some("x");
    let c = script(None, Act::Nothing);
    let reg = registry(vec![
        Box::new(First(a)),
        Box::new(Second(b)),
        Box::new(Third(c)),
    ]);
    let plan = planned(&reg, 64, &["first", "second", "third"]);
    (reg, plan)
}

#[test]
fn progress_reports_every_stage_in_order() {
    let (reg, plan) = simple();
    let (result, events) = go(&plan, &reg, &RunOptions::default(), &CancelToken::new());
    result.unwrap();
    let seen: Vec<(String, u32, u32)> = events
        .into_iter()
        .map(|p| (p.stage, p.done, p.total))
        .collect();
    assert_eq!(
        seen,
        [
            ("first".to_owned(), 0, 3),
            ("second".to_owned(), 1, 3),
            ("third".to_owned(), 2, 3)
        ]
    );
}

#[test]
fn cancel_between_stages_returns_cancelled() {
    let (reg, plan) = simple();
    let cancel = CancelToken::new();
    let mut events = Vec::new();
    let token = cancel.clone();
    let result = execute(
        &plan,
        5,
        &reg,
        &RunOptions::default(),
        &mut |p| {
            events.push(p.stage.clone());
            if p.done == 1 {
                token.cancel();
            }
        },
        &cancel,
    );
    assert!(matches!(result, Err(RunError::Cancelled)));
    assert_eq!(events, ["first", "second"]);
}

#[test]
fn a_stage_cancelling_itself_is_cancelled() {
    let reg = registry(vec![Box::new(First(script(None, Act::Cancel)))]);
    let plan = planned(&reg, 8, &["first"]);
    let (result, _) = go(&plan, &reg, &RunOptions::default(), &CancelToken::new());
    assert!(matches!(result, Err(RunError::Cancelled)));
}

#[test]
fn a_failing_stage_names_itself() {
    let reg = registry(vec![Box::new(First(script(None, Act::Fail)))]);
    let plan = planned(&reg, 8, &["first"]);
    let (result, _) = go(&plan, &reg, &RunOptions::default(), &CancelToken::new());
    match result {
        Err(RunError::StageFailed { stage, source }) => {
            assert_eq!(stage, "first");
            assert!(source.to_string().contains("it broke"));
        }
        other => panic!("{:?}", other.map(|_| ())),
    }
}

#[test]
fn a_stage_that_misses_its_write_is_a_failure() {
    let reg = registry(vec![Box::new(First(script(Some("x"), Act::Nothing)))]);
    let plan = planned(&reg, 8, &["first"]);
    let (result, _) = go(&plan, &reg, &RunOptions::default(), &CancelToken::new());
    assert!(matches!(
        result,
        Err(RunError::StageFailed {
            source: StageError::NotWritten { .. },
            ..
        })
    ));
}

#[test]
fn a_panicking_stage_fails_the_run_with_its_message() {
    let reg = registry(vec![Box::new(Boom(script(None, Act::Panic)))]);
    let plan = planned(&reg, 8, &["boom"]);
    let (result, _) = go(&plan, &reg, &RunOptions::default(), &CancelToken::new());
    match result {
        Err(RunError::StagePanicked { stage, message }) => {
            assert_eq!(stage, "boom");
            assert!(message.contains("boom at cell 7"), "{message}");
        }
        other => panic!("{:?}", other.map(|_| ())),
    }
}

#[test]
fn a_string_panic_payload_is_read_too() {
    let reg = registry(vec![Box::new(Boom(script(None, Act::PanicString)))]);
    let plan = planned(&reg, 8, &["boom"]);
    let (result, _) = go(&plan, &reg, &RunOptions::default(), &CancelToken::new());
    assert!(matches!(
        result,
        Err(RunError::StagePanicked { message, .. }) if message == "owned boom"
    ));
}

#[test]
fn an_other_panic_payload_gets_a_fixed_text() {
    assert_eq!(panic_message(&*Box::new(7_u8)), "the stage panicked");
}

#[test]
fn an_error_finding_fails_naming_the_stage() {
    let mut bad = script(None, Act::Nothing);
    bad.finding = Some(Finding::error("the coast is missing"));
    let reg = registry(vec![Box::new(Bad(bad))]);
    let plan = planned(&reg, 8, &["bad"]);
    let (result, _) = go(&plan, &reg, &RunOptions::default(), &CancelToken::new());
    match result {
        Err(RunError::FindingFailed { stage, message }) => {
            assert_eq!(stage, "bad");
            assert!(message.contains("the coast is missing"));
        }
        other => panic!("{:?}", other.map(|_| ())),
    }
}

#[test]
fn warnings_go_into_findings() {
    let mut warn = script(None, Act::Nothing);
    warn.finding = Some(Finding::warning("a lake is small"));
    let reg = registry(vec![Box::new(Warn(warn))]);
    let plan = planned(&reg, 8, &["warn"]);
    let (result, _) = go(&plan, &reg, &RunOptions::default(), &CancelToken::new());
    let run = result.unwrap();
    assert_eq!(
        run.findings,
        [StageFinding {
            stage: "warn".to_owned(),
            message: "a lake is small".to_owned()
        }]
    );
}

#[test]
fn a_nan_in_a_written_layer_names_stage_layer_and_cell() {
    let reg = registry(vec![Box::new(Nan(script(Some("h"), Act::Write(f64::NAN))))]);
    let plan = planned(&reg, 8, &["nan"]);
    let (result, _) = go(&plan, &reg, &RunOptions::default(), &CancelToken::new());
    match result {
        Err(RunError::NonFinite { stage, layer, cell }) => {
            assert_eq!((stage.as_str(), layer.as_str(), cell), ("nan", "h", 0));
        }
        other => panic!("{:?}", other.map(|_| ())),
    }
}

/// A value layer holding one number (`x`).
#[derive(Debug)]
struct Number(f64);

impl crate::world::WorldValue for Number {
    fn type_name(&self) -> &'static str {
        <Self as crate::world::ValueType>::NAME
    }
    fn canonical_json(&self) -> Result<serde_json::Value, theos_map_format::FormatError> {
        theos_map_format::json::to_checked_value(&BTreeMap::from([("x", self.0)]))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl crate::world::ValueType for Number {
    const NAME: &'static str = "number";
}

/// Writes the value layer `v` holding its number.
struct WritesNumber(f64);

impl TypedStage for WritesNumber {
    type Params = Knob;
    const ID: &'static str = "number";
    const SUMMARY: &'static str = "Writes a number value.";
    const KINDS: &'static [MapKind] = KINDS;
    fn reads(&self, _: &Knob) -> Vec<LayerSpec> {
        vec![]
    }
    fn writes(&self, _: &Knob) -> Vec<LayerSpec> {
        vec![LayerSpec::value("v", "number")]
    }
    fn run(&self, ctx: &mut StageContext<'_>, _: &Knob) -> Result<StageReport, StageError> {
        ctx.write("v", Layer::Value(Box::new(Number(self.0))))?;
        Ok(StageReport::default())
    }
}

// Spec §5.4: a NaN or an infinity in a value is an error naming the stage and the layer, as in
// a float raster; `serde_json` would turn it into `null` and hash that.
#[test]
fn a_nan_in_a_written_value_names_stage_and_layer() {
    for bad in [f64::NAN, f64::INFINITY] {
        let reg = registry(vec![Box::new(WritesNumber(bad))]);
        let plan = planned(&reg, 8, &["number"]);
        let (result, _) = go(&plan, &reg, &RunOptions::default(), &CancelToken::new());
        match result {
            Err(error @ RunError::NonFiniteValue { .. }) => {
                let message = error.to_string();
                assert!(message.contains("`number`"), "{message}");
                assert!(message.contains("`v`"), "{message}");
            }
            other => panic!("{bad}: {:?}", other.map(|_| ())),
        }
    }
}

/// Reports the metric `level` with its number; writes nothing.
struct ReportsLevel(f64);

impl TypedStage for ReportsLevel {
    type Params = Knob;
    const ID: &'static str = "level";
    const SUMMARY: &'static str = "Reports one metric.";
    const KINDS: &'static [MapKind] = KINDS;
    fn reads(&self, _: &Knob) -> Vec<LayerSpec> {
        vec![]
    }
    fn writes(&self, _: &Knob) -> Vec<LayerSpec> {
        vec![]
    }
    fn metric_names(&self, _: &Knob) -> Vec<&'static str> {
        vec!["depth"]
    }
    fn run(&self, _: &mut StageContext<'_>, _: &Knob) -> Result<StageReport, StageError> {
        Ok(StageReport::default())
    }
    fn metrics(&self, _: &World, _: &Knob, _: &StageReport) -> Metrics {
        Metrics(BTreeMap::from([("depth".to_owned(), self.0)]))
    }
}

// Spec §5.13 and §5.14: the summary is JSON, which holds no NaN or infinity; `serde_json` would
// write one as `null` and a constraint on it would fail without naming the stage.
#[test]
fn a_non_finite_metric_names_stage_and_metric() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let reg = registry(vec![Box::new(ReportsLevel(bad))]);
        let plan = planned(&reg, 8, &["level"]);
        let (result, _) = go(&plan, &reg, &RunOptions::default(), &CancelToken::new());
        match result {
            Err(error @ RunError::NonFiniteMetric { .. }) => {
                let message = error.to_string();
                assert!(message.contains("`level`"), "{message}");
                assert!(message.contains("`depth`"), "{message}");
            }
            other => panic!("{bad}: {:?}", other.map(|_| ())),
        }
    }
    let reg = registry(vec![Box::new(ReportsLevel(-0.0))]);
    let plan = planned(&reg, 8, &["level"]);
    let (result, _) = go(&plan, &reg, &RunOptions::default(), &CancelToken::new());
    assert!(result.is_ok(), "a finite metric passes");
}

#[test]
fn the_memory_budget_refuses_before_any_stage_runs() {
    let counter = script(Some("big"), Act::Write(0.0));
    let runs = counter.runs.clone();
    let reg = registry(vec![Box::new(Count(counter))]);
    let plan = planned(&reg, 64, &["count"]);
    let options = RunOptions {
        memory_budget: Some(1024),
    };
    let (result, events) = go(&plan, &reg, &options, &CancelToken::new());
    match result {
        Err(RunError::OverBudget {
            needed,
            budget,
            largest,
        }) => {
            assert_eq!((needed, budget), (64 * 64 * 8 * 3 / 2, 1024));
            assert!(largest.contains("big"), "{largest}");
        }
        other => panic!("{:?}", other.map(|_| ())),
    }
    assert_eq!(runs.load(Ordering::SeqCst), 0);
    assert_eq!(events.len(), 0);
}

#[test]
fn estimate_counts_each_layer_once() {
    let a = script(Some("x"), Act::Write(1.0));
    let mut b = script(Some("y"), Act::Write(2.0));
    b.reads = Some("x");
    let c = script(Some("x"), Act::Write(3.0));
    let reg = registry(vec![
        Box::new(First(a)),
        Box::new(Second(b)),
        Box::new(Third(c)),
    ]);
    let plan = planned(&reg, 64, &["first", "second", "third"]);
    let e = estimate(&plan);
    assert_eq!(e.bytes, 64 * 64 * 8 * 2 * 3 / 2);
    assert_eq!(
        e.largest,
        [("x".to_owned(), 64 * 64 * 8), ("y".to_owned(), 64 * 64 * 8)]
    );
}

#[test]
fn the_largest_layers_are_three_by_size_then_name() {
    let a = script(Some("b"), Act::Write(1.0));
    let mut b = script(Some("a"), Act::Write(1.0));
    b.reads = Some("b");
    let reg = registry(vec![Box::new(First(a)), Box::new(Second(b))]);
    let plan = planned(&reg, 8, &["first", "second"]);
    let names: Vec<String> = estimate(&plan).largest.into_iter().map(|l| l.0).collect();
    assert_eq!(names, ["a", "b"]);
}

#[test]
fn an_estimate_exactly_at_the_budget_runs() {
    let a = script(Some("x"), Act::Write(1.0));
    let mut b = script(Some("y"), Act::Write(2.0));
    b.reads = Some("x");
    let reg = registry(vec![Box::new(First(a)), Box::new(Second(b))]);
    let plan = planned(&reg, 64, &["first", "second"]);
    let exact = estimate(&plan).bytes;
    let at = RunOptions {
        memory_budget: Some(exact),
    };
    assert!(go(&plan, &reg, &at, &CancelToken::new()).0.is_ok());
    let below = RunOptions {
        memory_budget: Some(exact - 1),
    };
    assert!(matches!(
        go(&plan, &reg, &below, &CancelToken::new()).0,
        Err(RunError::OverBudget { .. })
    ));
}

#[test]
fn layer_hashes_are_recorded_per_stage_in_pipeline_order() {
    let (reg, plan) = simple();
    let (result, _) = go(&plan, &reg, &RunOptions::default(), &CancelToken::new());
    let run = result.unwrap();
    let keys: Vec<&str> = run.golden.stages.iter().map(|s| s.key.as_str()).collect();
    assert_eq!(keys, ["first#0", "second#0", "third#0"]);
    assert_eq!(run.golden.recipe, "t");
    assert_eq!(run.golden.seed, 5);
    let first = &run.golden.stages[0].layers;
    assert_eq!(first.keys().collect::<Vec<_>>(), ["x"]);
    assert_eq!(first["x"].len(), 64);
    assert_eq!(
        run.golden.stages[1].layers.keys().collect::<Vec<_>>(),
        ["y"]
    );
    assert_ne!(first["x"], run.golden.stages[1].layers["y"]);
    assert!(run.golden.stages[2].layers.is_empty());
    assert!(run.world.f64("x").is_some() && run.world.f64("y").is_some());
}

#[test]
fn metrics_reports_and_timings_are_keyed_by_the_stage_key() {
    let a = script(Some("x"), Act::Write(1.0));
    let reg = registry(vec![Box::new(First(a))]);
    let plan = planned(&reg, 8, &["first", "first"]);
    let (result, _) = go(&plan, &reg, &RunOptions::default(), &CancelToken::new());
    let run = result.unwrap();
    let names: Vec<&str> = run.stage_metrics.iter().map(|m| m.0.as_str()).collect();
    assert_eq!(names, ["first", "first#1"]);
    assert!((run.stage_metrics[0].1.0["cells"] - 64.0).abs() < 1e-12);
    assert_eq!(
        run.reports.keys().map(String::as_str).collect::<Vec<_>>(),
        ["first", "first#1"]
    );
    assert_eq!(
        run.timing_s.keys().map(String::as_str).collect::<Vec<_>>(),
        ["first", "first#1"]
    );
    assert_eq!(run.golden.stages[1].key, "first#1");
}

#[test]
fn run_options_are_serde_and_default_to_no_budget() {
    let options: RunOptions = serde_json::from_str("{}").unwrap();
    assert_eq!(options, RunOptions::default());
    assert_eq!(options.memory_budget, None);
    assert_eq!(
        serde_json::to_string(&RunOptions {
            memory_budget: Some(5)
        })
        .unwrap(),
        "{\"memory_budget\":5}"
    );
}

#[test]
fn an_unregistered_stage_fails_cleanly() {
    let (_, plan) = simple();
    let empty = Registry::empty();
    let (result, _) = go(&plan, &empty, &RunOptions::default(), &CancelToken::new());
    assert!(matches!(result, Err(RunError::StageFailed { .. })));
}

#[test]
fn an_error_in_the_stages_own_report_fails_naming_the_stage() {
    let reg = registry(vec![Box::new(Bad(script(None, Act::ReportError)))]);
    let plan = planned(&reg, 8, &["bad"]);
    let (result, _) = go(&plan, &reg, &RunOptions::default(), &CancelToken::new());
    match result {
        Err(RunError::FindingFailed { stage, message }) => {
            assert_eq!(
                (stage.as_str(), message.as_str()),
                ("bad", "found while running")
            );
        }
        other => panic!("{:?}", other.map(|_| ())),
    }
}

#[test]
fn a_panic_in_checks_or_metrics_fails_the_run_naming_the_stage() {
    for (in_checks, text) in [(true, "checks blew up"), (false, "metrics blew up")] {
        let mut s = script(None, Act::Nothing);
        s.check_panics = in_checks;
        s.metrics_panic = !in_checks;
        let reg = registry(vec![Box::new(Boom(s))]);
        let plan = planned(&reg, 8, &["boom"]);
        let (result, _) = go(&plan, &reg, &RunOptions::default(), &CancelToken::new());
        match result {
            Err(RunError::StagePanicked { stage, message }) => {
                assert_eq!(stage, "boom");
                assert!(message.contains(text), "{message}");
            }
            other => panic!("{:?}", other.map(|_| ())),
        }
    }
}

#[test]
fn a_layer_updated_in_place_is_hashed_for_the_updating_stage() {
    let first = script(Some("x"), Act::Write(1.0));
    let mut upd = script(Some("x"), Act::Update);
    upd.reads = Some("x");
    let reg = registry(vec![Box::new(First(first)), Box::new(Upd(upd))]);
    let plan = planned(&reg, 8, &["first", "upd"]);
    let (result, _) = go(&plan, &reg, &RunOptions::default(), &CancelToken::new());
    let run = result.unwrap();
    let written = &run.golden.stages[0].layers["x"];
    let updated = &run.golden.stages[1].layers["x"];
    assert_ne!(written, updated);
    let cell = run.world.f64("x").map_or(0.0, |g| g.as_slice()[0]);
    assert!((cell - 2.0).abs() < 1e-12);
}
