//! Rule `leaf` (spec §5.6): attaches an open dead-end room of the program to a room that does
//! not lie behind a lock.
//!
//! With `role = "exit"` (ruling R29) it attaches the program's open exit rooms the same way. An
//! exit has no generic room, so the rule has no match once the program has no open exit left.

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

/// The role of the rooms the rule attaches.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Attach {
    /// Dead ends, from the program or generic `leaf` rooms.
    #[default]
    Leaf,
    /// Exit rooms, from the program only.
    Exit,
}

impl Attach {
    fn role(self) -> Role {
        match self {
            Self::Leaf => Role::Leaf,
            Self::Exit => Role::Exit,
        }
    }
}

/// The parameters of `leaf`.
#[derive(Debug, Deserialize)]
struct Params {
    /// The roles a host may have; locked rooms are never hosts.
    #[serde(default = "every_role")]
    from_roles: Vec<Role>,
    /// The role of the attached rooms; `leaf` when left out.
    #[serde(default)]
    role: Attach,
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
        if p.role == Attach::Exit && !s.program.has(Role::Exit, false) {
            return Vec::new();
        }
        attach_matches(s, |_, room| {
            !room.locked && p.from_roles.contains(&room.role)
        })
    }

    /// Places the next open room of the program of the rule's role on the match cell, joined
    /// open to the host.
    fn apply(&self, s: &mut MissionState, m: &Match, p: &dyn RuleParams, _: &mut StageRng) {
        let (Some(&cell), Some(&host), Some(p)) =
            (m.cells.first(), m.rooms.first(), params_of::<Params>(p))
        else {
            return;
        };
        let room = Room {
            cells: vec![cell],
            ..s.program.take_with_lock(p.role.role(), false)
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
    fn leaf_places_exit_rooms() {
        let more = [
            program_room("strongroom", Role::Exit, (1, 1), true),
            program_room("gate", Role::Exit, (1, 1), false),
            program_room("crypt", Role::Leaf, (1, 1), false),
        ];
        let mut s = with_vault(&more);
        let exits = parsed("role = \"exit\"");
        assert_eq!(
            Leaf.matches(&s, exits.as_ref()),
            Leaf.matches(&s, parsed("").as_ref()),
            "exits attach where leaves do"
        );
        let r = run(&mut s, &[entry("leaf", 3, false, "role = \"exit\"")], 2);
        assert_eq!(
            r.applied, 1,
            "one open exit in the program, no generic exit"
        );
        assert_eq!(s.program.exhausted(), None);
        assert_eq!(solvable(&s.graph), Ok(()));
        let placed = &s.graph.rooms[3];
        assert_eq!(
            (placed.ty.as_str(), placed.role, placed.locked),
            ("gate", Role::Exit, false)
        );
        assert_eq!(s.graph.rooms.len(), 4);
        let c = s.graph.connections.iter().find(|c| c.b == 3).unwrap();
        assert_eq!((c.kind, c.main), (Passage::Open, false));
        assert_ne!(c.a, 2, "nothing hangs off the vault");
        // The plain rule still takes the leaf, not the exit left behind its lock.
        run(&mut s, &[entry("leaf", 1, true, "")], 3);
        assert_eq!(s.graph.rooms[4].ty, "crypt");
    }

    // AC: CE-80.2
    #[test]
    fn exit_leaf_without_program_exit_has_no_match() {
        let more = [program_room("strongroom", Role::Exit, (1, 1), true)];
        let mut s = with_vault(&more);
        let exits = parsed("role = \"exit\"");
        assert_eq!(Leaf.matches(&s, exits.as_ref()), []);
        let r = run(&mut s, &[entry("leaf", 2, true, "role = \"exit\"")], 1);
        assert_eq!((r.applied, r.short), (0, 2));
        assert_eq!(s.program.exhausted(), None);
        assert_eq!(s.graph.rooms.len(), 3);
    }

    // AC: CE-80.2
    #[test]
    fn leaf_refuses_unknown_roles() {
        let err = Leaf
            .params(&toml::from_str("from_roles = [\"attic\"]").unwrap())
            .unwrap_err();
        assert_eq!(err.parameter.as_deref(), Some("from_roles[0]"));
        let err = Leaf
            .params(&toml::from_str("role = \"goal\"").unwrap())
            .unwrap_err();
        assert_eq!(err.parameter.as_deref(), Some("role"));
        assert_eq!(Leaf.makes_keys(parsed("").as_ref(), 9), 0);
    }
}
