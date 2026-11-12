//! The per-step processes `landscape_evolution` can run, and their parsing from the recipe's
//! `[[stage.process]]` tables.

use std::collections::BTreeMap;

use super::Evolution;
use super::climate_update::ClimateUpdate;
use super::fluvial::Fluvial;
use super::hillslope::Hillslope;
use super::partition_flow::PartitionFlow;
use super::resolve_sinks::ResolveSinks;
use super::route_steepest::RouteSteepest;
use super::slope_limit::SlopeLimit;
use super::uplift::Uplift;
use crate::stage::{ParamError, Registry, Stage, StageParams, TypedStage};

/// Builds one per-step process.
pub(crate) type Constructor = fn() -> Box<dyn Stage>;

/// Every per-step process, one line each. A new process is one module plus one line here.
pub(crate) const PROCESSES: &[Constructor] = &[
    || Box::new(Uplift),
    || Box::new(RouteSteepest),
    || Box::new(ResolveSinks),
    || Box::new(PartitionFlow),
    || Box::new(Fluvial),
    || Box::new(Hillslope),
    || Box::new(SlopeLimit),
    || Box::new(ClimateUpdate),
];

/// One per-step process of a recipe, with its parsed parameters.
#[derive(Debug)]
pub(crate) struct PlannedProcess {
    /// The process id.
    pub id: &'static str,
    /// `<id>#<occurrence>`, the occurrence counted per id from 0. Inside a run the process's
    /// key is `<evolution key>/<this key>` (plan ruling L12).
    pub key: String,
    /// The parsed parameters.
    pub params: Box<dyn StageParams>,
    /// Run only at step 0, at every multiple of this and on the last step; `None` runs every
    /// step (plan ruling L27).
    pub every: Option<u32>,
}

impl PlannedProcess {
    /// Whether the process runs at `step` of `steps`.
    pub(crate) fn runs_at(&self, step: u32, steps: u32) -> bool {
        step + 1 == steps
            || self
                .every
                .is_none_or(|n| step.checked_rem(n).is_some_and(|r| r == 0))
    }
}

/// Plans the recipe's process `tables`, in order, with the processes of `registry`.
///
/// Each table names its process with `id`; `every` is taken out before the process parses the
/// rest of the table as its parameters.
///
/// # Errors
/// [`ParamError`] of `landscape_evolution` naming `process[<i>]` (and the parameter inside it)
/// for a table without an `id`, an unknown process, a bad `every`, or parameters the process
/// refuses; the message names the process.
pub(crate) fn plan(
    registry: &Registry,
    tables: &[toml::Table],
) -> Result<Vec<PlannedProcess>, ParamError> {
    let mut seen: BTreeMap<&'static str, u32> = BTreeMap::new();
    let mut planned = Vec::with_capacity(tables.len());
    for (i, table) in tables.iter().enumerate() {
        let at = format!("process[{i}]");
        let Some(id) = table.get("id").and_then(toml::Value::as_str) else {
            return Err(error(at, "a process needs an `id` naming it"));
        };
        let Some(stage) = registry.get(id) else {
            let known: Vec<&str> = registry.ids().collect();
            let message = format!("unknown process `{id}` (known: {})", known.join(", "));
            return Err(error(at, message));
        };
        let mut table = table.clone();
        let every = table
            .remove("every")
            .map(|value| every(&value).ok_or_else(|| bad_every(&at, &value)))
            .transpose()?;
        let params = stage.params(&table).map_err(|e| inside(&at, id, e))?;
        let occurrence = seen.entry(stage.id()).or_insert(0);
        let key = format!("{}#{occurrence}", stage.id());
        *occurrence += 1;
        planned.push(PlannedProcess {
            id: stage.id(),
            key,
            params,
            every,
        });
    }
    Ok(planned)
}

/// `value` as a step count of at least 1.
fn every(value: &toml::Value) -> Option<u32> {
    value
        .as_integer()
        .and_then(|n| u32::try_from(n).ok())
        .filter(|n| *n >= 1)
}

fn bad_every(at: &str, value: &toml::Value) -> ParamError {
    error(
        format!("{at}.every"),
        format!("`every` must be a whole number of steps, at least 1, not {value}"),
    )
}

/// A [`ParamError`] of `landscape_evolution` at `parameter`.
fn error(parameter: String, message: impl Into<String>) -> ParamError {
    ParamError::at(Evolution::ID, parameter, message)
}

/// The process `id`'s own error `e`, as an error of `landscape_evolution` inside `at`.
fn inside(at: &str, id: &str, e: ParamError) -> ParamError {
    let parameter = match e.parameter {
        Some(inner) => format!("{at}.{inner}"),
        None => at.to_owned(),
    };
    error(parameter, format!("process `{id}`: {}", e.message))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{PROCESSES, PlannedProcess};

    fn planned(every: Option<u32>) -> PlannedProcess {
        PlannedProcess {
            id: "p",
            key: "p#0".to_owned(),
            params: Box::new(()),
            every,
        }
    }

    #[test]
    fn every_process_id_is_unique_and_snake_case() {
        let ids: Vec<&str> = PROCESSES.iter().map(|make| make().id()).collect();
        for id in &ids {
            let mut chars = id.chars();
            let snake = chars.next().is_some_and(|c| c.is_ascii_lowercase())
                && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
            assert!(snake, "process id {id:?} is not snake case");
        }
        let unique: BTreeSet<&str> = ids.iter().copied().collect();
        assert_eq!(unique.len(), ids.len(), "duplicate process ids in {ids:?}");
    }

    #[test]
    fn a_zero_every_runs_only_on_the_last_step() {
        let p = planned(Some(0));
        let runs: Vec<u32> = (0..5).filter(|&s| p.runs_at(s, 5)).collect();
        assert_eq!(runs, [4]);
        let p = planned(None);
        assert!((0..5).all(|s| p.runs_at(s, 5)));
    }
}
