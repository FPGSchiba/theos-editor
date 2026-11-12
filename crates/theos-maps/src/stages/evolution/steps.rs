//! The time loop of `landscape_evolution`: every step, each planned process runs in its own
//! child context and is checked, as a pipeline stage is by the runner (spec §5.3, §5.6).

use super::processes::PlannedProcess;
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

/// Runs the `planned` processes of `registry` for `schedule.steps` steps inside the stage of
/// `ctx`. Every step it checks the cancel token, reports progress every [`PROGRESS_EVERY`]
/// steps, and runs each process that is due (`every`) in a child context keyed
/// `<ctx key>/<id>#<occurrence>`, then checks that the process wrote its declared layers. The
/// findings of a process's own report count every step it runs; its `checks` run at every
/// multiple of `check_every` and on the last step.
///
/// The returned report holds the first warning of each process that warned, in process order,
/// with the count of the warnings that followed it. The values of the processes' own reports are
/// not carried over.
///
/// A child context builds its random streams from its key, which is the same every step, so a
/// process's `ctx.rng()` starts afresh each step; a process that needs new numbers every step
/// derives them with `ctx.sub_rng` and the step.
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
    let mut stages: Vec<&dyn Stage> = Vec::with_capacity(planned.len());
    for process in planned {
        let stage = registry
            .get(process.id)
            .ok_or_else(|| ctx.fail(format!("unknown process `{}`", process.id)))?;
        stages.push(stage);
    }
    let keys: Vec<String> = planned
        .iter()
        .map(|p| format!("{}/{}", ctx.key(), p.key))
        .collect();
    let mut warnings: Vec<Option<Folded>> = planned.iter().map(|_| None).collect();
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
        let checking = last || step.checked_rem(check_every).is_some_and(|r| r == 0);
        let info = StepInfo { step, steps, dt_yr };
        for (k, (process, &stage)) in planned.iter().zip(&stages).enumerate() {
            if !process.runs_at(step, steps) {
                continue;
            }
            let params = process.params.as_ref();
            let report = {
                let mut child = ctx.child(stage, params, &keys[k], info);
                let report = stage.run(&mut child, params)?;
                child.finish()?;
                report
            };
            let checked = if checking {
                stage.checks(ctx.world(), params, &report)
            } else {
                Vec::new()
            };
            for finding in report.findings.into_iter().chain(checked) {
                if finding.severity == Severity::Error {
                    return Err(StageError::Failed {
                        stage: format!("{}/{}", ctx.stage(), process.id),
                        message: format!("at step {step}: {}", finding.message),
                    });
                }
                match &mut warnings[k] {
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
        }
    }
    let findings = planned
        .iter()
        .zip(warnings)
        .filter_map(|(process, folded)| folded.map(|f| summary(process, f)))
        .collect();
    Ok(StageReport {
        findings,
        ..StageReport::default()
    })
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
