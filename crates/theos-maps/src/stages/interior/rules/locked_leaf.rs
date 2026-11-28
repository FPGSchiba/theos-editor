//! Rule `locked_leaf` (spec §5.6): attaches a `locked` dead-end room behind a new lock, and puts
//! the lock's key in a room the solver reaches without it, so the key always comes before its
//! lock.

use serde::Deserialize;

use crate::rng::StageRng;
use crate::rooms::solve::fixed_point;
use crate::rooms::{Passage, Role, Room, RoomId};
use crate::stage::ParamError;
use crate::stages::interior::mission_core::{Match, MissionState, Rule, RuleParams, parse_params};
use crate::stages::interior::rules::leaf::attach_matches;

/// The rule `locked_leaf`.
pub struct LockedLeaf;

/// The parameters of `locked_leaf`: none.
#[derive(Debug, Deserialize)]
struct Params {}

impl Rule for LockedLeaf {
    fn id(&self) -> &'static str {
        "locked_leaf"
    }

    fn params(&self, table: &toml::Table) -> Result<Box<dyn RuleParams>, ParamError> {
        Ok(Box::new(parse_params::<Params>(self.id(), table)?))
    }

    /// Every free cell next to a reached, non-locked room; none before a start exists.
    fn matches(&self, s: &MissionState, _: &dyn RuleParams) -> Vec<Match> {
        let reach = fixed_point(&s.graph, true);
        attach_matches(s, |i, room| {
            !room.locked && reach.rooms.get(i).copied().unwrap_or(false)
        })
    }

    /// Places the next locked leaf of the program (else a generic one) behind a new lock, and
    /// draws the key's room among the rooms reached before, ordered by id.
    fn apply(&self, s: &mut MissionState, m: &Match, _: &dyn RuleParams, rng: &mut StageRng) {
        let (Some(&cell), Some(&host)) = (m.cells.first(), m.rooms.first()) else {
            return;
        };
        let reached: Vec<usize> = fixed_point(&s.graph, true)
            .rooms
            .iter()
            .enumerate()
            .filter_map(|(i, &r)| r.then_some(i))
            .collect();
        let len = u64::try_from(reached.len()).unwrap_or(u64::MAX);
        let Some(&key_room) = usize::try_from(rng.below(len))
            .ok()
            .and_then(|i| reached.get(i))
        else {
            return;
        };
        let key = s.new_key();
        let room = Room {
            cells: vec![cell],
            locked: true,
            ..s.program.take_with_lock(Role::Leaf, true)
        };
        let id: RoomId = s.add_room(room);
        s.connect(host, id, Passage::Locked(key), false);
        if let Some(r) = s.graph.rooms.get_mut(key_room) {
            r.keys.push(key);
        }
    }

    fn makes_keys(&self, _: &dyn RuleParams, count_max: u32) -> u32 {
        count_max
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rooms::solve::solvable;
    use crate::stages::interior::rules::testkit::{
        cell, entry, lattice, program, program_room, room, run, state_on,
    };

    // AC: CE-80.2
    #[test]
    fn locked_leaf_key_is_reachable_before_its_lock() {
        let vault = [program_room("strongroom", Role::Leaf, (1, 1), true)];
        for seed in 1..=12 {
            let mut s = state_on(lattice(6, 6), &program(&vault));
            let entries = [
                entry(
                    "main_cycle",
                    1,
                    true,
                    "side = \"south\"\ncycle = \"lock_key\"",
                ),
                entry("locked_leaf", 4, true, ""),
            ];
            let r = run(&mut s, &entries, seed);
            assert_eq!((r.applied, r.short), (5, 0));
            assert!(r.per_entry.iter().all(|e| e.undone == 0));
            assert_eq!(s.graph.validate(&|c| s.lattice.is_enabled(c)), Ok(()));
            assert_eq!(solvable(&s.graph), Ok(()));
            let locks: Vec<(usize, RoomId, u8)> = s
                .graph
                .connections
                .iter()
                .enumerate()
                .filter_map(|(i, c)| match c.kind {
                    Passage::Locked(k) if !c.main => Some((i, c.b, k)),
                    _ => None,
                })
                .collect();
            assert_eq!(locks.len(), 4);
            let mut types = Vec::new();
            for (i, leaf, key) in locks {
                let room = &s.graph.rooms[usize::from(leaf)];
                assert_eq!((room.role, room.locked), (Role::Leaf, true));
                types.push(room.ty.clone());
                let mut without = s.graph.clone();
                without.connections.remove(i);
                let reach = fixed_point(&without, true);
                let holder = s
                    .graph
                    .rooms
                    .iter()
                    .position(|r| r.keys.contains(&key))
                    .unwrap();
                assert_ne!(holder, usize::from(leaf));
                assert!(reach.rooms[holder], "key {key} lies before its lock");
            }
            assert_eq!(types.iter().filter(|t| *t == "strongroom").count(), 1);
            assert_eq!(s.next_key, 6);
        }
    }

    // AC: CE-80.2
    #[test]
    fn locked_leaf_needs_a_free_cell_by_a_reached_open_room() {
        // No start yet: nothing is reached.
        let mut empty = state_on(lattice(2, 2), &program(&[]));
        let r = run(&mut empty, &[entry("locked_leaf", 1, true, "")], 1);
        assert_eq!((r.applied, r.short), (0, 1));

        // The only free cell borders a locked room only.
        let mut s = state_on(lattice(1, 4), &program(&[]));
        let mut start = room("porch", Role::Start, cell(0, 0), false);
        start.keys.push(1);
        let start = s.add_room(start);
        let goal = s.add_room(room("altar", Role::Goal, cell(0, 1), false));
        let vault = s.add_room(room("vault", Role::Leaf, cell(0, 2), true));
        s.connect(start, goal, Passage::Open, true);
        s.connect(goal, vault, Passage::Locked(1), false);
        s.next_key = 2;
        let before = s.clone();
        let r = run(&mut s, &[entry("locked_leaf", 1, true, "")], 1);
        assert_eq!((r.applied, r.short), (0, 1));
        assert_eq!(s, before);
    }

    // AC: CE-80.2
    #[test]
    fn locked_leaf_makes_one_key_per_application() {
        let p = LockedLeaf.params(&toml::Table::new()).unwrap();
        assert_eq!(LockedLeaf.makes_keys(p.as_ref(), 7), 7);
        let err = LockedLeaf
            .params(&toml::from_str("key = 1").unwrap())
            .unwrap_err();
        assert_eq!(err.parameter.as_deref(), Some("key"));
    }
}
