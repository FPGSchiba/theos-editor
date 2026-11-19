//! The `landscape_evolution` stage: the initial surface, then `steps` time steps of its own list
//! of per-step processes (spec §5.3 "Two levels, one trait", §5.9).
//!
//! Every per-step process is a [`Stage`](crate::stage::Stage) under the same contract as a
//! pipeline stage: it declares its reads and writes for its parameters, runs in a child context
//! that hands out only those layers, and is checked. A recipe switches a process on by listing it
//! in a `[[stage.process]]` table. The stage's own reads and writes are the union of its
//! processes' (minus what one process writes for a later one), so validation sees exactly what
//! the process list needs.
//!
//! The stage's report carries the values of its processes' reports, summed over the run under
//! `<process id>_<value name>`, and the first warning of each process with a count of the rest.
//! Process metrics are not forwarded in P08: no process declares any.
//!
//! A new process is one module here plus one line in `PROCESSES` (`processes.rs`).

mod booking;
mod climate_update;
mod declaration;
mod flow;
mod fluvial;
mod hillslope;
mod hillslope_order;
mod initial;
mod partition_flow;
mod processes;
mod resolve_sinks;
mod route_steepest;
mod slope_limit;
mod steps;
#[cfg(test)]
pub(crate) mod test_support;
#[cfg(test)]
mod tests;
mod uplift;

use std::collections::BTreeSet;

use serde::Deserialize;

pub(crate) use self::processes::{Constructor, PROCESSES, PlannedProcess};
use self::steps::Schedule;
use crate::stage::{
    Check, LayerSpec, MapKind, ParamError, Registry, StageContext, StageError, StageReport,
    TypedStage,
};

/// The parameters of `landscape_evolution`; the defaults are 0.3.0's (`check_every` is plan
/// ruling L6).
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct EvolutionParams {
    /// The number of time steps.
    pub steps: u32,
    /// The length of one time step in years.
    pub dt_yr: f64,
    /// The processes' checks run at every multiple of this many steps and on the last step.
    pub check_every: u32,
    /// The relief of the initial surface's noise, in metres.
    pub initial_relief_m: f64,
    /// The `[[stage.process]]` tables, in order, as the recipe gives them. A table's `every = n`
    /// runs its process only at step 0, at every multiple of `n` and on the last step. Each run
    /// covers one `dt_yr`, so `every` is meant for refreshes such as `climate_update`; on a rate
    /// process (`uplift`, `hillslope`, `fluvial`) it slows the process by about `n`.
    pub process: Vec<toml::Table>,
    /// The processes of `process`, parsed by `validate`; not a recipe parameter.
    #[serde(skip)]
    pub(crate) planned: Vec<PlannedProcess>,
}

impl Default for EvolutionParams {
    fn default() -> Self {
        Self {
            steps: 240,
            dt_yr: 2.5e4,
            check_every: 10,
            initial_relief_m: 40.0,
            process: Vec::new(),
            planned: Vec::new(),
        }
    }
}

/// The stage `landscape_evolution` with the per-step processes it can run.
pub struct Evolution {
    processes: Registry,
}

impl Evolution {
    /// The stage with every built-in process.
    #[must_use]
    pub fn new() -> Self {
        Self::with_processes(PROCESSES)
    }

    /// The stage with the processes `list` builds, in its order.
    #[must_use]
    pub(crate) fn with_processes(list: &[Constructor]) -> Self {
        let mut processes = Registry::empty();
        for make in list {
            processes.add(make());
        }
        Self { processes }
    }

    /// The processes the stage can run.
    #[cfg(test)]
    pub(crate) fn processes(&self) -> &Registry {
        &self.processes
    }
}

impl Default for Evolution {
    fn default() -> Self {
        Self::new()
    }
}

impl TypedStage for Evolution {
    type Params = EvolutionParams;
    const ID: &'static str = "landscape_evolution";
    const SUMMARY: &'static str = "Initial surface from spectral noise, then `steps` time steps of the per-step processes the recipe lists (uplift, hillslope, flow routing, fluvial erosion and deposition, slope limit, climate update).";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, params: &EvolutionParams) -> Vec<LayerSpec> {
        declaration::reads(&self.processes, &params.planned)
    }

    fn writes(&self, params: &EvolutionParams) -> Vec<LayerSpec> {
        declaration::writes(&self.processes, &params.planned)
    }

    fn validate(&self, params: &mut EvolutionParams) -> Result<(), ParamError> {
        let check = Check::of(Self::ID);
        check.count_at_least("steps", params.steps, 1)?;
        check.positive("dt_yr", params.dt_yr)?;
        check.count_at_least("check_every", params.check_every, 1)?;
        check.at_least("initial_relief_m", params.initial_relief_m, 0.0)?;
        params.planned = processes::plan(&self.processes, &params.process)?;
        Ok(())
    }

    fn check_order(
        &self,
        params: &EvolutionParams,
        available: &BTreeSet<&str>,
    ) -> Result<(), ParamError> {
        declaration::check_order(&self.processes, &params.planned, available)
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        params: &EvolutionParams,
    ) -> Result<StageReport, StageError> {
        initial::write(ctx, params.initial_relief_m)?;
        let schedule = Schedule {
            steps: params.steps,
            dt_yr: params.dt_yr,
            check_every: params.check_every,
        };
        steps::run(ctx, &self.processes, &params.planned, schedule)
    }
}
