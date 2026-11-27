//! Rule `main_cycle` (spec §5.6): the entrance on a rim side, the goal near the lattice centre,
//! and a loop of two disjoint arcs of `path` rooms between them.
//!
//! The arcs are found by a breadth-first search over free enabled cells with neighbours in
//! (N, E, S, W) order, so equal-length routes are decided by that order, not the stream. Arc A
//! is the shortest route; when it has fewer than `min_arc` rooms the start is not offered
//! (no detour is grown). Arc B is the shortest route that avoids arc A. [`Rule::matches`] offers
//! only the starts from which a goal and both arcs can be placed, because [`Rule::apply`] cannot
//! fail.

use std::collections::VecDeque;

use serde::Deserialize;

use crate::rng::StageRng;
use crate::rooms::graph::type_ok;
use crate::rooms::lattice::Lattice;
use crate::rooms::{Cell, Passage, Role, Room, RoomId, Shape};
use crate::stage::ParamError;
use crate::stages::interior::mission_core::{
    Match, MissionState, Rule, RuleParams, params_of, parse_params,
};

/// The rule `main_cycle`.
pub struct MainCycle;

/// The rim side the entrance lies on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Side {
    North,
    East,
    South,
    West,
}

/// What the two arcs are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Cycle {
    /// Arc A is locked into the goal, its key lies on arc B, arc B ends in a drop from the goal.
    LockKey,
    /// Both arcs open.
    TwoRoutes,
    /// Arc A has exactly `min_arc` rooms, the first of type `risk_type`; arc B is longer.
    RiskReward,
}

/// The default of [`Params::min_arc`].
fn two() -> u32 {
    2
}

/// The parameters of `main_cycle`.
#[derive(Debug, Deserialize)]
struct Params {
    side: Side,
    cycle: Cycle,
    #[serde(default = "two")]
    min_arc: u32,
    #[serde(default)]
    risk_type: Option<String>,
}

/// Where the goal and the two arcs go for one start.
struct Layout {
    goal: Vec<Cell>,
    a: Vec<Cell>,
    b: Vec<Cell>,
}

/// The cells still free for the cycle: enabled and without a room, row-major.
#[derive(Clone)]
struct Free {
    rows: u16,
    cols: u16,
    cells: Vec<bool>,
}

impl Free {
    fn of(s: &MissionState) -> Self {
        Self {
            rows: s.lattice.rows,
            cols: s.lattice.cols,
            cells: s.lattice.cells().map(|c| s.is_free(c)).collect(),
        }
    }

    fn index(&self, c: Cell) -> Option<usize> {
        (c.row < self.rows && c.col < self.cols)
            .then(|| usize::from(c.row) * usize::from(self.cols) + usize::from(c.col))
    }

    fn get(&self, c: Cell) -> bool {
        self.index(c)
            .and_then(|i| self.cells.get(i).copied())
            .unwrap_or(false)
    }

    fn take(&mut self, c: Cell) {
        if let Some(slot) = self.index(c).and_then(|i| self.cells.get_mut(i)) {
            *slot = false;
        }
    }

    /// The free neighbours of `c` in (N, E, S, W) order.
    fn neighbours(&self, c: Cell) -> impl Iterator<Item = Cell> + '_ {
        let Cell { row, col } = c;
        [
            row.checked_sub(1).map(|row| Cell { row, col }),
            col.checked_add(1).map(|col| Cell { row, col }),
            row.checked_add(1).map(|row| Cell { row, col }),
            col.checked_sub(1).map(|col| Cell { row, col }),
        ]
        .into_iter()
        .flatten()
        .filter(|&n| self.get(n))
    }
}

/// The rim of the enabled shape on `side` (ruling R9): per column (north, south) or row (east,
/// west), the enabled cell with no enabled cell further towards `side`, kept when it is free; in
/// (row, col) order. The lattice's edge row need not be enabled, so round and ragged envelopes
/// have a rim too.
fn rim_cells(lattice: &Lattice, free: &Free, side: Side) -> Vec<Cell> {
    let (rows, cols) = (lattice.rows, lattice.cols);
    let column = |col: u16| (0..rows).map(move |row| Cell { row, col });
    let row = |row: u16| (0..cols).map(move |col| Cell { row, col });
    let enabled = |c: &Cell| lattice.is_enabled(*c);
    let mut cells: Vec<Cell> = match side {
        Side::North => (0..cols).filter_map(|c| column(c).find(enabled)).collect(),
        Side::South => (0..cols)
            .filter_map(|c| column(c).rev().find(enabled))
            .collect(),
        Side::West => (0..rows).filter_map(|r| row(r).find(enabled)).collect(),
        Side::East => (0..rows)
            .filter_map(|r| row(r).rev().find(enabled))
            .collect(),
    };
    cells.retain(|&c| free.get(c));
    cells.sort_unstable();
    cells
}

/// The free blocks of `footprint` (rows, cols) whose centre is nearest the lattice centre by
/// integer squared distance (ruling R11), in (row, col) order of the top-left cell. In the doubled
/// frame a block's centre is `2 * top_left + side - 1` per axis and the lattice centre is
/// `lattice side - 1`. Each block's cells are sorted.
fn goal_blocks(free: &Free, footprint: (u8, u8)) -> Vec<Vec<Cell>> {
    let (h, w) = (u16::from(footprint.0), u16::from(footprint.1));
    let squared = |top_left: u16, side: u16, lattice_side: u16| {
        let d = (2 * i64::from(top_left) + i64::from(side) - 1) - (i64::from(lattice_side) - 1);
        d * d
    };
    let mut best: Option<i64> = None;
    let mut blocks = Vec::new();
    for row in 0..free.rows {
        for col in 0..free.cols {
            let cells: Vec<Cell> = (row..row.saturating_add(h))
                .flat_map(|r| (col..col.saturating_add(w)).map(move |c| Cell { row: r, col: c }))
                .collect();
            if !cells.iter().all(|&c| free.get(c)) {
                continue;
            }
            let d = squared(row, h, free.rows) + squared(col, w, free.cols);
            if best.is_none_or(|b| d < b) {
                best = Some(d);
                blocks.clear();
            }
            if best == Some(d) {
                blocks.push(cells);
            }
        }
    }
    blocks
}

/// The shortest route of free cells from a neighbour of `start` to a neighbour of a `goal`
/// cell: breadth-first, neighbours in (N, E, S, W) order.
fn shortest_arc(free: &Free, start: Cell, goal: &[Cell]) -> Option<Vec<Cell>> {
    let n = free.cells.len();
    let mut prev: Vec<Option<Cell>> = vec![None; n];
    let mut seen = vec![false; n];
    let mut queue = VecDeque::new();
    for c in free.neighbours(start) {
        let i = free.index(c)?;
        seen[i] = true;
        queue.push_back(c);
    }
    while let Some(c) = queue.pop_front() {
        if goal.iter().any(|&g| g.touches(c)) {
            let mut arc = vec![c];
            let mut at = c;
            while let Some(p) = free.index(at).and_then(|i| prev[i]) {
                arc.push(p);
                at = p;
            }
            arc.reverse();
            return Some(arc);
        }
        for next in free.neighbours(c) {
            let i = free.index(next)?;
            if !seen[i] {
                seen[i] = true;
                prev[i] = Some(c);
                queue.push_back(next);
            }
        }
    }
    None
}

/// The two arcs between `start` and `goal`, if both can be placed as `p` asks.
fn arcs(free: &Free, start: Cell, goal: &[Cell], p: &Params) -> Option<(Vec<Cell>, Vec<Cell>)> {
    let min = usize::try_from(p.min_arc).unwrap_or(usize::MAX);
    let risk = p.cycle == Cycle::RiskReward;
    let mut f = free.clone();
    for &c in goal {
        f.take(c);
    }
    let a = shortest_arc(&f, start, goal)?;
    if a.len() < min || (risk && a.len() != min) {
        return None;
    }
    for &c in &a {
        f.take(c);
    }
    let b = shortest_arc(&f, start, goal)?;
    if b.len() < min || (risk && b.len() <= a.len()) {
        return None;
    }
    Some((a, b))
}

/// Every layout from `start`: the nearest goal blocks from which both arcs can be placed.
fn layouts(s: &MissionState, start: Cell, p: &Params) -> Vec<Layout> {
    let footprint = s.program.peek(Role::Goal).map_or((1, 1), |r| r.footprint);
    let mut free = Free::of(s);
    free.take(start);
    goal_blocks(&free, footprint)
        .into_iter()
        .filter_map(|goal| arcs(&free, start, &goal, p).map(|(a, b)| Layout { goal, a, b }))
        .collect()
}

/// Places one arc of 1x1 `path` rooms from `from`, each joined open to the one before; the
/// first room gets the type `first_ty` when given. Returns the rooms in order.
fn place_arc(
    s: &mut MissionState,
    from: RoomId,
    cells: &[Cell],
    first_ty: Option<&str>,
) -> Vec<RoomId> {
    let mut prev = from;
    let mut ids = Vec::with_capacity(cells.len());
    for (i, &c) in cells.iter().enumerate() {
        let room = match first_ty.filter(|_| i == 0) {
            Some(ty) => Room {
                ty: ty.to_owned(),
                role: Role::Path,
                cells: vec![c],
                shape: Shape::Rect,
                keys: Vec::new(),
                locked: false,
            },
            None => Room {
                cells: vec![c],
                ..s.program.take(Role::Path)
            },
        };
        let id = s.add_room(room);
        s.connect(prev, id, Passage::Open, true);
        ids.push(id);
        prev = id;
    }
    ids
}

/// Builds the cycle of `layout` from `start`.
fn build(s: &mut MissionState, start: Cell, layout: Layout, p: &Params) {
    let start_room = Room {
        cells: vec![start],
        ..s.program.take(Role::Start)
    };
    let start_id = s.add_room(start_room);
    let goal_room = Room {
        cells: layout.goal,
        ..s.program.take(Role::Goal)
    };
    let goal_id = s.add_room(goal_room);
    let risk = p
        .risk_type
        .as_deref()
        .filter(|_| p.cycle == Cycle::RiskReward);
    let a = place_arc(s, start_id, &layout.a, risk);
    let b = place_arc(s, start_id, &layout.b, None);
    let (Some(&a_end), Some(&b_end)) = (a.last(), b.last()) else {
        return;
    };
    if p.cycle == Cycle::LockKey {
        let key = s.new_key();
        s.connect(a_end, goal_id, Passage::Locked(key), true);
        if let Some(room) = b
            .get(b.len() / 2)
            .and_then(|&id| s.graph.rooms.get_mut(usize::from(id)))
        {
            room.keys.push(key);
        }
        s.connect(b_end, goal_id, Passage::Drop { from: goal_id }, true);
    } else {
        s.connect(a_end, goal_id, Passage::Open, true);
        s.connect(b_end, goal_id, Passage::Open, true);
    }
}

impl Rule for MainCycle {
    fn id(&self) -> &'static str {
        "main_cycle"
    }

    fn params(&self, table: &toml::Table) -> Result<Box<dyn RuleParams>, ParamError> {
        let p: Params = parse_params(self.id(), table)?;
        if p.min_arc == 0 {
            return Err(ParamError::at(self.id(), "min_arc", "must be at least 1"));
        }
        match (p.cycle, p.risk_type.as_deref()) {
            (Cycle::RiskReward, None) => Err(ParamError::at(
                self.id(),
                "risk_type",
                "is required when cycle = \"risk_reward\"",
            )),
            (Cycle::RiskReward, Some(ty)) if !type_ok(ty) => Err(ParamError::at(
                self.id(),
                "risk_type",
                "must match ^[a-z][a-z0-9-]*$",
            )),
            (Cycle::LockKey | Cycle::TwoRoutes, Some(_)) => Err(ParamError::at(
                self.id(),
                "risk_type",
                "is only allowed when cycle = \"risk_reward\"",
            )),
            _ => Ok(Box::new(p)),
        }
    }

    /// The rim cells from which a cycle can be built, weight 1 each; none once a start exists.
    fn matches(&self, s: &MissionState, p: &dyn RuleParams) -> Vec<Match> {
        let Some(p) = params_of::<Params>(p) else {
            return Vec::new();
        };
        if s.graph.rooms.iter().any(|r| r.role == Role::Start) {
            return Vec::new();
        }
        rim_cells(&s.lattice, &Free::of(s), p.side)
            .into_iter()
            .filter(|&c| !layouts(s, c, p).is_empty())
            .map(|c| Match {
                cells: vec![c],
                rooms: Vec::new(),
                weight: 1,
            })
            .collect()
    }

    /// Draws the goal block among the nearest feasible ones, then builds the cycle.
    fn apply(&self, s: &mut MissionState, m: &Match, p: &dyn RuleParams, rng: &mut StageRng) {
        let (Some(p), Some(&start)) = (params_of::<Params>(p), m.cells.first()) else {
            return;
        };
        let mut options = layouts(s, start, p);
        if options.is_empty() {
            return;
        }
        let len = u64::try_from(options.len()).unwrap_or(u64::MAX);
        let pick = usize::try_from(rng.below(len)).unwrap_or(0);
        if pick < options.len() {
            let layout = options.swap_remove(pick);
            build(s, start, layout, p);
        }
    }

    fn makes_keys(&self, p: &dyn RuleParams, count_max: u32) -> u32 {
        match params_of::<Params>(p) {
            Some(p) if p.cycle == Cycle::LockKey => count_max.min(1),
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rooms::solve::solvable;
    use crate::stages::interior::rules::testkit::{
        arcs as arcs_of, entry, lattice, program, program_room, room_of, run, state_on,
    };
    use crate::world::WorldValue;

    const LOCK_KEY: &str = "side = \"south\"\ncycle = \"lock_key\"";

    fn cycle_state() -> MissionState {
        state_on(lattice(6, 6), &program(&[]))
    }

    fn valid_and_solvable(s: &MissionState) {
        assert_eq!(s.graph.validate(&|c| s.lattice.is_enabled(c)), Ok(()));
        assert_eq!(solvable(&s.graph), Ok(()));
    }

    // AC: CE-80.2
    #[test]
    fn lock_key_cycle_is_solvable_and_shaped() {
        for seed in 1..=16 {
            let mut s = cycle_state();
            let r = run(&mut s, &[entry("main_cycle", 1, true, LOCK_KEY)], seed);
            assert_eq!((r.applied, r.short, r.per_entry[0].undone), (1, 0, 0));
            valid_and_solvable(&s);
            let g = &s.graph;
            let (start, goal) = (room_of(&s, Role::Start), room_of(&s, Role::Goal));
            assert_eq!(g.rooms[usize::from(start)].cells[0].row, 5);
            assert_eq!(g.rooms[usize::from(start)].ty, "porch");
            assert_eq!(g.rooms[usize::from(goal)].ty, "altar");
            let arcs = arcs_of(&s);
            assert_eq!(arcs.len(), 2);
            let lock = arcs
                .iter()
                .find_map(|(arc, kind)| match kind {
                    Passage::Locked(k) => Some((arc, *k)),
                    _ => None,
                })
                .unwrap();
            let drop = arcs
                .iter()
                .find(|(_, kind)| *kind == Passage::Drop { from: goal })
                .map(|(arc, _)| arc)
                .unwrap();
            let (_, key) = lock;
            let holders: Vec<RoomId> = (0..g.rooms.len())
                .filter(|&i| g.rooms[i].keys.contains(&key))
                .map(|i| RoomId::try_from(i).unwrap())
                .collect();
            assert_eq!(holders.len(), 1);
            assert!(drop.contains(&holders[0]), "the key lies on arc B");
            for (arc, _) in &arcs {
                assert!(arc.len() >= 2, "{arc:?}");
                assert!(
                    arc.iter()
                        .all(|&r| g.rooms[usize::from(r)].role == Role::Path)
                );
            }
            assert_eq!(g.rooms.len(), 2 + arcs[0].0.len() + arcs[1].0.len());
            assert_eq!(s.next_key, 2);
        }
    }

    // AC: CE-80.2
    #[test]
    fn two_routes_has_no_lock() {
        for seed in 1..=8 {
            let mut s = cycle_state();
            let params = "side = \"north\"\ncycle = \"two_routes\"\nmin_arc = 3";
            let r = run(&mut s, &[entry("main_cycle", 1, true, params)], seed);
            assert_eq!((r.applied, r.short), (1, 0));
            valid_and_solvable(&s);
            assert!(
                s.graph
                    .connections
                    .iter()
                    .all(|c| c.kind == Passage::Open && c.main)
            );
            let start = room_of(&s, Role::Start);
            assert_eq!(s.graph.rooms[usize::from(start)].cells[0].row, 0);
            let arcs = arcs_of(&s);
            assert_eq!(arcs.len(), 2);
            assert!(arcs.iter().all(|(arc, _)| arc.len() >= 3));
            assert_eq!(s.next_key, 1);
        }
    }

    // AC: CE-80.2
    #[test]
    fn risk_reward_short_arc_has_the_risk_type() {
        for seed in 1..=8 {
            let mut s = cycle_state();
            let params =
                "side = \"west\"\ncycle = \"risk_reward\"\nmin_arc = 2\nrisk_type = \"trap-hall\"";
            let r = run(&mut s, &[entry("main_cycle", 1, true, params)], seed);
            assert_eq!((r.applied, r.short), (1, 0));
            valid_and_solvable(&s);
            assert!(s.graph.connections.iter().all(|c| c.kind == Passage::Open));
            let arcs = arcs_of(&s);
            let ty = |r: RoomId| s.graph.rooms[usize::from(r)].ty.as_str();
            let (risky, safe): (Vec<_>, Vec<_>) =
                arcs.iter().partition(|(arc, _)| ty(arc[0]) == "trap-hall");
            assert_eq!((risky.len(), safe.len()), (1, 1));
            assert_eq!(risky[0].0.len(), 2);
            assert!(safe[0].0.len() > 2);
            assert!(safe[0].0.iter().all(|&r| ty(r) == "path"));
        }
    }

    // AC: CE-80.2
    #[test]
    fn main_cycle_params_are_checked() {
        let parse = |text: &str| MainCycle.params(&toml::from_str(text).unwrap());
        let bad = [
            ("side = \"south\"\ncycle = \"risk_reward\"", "risk_type"),
            (
                "side = \"south\"\ncycle = \"risk_reward\"\nrisk_type = \"Trap\"",
                "risk_type",
            ),
            (
                "side = \"south\"\ncycle = \"two_routes\"\nrisk_type = \"trap\"",
                "risk_type",
            ),
            (
                "side = \"south\"\ncycle = \"lock_key\"\nmin_arc = 0",
                "min_arc",
            ),
            ("side = \"up\"\ncycle = \"lock_key\"", "side"),
            ("side = \"south\"\ncycle = \"maze\"", "cycle"),
        ];
        for (text, path) in bad {
            let err = parse(text).unwrap_err();
            assert_eq!(err.parameter.as_deref(), Some(path), "{text}: {err}");
        }
        let p = parse(LOCK_KEY).unwrap();
        assert_eq!(MainCycle.makes_keys(p.as_ref(), 3), 1);
        assert_eq!(MainCycle.makes_keys(p.as_ref(), 0), 0);
        let open = parse("side = \"south\"\ncycle = \"two_routes\"").unwrap();
        assert_eq!(MainCycle.makes_keys(open.as_ref(), 3), 0);
    }

    // AC: CE-80.2
    #[test]
    fn main_cycle_reports_short_without_rim_cell() {
        // Envelopes whose enabled cells cannot hold two disjoint arcs: one enabled row in a
        // 6x6 lattice, and a lattice of one row.
        let mut one_row = lattice(6, 6);
        for (i, on) in one_row.enabled.iter_mut().enumerate() {
            *on = i / 6 == 2;
        }
        for l in [one_row, lattice(1, 6)] {
            for side in ["south", "north", "east", "west"] {
                let mut s = state_on(l.clone(), &program(&[]));
                let params = format!(
                    "side = \"{side}\"
cycle = \"lock_key\""
                );
                let r = run(&mut s, &[entry("main_cycle", 1, true, &params)], 1);
                assert_eq!((r.applied, r.short), (0, 1), "{side}");
                assert_eq!(s.graph.rooms.len(), 0);
            }
        }
    }

    /// An 8x8 lattice with only a circle of cells enabled; rows 0 and 7 are fully disabled.
    fn round_lattice() -> Lattice {
        let mut l = lattice(8, 8);
        for (i, on) in l.enabled.iter_mut().enumerate() {
            let (r, c) = (i64::try_from(i / 8).unwrap(), i64::try_from(i % 8).unwrap());
            *on = (2 * r - 7).pow(2) + (2 * c - 7).pow(2) <= 49;
        }
        l
    }

    // AC: CE-80.2
    #[test]
    fn main_cycle_starts_on_a_round_envelope_rim() {
        let l = round_lattice();
        assert!((0..8).all(|col| !l.is_enabled(Cell { row: 0, col })));
        assert!((0..8).all(|col| !l.is_enabled(Cell { row: 7, col })));
        for seed in 1..=8 {
            let mut s = state_on(l.clone(), &program(&[]));
            let r = run(&mut s, &[entry("main_cycle", 1, true, LOCK_KEY)], seed);
            assert_eq!((r.applied, r.short), (1, 0));
            valid_and_solvable(&s);
            let start = s.graph.rooms[usize::from(room_of(&s, Role::Start))].cells[0];
            assert!(s.lattice.is_enabled(start));
            assert!(
                (start.row + 1..8).all(|row| !s.lattice.is_enabled(Cell {
                    row,
                    col: start.col
                })),
                "{start:?} lies on the south rim"
            );
            assert!(start.row < 7);
        }
    }

    // AC: CE-80.2
    #[test]
    fn main_cycle_applies_once() {
        let mut s = cycle_state();
        let r = run(&mut s, &[entry("main_cycle", 2, true, LOCK_KEY)], 3);
        assert_eq!((r.applied, r.short), (1, 1));
        assert_eq!(
            s.graph
                .rooms
                .iter()
                .filter(|r| r.role == Role::Start)
                .count(),
            1
        );
    }

    // AC: CE-80.2
    #[test]
    fn goal_takes_its_program_footprint() {
        let prog = [
            program_room("porch", Role::Start, (1, 1), false),
            program_room("choir-hall", Role::Goal, (2, 2), false),
        ];
        for seed in 1..=8 {
            let mut s = state_on(lattice(6, 6), &prog);
            let r = run(&mut s, &[entry("main_cycle", 1, true, LOCK_KEY)], seed);
            assert_eq!((r.applied, r.short), (1, 0));
            valid_and_solvable(&s);
            let goal = &s.graph.rooms[usize::from(room_of(&s, Role::Goal))];
            assert_eq!(goal.ty, "choir-hall");
            // The block's centre is the lattice centre: the centre four cells.
            let centre: Vec<Cell> = (2..4)
                .flat_map(|row| (2..4).map(move |col| Cell { row, col }))
                .collect();
            assert_eq!(goal.cells, centre);
        }
    }

    // AC: CE-80.2
    #[test]
    fn main_cycle_is_deterministic_and_drawn() {
        let json = |seed: u64| {
            let mut s = state_on(lattice(20, 20), &program(&[]));
            let r = run(&mut s, &[entry("main_cycle", 1, true, LOCK_KEY)], seed);
            assert_eq!((r.applied, r.short), (1, 0));
            valid_and_solvable(&s);
            serde_json::to_string(&s.graph.canonical_json().unwrap()).unwrap()
        };
        assert_eq!(json(1), json(1));
        let distinct: std::collections::BTreeSet<String> = (1..=6).map(json).collect();
        assert!(distinct.len() > 1);
    }
}
