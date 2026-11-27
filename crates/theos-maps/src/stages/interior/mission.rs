//! The `mission` stage (spec §5.5, §5.6): the room graph from the recipe's room program and
//! rule entries, on the lattice of `envelope`.
//!
//! A thin wrapper over [`run_mission`]. Draws, all from the stage's stream (ruling R4): the
//! program's range counts first ([`ProgramQueue::new`]), then the engine's (each entry's count,
//! the match choices, the rules' own draws). The metrics are `applied` and `short`.

use serde::{Deserialize, Serialize};

use crate::rooms::lattice::Lattice;
use crate::stage::{
    LayerSpec, MAX_COUNT, MapKind, ParamError, StageContext, StageError, StageReport, TypedStage,
    too_large_count,
};
use crate::stages::interior::mission_core::{MissionState, RuleEntry, key_budget, run_mission};
use crate::stages::interior::program::{CountSpec, ProgramQueue, ProgramRoom, check_program};
use crate::stages::interior::rules::rules;
use crate::stages::interior::support::ROOM_GRAPH;
use crate::world::{Layer, ValueType};

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
        vec![ROOM_GRAPH]
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
        state
            .graph
            .validate(&|c| state.lattice.is_enabled(c))
            .map_err(|e| ctx.fail(format!("the room graph breaks a rule: {e}")))?;
        ctx.write("room_graph", Layer::Value(Box::new(state.graph)))?;
        let mut report = StageReport::default();
        report.put("applied", f64::from(done.applied));
        report.put("short", f64::from(done.short));
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::Stage;

    fn refusal(text: &str) -> Option<(String, Option<String>)> {
        Mission
            .params(&toml::from_str(text).unwrap())
            .err()
            .map(|e| (e.stage, e.parameter))
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
