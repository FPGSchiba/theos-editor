//! Helpers for the tests of `landscape_evolution` and its processes.
//!
//! [`evolve`] drives a world through planned processes the way the stage's time loop does, and
//! [`processes`] plans the built-in processes from `[[process]]` tables; physics tests of the
//! process modules use both. The test processes below exercise the loop itself.

use std::any::Any;

use serde::Deserialize;

use super::processes::{self, Constructor, PlannedProcess};
use super::steps::{self, Schedule};
use super::{Evolution, EvolutionParams};
use crate::grid::Grid;
use crate::progress::{CancelToken, Progress};
use crate::stage::{
    Finding, LayerSpec, MapKind, Stage, StageContext, StageError, StageReport, TypedStage,
};
use crate::stages::terrain::grid::GridStage;
use crate::stages::terrain::support::{run_stage, world};
use crate::world::{Layer, ValueType, World, WorldValue};

/// The key of the first `landscape_evolution` of a recipe; [`evolve`] runs under it.
pub(crate) const KEY: &str = "landscape_evolution#0";

/// The run seed of [`run_evolution`].
pub(crate) const SEED: u64 = 3;

/// Runs the planned built-in `processes` for `steps` time steps of `dt_yr` years over `world`,
/// keyed as the first `landscape_evolution` of a run with `seed`. It skips the initial surface,
/// so the world holds whatever the test put there, and it runs every process's checks every
/// step. The report holds the processes' values summed over the run (`<id>_<name>`) and their
/// folded warnings.
pub(crate) fn evolve(
    world: &mut World,
    processes: &[PlannedProcess],
    steps: u32,
    dt_yr: f64,
    seed: u64,
) -> Result<StageReport, StageError> {
    evolve_with(&Evolution::new(), world, processes, steps, dt_yr, seed)
}

/// [`evolve`] with the processes of `stage`.
pub(crate) fn evolve_with(
    stage: &Evolution,
    world: &mut World,
    processes: &[PlannedProcess],
    steps: u32,
    dt_yr: f64,
    seed: u64,
) -> Result<StageReport, StageError> {
    let params = EvolutionParams::default();
    let cancel = CancelToken::new();
    let mut sink = |_: &Progress| {};
    let mut ctx = StageContext::new(world, stage, &params, KEY, seed, &cancel, &mut sink);
    let schedule = Schedule {
        steps,
        dt_yr,
        check_every: 1,
    };
    steps::run(&mut ctx, stage.processes(), processes, schedule)
}

/// The built-in processes that the `[[process]]` tables of `toml` name, planned.
pub(crate) fn processes(toml: &str) -> Vec<PlannedProcess> {
    processes_of(&Evolution::new(), toml)
}

/// The processes of `stage` that the `[[process]]` tables of `toml` name, planned.
pub(crate) fn processes_of(stage: &Evolution, toml: &str) -> Vec<PlannedProcess> {
    #[derive(Deserialize)]
    struct Doc {
        process: Vec<toml::Table>,
    }
    let doc: Doc = toml::from_str(toml).unwrap();
    processes::plan(stage.processes(), &doc.process).unwrap()
}

/// A 64 x 64 world of 100 m cells after `grid` (the south edge drains).
pub(crate) fn grid_world() -> World {
    let mut w = world(64, 64);
    run_stage(&mut w, &GridStage, "").unwrap();
    w
}

/// Runs `stage` with the parameters `toml` over `world`, keyed [`KEY`] with [`SEED`], with
/// `cancel` and the progress events pushed to `events`; then checks its declared writes.
pub(crate) fn run_evolution(
    world: &mut World,
    stage: &Evolution,
    toml: &str,
    cancel: &CancelToken,
    events: &mut Vec<Progress>,
) -> Result<StageReport, StageError> {
    let stage: &dyn Stage = stage;
    let params = stage.params(&toml::from_str(toml).unwrap()).unwrap();
    let mut sink = |p: &Progress| events.push(p.clone());
    let mut ctx = StageContext::new(world, stage, params.as_ref(), KEY, SEED, cancel, &mut sink);
    let report = stage.run(&mut ctx, params.as_ref())?;
    ctx.finish()?;
    Ok(report)
}

/// The steps at which [`Log`] ran.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct StepLog(pub(crate) Vec<u32>);

impl WorldValue for StepLog {
    fn type_name(&self) -> &'static str {
        Self::NAME
    }
    fn canonical_json(&self) -> serde_json::Value {
        serde_json::json!(self.0)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl ValueType for StepLog {
    const NAME: &'static str = "step_log";
}

/// The steps `log` ran at in `world`.
pub(crate) fn logged(world: &World) -> Vec<u32> {
    world.value::<StepLog>("log").unwrap().0.clone()
}

/// The parameters of a test process that takes none.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct Nothing {}

/// A test process with fixed reads and writes and a run that does nothing.
macro_rules! declaring {
    ($name:ident, $id:literal, reads: $reads:expr, writes: $writes:expr) => {
        #[doc = concat!("The test process `", $id, "`.")]
        pub(crate) struct $name;
        impl TypedStage for $name {
            type Params = Nothing;
            const ID: &'static str = $id;
            const SUMMARY: &'static str = "A test process.";
            const KINDS: &'static [MapKind] = &[MapKind::Terrain];
            fn reads(&self, _: &Nothing) -> Vec<LayerSpec> {
                $reads
            }
            fn writes(&self, _: &Nothing) -> Vec<LayerSpec> {
                $writes
            }
            fn run(
                &self,
                _: &mut StageContext<'_>,
                _: &Nothing,
            ) -> Result<StageReport, StageError> {
                Ok(StageReport::default())
            }
        }
    };
}

declaring!(Route, "route",
    reads: vec![LayerSpec::f64("height"), LayerSpec::mask("fixed")],
    writes: vec![LayerSpec::u32("receivers")]);
declaring!(Erode, "erode",
    reads: vec![
        LayerSpec::f64("height"),
        LayerSpec::u32("receivers"),
        LayerSpec::f64("kd"),
        LayerSpec::f64("omega").optional(),
    ],
    writes: vec![LayerSpec::f64("height"), LayerSpec::f64("drainage_area")]);
declaring!(Lazy, "lazy", reads: vec![], writes: vec![LayerSpec::u32("receivers")]);
declaring!(Tint, "tint", reads: vec![LayerSpec::f64("omega")], writes: vec![]);
declaring!(Misread, "misread", reads: vec![LayerSpec::u8("height")], writes: vec![]);
declaring!(Floaty, "floaty",
    reads: vec![LayerSpec::f64("receivers"), LayerSpec::u8("kd")],
    writes: vec![]);
declaring!(Rewrite, "rewrite", reads: vec![], writes: vec![LayerSpec::f64("receivers")]);

/// The numbers [`Draw`] drew, one per step.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Draws(pub(crate) Vec<u64>);

impl WorldValue for Draws {
    fn type_name(&self) -> &'static str {
        Self::NAME
    }
    fn canonical_json(&self) -> serde_json::Value {
        serde_json::json!(self.0)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl ValueType for Draws {
    const NAME: &'static str = "draws";
}

/// Draws one number from its own stream per step into the value `draws`.
pub(crate) struct Draw;

impl TypedStage for Draw {
    type Params = Nothing;
    const ID: &'static str = "draw";
    const SUMMARY: &'static str = "Draws a number every step.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &Nothing) -> Vec<LayerSpec> {
        vec![LayerSpec::value("draws", Draws::NAME).optional()]
    }

    fn writes(&self, _: &Nothing) -> Vec<LayerSpec> {
        vec![LayerSpec::value("draws", Draws::NAME)]
    }

    fn run(&self, ctx: &mut StageContext<'_>, _: &Nothing) -> Result<StageReport, StageError> {
        let mut draws = ctx
            .value_opt::<Draws>("draws")?
            .cloned()
            .unwrap_or_default();
        draws.0.push(ctx.rng().next_u64());
        ctx.write("draws", Layer::Value(Box::new(draws)))?;
        Ok(StageReport::default())
    }
}

/// Adds 1 m to every cell's `height`.
pub(crate) struct Bump;

impl TypedStage for Bump {
    type Params = Nothing;
    const ID: &'static str = "bump";
    const SUMMARY: &'static str = "Raises the height by 1 m.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &Nothing) -> Vec<LayerSpec> {
        vec![LayerSpec::f64("height")]
    }

    fn writes(&self, _: &Nothing) -> Vec<LayerSpec> {
        vec![LayerSpec::f64("height")]
    }

    fn run(&self, ctx: &mut StageContext<'_>, _: &Nothing) -> Result<StageReport, StageError> {
        let mut height = ctx.take_f64("height")?;
        for h in height.as_mut_slice() {
            *h += 1.0;
        }
        ctx.write("height", Layer::F64(height))?;
        Ok(StageReport::default())
    }
}

/// Appends its step to the value `log`.
pub(crate) struct Log;

impl TypedStage for Log {
    type Params = Nothing;
    const ID: &'static str = "log";
    const SUMMARY: &'static str = "Logs the steps it runs at.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &Nothing) -> Vec<LayerSpec> {
        vec![LayerSpec::value("log", StepLog::NAME).optional()]
    }

    fn writes(&self, _: &Nothing) -> Vec<LayerSpec> {
        vec![LayerSpec::value("log", StepLog::NAME)]
    }

    fn run(&self, ctx: &mut StageContext<'_>, _: &Nothing) -> Result<StageReport, StageError> {
        let step = ctx.step().unwrap().step;
        let mut log = ctx
            .value_opt::<StepLog>("log")?
            .cloned()
            .unwrap_or_default();
        log.0.push(step);
        ctx.write("log", Layer::Value(Box::new(log)))?;
        Ok(StageReport::default())
    }
}

/// The parameters of [`Scale`].
#[derive(Debug, Deserialize)]
#[serde(default)]
pub(crate) struct ScaleParams {
    pub(crate) factor: f64,
}

impl Default for ScaleParams {
    fn default() -> Self {
        Self { factor: 1.0 }
    }
}

/// Multiplies `height` by `factor`.
pub(crate) struct Scale;

impl TypedStage for Scale {
    type Params = ScaleParams;
    const ID: &'static str = "scale";
    const SUMMARY: &'static str = "Scales the height.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &ScaleParams) -> Vec<LayerSpec> {
        vec![LayerSpec::f64("height")]
    }

    fn writes(&self, _: &ScaleParams) -> Vec<LayerSpec> {
        vec![LayerSpec::f64("height")]
    }

    fn run(&self, ctx: &mut StageContext<'_>, p: &ScaleParams) -> Result<StageReport, StageError> {
        let mut height: Grid<f64> = ctx.take_f64("height")?;
        for h in height.as_mut_slice() {
            *h *= p.factor;
        }
        ctx.write("height", Layer::F64(height))?;
        Ok(StageReport::default())
    }
}

/// Panics.
pub(crate) struct Boom;

impl TypedStage for Boom {
    type Params = Nothing;
    const ID: &'static str = "boom";
    const SUMMARY: &'static str = "Panics.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &Nothing) -> Vec<LayerSpec> {
        Vec::new()
    }

    fn writes(&self, _: &Nothing) -> Vec<LayerSpec> {
        Vec::new()
    }

    fn run(&self, _: &mut StageContext<'_>, _: &Nothing) -> Result<StageReport, StageError> {
        panic!("boom in a process")
    }
}

/// The parameters of [`Judge`].
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct JudgeParams {
    /// The step whose checks find an error; the others warn.
    pub(crate) fail_at: Option<u32>,
    /// Whether the run itself reports a warning every step.
    pub(crate) warn_in_run: bool,
}

/// Reports its step; its checks warn `checked at step <n>`, or fail at `fail_at`.
pub(crate) struct Judge;

impl TypedStage for Judge {
    type Params = JudgeParams;
    const ID: &'static str = "judge";
    const SUMMARY: &'static str = "Checks every time it is asked.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &JudgeParams) -> Vec<LayerSpec> {
        Vec::new()
    }

    fn writes(&self, _: &JudgeParams) -> Vec<LayerSpec> {
        Vec::new()
    }

    fn run(&self, ctx: &mut StageContext<'_>, p: &JudgeParams) -> Result<StageReport, StageError> {
        let mut report = StageReport::default();
        let step = ctx.step().unwrap().step;
        report.values.insert("step".to_owned(), f64::from(step));
        if p.warn_in_run {
            report
                .findings
                .push(Finding::warning(format!("ran at step {step}")).on_layer("height"));
        }
        Ok(report)
    }

    fn checks(&self, _: &World, p: &JudgeParams, report: &StageReport) -> Vec<Finding> {
        let step = report.values["step"];
        if p.fail_at
            .is_some_and(|at| (f64::from(at) - step).abs() < 0.5)
        {
            vec![Finding::error(format!("judged at step {step}"))]
        } else {
            vec![Finding::warning(format!("checked at step {step}"))]
        }
    }
}

/// The test processes and `uplift`.
pub(crate) const TEST_PROCESSES: &[Constructor] = &[
    || Box::new(super::uplift::Uplift),
    || Box::new(Log),
    || Box::new(Scale),
    || Box::new(Route),
    || Box::new(Erode),
    || Box::new(Lazy),
    || Box::new(Tint),
    || Box::new(Misread),
    || Box::new(Floaty),
    || Box::new(Rewrite),
    || Box::new(Draw),
    || Box::new(Bump),
    || Box::new(Boom),
    || Box::new(Judge),
];

/// `landscape_evolution` over [`TEST_PROCESSES`].
pub(crate) fn test_evolution() -> Evolution {
    Evolution::with_processes(TEST_PROCESSES)
}
