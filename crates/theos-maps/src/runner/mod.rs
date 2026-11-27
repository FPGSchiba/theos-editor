//! The runner: executes a validated [`Plan`] stage by stage over a [`World`] (spec §5.6).
//!
//! Per stage it reports progress, honours the cancel token, runs the stage with panics caught,
//! checks the declared writes, runs the stage's checks, records its metrics (refusing a NaN or an
//! infinity, which the JSON summary cannot hold) and hashes every layer it wrote (the golden
//! record). Before the first stage it refuses a plan whose memory
//! [`estimate`] is above the budget.

#[cfg(test)]
mod child_tests;
mod memory;
mod seed;
#[cfg(test)]
mod tests;

use std::any::Any;
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

use serde::{Deserialize, Serialize};

pub use self::memory::{DEFAULT_MEMORY_BUDGET, MemoryEstimate, estimate};
use crate::child::ParentContext;
use crate::error::RunError;
use crate::golden::{GoldenRun, GoldenStage};
use crate::kernels::num::to_u32;
use crate::progress::{CancelToken, Progress};
use crate::recipe::{Plan, PlannedStage};
use crate::stage::{Metrics, Registry, Severity, Stage, StageContext, StageError, StageReport};
use crate::world::{Layer, LayerBytesError, World, layer_sha256};

/// How a run is carried out.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RunOptions {
    /// The most memory the layers may take, in bytes; `None` means [`DEFAULT_MEMORY_BUDGET`].
    pub memory_budget: Option<u64>,
}

/// A warning of a stage's checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageFinding {
    /// The stage id.
    pub stage: String,
    /// What was found.
    pub message: String,
}

/// Everything a finished run holds.
#[derive(Debug)]
pub(crate) struct Execution {
    /// The world after the last stage.
    pub world: World,
    /// The metrics of each stage, keyed by its id (`<id>#<n>` for later occurrences), in order.
    pub stage_metrics: Vec<(String, Metrics)>,
    /// The warnings of all stages.
    pub findings: Vec<StageFinding>,
    /// The report of each stage, under the same keys as the metrics.
    pub reports: BTreeMap<String, StageReport>,
    /// The layer hashes; `features` and `package` are filled in by the package step.
    pub golden: GoldenRun,
    /// Wall time per stage in seconds, under the same keys. Never written into the package.
    pub timing_s: BTreeMap<String, f64>,
}

/// Runs `plan` with `seed` over a new world. A child plan's `parent` supplies the values the
/// world is seeded with before the first stage (`seed` module); their hashes are the golden
/// pseudo stage `parent#0` (L12).
///
/// # Errors
/// [`RunError::OverBudget`] before any stage when the memory estimate exceeds the budget;
/// [`RunError::Cancelled`]; [`RunError::StageFailed`], [`RunError::StagePanicked`],
/// [`RunError::FindingFailed`], [`RunError::NonFinite`], [`RunError::NonFiniteValue`] and
/// [`RunError::NonFiniteMetric`], each naming the stage.
pub(crate) fn execute(
    plan: &Plan,
    seed: u64,
    registry: &Registry,
    options: &RunOptions,
    parent: Option<&ParentContext>,
    progress: &mut dyn FnMut(&Progress),
    cancel: &CancelToken,
) -> Result<Execution, RunError> {
    check_budget(plan, options)?;
    let mut world = World::new(plan.frame);
    let mut out = Execution {
        world: World::new(plan.frame),
        stage_metrics: Vec::new(),
        findings: Vec::new(),
        reports: BTreeMap::new(),
        golden: GoldenRun {
            recipe: plan.recipe_id.clone(),
            seed,
            features: String::new(),
            package: String::new(),
            stages: Vec::new(),
        },
        timing_s: BTreeMap::new(),
    };
    if let Some(stage) = seed::seed_world(&mut world, plan, parent, cancel)? {
        out.golden.stages.push(stage);
    }
    let total = to_u32(plan.stages.len());
    for (index, planned) in plan.stages.iter().enumerate() {
        progress(&Progress {
            stage: planned.id.to_owned(),
            done: to_u32(index),
            total,
        });
        if cancel.is_cancelled() {
            return Err(RunError::Cancelled);
        }
        let stage = registry
            .get(planned.id)
            .ok_or_else(|| RunError::StageFailed {
                stage: planned.id.to_owned(),
                source: StageError::Failed {
                    stage: planned.id.to_owned(),
                    message: "the stage is not registered".to_owned(),
                },
            })?;
        let started = Instant::now();
        let report = run_stage(&mut world, stage, planned, seed, cancel, progress)?;
        let label = label(planned);
        record(&world, stage, planned, report, &label, &mut out)?;
        out.timing_s.insert(label, started.elapsed().as_secs_f64());
    }
    out.world = world;
    Ok(out)
}

/// The id for the first occurrence of a stage, `<id>#<n>` for later ones.
fn label(planned: &PlannedStage) -> String {
    planned
        .key
        .strip_suffix("#0")
        .unwrap_or(&planned.key)
        .to_owned()
}

fn check_budget(plan: &Plan, options: &RunOptions) -> Result<(), RunError> {
    let budget = options.memory_budget.unwrap_or(DEFAULT_MEMORY_BUDGET);
    let needed = estimate(plan);
    if needed.bytes > budget {
        return Err(RunError::OverBudget {
            needed: needed.bytes,
            budget,
            largest: memory::describe(&needed.largest),
        });
    }
    Ok(())
}

/// Runs the stage in its context with panics caught and the declared writes checked.
fn run_stage(
    world: &mut World,
    stage: &dyn Stage,
    planned: &PlannedStage,
    seed: u64,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(&Progress),
) -> Result<StageReport, RunError> {
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let mut ctx = StageContext::new(
            world,
            stage,
            planned.params.as_ref(),
            &planned.key,
            seed,
            cancel,
            progress,
        );
        let report = stage.run(&mut ctx, planned.params.as_ref())?;
        ctx.finish()?;
        Ok(report)
    }));
    match outcome {
        Ok(Ok(report)) => Ok(report),
        Ok(Err(StageError::Cancelled)) => Err(RunError::Cancelled),
        Ok(Err(source)) => Err(RunError::StageFailed {
            stage: planned.id.to_owned(),
            source,
        }),
        Err(payload) => Err(panicked(planned.id, &*payload)),
    }
}

/// The error of `stage` (a stage, or a step after the pipeline) that panicked with `payload`:
/// the message is the payload's text when it is a `&str` or a `String`.
pub(crate) fn panicked(stage: &str, payload: &dyn Any) -> RunError {
    let message = payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "the stage panicked".to_owned());
    RunError::StagePanicked {
        stage: stage.to_owned(),
        message,
    }
}

/// Checks, metrics and hashes of a finished stage go into `out`. A metric that is NaN or an
/// infinity is refused naming the stage and the metric (spec §5.13: the summary is JSON).
fn record(
    world: &World,
    stage: &dyn Stage,
    planned: &PlannedStage,
    report: StageReport,
    label: &str,
    out: &mut Execution,
) -> Result<(), RunError> {
    let params = planned.params.as_ref();
    let (checked, metrics) = catch_unwind(AssertUnwindSafe(|| {
        (
            stage.checks(world, params, &report),
            stage.metrics(world, params, &report),
        )
    }))
    .map_err(|payload| panicked(planned.id, &*payload))?;
    let found = report.findings.iter().cloned().chain(checked);
    for finding in found {
        match finding.severity {
            Severity::Error => {
                return Err(RunError::FindingFailed {
                    stage: planned.id.to_owned(),
                    message: finding.message,
                });
            }
            Severity::Warning => out.findings.push(StageFinding {
                stage: planned.id.to_owned(),
                message: finding.message,
            }),
        }
    }
    if let Some(metric) = metrics
        .0
        .iter()
        .find(|(_, v)| !v.is_finite())
        .map(|(k, _)| k)
    {
        return Err(RunError::NonFiniteMetric {
            stage: planned.id.to_owned(),
            metric: metric.clone(),
        });
    }
    out.stage_metrics.push((label.to_owned(), metrics));
    out.golden.stages.push(GoldenStage {
        key: planned.key.clone(),
        layers: hash_writes(world, planned)?,
    });
    out.reports.insert(label.to_owned(), report);
    Ok(())
}

/// The SHA-256 of every layer the stage declared as a write. A layer the stage was bound to write
/// (not optional) and the world does not hold is a contract violation.
fn hash_writes(
    world: &World,
    planned: &PlannedStage,
) -> Result<BTreeMap<String, String>, RunError> {
    let mut layers = BTreeMap::new();
    for spec in &planned.writes {
        let Some(layer) = world.get(spec.name) else {
            if spec.optional {
                continue;
            }
            return Err(RunError::StageFailed {
                stage: planned.id.to_owned(),
                source: StageError::NotWritten {
                    stage: planned.id.to_owned(),
                    layer: spec.name.to_owned(),
                },
            });
        };
        let hash = hash_layer(planned.id, spec.name, layer)?;
        layers.insert(spec.name.to_owned(), hash);
    }
    Ok(layers)
}

/// The SHA-256 of `layer`, which `stage` wrote under `name`.
fn hash_layer(stage: &str, name: &str, layer: &Layer) -> Result<String, RunError> {
    layer_sha256(layer).map_err(|error| match error {
        LayerBytesError::NonFinite { cell } => RunError::NonFinite {
            stage: stage.to_owned(),
            layer: name.to_owned(),
            cell,
        },
        LayerBytesError::ValueNotFinite => RunError::NonFiniteValue {
            stage: stage.to_owned(),
            layer: name.to_owned(),
        },
        LayerBytesError::Json(source) => RunError::StageFailed {
            stage: stage.to_owned(),
            source: StageError::Failed {
                stage: stage.to_owned(),
                message: format!("layer `{name}` has no canonical bytes: {source}"),
            },
        },
    })
}
