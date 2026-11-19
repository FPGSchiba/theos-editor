//! The place of `hillslope` in a process list (spec §5.3, ruling 58): it runs before the
//! routing of its step, so that each step ends on the surface `fluvial` cut and no hillslope
//! pass fills the channels' own slopes after the fact (that put dams into the rivers).

use super::Evolution;
use super::fluvial::Fluvial;
use super::hillslope::Hillslope;
use super::partition_flow::PartitionFlow;
use super::processes::PlannedProcess;
use super::resolve_sinks::ResolveSinks;
use super::route_steepest::RouteSteepest;
use crate::stage::{ParamError, TypedStage};

/// The process the rule is about.
const HILLSLOPE: &str = Hillslope::ID;

/// The processes that route flow. `hillslope` may not follow any of them, nor `fluvial`, which
/// moves sediment by the flow.
const ROUTING: [&str; 3] = [RouteSteepest::ID, ResolveSinks::ID, PartitionFlow::ID];

/// Refuses a list in which `hillslope` comes after `fluvial` or after a routing process
/// (`route_steepest`, `resolve_sinks`, `partition_flow`). Lists without `hillslope` pass.
///
/// # Errors
/// [`ParamError`] of `landscape_evolution` at the `process[<i>]` of the misplaced `hillslope`,
/// naming `hillslope`, the process it follows and the rule.
pub(super) fn check(planned: &[PlannedProcess]) -> Result<(), ParamError> {
    for (i, process) in planned.iter().enumerate() {
        if process.id != HILLSLOPE {
            continue;
        }
        let moves_by_flow = |id: &str| ROUTING.contains(&id) || id == Fluvial::ID;
        if let Some(before) = planned[..i].iter().find(|p| moves_by_flow(p.id)) {
            let routing = ROUTING.map(|id| format!("`{id}`")).join(", ");
            let fluvial = Fluvial::ID;
            let message = format!(
                "process `{HILLSLOPE}` comes after `{}`; `{HILLSLOPE}` must run before every \
                 routing process ({routing}) and before `{fluvial}`, so that each step ends on \
                 the surface `{fluvial}` cut",
                before.id
            );
            return Err(ParamError::at(
                Evolution::ID,
                format!("process[{i}]"),
                message,
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::stages::evolution::processes;

    /// The parameters a process cannot do without.
    fn needed(id: &str) -> &'static str {
        match id {
            "fluvial" => "g = 1.0",
            "partition_flow" => "exponent = \"slope_adaptive\"",
            _ => "",
        }
    }

    fn planned(ids: &[&str]) -> Vec<PlannedProcess> {
        let tables: Vec<toml::Table> = ids
            .iter()
            .map(|id| {
                toml::from_str(&format!(
                    "id = \"{id}\"
{}",
                    needed(id)
                ))
                .unwrap()
            })
            .collect();
        processes::plan(Evolution::default().processes(), &tables).unwrap()
    }

    #[test]
    fn hillslope_after_a_routing_process_or_fluvial_is_refused() {
        for before in [
            "route_steepest",
            "resolve_sinks",
            "partition_flow",
            "fluvial",
        ] {
            let list = planned(&["uplift", before, "hillslope"]);
            let err = check(&list).unwrap_err();
            assert_eq!(err.stage, "landscape_evolution");
            assert_eq!(err.parameter.as_deref(), Some("process[2]"), "{before}");
            assert!(
                err.message.contains("`hillslope`") && err.message.contains(&format!("`{before}`")),
                "{err}"
            );
        }
    }

    #[test]
    fn the_stage_refuses_the_old_order_at_check_order() {
        let list = planned(&["uplift", "fluvial", "hillslope"]);
        let evolution = Evolution::default();
        let registry = evolution.processes();
        let err = super::super::declaration::check_order(registry, &list, &BTreeSet::new());
        let err = err.unwrap_err();
        assert!(err.message.contains("`hillslope`"), "{err}");
        assert_eq!(err.parameter.as_deref(), Some("process[2]"));
    }

    #[test]
    fn lists_in_order_or_without_hillslope_are_valid() {
        let listed = [
            vec!["uplift", "hillslope", "route_steepest", "fluvial"],
            vec!["hillslope"],
            vec![
                "uplift",
                "route_steepest",
                "resolve_sinks",
                "fluvial",
                "slope_limit",
            ],
            vec![],
        ];
        for ids in listed {
            assert_eq!(check(&planned(&ids)), Ok(()), "{ids:?}");
        }
    }
}
