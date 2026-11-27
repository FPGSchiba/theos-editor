//! Shared fixtures of the rule tests: a small lattice with an entrance and a goal in the program.

use crate::rng::StageRng;
use crate::rooms::graph::Passage;
use crate::rooms::lattice::{Lattice, Stagger};
use crate::rooms::{Cell, Role, RoomId, Shape};
use crate::stages::interior::mission_core::{MissionReport, MissionState, RuleEntry, run_mission};
use crate::stages::interior::program::{CountSpec, ProgramQueue, ProgramRoom};

/// A `rows` x `cols` lattice of 6-cell blocks, every cell enabled, no offsets.
pub fn lattice(rows: usize, cols: usize) -> Lattice {
    let mut rng = StageRng::for_key(0, "lattice");
    Lattice::new(rows * 6, cols * 6, 6, 0, Stagger::Rows, &mut rng).unwrap()
}

/// One program room (synthetic fixture types).
pub fn program_room(ty: &str, role: Role, footprint: (u8, u8), locked: bool) -> ProgramRoom {
    ProgramRoom {
        ty: ty.to_owned(),
        role,
        count: CountSpec::Fixed(1),
        footprint,
        shape: Shape::Rect,
        locked,
    }
}

/// A program with one entrance (`porch`) and one goal (`altar`), plus `more`.
pub fn program(more: &[ProgramRoom]) -> Vec<ProgramRoom> {
    let mut p = vec![
        program_room("porch", Role::Start, (1, 1), false),
        program_room("altar", Role::Goal, (1, 1), false),
    ];
    p.extend_from_slice(more);
    p
}

/// An empty mission state on `lattice` with `program`.
pub fn state_on(lattice: Lattice, program: &[ProgramRoom]) -> MissionState {
    let mut rng = StageRng::for_key(0, "program");
    MissionState::new(lattice, ProgramQueue::new(program, &mut rng))
}

/// One rule entry.
pub fn entry(rule: &str, count: u32, required: bool, params: &str) -> RuleEntry {
    RuleEntry {
        rule: rule.to_owned(),
        count: CountSpec::Fixed(count),
        required,
        params: toml::from_str(params).unwrap(),
    }
}

/// Runs `entries` with the built-in rules and the stream of `seed`.
pub fn run(s: &mut MissionState, entries: &[RuleEntry], seed: u64) -> MissionReport {
    let mut rng = StageRng::for_key(seed, "mission");
    run_mission(s, entries, &super::rules(), &mut rng).unwrap()
}

/// The room with `role` (the first one).
pub fn room_of(s: &MissionState, role: Role) -> RoomId {
    let i = s.graph.rooms.iter().position(|r| r.role == role).unwrap();
    RoomId::try_from(i).unwrap()
}

/// The two arcs of the main cycle, each from the start side to the goal side, with the passage
/// of its last connection into the goal.
pub fn arcs(s: &MissionState) -> Vec<(Vec<RoomId>, Passage)> {
    let (start, goal) = (room_of(s, Role::Start), room_of(s, Role::Goal));
    let g = &s.graph;
    g.neighbours(goal)
        .filter(|(_, c)| c.main)
        .map(|(last, c)| {
            let mut arc = vec![last];
            let mut prev = goal;
            let mut at = last;
            while at != start {
                let next = g
                    .neighbours(at)
                    .find(|(n, c)| c.main && *n != prev)
                    .map(|(n, _)| n)
                    .unwrap();
                prev = at;
                at = next;
                if at != start {
                    arc.push(at);
                }
            }
            arc.reverse();
            (arc, c.kind)
        })
        .collect()
}

/// A hand-made cell.
pub fn cell(row: u16, col: u16) -> Cell {
    Cell { row, col }
}

/// A hand-made 1x1 room (synthetic fixture types).
pub fn room(ty: &str, role: Role, at: Cell, locked: bool) -> crate::rooms::Room {
    crate::rooms::Room {
        ty: ty.to_owned(),
        role,
        cells: vec![at],
        shape: Shape::Rect,
        keys: Vec::new(),
        locked,
    }
}
