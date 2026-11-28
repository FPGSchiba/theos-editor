//! The `mission` stage (spec §5.5, §5.6): the room graph from the recipe's room program and
//! rule entries, on the lattice of `envelope`.
//!
//! A thin wrapper over [`run_mission`]. Draws, all from the stage's stream (ruling R4): the
//! program's range counts first ([`ProgramQueue::new`]), then the engine's (each entry's count,
//! the match choices, the rules' own draws). The metrics are `applied` and `short`.

use serde::{Deserialize, Serialize};

use crate::rooms::lattice::Lattice;
use crate::rooms::solve::behind_lock;
use crate::rooms::{Role, RoomGraph};
use crate::stage::{
    LayerSpec, MAX_COUNT, MapKind, ParamError, StageContext, StageError, StageReport, TypedStage,
    too_large_count,
};
use crate::stages::interior::mission_core::{
    MissionReport, MissionState, RuleEntry, key_budget, run_mission,
};
use crate::stages::interior::program::{CountSpec, ProgramQueue, ProgramRoom, check_program};
use crate::stages::interior::rules::rules;
use crate::stages::interior::support::ROOM_GRAPH;
use crate::world::{Layer, ValueType};

/// What the mission's rule entries did, for the summary's `mission.entries` (ruling R33).
pub const MISSION_REPORT: LayerSpec = LayerSpec::value("mission_report", MissionReport::NAME);

/// The parameters of `mission`: `[[stage.room]]` and `[[stage.rule]]`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct MissionParams {
    /// The room program, in order.
    pub room: Vec<ProgramRoom>,
    /// The rule entries, applied in order.
    pub rule: Vec<RuleEntry>,
}

/// Refuses a count whose maximum exceeds P08's [`MAX_COUNT`], as other stages refuse counts.
fn check_count(count: CountSpec, path: &str) -> Result<(), ParamError> {
    if i64::from(count.max()) > MAX_COUNT {
        return Err(ParamError::at(Mission::ID, path, too_large_count()));
    }
    Ok(())
}

/// Why a mission placed no start room: the entries that came up short, each with its index,
/// rule id, the count asked and the count applied. A rule such as `main_cycle` that could not
/// fit its loop leaves the graph without a start, and the graph check alone would only say
/// "expected one start room, found 0".
fn no_start(report: &MissionReport) -> String {
    let short: Vec<String> = report
        .per_entry
        .iter()
        .enumerate()
        .filter(|(_, e)| e.applied < e.asked)
        .map(|(i, e)| {
            format!(
                "rule[{i}] `{}` asked {}, applied {}",
                e.rule, e.asked, e.applied
            )
        })
        .collect();
    if short.is_empty() {
        "no rule placed a start room".to_owned()
    } else {
        format!("no start room was placed; short: {}", short.join("; "))
    }
}

/// Sets every room's `locked` to whether it lies behind a lock (spec §5.4, ruling R55). The rules
/// read `locked` only while the mission runs; afterwards it means what the package says.
fn mark_locked(graph: &mut RoomGraph) {
    let behind = behind_lock(graph);
    for (room, locked) in graph.rooms.iter_mut().zip(behind) {
        room.locked = locked;
    }
}

/// The stage `mission`.
pub struct Mission;

impl TypedStage for Mission {
    type Params = MissionParams;
    const ID: &'static str = "mission";
    const SUMMARY: &'static str =
        "The room graph of an interior, rewritten by the recipe's mission rules.";
    const KINDS: &'static [MapKind] = &[MapKind::Interior];

    fn reads(&self, _: &MissionParams) -> Vec<LayerSpec> {
        vec![LayerSpec::value("lattice", Lattice::NAME)]
    }

    fn writes(&self, _: &MissionParams) -> Vec<LayerSpec> {
        vec![ROOM_GRAPH, MISSION_REPORT]
    }

    fn metric_names(&self, _: &MissionParams) -> Vec<&'static str> {
        vec!["applied", "short"]
    }

    /// Room types, ranges and footprints of the program; counts up to [`MAX_COUNT`]; known
    /// rules with valid parameters; at most 16 keys.
    fn validate(&self, p: &mut MissionParams) -> Result<(), ParamError> {
        check_program(Self::ID, &p.room)?;
        for (i, room) in p.room.iter().enumerate() {
            check_count(room.count, &format!("room[{i}].count"))?;
        }
        for (i, entry) in p.rule.iter().enumerate() {
            check_count(entry.count, &format!("rule[{i}].count"))?;
        }
        key_budget(&rules(), &p.rule).map(|_| ())
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        p: &MissionParams,
    ) -> Result<StageReport, StageError> {
        let lattice = ctx.value::<Lattice>("lattice")?.clone();
        let program = ProgramQueue::new(&p.room, ctx.rng());
        let mut state = MissionState::new(lattice, program);
        let done = run_mission(&mut state, &p.rule, &rules(), ctx.rng())
            .map_err(|e| ctx.fail(e.to_string()))?;
        if !state.graph.rooms.iter().any(|r| r.role == Role::Start) {
            return Err(ctx.fail(no_start(&done)));
        }
        mark_locked(&mut state.graph);
        state
            .graph
            .validate(&|c| state.lattice.is_enabled(c))
            .map_err(|e| ctx.fail(format!("the room graph breaks a rule: {e}")))?;
        ctx.write("room_graph", Layer::Value(Box::new(state.graph)))?;
        let mut report = StageReport::default();
        report.put("applied", f64::from(done.applied));
        report.put("short", f64::from(done.short));
        ctx.write(MISSION_REPORT.name, Layer::Value(Box::new(done)))?;
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::{Registry, Stage};
    use crate::stages::interior::support::execute_recipe;

    fn refusal(text: &str) -> Option<(String, Option<String>)> {
        Mission
            .params(&toml::from_str(text).unwrap())
            .err()
            .map(|e| (e.stage, e.parameter))
    }

    // AC: CE-80.2
    #[test]
    fn the_fixture_places_exactly_its_one_exit() {
        let text = include_str!("../../../tests/fixtures/recipes/fixture-interior.toml");
        for seed in 1..=3 {
            let done = execute_recipe(text, &Registry::builtin(), seed).unwrap();
            let g = done.world.value::<RoomGraph>("room_graph").unwrap();
            let exits: Vec<&str> = g
                .rooms
                .iter()
                .filter(|r| r.role == Role::Exit)
                .map(|r| r.ty.as_str())
                .collect();
            assert_eq!(exits, ["gate"], "seed {seed}");
        }
    }

    // AC: CE-80.2
    #[test]
    fn rooms_behind_the_lock_are_marked_locked() {
        use crate::stages::interior::rules::testkit::{
            arcs, cell, entry, lattice, program, room, room_of, run, state_on,
        };
        let mut goal_leaves = 0;
        for seed in 1..=8 {
            let mut s = state_on(lattice(6, 6), &program(&[]));
            let cycle = "side = \"south\"
cycle = \"lock_key\"";
            run(&mut s, &[entry("main_cycle", 1, true, cycle)], seed);
            let goal = room_of(&s, Role::Goal);
            // A leaf hung on the goal, on a free neighbour of its first cell.
            let at = s.graph.rooms[usize::from(goal)].cells[0];
            let free = s.free_neighbours(at);
            if let Some(&c) = free.first() {
                let leaf = s.add_room(room("cellar", Role::Leaf, cell(c.row, c.col), false));
                s.connect(goal, leaf, crate::rooms::Passage::Open, false);
                goal_leaves += 1;
            }
            assert!(s.graph.rooms.iter().all(|r| !r.locked), "seed {seed}");
            mark_locked(&mut s.graph);
            let g = &s.graph;
            assert!(!g.rooms[usize::from(room_of(&s, Role::Start))].locked);
            assert!(g.rooms[usize::from(goal)].locked, "seed {seed}: the goal");
            for (arc, _) in arcs(&s) {
                assert!(arc.iter().all(|&r| !g.rooms[usize::from(r)].locked));
            }
            for r in g.rooms.iter().filter(|r| r.role == Role::Leaf) {
                assert!(r.locked, "seed {seed}: the goal's leaf");
            }
        }
        assert!(goal_leaves > 0, "no seed had a free cell next to the goal");
    }

    // AC: CE-80.2
    #[test]
    fn a_short_main_cycle_is_named_when_no_start_is_placed() {
        // The fixture on a frame of one lattice row: main_cycle cannot fit its loop.
        let text = include_str!("../../../tests/fixtures/recipes/fixture-interior.toml")
            .replace("grid    = [40, 40]", "grid    = [8, 40]")
            .replace("size_km = [0.04, 0.04]", "size_km = [0.04, 0.008]");
        let Err(err) = execute_recipe(&text, &Registry::builtin(), 1) else {
            panic!("the run fails");
        };
        let err = err.to_string();
        assert!(
            err.contains(
                "no start room was placed; short: rule[0] `main_cycle` asked 1, applied 0"
            ),
            "{err}"
        );
        assert!(!err.contains("expected one start room"), "{err}");
        let none = MissionReport {
            applied: 0,
            short: 0,
            per_entry: Vec::new(),
        };
        assert_eq!(no_start(&none), "no rule placed a start room");
    }

    #[test]
    fn the_parameters_refuse_bad_programs_and_rules() {
        assert_eq!(refusal(""), None);
        let ok = "[[room]]\ntype = \"entrance\"\nrole = \"start\"\n\
                  [[rule]]\nrule = \"leaf\"\ncount = [0, 3]";
        assert_eq!(refusal(ok), None);
        for (text, parameter) in [
            (
                "[[room]]\ntype = \"store_room\"\nrole = \"leaf\"",
                "room[0].type",
            ),
            (
                "[[room]]\ntype = \"store\"\nrole = \"leaf\"\ncount = [0, 2000000]",
                "room[0].count",
            ),
            (
                "[[rule]]\nrule = \"leaf\"\ncount = 2000000",
                "rule[0].count",
            ),
            ("[[rule]]\nrule = \"attic\"", "rule[0].rule"),
            ("[[rule]]\nrule = \"locked_leaf\"\ncount = 17", "rule"),
            (
                "[[rule]]\nrule = \"leaf\"\nfrom_roles = [\"attic\"]",
                "rule[0].from_roles[0]",
            ),
        ] {
            assert_eq!(
                refusal(text),
                Some(("mission".to_owned(), Some(parameter.to_owned()))),
                "{text}"
            );
        }
    }
}
