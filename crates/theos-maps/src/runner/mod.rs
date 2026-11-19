//! The runner: executes a validated [`Plan`] stage by stage over a [`World`] (spec §5.6).
//!
//! Per stage it reports progress, honours the cancel token, runs the stage with panics caught,
//! checks the declared writes, runs the stage's checks, records its metrics and hashes every
//! layer it wrote (the golden record). Before the first stage it refuses a plan whose memory
//! [`estimate`] is above the budget.

mod memory;
#[cfg(test)]
mod tests;

use std::any::Any;
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

use serde::{Deserialize, Serialize};

pub use self::memory::{DEFAULT_MEMORY_BUDGET, MemoryEstimate, estimate};
use crate::error::RunError;
use crate::golden::{GoldenRun, GoldenStage};
use crate::progress::{CancelToken, Progress};
use crate::recipe::{Plan, PlannedStage};
use crate::stage::{Metrics, Registry, Severity, Stage, StageContext, StageError, StageReport};
use crate::world::{LayerBytesError, World, layer_sha256};

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

/// Runs `plan` with `seed` over a new world.
///
/// # Errors
/// [`RunError::OverBudget`] before any stage when the memory estimate exceeds the budget;
/// [`RunError::Cancelled`]; [`RunError::StageFailed`], [`RunError::StagePanicked`],
/// [`RunError::FindingFailed`], [`RunError::NonFinite`] and [`RunError::NonFiniteValue`], each
/// naming the stage.
pub(crate) fn execute(
    plan: &Plan,
    seed: u64,
    registry: &Registry,
    options: &RunOptions,
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
    let total = count(plan.stages.len());
    for (index, planned) in plan.stages.iter().enumerate() {
        progress(&Progress {
            stage: planned.id.to_owned(),
            done: count(index),
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

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
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
        Err(payload) => Err(RunError::StagePanicked {
            stage: planned.id.to_owned(),
            message: panic_message(&*payload),
        }),
    }
}

/// The text of a panic payload when it is a `&str` or a `String`.
pub(crate) fn panic_message(payload: &dyn Any) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "the stage panicked".to_owned())
}

/// Checks, metrics and hashes of a finished stage go into `out`.
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
    .map_err(|payload| RunError::StagePanicked {
        stage: planned.id.to_owned(),
        message: panic_message(&*payload),
    })?;
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
        let hash = layer_sha256(layer).map_err(|error| match error {
            LayerBytesError::NonFinite { cell } => RunError::NonFinite {
                stage: planned.id.to_owned(),
                layer: spec.name.to_owned(),
                cell,
            },
            LayerBytesError::ValueNotFinite => RunError::NonFiniteValue {
                stage: planned.id.to_owned(),
                layer: spec.name.to_owned(),
            },
            LayerBytesError::Json(source) => RunError::StageFailed {
                stage: planned.id.to_owned(),
                source: StageError::Failed {
                    stage: planned.id.to_owned(),
                    message: format!("layer `{}` has no canonical bytes: {source}", spec.name),
                },
            },
        })?;
        layers.insert(spec.name.to_owned(), hash);
    }
    Ok(layers)
}
