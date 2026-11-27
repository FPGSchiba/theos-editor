//! The `playability` stage (spec §5.5, §5.9): the analysis of an interior, which fails the run
//! when the interior is unplayable.
//!
//! A thin wrapper over [`analyse`]. It reads the room graph, every layer of `space`, `cover`
//! and `envelope` (for `footprint_used_pct`), writes the `analysis` value, reports §5.9's
//! metrics (the two exit distances only when the graph has a reachable exit room) and turns
//! every finding of the analysis into an error finding, whose message starts with its code.

use serde::{Deserialize, Serialize};

use crate::rooms::RoomGraph;
use crate::stage::{
    Finding, LayerSpec, MapKind, StageContext, StageError, StageReport, TypedStage,
};
use crate::stages::interior::playability_core::{Analysis, analyse};
use crate::stages::interior::support::{ROOM_GRAPH, SPACE_LAYERS, read_space};
use crate::world::{Layer, ValueType, World};

/// The metric names of §5.9, in its order.
const METRICS: [&str; 13] = [
    "rooms",
    "connections",
    "cycles",
    "locks",
    "one_way",
    "chokepoints",
    "dead_end_pct",
    "footprint_used_pct",
    "hall_share_pct",
    "critical_path_m",
    "longest_sightline_m",
    "start_to_exit_min_m",
    "goal_to_exit_min_m",
];

/// The parameters of `playability`: none.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct PlayabilityParams {}

/// The metrics of `a` by name; an exit distance only when it is known.
fn metrics_of(a: &Analysis) -> Vec<(&'static str, f64)> {
    let mut out = vec![
        ("rooms", a.rooms),
        ("connections", a.connections),
        ("cycles", a.cycles),
        ("locks", a.locks),
        ("one_way", a.one_way),
        ("chokepoints", a.chokepoints),
        ("dead_end_pct", a.dead_end_pct),
        ("footprint_used_pct", a.footprint_used_pct),
        ("hall_share_pct", a.hall_share_pct),
        ("critical_path_m", a.critical_path_m),
        ("longest_sightline_m", a.longest_sightline_m),
    ];
    out.extend(a.start_to_exit_min_m.map(|m| ("start_to_exit_min_m", m)));
    out.extend(a.goal_to_exit_min_m.map(|m| ("goal_to_exit_min_m", m)));
    out
}

/// The stage `playability`.
pub struct Playability;

impl TypedStage for Playability {
    type Params = PlayabilityParams;
    const ID: &'static str = "playability";
    const SUMMARY: &'static str =
        "The playability analysis of an interior; an unplayable interior fails the run.";
    const KINDS: &'static [MapKind] = &[MapKind::Interior];

    fn reads(&self, _: &PlayabilityParams) -> Vec<LayerSpec> {
        let mut reads = vec![ROOM_GRAPH];
        reads.extend(SPACE_LAYERS);
        reads.extend([LayerSpec::mask("cover"), LayerSpec::mask("envelope")]);
        reads
    }

    fn writes(&self, _: &PlayabilityParams) -> Vec<LayerSpec> {
        vec![LayerSpec::value("analysis", Analysis::NAME)]
    }

    fn metric_names(&self, _: &PlayabilityParams) -> Vec<&'static str> {
        METRICS.to_vec()
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        _: &PlayabilityParams,
    ) -> Result<StageReport, StageError> {
        let frame = *ctx.frame();
        let analysis = {
            let graph = ctx.value::<RoomGraph>("room_graph")?;
            let space = read_space(ctx, graph.rooms.len())?;
            let cover = ctx.mask("cover")?;
            let envelope = ctx.mask("envelope")?;
            analyse(
                graph,
                &space,
                cover,
                envelope,
                frame.cell_m,
                (frame.rows, frame.cols),
            )
            .map_err(|e| ctx.kernel_failed(e))?
        };
        let mut report = StageReport::default();
        for (name, value) in metrics_of(&analysis) {
            report.put(name, value);
        }
        ctx.write("analysis", Layer::Value(Box::new(analysis)))?;
        Ok(report)
    }

    fn checks(&self, world: &World, _: &PlayabilityParams, _: &StageReport) -> Vec<Finding> {
        match world.value::<Analysis>("analysis") {
            Some(a) => a
                .findings
                .iter()
                .map(|(_, message)| Finding::error(message.clone()))
                .collect(),
            None => vec![Finding::error("the analysis is missing")],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // AC: CE-80.2
    #[test]
    fn the_metrics_follow_the_spec_and_skip_unknown_exit_distances() {
        let a = Analysis {
            rooms: 7.0,
            connections: 7.0,
            cycles: 1.0,
            locks: 1.0,
            one_way: 1.0,
            chokepoints: 0.0,
            dead_end_pct: 0.0,
            footprint_used_pct: 50.0,
            hall_share_pct: 20.0,
            critical_path_m: 30.0,
            longest_sightline_m: 12.0,
            start_to_exit_min_m: None,
            goal_to_exit_min_m: Some(4.0),
            route: Vec::new(),
            route_points: Vec::new(),
            findings: Vec::new(),
        };
        let names: Vec<&str> = metrics_of(&a).iter().map(|(n, _)| *n).collect();
        let mut want: Vec<&str> = METRICS.to_vec();
        want.retain(|&n| n != "start_to_exit_min_m");
        assert_eq!(names, want);
        let stage = Playability;
        assert_eq!(stage.metric_names(&PlayabilityParams {}), METRICS.to_vec());
    }
}
