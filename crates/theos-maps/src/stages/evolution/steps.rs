//! The time loop of `landscape_evolution`: every step, each planned process runs in its own
//! child context and is checked, as a pipeline stage is by the runner (spec §5.3, §5.6).

use std::collections::BTreeMap;

use super::processes::PlannedProcess;
use crate::rng::StageRng;
use crate::stage::{
    Finding, Registry, Severity, Stage, StageContext, StageError, StageReport, StepInfo,
};

/// How often the loop reports progress, in steps (spec §5.6).
const PROGRESS_EVERY: u32 = 10;

/// How the loop runs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Schedule {
    /// The number of time steps.
    pub steps: u32,
    /// The length of one step in years.
    pub dt_yr: f64,
    /// The processes' checks run at every multiple of this and on the last step.
    pub check_every: u32,
}

/// The first warning of one process and how many followed it.
struct Folded {
    first: Finding,
    step: u32,
    more: usize,
}

/// One planned process during the run.
struct Running<'a> {
    process: &'a PlannedProcess,
    stage: &'a dyn Stage,
    /// `<ctx key>/<id>#<occurrence>`.
    key: String,
    /// The process's one random stream for the whole run, `for_key(seed, key)`.
    stream: StageRng,
    /// Its warnings so far.
    warnings: Option<Folded>,
}

/// What one step asks of every process.
#[derive(Clone, Copy)]
struct Step {
    info: StepInfo,
    /// Whether the processes' checks run.
    checking: bool,
}

/// Runs the `planned` processes of `registry` for `schedule.steps` steps inside the stage of
/// `ctx`. Every step, and before each process, it checks the cancel token; it reports progress
/// every [`PROGRESS_EVERY`] steps, and runs each process that is due (`every`) in a child context keyed
/// `<ctx key>/<id>#<occurrence>`, then checks that the process wrote its declared layers. The
/// findings of a process's own report count every step it runs; its `checks` run at every
/// multiple of `check_every` and on the last step.
///
/// Each process keeps one random stream for the whole run, made once from its key (plan ruling
/// L12), so `ctx.rng()` in a process continues where the last step left it.
///
/// The returned report holds the values of the processes' reports, each summed over the steps
/// it ran, under `<id>_<name>` (`<id>_<occurrence>_<name>` past the first occurrence), and the
/// first warning of each process that warned, in process order, with the count of the warnings
/// that followed it.
///
/// # Errors
/// [`StageError::Cancelled`]; any error of a process (named `<stage>/<process id>` by its child
/// context); [`StageError::Failed`] naming `<stage>/<process id>` and the step for an error
/// finding, or for a process `registry` does not hold.
pub(crate) fn run(
    ctx: &mut StageContext<'_>,
    registry: &Registry,
    planned: &[PlannedProcess],
    schedule: Schedule,
) -> Result<StageReport, StageError> {
    let mut running = Vec::with_capacity(planned.len());
    for process in planned {
        let stage = registry
            .get(process.id)
            .ok_or_else(|| ctx.fail(format!("unknown process `{}`", process.id)))?;
        running.push(Running {
            process,
            stage,
            key: format!("{}/{}", ctx.key(), process.key),
            stream: ctx.sub_rng(&process.key),
            warnings: None,
        });
    }
    let mut values = BTreeMap::new();
    let Schedule {
        steps,
        dt_yr,
        check_every,
    } = schedule;
    for step in 0..steps {
        ctx.check_cancel()?;
        if step % PROGRESS_EVERY == 0 {
            ctx.progress(step, steps);
        }
        let last = step + 1 == steps;
        let now = Step {
            info: StepInfo { step, steps, dt_yr },
            checking: last || step.checked_rem(check_every).is_some_and(|r| r == 0),
        };
        for one in &mut running {
            if one.process.runs_at(step, steps) {
                ctx.check_cancel()?;
                run_process(ctx, one, now, &mut values)?;
            }
        }
    }
    let findings = running
        .into_iter()
        .filter_map(|one| one.warnings.map(|f| summary(one.process, f)))
        .collect();
    Ok(StageReport { values, findings })
}

/// Runs `one` process for the step `now` in its child context with its own stream, checks its
/// writes and, when due, its checks; adds its values to `values` and folds its warnings.
fn run_process(
    ctx: &mut StageContext<'_>,
    one: &mut Running<'_>,
    now: Step,
    values: &mut BTreeMap<String, f64>,
) -> Result<(), StageError> {
    let (stage, params) = (one.stage, one.process.params.as_ref());
    let report = {
        let mut child = ctx.child(stage, params, &one.key, now.info);
        child.swap_rng(&mut one.stream);
        let outcome = stage.run(&mut child, params);
        child.swap_rng(&mut one.stream);
        let report = outcome?;
        child.finish()?;
        report
    };
    let checked = if now.checking {
        ctx.checks_of(stage, params, &report)
    } else {
        Vec::new()
    };
    let prefix = value_prefix(one.process);
    for (name, value) in &report.values {
        *values.entry(format!("{prefix}_{name}")).or_insert(0.0) += value;
    }
    let step = now.info.step;
    for finding in report.findings.into_iter().chain(checked) {
        if finding.severity == Severity::Error {
            return Err(StageError::Failed {
                stage: format!("{}/{}", ctx.stage(), one.process.id),
                message: format!("at step {step}: {}", finding.message),
            });
        }
        match &mut one.warnings {
            Some(folded) => folded.more += 1,
            slot @ None => {
                *slot = Some(Folded {
                    first: finding,
                    step,
                    more: 0,
                });
            }
        }
    }
    Ok(())
}

/// The prefix of `process`'s values in the stage report: its id, or `<id>_<occurrence>` past
/// the first occurrence.
fn value_prefix(process: &PlannedProcess) -> String {
    match process.key.strip_suffix("#0") {
        Some(id) => id.to_owned(),
        None => process.key.replace('#', "_"),
    }
}

/// The warning that stands for all warnings of `process`: the first, naming the process (its
/// key past the first occurrence) and the step, with the count of the others.
fn summary(process: &PlannedProcess, folded: Folded) -> Finding {
    let name = process.key.strip_suffix("#0").unwrap_or(&process.key);
    let more = match folded.more {
        0 => String::new(),
        1 => " (1 more warning from this process)".to_owned(),
        n => format!(" ({n} more warnings from this process)"),
    };
    let message = format!(
        "process `{name}` at step {}: {}{more}",
        folded.step, folded.first.message
    );
    Finding {
        message,
        ..folded.first
    }
}

#[cfg(test)]
mod tests;
