//! Rule `leaf` (spec §5.6): attaches an open dead-end room of the program to a room that does
//! not lie behind a lock.

use serde::Deserialize;

use crate::rng::StageRng;
use crate::rooms::{Cell, Passage, Role, Room, RoomId};
use crate::stage::ParamError;
use crate::stages::interior::mission_core::{
    Match, MissionState, Rule, RuleParams, params_of, parse_params,
};

/// The rule `leaf`.
pub struct Leaf;

/// Every role, the default of [`Params::from_roles`].
fn every_role() -> Vec<Role> {
    vec![Role::Start, Role::Goal, Role::Path, Role::Leaf, Role::Exit]
}

/// The parameters of `leaf`.
#[derive(Debug, Deserialize)]
struct Params {
    /// The roles a host may have; locked rooms are never hosts.
    #[serde(default = "every_role")]
    from_roles: Vec<Role>,
}

/// Every (free cell, host room) pair where the cell borders the host and `host_ok` accepts the
/// host, as matches of weight 1 in (cell, host) order. Shared by the rules that attach a room.
pub(super) fn attach_matches(
    s: &MissionState,
    host_ok: impl Fn(usize, &Room) -> bool,
) -> Vec<Match> {
    let mut pairs: Vec<(Cell, RoomId)> = Vec::new();
    for (i, room) in s.graph.rooms.iter().enumerate() {
        let Ok(id) = RoomId::try_from(i) else {
            continue;
        };
        if !host_ok(i, room) {
            continue;
        }
        let mut cells: Vec<Cell> = room
            .cells
            .iter()
            .flat_map(|&c| s.free_neighbours(c))
            .collect();
        cells.sort_unstable();
        cells.dedup();
        pairs.extend(cells.into_iter().map(|c| (c, id)));
    }
    pairs.sort_unstable();
    pairs
        .into_iter()
        .map(|(c, id)| Match {
            cells: vec![c],
            rooms: vec![id],
            weight: 1,
        })
        .collect()
}

impl Rule for Leaf {
    fn id(&self) -> &'static str {
        "leaf"
    }

    fn params(&self, table: &toml::Table) -> Result<Box<dyn RuleParams>, ParamError> {
        Ok(Box::new(parse_params::<Params>(self.id(), table)?))
    }

    fn matches(&self, s: &MissionState, p: &dyn RuleParams) -> Vec<Match> {
        let Some(p) = params_of::<Params>(p) else {
            return Vec::new();
        };
        attach_matches(s, |_, room| {
            !room.locked && p.from_roles.contains(&room.role)
        })
    }

    /// Places the next open leaf room of the program on the match cell, joined open to the host.
    fn apply(&self, s: &mut MissionState, m: &Match, _: &dyn RuleParams, _: &mut StageRng) {
        let (Some(&cell), Some(&host)) = (m.cells.first(), m.rooms.first()) else {
            return;
        };
        let room = Room {
            cells: vec![cell],
            ..s.program.take_with_lock(Role::Leaf, false)
        };
        let id = s.add_room(room);
        s.connect(host, id, Passage::Open, false);
    }

    fn makes_keys(&self, _: &dyn RuleParams, _: u32) -> u32 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rooms::solve::solvable;
    use crate::stages::interior::rules::testkit::{
        cell, entry, lattice, program, program_room, room, run, state_on,
    };

    /// A 3x3 lattice: start (1, 1) and goal (1, 2) joined open, and a locked vault at (0, 1)
    /// behind key 1, which lies in the start.
    fn with_vault(more: &[crate::stages::interior::program::ProgramRoom]) -> MissionState {
        let mut s = state_on(lattice(3, 3), &program(more));
        let mut start = room("porch", Role::Start, cell(1, 1), false);
        start.keys.push(1);
        let start = s.add_room(start);
        let goal = s.add_room(room("altar", Role::Goal, cell(1, 2), false));
        let vault = s.add_room(room("vault", Role::Leaf, cell(0, 1), true));
        s.connect(start, goal, Passage::Open, true);
        s.connect(start, vault, Passage::Locked(1), false);
        s.next_key = 2;
        s
    }

    fn parsed(text: &str) -> Box<dyn RuleParams> {
        Leaf.params(&toml::from_str(text).unwrap()).unwrap()
    }

    // AC: CE-80.2
    #[test]
    fn leaf_attaches_to_non_locked_rooms_only() {
        let s = with_vault(&[]);
        let all = Leaf.matches(&s, parsed("").as_ref());
        assert_ne!(all.len(), 0);
        assert!(all.iter().all(|m| m.rooms[0] != 2), "the vault is no host");
        for m in &all {
            let host = &s.graph.rooms[usize::from(m.rooms[0])];
            assert!(s.is_free(m.cells[0]));
            assert!(host.cells.iter().any(|c| c.touches(m.cells[0])));
        }
        let keyed: Vec<(Cell, RoomId)> = all.iter().map(|m| (m.cells[0], m.rooms[0])).collect();
        let mut sorted = keyed.clone();
        sorted.sort_unstable();
        assert_eq!(keyed, sorted);
        let goal_only = Leaf.matches(&s, parsed("from_roles = [\"goal\"]").as_ref());
        assert_ne!(goal_only.len(), 0);
        assert!(goal_only.iter().all(|m| m.rooms[0] == 1));

        let mut s = with_vault(&[]);
        let r = run(&mut s, &[entry("leaf", 20, false, "")], 4);
        assert_eq!(r.applied, 6, "every free cell gets a leaf");
        assert_eq!(r.per_entry[0].undone, 0);
        assert_eq!(solvable(&s.graph), Ok(()));
        for c in s.graph.connections.iter().filter(|c| c.b >= 3) {
            assert_ne!(c.a, 2, "nothing hangs off the vault");
            assert_eq!((c.kind, c.main), (Passage::Open, false));
            let leaf = &s.graph.rooms[usize::from(c.b)];
            assert_eq!((leaf.role, leaf.locked), (Role::Leaf, false));
        }
    }

    // AC: CE-80.2
    #[test]
    fn leaf_skips_the_programs_locked_rooms() {
        let more = [
            program_room("strongroom", Role::Leaf, (1, 1), true),
            program_room("crypt", Role::Leaf, (1, 1), false),
        ];
        let mut s = with_vault(&more);
        run(&mut s, &[entry("leaf", 2, true, "")], 1);
        let types: Vec<&str> = s.graph.rooms[3..].iter().map(|r| r.ty.as_str()).collect();
        assert_eq!(types, ["crypt", "leaf"]);
        assert!(s.graph.rooms[3..].iter().all(|r| !r.locked));
    }

    // AC: CE-80.2
    #[test]
    fn leaf_refuses_unknown_roles() {
        let err = Leaf
            .params(&toml::from_str("from_roles = [\"attic\"]").unwrap())
            .unwrap_err();
        assert_eq!(err.parameter.as_deref(), Some("from_roles[0]"));
        assert_eq!(Leaf.makes_keys(parsed("").as_ref(), 9), 0);
    }
}
