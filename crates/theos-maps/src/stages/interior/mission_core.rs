//! The mission engine (spec §5.6): the rule contract, the state rules rewrite, and the loop that
//! applies the recipe's `[[stage.rule]]` entries with the solvability guard.
//!
//! A rule (one module under `rules/`) lists the places it can apply ([`Rule::matches`]) and
//! rewrites the [`MissionState`] at one of them ([`Rule::apply`]). [`run_mission`] owns the
//! stream order, the weighted choice, the guard and the report, so a rule only knows its own
//! rewrite.

use std::any::Any;
use std::fmt::Debug;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::rng::StageRng;
use crate::rooms::lattice::Lattice;
use crate::rooms::solve::solvable;
use crate::rooms::{Cell, Connection, KeyId, LatticeRef, Passage, Role, Room, RoomGraph, RoomId};
use crate::stage::{ParamError, parse_table};
use crate::stages::interior::program::{CountSpec, ProgramQueue};

/// The stage id mission errors carry.
pub const STAGE: &str = "mission";

/// The most keys a mission may make: the solver holds keys in a 16-bit mask.
pub const MAX_KEYS: u32 = 16;

/// What rules rewrite: the room graph on its lattice, the program still to place, and the
/// next key number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionState {
    /// The rooms and connections so far.
    pub graph: RoomGraph,
    /// The lattice the rooms lie on.
    pub lattice: Lattice,
    /// The rooms of the program not yet placed.
    pub program: ProgramQueue,
    /// The number [`MissionState::new_key`] hands out next, from 1.
    pub next_key: KeyId,
}

impl MissionState {
    /// An empty graph on `lattice`, drawing rooms from `program`.
    #[must_use]
    pub fn new(lattice: Lattice, program: ProgramQueue) -> Self {
        let graph = RoomGraph {
            rooms: Vec::new(),
            connections: Vec::new(),
            lattice: LatticeRef {
                rows: lattice.rows,
                cols: lattice.cols,
            },
        };
        Self {
            graph,
            lattice,
            program,
            next_key: 1,
        }
    }

    /// A new key number. [`key_budget`] keeps the total at most [`MAX_KEYS`].
    pub fn new_key(&mut self) -> KeyId {
        let key = self.next_key;
        self.next_key = self.next_key.saturating_add(1);
        key
    }

    /// Appends `room` and returns its number.
    pub fn add_room(&mut self, room: Room) -> RoomId {
        let id = RoomId::try_from(self.graph.rooms.len()).unwrap_or(RoomId::MAX);
        self.graph.rooms.push(room);
        id
    }

    /// Joins rooms `x` and `y` (in either order), keeping the connections sorted by `(a, b)`.
    pub fn connect(&mut self, x: RoomId, y: RoomId, kind: Passage, main: bool) {
        let (a, b) = (x.min(y), x.max(y));
        let at = self
            .graph
            .connections
            .partition_point(|c| (c.a, c.b) < (a, b));
        self.graph
            .connections
            .insert(at, Connection { a, b, kind, main });
    }

    /// The room on `cell`, if any.
    #[must_use]
    pub fn owner(&self, cell: Cell) -> Option<RoomId> {
        self.graph
            .rooms
            .iter()
            .position(|r| r.cells.binary_search(&cell).is_ok())
            .and_then(|i| RoomId::try_from(i).ok())
    }

    /// Whether `cell` is enabled and holds no room.
    #[must_use]
    pub fn is_free(&self, cell: Cell) -> bool {
        self.lattice.is_enabled(cell) && self.owner(cell).is_none()
    }

    /// The free cells sharing an edge with `cell`, in ascending (row, col) order.
    #[must_use]
    pub fn free_neighbours(&self, cell: Cell) -> Vec<Cell> {
        let Cell { row, col } = cell;
        [
            row.checked_sub(1).map(|row| Cell { row, col }),
            col.checked_sub(1).map(|col| Cell { row, col }),
            col.checked_add(1).map(|col| Cell { row, col }),
            row.checked_add(1).map(|row| Cell { row, col }),
        ]
        .into_iter()
        .flatten()
        .filter(|&c| self.is_free(c))
        .collect()
    }

    /// Whether a connection between cells `a` and cells `b` can have a door: some pair, one cell
    /// from each, shares a wall segment of at least `MIN_SPAN` raster cells (ruling R1). Every
    /// rule that creates a connection asks this.
    #[must_use]
    pub fn joinable(&self, a: &[Cell], b: &[Cell]) -> bool {
        a.iter().any(|&x| {
            b.iter()
                .any(|&y| self.lattice.shared_segment(x, y).is_some())
        })
    }

    /// Whether rooms `a < b` are joined already.
    #[must_use]
    pub fn connected(&self, a: RoomId, b: RoomId) -> bool {
        self.graph
            .connections
            .binary_search_by(|c| (c.a, c.b).cmp(&(a, b)))
            .is_ok()
    }

    /// The cells of `rooms`, sorted: the cells of a match that names existing rooms.
    #[must_use]
    pub fn cells_of(&self, rooms: &[RoomId]) -> Vec<Cell> {
        let cells: Vec<Cell> = rooms
            .iter()
            .filter_map(|&r| self.graph.rooms.get(usize::from(r)))
            .flat_map(|r| r.cells.iter().copied())
            .collect();
        sorted(&cells)
    }

    /// The free cells that `cells` can be joined to, in ascending (row, col) order, each once.
    #[must_use]
    pub fn joinable_free(&self, cells: &[Cell]) -> Vec<Cell> {
        let mut out: Vec<Cell> = cells
            .iter()
            .flat_map(|&c| {
                self.free_neighbours(c)
                    .into_iter()
                    .filter(move |&n| self.joinable(&[c], &[n]))
            })
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// Rooms `a` and `b` as (near, far): the near room comes first in `order`, the
/// [`later_order`](crate::rooms::solve::later_order) of the graph (the fixed point's round, then
/// the hop distance from the start, then the id). A lock between them faces the far room.
#[must_use]
pub fn near_far(order: &[(u16, u32, RoomId)], a: RoomId, b: RoomId) -> (RoomId, RoomId) {
    let key = |r: RoomId| {
        order
            .get(usize::from(r))
            .copied()
            .unwrap_or((u16::MAX, u32::MAX, r))
    };
    if key(b) < key(a) { (b, a) } else { (a, b) }
}

/// `cells` sorted: the cells of a match. Shared by the rules that place a path of rooms.
#[must_use]
pub fn sorted(cells: &[Cell]) -> Vec<Cell> {
    let mut out = cells.to_vec();
    out.sort_unstable();
    out
}

/// One place a rule can apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    /// The lattice cells involved, sorted.
    pub cells: Vec<Cell>,
    /// The rooms involved, in the rule's own order.
    pub rooms: Vec<RoomId>,
    /// The weight of the weighted choice; 0 is never chosen.
    pub weight: u32,
}

/// The parsed parameters of one rule entry, held as a trait object (as P08's `StageParams`).
///
/// Every `DeserializeOwned + Debug + Send + Sync + 'static` type is one, so a rule's own params
/// type needs no impl; `Box<dyn RuleParams>` is not one, so pass `boxed.as_ref()`.
pub trait RuleParams: Any + Debug + Send + Sync {
    /// The parameters as [`Any`], for the downcast to the rule's own type.
    fn as_any(&self) -> &dyn Any;
}

impl<T: DeserializeOwned + Debug + Send + Sync + 'static> RuleParams for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The parameters `p` as the rule's own type `P`, or `None` when they are another type.
#[must_use]
pub fn params_of<P: 'static>(p: &dyn RuleParams) -> Option<&P> {
    p.as_any().downcast_ref::<P>()
}

/// Parses the parameter table of rule `rule_id` into `P`, refusing unknown keys with their path
/// (P08's stage parser, ruling L3). The errors carry `rule_id` as their stage;
/// [`run_mission`] and [`key_budget`] report them under the entry's path.
///
/// # Errors
/// [`ParamError`] for an unknown key or a value of the wrong type.
pub fn parse_params<P: DeserializeOwned>(
    rule_id: &str,
    table: &toml::Table,
) -> Result<P, ParamError> {
    parse_table(rule_id, table.clone())
}

/// One rewriting rule of the mission graph (Dormans 2017; Boris the Brave 2021), spec §5.6.
pub trait Rule: Send + Sync {
    /// The id recipes name in `rule = "…"`, `^[a-z][a-z0-9_]*$`.
    fn id(&self) -> &'static str;

    /// Parses the entry's parameters (its keys other than `rule`, `count` and `required`).
    ///
    /// # Errors
    /// [`ParamError`], best made with [`parse_params`].
    fn params(&self, table: &toml::Table) -> Result<Box<dyn RuleParams>, ParamError>;

    /// Every place the rule can apply, in a total order (by lattice cells, then room ids); the
    /// engine sorts them by `(cells, rooms)` again, keeping the rule's order among equals.
    fn matches(&self, g: &MissionState, p: &dyn RuleParams) -> Vec<Match>;

    /// Applies the rule at `m`; the engine chose `m` with its stream, and the rule draws its own
    /// choices from `rng` after it.
    fn apply(&self, g: &mut MissionState, m: &Match, p: &dyn RuleParams, rng: &mut StageRng);

    /// The most keys `count_max` applications can make, for the bound of [`MAX_KEYS`].
    fn makes_keys(&self, p: &dyn RuleParams, count_max: u32) -> u32;
}

/// The default of [`RuleEntry::required`].
fn yes() -> bool {
    true
}

/// One `[[stage.rule]]` entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleEntry {
    /// The rule id.
    pub rule: String,
    /// How many applications to ask for; 1 when left out.
    #[serde(default)]
    pub count: CountSpec,
    /// Whether missed applications count as short; `true` when left out.
    #[serde(default = "yes")]
    pub required: bool,
    /// The rule's own parameters: every other key of the entry.
    #[serde(flatten)]
    pub params: toml::Table,
}

/// What one entry did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EntryReport {
    /// The rule id.
    pub rule: String,
    /// The count drawn.
    pub asked: u32,
    /// The applications kept.
    pub applied: u32,
    /// The applications the guard undid.
    pub undone: u32,
}

/// What the mission did, for the metrics `stages.mission.applied` and `stages.mission.short`
/// and the summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MissionReport {
    /// Applications kept, over all entries.
    pub applied: u32,
    /// Applications that required entries missed.
    pub short: u32,
    /// Per entry, in recipe order.
    pub per_entry: Vec<EntryReport>,
}

/// Why the mission cannot run.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MissionError {
    /// An entry names no known rule.
    #[error("stage `mission`, rule[{entry}]: unknown rule `{rule}`")]
    UnknownRule {
        /// The entry index.
        entry: usize,
        /// The id it names.
        rule: String,
    },
    /// An entry's parameters or count are invalid, or the rules make too many keys.
    #[error(transparent)]
    Param(#[from] ParamError),
    /// A rule asked for a `start`, `goal` or `exit` room after the program had used the role
    /// up; those roles have no generic room (see [`ProgramQueue`]).
    #[error(
        "stage `mission`, rule[{entry}] (`{rule}`): the room program has no {role:?} room left"
    )]
    ProgramExhausted {
        /// The entry index.
        entry: usize,
        /// The rule id.
        rule: String,
        /// The role asked for.
        role: Role,
    },
}

/// A rule with its parsed parameters.
type Resolved<'r> = (&'r dyn Rule, Box<dyn RuleParams>);

/// The rule named `id`.
fn find<'r>(rules: &'r [Box<dyn Rule>], id: &str) -> Option<&'r dyn Rule> {
    rules.iter().find(|r| r.id() == id).map(AsRef::as_ref)
}

/// Re-homes an error of entry `i` under the stage `mission` and the path `rule[i].…`.
fn at_entry(i: usize, err: ParamError) -> ParamError {
    let path = match err.parameter {
        Some(p) => format!("rule[{i}].{p}"),
        None => format!("rule[{i}]"),
    };
    ParamError::at(STAGE, path, err.message)
}

/// Finds every entry's rule and parses its count and parameters.
fn resolve<'r>(
    rules: &'r [Box<dyn Rule>],
    entries: &[RuleEntry],
) -> Result<Vec<Resolved<'r>>, ParamError> {
    entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let rule = find(rules, &e.rule)
                .ok_or_else(|| ParamError::at(STAGE, format!("rule[{i}].rule"), "unknown rule"))?;
            e.count.check(STAGE, &format!("rule[{i}].count"))?;
            let params = rule.params(&e.params).map_err(|err| at_entry(i, err))?;
            Ok((rule, params))
        })
        .collect()
}

/// The keys the resolved entries can make at most; refuses more than [`MAX_KEYS`].
fn budget(resolved: &[Resolved<'_>], entries: &[RuleEntry]) -> Result<u32, ParamError> {
    let total = resolved
        .iter()
        .zip(entries)
        .map(|((rule, p), e)| u64::from(rule.makes_keys(p.as_ref(), e.count.max())))
        .sum::<u64>();
    if total > u64::from(MAX_KEYS) {
        return Err(ParamError::at(
            STAGE,
            "rule",
            format!("the rules can make {total} keys; at most {MAX_KEYS}"),
        ));
    }
    Ok(u32::try_from(total).unwrap_or(MAX_KEYS))
}

/// The most keys `entries` can make, each at its largest count.
///
/// # Errors
/// [`ParamError`] of the stage `mission` for an unknown rule, a bad count or parameter (at
/// `rule[i].…`), or more than [`MAX_KEYS`] keys (at `rule`).
pub fn key_budget(rules: &[Box<dyn Rule>], entries: &[RuleEntry]) -> Result<u32, ParamError> {
    budget(&resolve(rules, entries)?, entries)
}

/// Picks a match with weight in proportion to its `weight`: one draw of `below(sum)`, then a
/// walk over the matches in order. `None` without a draw when the weights sum to 0.
fn choose(matches: &[Match], rng: &mut StageRng) -> Option<usize> {
    let sum: u64 = matches.iter().map(|m| u64::from(m.weight)).sum();
    if sum == 0 {
        return None;
    }
    let mut left = rng.below(sum);
    matches.iter().position(|m| {
        let w = u64::from(m.weight);
        if left < w {
            true
        } else {
            left -= w;
            false
        }
    })
}

/// The guard: the graph validates on the lattice and is solvable. Until a start and a goal
/// exist (before `main_cycle`), there is nothing to solve and the guard holds.
fn holds(state: &MissionState) -> bool {
    let g = &state.graph;
    let has = |role: Role| g.rooms.iter().any(|r| r.role == role);
    if !(has(Role::Start) && has(Role::Goal)) {
        return true;
    }
    g.validate(&|c| state.lattice.is_enabled(c)).is_ok() && solvable(g).is_ok()
}

/// Runs the rule entries on `state` in order (spec §5.6).
///
/// All entries are checked first (rule ids, counts, parameters, the key budget), so a bad
/// recipe changes nothing. Then, per entry: its count is drawn, and per application the matches
/// are listed and sorted, one is chosen by weight, the rule applies it (and makes its own
/// draws), and the guard checks the result. A failed application is undone (the whole state is
/// restored, the stream is not) and its match dropped; the next draw comes from the rest. When
/// no match is left, the entry stops short.
///
/// # Errors
/// [`MissionError`] for an unknown rule, a bad parameter or key budget, or a used-up program
/// role without a generic room.
pub fn run_mission(
    state: &mut MissionState,
    entries: &[RuleEntry],
    rules: &[Box<dyn Rule>],
    rng: &mut StageRng,
) -> Result<MissionReport, MissionError> {
    if let Some((entry, e)) = entries
        .iter()
        .enumerate()
        .find(|(_, e)| find(rules, &e.rule).is_none())
    {
        return Err(MissionError::UnknownRule {
            entry,
            rule: e.rule.clone(),
        });
    }
    let resolved = resolve(rules, entries)?;
    budget(&resolved, entries)?;
    let mut report = MissionReport {
        applied: 0,
        short: 0,
        per_entry: Vec::with_capacity(entries.len()),
    };
    for (i, ((rule, params), e)) in resolved.iter().zip(entries).enumerate() {
        let asked = e.count.draw(rng);
        let mut done = EntryReport {
            rule: e.rule.clone(),
            asked,
            applied: 0,
            undone: 0,
        };
        while done.applied < asked {
            if !apply_once(state, *rule, params.as_ref(), rng, &mut done.undone) {
                break;
            }
            if let Some(role) = state.program.exhausted() {
                return Err(MissionError::ProgramExhausted {
                    entry: i,
                    rule: e.rule.clone(),
                    role,
                });
            }
            done.applied += 1;
        }
        report.applied += done.applied;
        if e.required {
            report.short += asked - done.applied;
        }
        report.per_entry.push(done);
    }
    Ok(report)
}

/// One application of `rule`: draws among its matches until one passes the guard. Returns
/// whether one did; each undone try adds 1 to `undone`.
fn apply_once(
    state: &mut MissionState,
    rule: &dyn Rule,
    params: &dyn RuleParams,
    rng: &mut StageRng,
    undone: &mut u32,
) -> bool {
    let mut matches = rule.matches(state, params);
    matches.sort_by(|x, y| (&x.cells, &x.rooms).cmp(&(&y.cells, &y.rooms)));
    while let Some(at) = choose(&matches, rng) {
        let m = matches.remove(at);
        let snapshot = state.clone();
        rule.apply(state, &m, params, rng);
        if state.program.exhausted().is_some() || holds(state) {
            return true;
        }
        *state = snapshot;
        *undone += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;
    use crate::rooms::lattice::Stagger;
    use crate::rooms::{Cell, Passage, Role, Room, RoomId, Shape};
    use crate::stages::interior::program::{CountSpec, ProgramQueue, ProgramRoom};
    use crate::world::WorldValue;

    /// Test rule: attaches a room of `role` (a leaf by default) to each room's lowest free
    /// neighbour cell.
    struct AddLeaf;

    #[derive(Debug, Deserialize)]
    struct AddLeafParams {
        #[serde(default = "leaf")]
        role: Role,
    }

    fn leaf() -> Role {
        Role::Leaf
    }

    impl Rule for AddLeaf {
        fn id(&self) -> &'static str {
            "add_leaf"
        }
        fn params(&self, table: &toml::Table) -> Result<Box<dyn RuleParams>, ParamError> {
            Ok(Box::new(parse_params::<AddLeafParams>(self.id(), table)?))
        }
        fn matches(&self, s: &MissionState, _: &dyn RuleParams) -> Vec<Match> {
            (0..s.graph.rooms.len())
                .filter_map(|i| {
                    let room = &s.graph.rooms[i];
                    let cell = room
                        .cells
                        .iter()
                        .flat_map(|&c| s.free_neighbours(c))
                        .min()?;
                    (!room.locked).then(|| Match {
                        cells: vec![cell],
                        rooms: vec![RoomId::try_from(i).unwrap()],
                        weight: 1,
                    })
                })
                .collect()
        }
        fn apply(&self, s: &mut MissionState, m: &Match, p: &dyn RuleParams, _: &mut StageRng) {
            let role = params_of::<AddLeafParams>(p).map_or(Role::Leaf, |p| p.role);
            let mut room = s.program.take(role);
            room.cells = m.cells.clone();
            let new = s.add_room(room);
            s.connect(m.rooms[0], new, Passage::Open, false);
        }
        fn makes_keys(&self, _: &dyn RuleParams, _: u32) -> u32 {
            0
        }
    }

    #[derive(Debug, Deserialize)]
    struct NoParams {}

    /// Test rule: locks an open connection and puts its key in the room behind it, so the
    /// application is never solvable.
    struct BrokenLock;

    impl Rule for BrokenLock {
        fn id(&self) -> &'static str {
            "broken_lock"
        }
        fn params(&self, table: &toml::Table) -> Result<Box<dyn RuleParams>, ParamError> {
            Ok(Box::new(parse_params::<NoParams>(self.id(), table)?))
        }
        fn matches(&self, s: &MissionState, _: &dyn RuleParams) -> Vec<Match> {
            s.graph
                .connections
                .iter()
                .filter(|c| c.kind == Passage::Open)
                .map(|c| Match {
                    cells: Vec::new(),
                    rooms: vec![c.a, c.b],
                    weight: 1,
                })
                .collect()
        }
        fn apply(&self, s: &mut MissionState, m: &Match, _: &dyn RuleParams, _: &mut StageRng) {
            let key = s.new_key();
            let (a, b) = (m.rooms[0], m.rooms[1]);
            for c in &mut s.graph.connections {
                if (c.a, c.b) == (a, b) {
                    c.kind = Passage::Locked(key);
                }
            }
            s.graph.rooms[usize::from(b)].keys.push(key);
        }
        fn makes_keys(&self, _: &dyn RuleParams, count_max: u32) -> u32 {
            count_max
        }
    }

    /// Test rule: places a room off the lattice, which `validate` refuses.
    struct OffLattice;

    impl Rule for OffLattice {
        fn id(&self) -> &'static str {
            "off_lattice"
        }
        fn params(&self, table: &toml::Table) -> Result<Box<dyn RuleParams>, ParamError> {
            Ok(Box::new(parse_params::<NoParams>(self.id(), table)?))
        }
        fn matches(&self, _: &MissionState, _: &dyn RuleParams) -> Vec<Match> {
            vec![Match {
                cells: vec![Cell { row: 0, col: 9 }],
                rooms: Vec::new(),
                weight: 1,
            }]
        }
        fn apply(&self, s: &mut MissionState, m: &Match, _: &dyn RuleParams, _: &mut StageRng) {
            let mut room = s.program.take(Role::Leaf);
            room.cells = m.cells.clone();
            s.add_room(room);
        }
        fn makes_keys(&self, _: &dyn RuleParams, _: u32) -> u32 {
            0
        }
    }

    fn test_rules() -> Vec<Box<dyn Rule>> {
        vec![
            Box::new(AddLeaf),
            Box::new(BrokenLock),
            Box::new(OffLattice),
        ]
    }

    fn lattice(rows: usize, cols: usize) -> Lattice {
        let mut rng = StageRng::for_key(0, "lattice");
        Lattice::new(rows * 6, cols * 6, 6, 0, Stagger::Rows, &mut rng).unwrap()
    }

    fn room(role: Role, row: u16, col: u16) -> Room {
        Room {
            ty: "hall".to_owned(),
            role,
            cells: vec![Cell { row, col }],
            shape: Shape::Rect,
            keys: Vec::new(),
            locked: false,
        }
    }

    /// A `rows` x `cols` lattice with `start` at (0, 0) and `goal` at (0, 1), joined.
    fn state(rows: usize, cols: usize, program: &[ProgramRoom]) -> MissionState {
        let mut rng = StageRng::for_key(0, "program");
        let mut s = MissionState::new(lattice(rows, cols), ProgramQueue::new(program, &mut rng));
        let start = s.add_room(room(Role::Start, 0, 0));
        let goal = s.add_room(room(Role::Goal, 0, 1));
        s.connect(start, goal, Passage::Open, true);
        s
    }

    fn entry(rule: &str, count: CountSpec, required: bool, params: &str) -> RuleEntry {
        RuleEntry {
            rule: rule.to_owned(),
            count,
            required,
            params: toml::from_str(params).unwrap(),
        }
    }

    fn json(s: &MissionState) -> String {
        serde_json::to_string(&s.graph.canonical_json().unwrap()).unwrap()
    }

    fn weighted(weights: &[u32]) -> Vec<Match> {
        weights
            .iter()
            .map(|&weight| Match {
                cells: Vec::new(),
                rooms: Vec::new(),
                weight,
            })
            .collect()
    }

    // AC: CE-80.3
    #[test]
    fn counts_and_shortfalls() {
        for (required, short) in [(true, 2), (false, 0)] {
            let mut s = state(1, 5, &[]);
            let mut rng = StageRng::for_key(1, "mission");
            let entries = [entry("add_leaf", CountSpec::Fixed(5), required, "")];
            let report = run_mission(&mut s, &entries, &test_rules(), &mut rng).unwrap();
            assert_eq!((report.applied, report.short), (3, short));
            assert_eq!(
                report.per_entry,
                [EntryReport {
                    rule: "add_leaf".to_owned(),
                    asked: 5,
                    applied: 3,
                    undone: 0,
                }]
            );
            assert_eq!(s.graph.rooms.len(), 5);
            assert_eq!(s.graph.rooms[4].ty, "leaf");
        }
    }

    // AC: CE-80.3
    #[test]
    fn guard_undoes_an_unsolvable_application() {
        let mut s = state(1, 2, &[]);
        let before = s.clone();
        let mut rng = StageRng::for_key(1, "mission");
        let entries = [entry("broken_lock", CountSpec::Fixed(1), true, "")];
        let report = run_mission(&mut s, &entries, &test_rules(), &mut rng).unwrap();
        assert_eq!((report.applied, report.short), (0, 1));
        assert_eq!(report.per_entry[0].undone, 1);
        assert_eq!(s, before);
        assert_eq!(s.next_key, 1);
    }

    // AC: CE-80.3
    #[test]
    fn guard_drops_each_failed_match_and_tries_the_rest() {
        let mut s = state(1, 3, &[]);
        let mut rng = StageRng::for_key(1, "mission");
        let entries = [
            entry("add_leaf", CountSpec::Fixed(1), true, ""),
            entry("broken_lock", CountSpec::Fixed(1), true, ""),
        ];
        let before_lock = {
            let mut t = s.clone();
            run_mission(&mut t, &entries[..1], &test_rules(), &mut rng.clone()).unwrap();
            t
        };
        let report = run_mission(&mut s, &entries, &test_rules(), &mut rng).unwrap();
        assert_eq!(report.per_entry[1].undone, 2);
        assert_eq!((report.applied, report.short), (1, 1));
        assert_eq!(s, before_lock);
    }

    // AC: CE-80.3
    #[test]
    fn guard_undoes_an_invalid_graph() {
        let mut s = state(1, 2, &[]);
        let before = s.clone();
        let mut rng = StageRng::for_key(1, "mission");
        let entries = [entry("off_lattice", CountSpec::Fixed(1), false, "")];
        let report = run_mission(&mut s, &entries, &test_rules(), &mut rng).unwrap();
        assert_eq!((report.applied, report.short), (0, 0));
        assert_eq!(report.per_entry[0].undone, 1);
        assert_eq!(s, before);
    }

    // AC: CE-80.3
    #[test]
    fn the_guard_waits_for_a_start_and_a_goal() {
        let mut s = MissionState::new(lattice(1, 3), ProgramQueue::default());
        assert!(holds(&s));
        s.add_room(room(Role::Start, 0, 0));
        s.add_room(room(Role::Path, 0, 2));
        assert!(holds(&s), "no goal yet: not checked");
        s.add_room(room(Role::Goal, 0, 1));
        assert!(!holds(&s), "rooms 1 and 2 are not reached");
        s.connect(0, 2, Passage::Open, true);
        s.connect(1, 2, Passage::Open, true);
        assert!(holds(&s));
    }

    // AC: CE-80.3
    #[test]
    fn draw_order_is_fixed() {
        let run = |seed: u64| {
            let mut s = state(4, 4, &[]);
            let mut rng = StageRng::for_key(seed, "mission");
            let entries = [entry("add_leaf", CountSpec::Range(3, 8), true, "")];
            let report = run_mission(&mut s, &entries, &test_rules(), &mut rng).unwrap();
            (json(&s), report)
        };
        assert_eq!(run(1), run(1));
        assert_ne!(run(1).0, run(2).0);
    }

    // AC: CE-80.3
    #[test]
    fn key_budget_refuses_seventeen() {
        let rules = test_rules();
        let sixteen = [
            entry("broken_lock", CountSpec::Fixed(16), true, ""),
            entry("add_leaf", CountSpec::Fixed(100), true, ""),
        ];
        assert_eq!(key_budget(&rules, &sixteen), Ok(16));
        for entries in [
            vec![entry("broken_lock", CountSpec::Range(1, 17), true, "")],
            vec![
                entry("broken_lock", CountSpec::Fixed(10), true, ""),
                entry("broken_lock", CountSpec::Fixed(7), false, ""),
            ],
        ] {
            let err = key_budget(&rules, &entries).unwrap_err();
            assert_eq!(err.stage, "mission");
            assert!(err.message.contains("17"), "{err}");
            let mut s = state(1, 2, &[]);
            let mut rng = StageRng::for_key(1, "mission");
            assert!(matches!(
                run_mission(&mut s, &entries, &rules, &mut rng),
                Err(MissionError::Param(_))
            ));
        }
    }

    // AC: CE-80.3
    #[test]
    fn bad_entries_are_refused_before_anything_runs() {
        let rules = test_rules();
        let mut s = state(1, 5, &[]);
        let before = s.clone();
        let mut rng = StageRng::for_key(1, "mission");
        let unknown = [
            entry("add_leaf", CountSpec::Fixed(1), true, ""),
            entry("nope", CountSpec::Fixed(1), true, ""),
        ];
        assert_eq!(
            run_mission(&mut s, &unknown, &rules, &mut rng),
            Err(MissionError::UnknownRule {
                entry: 1,
                rule: "nope".to_owned()
            })
        );
        let bad_key = [entry("add_leaf", CountSpec::Fixed(1), true, "colour = 1")];
        let Err(MissionError::Param(err)) = run_mission(&mut s, &bad_key, &rules, &mut rng) else {
            panic!("expected a parameter error");
        };
        assert_eq!(
            (err.stage.as_str(), err.parameter.as_deref()),
            ("mission", Some("rule[0].colour"))
        );
        let bad_value = [entry(
            "add_leaf",
            CountSpec::Fixed(1),
            true,
            "role = \"attic\"",
        )];
        let Err(MissionError::Param(err)) = run_mission(&mut s, &bad_value, &rules, &mut rng)
        else {
            panic!("expected a parameter error");
        };
        assert_eq!(err.parameter.as_deref(), Some("rule[0].role"));
        let bad_count = [entry("add_leaf", CountSpec::Range(3, 2), true, "")];
        let Err(MissionError::Param(err)) = run_mission(&mut s, &bad_count, &rules, &mut rng)
        else {
            panic!("expected a parameter error");
        };
        assert_eq!(err.parameter.as_deref(), Some("rule[0].count"));
        assert_eq!(s, before);
    }

    // AC: CE-80.3
    #[test]
    fn a_used_up_role_without_a_generic_room_fails_the_run() {
        let mut s = state(1, 3, &[]);
        let mut rng = StageRng::for_key(1, "mission");
        let entries = [entry(
            "add_leaf",
            CountSpec::Fixed(1),
            true,
            "role = \"exit\"",
        )];
        assert_eq!(
            run_mission(&mut s, &entries, &test_rules(), &mut rng),
            Err(MissionError::ProgramExhausted {
                entry: 0,
                rule: "add_leaf".to_owned(),
                role: Role::Exit
            })
        );
    }

    // AC: CE-80.3
    #[test]
    fn rules_draw_rooms_from_the_program() {
        let crypt = ProgramRoom {
            ty: "crypt".to_owned(),
            role: Role::Leaf,
            count: CountSpec::Fixed(1),
            footprint: (1, 1),
            shape: Shape::Round,
            locked: false,
        };
        let mut s = state(1, 4, &[crypt]);
        let mut rng = StageRng::for_key(1, "mission");
        let entries = [entry("add_leaf", CountSpec::Fixed(2), true, "")];
        run_mission(&mut s, &entries, &test_rules(), &mut rng).unwrap();
        let types: Vec<&str> = s.graph.rooms.iter().map(|r| r.ty.as_str()).collect();
        assert_eq!(types, ["hall", "hall", "crypt", "leaf"]);
        assert_eq!(s.graph.rooms[2].shape, Shape::Round);
    }

    // AC: CE-80.3
    #[test]
    fn choose_weighs_matches_and_skips_zero_weights() {
        let mut rng = StageRng::for_key(1, "choose");
        assert_eq!(choose(&weighted(&[]), &mut rng), None);
        assert_eq!(choose(&weighted(&[0, 0]), &mut rng), None);
        for _ in 0..50 {
            assert_eq!(choose(&weighted(&[0, 5, 0]), &mut rng), Some(1));
        }
        let mut seen = [0_u32; 2];
        for _ in 0..400 {
            seen[choose(&weighted(&[1, 3]), &mut rng).unwrap()] += 1;
        }
        assert!(seen[1] > 2 * seen[0], "{seen:?}");
    }

    // AC: CE-80.3
    #[test]
    fn state_helpers_keep_the_graph_in_order() {
        let mut s = MissionState::new(lattice(2, 2), ProgramQueue::default());
        assert_eq!(s.graph.lattice.rows, 2);
        let a = s.add_room(room(Role::Start, 0, 0));
        let b = s.add_room(room(Role::Path, 0, 1));
        let c = s.add_room(room(Role::Goal, 1, 0));
        assert_eq!((a, b, c), (0, 1, 2));
        s.connect(c, a, Passage::Open, false);
        s.connect(b, a, Passage::Open, true);
        let pairs: Vec<(RoomId, RoomId)> = s.graph.connections.iter().map(|x| (x.a, x.b)).collect();
        assert_eq!(pairs, [(0, 1), (0, 2)]);
        assert_eq!(s.owner(Cell { row: 1, col: 0 }), Some(2));
        assert!(s.is_free(Cell { row: 1, col: 1 }));
        assert!(!s.is_free(Cell { row: 0, col: 0 }));
        assert!(!s.is_free(Cell { row: 5, col: 5 }));
        assert_eq!(
            s.free_neighbours(Cell { row: 0, col: 1 }),
            [Cell { row: 1, col: 1 }]
        );
        assert_eq!((s.new_key(), s.new_key(), s.next_key), (1, 2, 3));
    }

    // AC: CE-80.3
    #[test]
    fn rule_entries_read_from_toml() {
        let e: RuleEntry = toml::from_str(
            "rule = \"add_leaf\"\ncount = [1, 3]\nrequired = false\nrole = \"exit\"",
        )
        .unwrap();
        assert_eq!(e.rule, "add_leaf");
        assert_eq!(e.count, CountSpec::Range(1, 3));
        assert!(!e.required);
        assert_eq!(
            e.params,
            toml::from_str::<toml::Table>("role = \"exit\"").unwrap()
        );
        let d: RuleEntry = toml::from_str("rule = \"leaf\"").unwrap();
        assert_eq!(
            (d.count, d.required, d.params.len()),
            (CountSpec::Fixed(1), true, 0)
        );
    }

    #[test]
    fn the_builtin_rule_list_has_unique_ids() {
        let ids: Vec<&str> = crate::stages::interior::rules::rules()
            .iter()
            .map(|r| r.id())
            .collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len());
    }
}
