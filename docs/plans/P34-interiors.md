# P34 Interiors Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Interior maps on one level in `theos-maps`: a cyclic mission graph grown by rules on a lattice, rooms that fill their lattice cells, cover, a playability analysis, interior features, renders, three built-in recipes and a ruin's child map.

**Architecture:** A new module `rooms/` holds the shared `room_graph` value, the lattice, the solver and graph measures as pure code. Five interior stages (`envelope`, `mission`, `space`, `cover`, `playability`) wrap pure cores behind P08's `TypedStage`. The mission's rules are one module each behind a `Rule` trait. Features, package layers and renders plug into the per-kind tables of the child-map plumbing that P29 builds (spec §5.2).

**Tech Stack:** Rust 2024 (toolchain 1.99.1); P08's `theos-maps` (Stage contract, `Grid`, `StageRng`, `kernels::{labels, paths, noise}`, features, renders on `tiny-skia`); `serde`, `toml`, `serde_json`; `thiserror`, `tracing`.

**Spec:** `docs/specs/phases/P34-interiors.md` (read it with this plan; § numbers below refer to it).

## Global Constraints

Umbrella G1–G20, verbatim:

- **G1** Repositories: `theos-editor` (this repo) holds the editor and the map generator
  (`theos-map-format`, `theos-maps`). `tower-of-theos` holds the content project, its map recipes
  and the blockout generator. `Theos-Engine` is never a dependency, except behind the cargo
  feature `engine`, which no phase here builds; the engine may depend on the map crates.
- **G2** Platforms: Windows 11 x64 (primary), Linux x64, and macOS 13 or later on Apple silicon
  and Intel (universal builds). Every PR is checked on Linux; Windows and macOS on Apple silicon
  are tested weekly, on demand (a label on the PR) and before every release, and macOS on Intel
  on demand and before every release (`docs/gate.md` §1).
- **G3** Rust: stable toolchain pinned in `rust-toolchain.toml`, edition 2024, `unsafe_code`
  forbidden, `Cargo.lock` committed, all builds `--locked`.
- **G4** Crates: `theos-content`, `theos-vcs`, `theos-studio`, `theos-map-format`, `theos-maps`
  (libs); `theos`, `theos-mcp`, `theos-editor` (bins); `xtask`. Paths `crates/<name>`, `xtask/`. UI in `editor-ui/`
  (Svelte 5, TypeScript, pnpm).
- **G5** Content files follow `docs/specs/data-model.md`; the restricted YAML parser is ours;
  any edit leaves untouched bytes identical.
- **G6** Git only through the `git` command line; credentials only through the user's
  credential helper.
- **G7** MCP: stdio only, `rmcp`, tool names `^[a-z][a-z0-9_]*$`, agents only propose.
- **G8** `never-confirm`, `no-overshoot`, `inworld-vocab` are always blocking; nothing lowers them.
- **G9** Text of a truth with target `never` never appears in an LLM prompt.
- **G10** Binaries never enter git; references are `path@sha256:<hex>` relative to the binaries
  folder.
- **G11** Generators only through `docs/generators.md`, the built-in map generator included; no
  generator-specific field in the data model; the UI works for any generator that produces a kind.
- **G12** Network only to ComfyUI and the configured LLM endpoint, both opt-in; no telemetry.
- **G13** UI uses `design/theos-ui/` and follows `docs/ui/`; shortcuts use Ctrl on Windows and
  Linux and Cmd on macOS, from one keymap.
- **G14** Settings in three layers (part default → user → project); secrets in the OS
  credential store.
- **G15** Every AC below has at least one test tagged with its id (`docs/gate.md` §4).
- **G16** Errors in library crates are typed (`thiserror`); `anyhow` only in binaries. Logging
  with `tracing`.
- **G17** Branches, commits, PR titles, versions and tags follow `CONTRIBUTING.md`; releases
  follow `docs/release.md`. One product version for all crates; tags only from the pipeline.
- **G18** UI references are the HTML files in `docs/ui/screens/` (built only from
  `design/theos-ui/`); the PNG files next to them are previews rendered from that HTML.
- **G19** Positions on maps are fractions of a map version's frame (x east, y south, origin
  north-west); metres are derived from the frame. Pins and cameras live in the map record's body,
  never on the placed record (`docs/specs/data-model.md` §9.6–9.7).
- **G20** Map generation is deterministic: the same generator version, recipe, seed and inputs
  give the same package bytes and layers on every platform of G2. Canon maps are baked into
  packages (`docs/map-package.md`) and shipped, never regenerated in the game (decision 0003).

Phase ACs, verbatim:

**CE-80 theos-maps interiors**
- CE-80.1 Interior maps run from built-in recipes (`choir-hall-ruin` among them), with the renders
  `base`, `blueprint` and `graph`, and packages whose layers and features follow
  `docs/map-package.md` §4.4 and §5 (rooms as polygons, doors, connections, the critical route).
- CE-80.2 For seeds 1–8 of each built-in interior recipe, every interior passes the playability
  analysis: every room carved and reachable, lock and key solvable with one-way drops.
- CE-80.3 The interior stages read rooms and connections from a `room_graph` value; a test stage
  that writes a hand-made graph yields an interior that passes the playability analysis.
- CE-80.4 A ruin's child map runs from a parent package and its ruin feature, with the frame and
  repeatability of CE-78.2.
- CE-80.5 The golden hashes of CE-77.1 cover an interior and a ruin's child map.

Project rules every task keeps (CLAUDE.md):
- Engineering directives 1–4 (one job per module; existing tools first; change approach when fixes leak; the gate catches shortcuts).
- No `unwrap`/`expect` outside tests. Public items documented. Tests carry `// AC: <id>` where they prove an AC.
- No protected path changes on this branch (`docs/**` except `docs/plans/` and `docs/status.md`).
- Subagents never push.

## The dependency on P29's plumbing

Spec §5.2 is built by P29 (agreed 27 Nov 2026: P29's build starts first). It provides:
`MapKind::Interior`; declared recipe inputs with `parent_types`; `from`; `child_frame`;
`parent.*` values; per-kind package layer tables, standard renders, feature types and id rules;
`Geometry::None`; golden runs with `from`; and the `VERSION` rule.

- **Part A** (Tasks 1–11) needs none of it and starts on `main` now.
- **Part B** (Tasks 12–17) starts once P29's plumbing commits exist. Merge P29's branch into
  `phase/P34-interiors` if it is not on `main` yet (merge, never rebase), or merge `main` once it
  is. The P29 session announces both moments.
- Names that P29 fixes are written below as in spec §5.2. If P29's code names them differently,
  use P29's names and keep the behaviour.
- If P29's plumbing is not committed when Part A is done, stop and report it under "Needs Jann".
  Do not build a second copy.

## Review Focus

The five uncovered input classes most likely to bite, each pinned by a test in its owning task:

1. A ragged or round envelope leaves too few enabled cells on the start's rim side, so `main_cycle` cannot place `start`. It must fail as a named shortfall, not a panic or an empty map (Task 6: `main_cycle_reports_short_without_rim_cell`).
2. `merge_hall` grows over the start, the goal or a key room and loses its role or its key (Task 8: `merge_hall_keeps_roles_and_keys`).
3. Two halls meet only at a corner, so there is no shared wall segment, yet a connection asks for a door (Task 9: `space_refuses_corner_only_connection`; the rules never produce it: `close_loop_needs_shared_segment`).
4. A round or ragged room in a single block cuts its own door stub (Task 9: `round_and_ragged_rooms_keep_door_paths`).
5. A child run whose ruin sits at the parent's corner, so the frame reaches past the parent (Task 16: `ruin_child_at_parent_corner_runs`).

---

## Part A — pure cores (base: `main`)

### Task 1: The room graph value

**Files:**
- Create: `crates/theos-maps/src/rooms/mod.rs`, `crates/theos-maps/src/rooms/graph.rs`
- Modify: `crates/theos-maps/src/lib.rs` (add `pub mod rooms;`)

**Interfaces:**
- Produces:
  - `pub struct Cell { pub row: u16, pub col: u16 }` (`Ord`, derives `Serialize`/`Deserialize`);
  - `pub type RoomId = u16; pub type KeyId = u8;`
  - `pub enum Role { Start, Goal, Path, Leaf, Exit }` (serialised lower case);
  - `pub enum Shape { Rect, Round, Ragged, Passage }`;
  - `pub enum Passage { Open, Locked(KeyId), Drop { from: RoomId } }`;
  - `pub struct Room { pub ty: String, pub role: Role, pub cells: Vec<Cell>, pub shape: Shape, pub keys: Vec<KeyId>, pub locked: bool }`;
  - `pub struct Connection { pub a: RoomId, pub b: RoomId, pub kind: Passage, pub main: bool }`;
  - `pub struct RoomGraph { pub rooms: Vec<Room>, pub connections: Vec<Connection>, pub lattice: LatticeRef }` with `pub struct LatticeRef { pub rows: u16, pub cols: u16 }`;
  - `impl RoomGraph { pub fn validate(&self, enabled: &dyn Fn(Cell) -> bool) -> Result<(), GraphError>; pub fn neighbours(&self, r: RoomId) -> impl Iterator<Item = (RoomId, &Connection)>; pub fn key_name(k: KeyId) -> String /* "key-<k>" */ }`;
  - `impl ValueType for RoomGraph { const NAME: &str = "room_graph"; }` and `WorldValue` (canonical JSON via serde);
  - `#[derive(thiserror::Error)] pub enum GraphError { CellNotEnabled{room,cell}, CellShared{cell}, NotAdjacent{a,b}, StartCount(usize), GoalCount(usize), KeyMissing(KeyId), ConnectionOrder{a,b}, BadType(String) }`.

- [ ] **Step 1: Write failing tests in `graph.rs`:**
  - `validate_accepts_a_minimal_loop`: four rooms in a 2×2 square with one start, one goal and open connections → `Ok`;
  - `validate_refuses_each_rule`: one case per `GraphError` variant (a room on a disabled cell; two rooms sharing a cell; a connection between non-adjacent rooms; zero or two starts; a lock whose key is in no room; connections not sorted by `(a, b)` or with `a >= b`; room type `choir_hall`, because types must match `^[a-z][a-z0-9-]*$`);
  - `canonical_json_is_stable`: the same graph built in two insertion orders of `cells` (sorted by `validate`'s caller) gives byte-equal `canonical_json()`; key names render as `"key-1"`.
- [ ] **Step 2:** Run `cargo nextest run -p theos-maps rooms::graph`. Expected: FAIL (module missing).
- [ ] **Step 3:** Implement the types and `validate` in `rooms/graph.rs`. Adjacency means two rooms have orthogonally neighbouring cells.
- [ ] **Step 4:** Run the same command. Expected: PASS.
- [ ] **Step 5:** Commit `feat(generators): the room graph value (P34)` with footers `Refs: CE-80.3` and `Phase: P34`.

### Task 2: The lattice

**Files:**
- Create: `crates/theos-maps/src/rooms/lattice.rs`

**Interfaces:**
- Consumes: `Cell` (Task 1), `crate::grid::Grid`, `crate::rng::StageRng`.
- Produces:
  - `pub enum Stagger { Rows, Columns }`;
  - `pub struct Lattice { pub pitch: u32, pub rows: u16, pub cols: u16, pub origin: (u32, u32), pub stagger: Stagger, pub enabled: Vec<bool>, offsets: Vec<i32> }`;
  - `impl Lattice { pub fn new(grid_rows: usize, grid_cols: usize, pitch: u32, jitter: u32, stagger: Stagger, rng: &mut StageRng) -> Result<Self, LatticeError>; pub fn block(&self, c: Cell) -> BlockRect /* raster rows/cols, half-open */; pub fn shared_segment(&self, a: Cell, b: Cell) -> Option<Segment>; pub fn enable_inside(&mut self, envelope: &Grid<bool>); pub fn is_enabled(&self, c: Cell) -> bool; pub fn cells(&self) -> impl Iterator<Item = Cell> /* row-major */ }`;
  - `pub struct BlockRect { pub r0: u32, pub r1: u32, pub c0: u32, pub c1: u32 }`;
  - `pub struct Segment { pub horizontal: bool, pub at: u32, pub from: u32, pub to: u32 }`, the raster line and its half-open span;
  - `impl ValueType for Lattice { const NAME: &str = "lattice"; }`;
  - `LatticeError { PitchTooSmall(u32) /* < 6 */, JitterTooLarge{jitter, max} /* > pitch/4 */, TooSmall{rows, cols} /* < 1 */ }`.
- Rules (§5.7):
  - `rows = grid_rows / pitch` and `cols = grid_cols / pitch`; the leftover cells are split floor to the north and west, the odd one south and east;
  - `Stagger::Rows`: horizontal lines are straight; each vertical line inside lattice row `r` gets an offset in `[-jitter, jitter]`, drawn row by row, west to east, interior lines only (the outer edges do not move);
  - `Stagger::Columns` mirrors this.

- [ ] **Step 1: Write failing tests:**
  - `size_and_centring`: grid 100×100 with pitch 16 gives 6×6 cells and origin (2, 2);
  - `blocks_partition_each_lattice_row`: for seeds 1–20 at `jitter = pitch/4`, the blocks of each lattice row cover its raster rows exactly, with no overlap and no gap;
  - `vertical_neighbours_share_at_least_half_a_pitch`: for every vertically neighbouring pair (Rows stagger), `shared_segment` has length ≥ `pitch - 2*jitter` and ≥ `pitch/2`;
  - `columns_stagger_mirrors_rows`;
  - `refuses_bad_parameters`: pitch 5, jitter 5 at pitch 16, a grid smaller than one pitch;
  - `enable_inside_needs_the_whole_block`: a circle mask disables the corner blocks and keeps the centre.
- [ ] **Step 2:** Run `cargo nextest run -p theos-maps rooms::lattice`. Expected: FAIL.
- [ ] **Step 3:** Implement in `rooms/lattice.rs`. Draw offsets with `rng.below(2*jitter+1)` minus `jitter`.
- [ ] **Step 4:** Run the same command. Expected: PASS.
- [ ] **Step 5:** Commit `feat(generators): the interior lattice with staggered walls (P34)`.

### Task 3: Solver and graph measures

**Files:**
- Create: `crates/theos-maps/src/rooms/solve.rs`, `crates/theos-maps/src/rooms/measure.rs`

**Interfaces:**
- Consumes: `RoomGraph`, `Passage`, `KeyId`, `RoomId` (Task 1).
- Produces:
  - `pub struct Reach { pub rooms: Vec<bool>, pub keys: u16 /* bitmask, bit k-1 */, pub round: Vec<Option<u16>> }`;
  - `pub fn fixed_point(g: &RoomGraph, drops: bool) -> Reach`: §5.9 check 3, at most K+1 floods; with `drops == false` drops are removed;
  - `pub fn unsafe_drops(g: &RoomGraph) -> Vec<usize /* connection index */>`: §5.9 check 4. "Surely held" is `fixed_point(g, false).keys`; from the drop's target with those keys and every passage, the goal must be reachable;
  - `pub fn solvable(g: &RoomGraph) -> Result<(), Unsolvable>`, where `pub enum Unsolvable { UnreachableRoom { room: RoomId, missing: Vec<KeyId> }, UnsafeDrop { connection: usize } }` and `missing` lists the keys of locks on the room's frontier;
  - `pub fn critical_route(g: &RoomGraph, len: &mut dyn FnMut(usize /* connection */, RoomId /* entered from */) -> f64) -> Option<(Vec<RoomId>, f64)>`: Dijkstra over (room, 16-bit mask), ordered by (length via `f64::total_cmp`, room id, mask); keys are picked up on entry;
  - `pub fn later_order(g: &RoomGraph) -> Vec<(u16, u32, RoomId)>`: per room (round with all keys held, hop distance from start, id), for `shortcut`;
  - in `measure.rs`: `pub fn cycles(g) -> usize /* E - V + C */; pub fn articulation_points(g) -> Vec<RoomId> /* Tarjan, iterative */; pub fn dead_end_pct(g) -> f64; pub fn hop_distance(g, from) -> Vec<Option<u32>>`.

- [ ] **Step 1: Write failing tests (§8 item 4):**
  - `key_in_start_room`;
  - `lock_whose_key_is_behind_another_lock_solves_in_order`;
  - `key_behind_its_own_lock_is_unreachable`, which names the room and `missing == [1]`;
  - `drop_into_goal_is_safe`;
  - `drop_from_start_is_refused_when_it_skips_a_needed_key`;
  - `two_drops_in_a_row`;
  - `critical_route_fetches_both_keys`, on a graph where the goal needs keys 1 and 2: the route visits both key rooms, its length is the hand sum, and ties are broken by room id;
  - `route_revisits_a_room`;
  - `sixteen_keys_solve` (a chain of 16 locks);
  - `cycles_and_articulation_points` on a figure-eight: 2 cycles, the joint room is an articulation point;
  - `dead_end_pct` on a star.
- [ ] **Step 2:** Run `cargo nextest run -p theos-maps rooms::solve rooms::measure`. Expected: FAIL.
- [ ] **Step 3:** Implement. Use `BinaryHeap` with a total-order wrapper; no `HashMap`.
- [ ] **Step 4:** Run the same command. Expected: PASS.
- [ ] **Step 5:** Commit `feat(generators): lock-and-key solver and room graph measures (P34)` with `Refs: CE-80.2`.

### Task 4: Cell-edge outlines

**Files:**
- Create: `crates/theos-maps/src/kernels/outline.rs`
- Modify: `crates/theos-maps/src/kernels/mod.rs` (one line)

**Interfaces:**
- Produces: `pub fn outline(mask: &Grid<bool>) -> Vec<Vec<[i64; 2]>>`.
  - It returns the cell-edge boundary rings of the `true` region as integer corner coordinates `[col, row]`, exterior rings first.
  - Exterior rings are counter-clockwise in (x east, y south) shoelace terms, as GeoJSON wants after the frame's flip; holes go the other way.
  - Collinear vertices are removed and each ring is closed.
  - Two cells touching only at a corner belong to separate rings (4-connectivity).

- [ ] **Step 1: Write failing tests:**
  - `single_cell_square`;
  - `l_shape_has_six_vertices`;
  - `ring_with_hole`: the hole is the second ring, oriented opposite;
  - `diagonal_touch_gives_two_rings`;
  - `area_equals_cell_count` over a random mask, seeds 1–10, by the shoelace sum.
- [ ] **Step 2:** Run `cargo nextest run -p theos-maps kernels::outline`. Expected: FAIL.
- [ ] **Step 3:** Implement edge tracing. Collect the directed boundary edges of each `true` cell, link them with a `BTreeMap` keyed by start corner, and turn left first at shared corners so diagonal touches stay separate.
- [ ] **Step 4:** Run the same command. Expected: PASS.
- [ ] **Step 5:** Commit `feat(generators): cell-edge outlines (P34)`.

### Task 5: The mission engine

**Files:**
- Create: `crates/theos-maps/src/stages/interior/mod.rs` (module only, no stage yet), `stages/interior/rules/mod.rs`, `stages/interior/mission_core.rs`
- Modify: `crates/theos-maps/src/stages/mod.rs` (`pub mod interior;` only, no registry line yet)

**Interfaces:**
- Consumes: Tasks 1–3, `StageRng`, `ParamError`, `Check`.
- Produces:
  - `pub struct ProgramRoom { pub ty: String, pub role: Role, pub count: CountSpec, pub footprint: (u8, u8), pub shape: Shape, pub locked: bool }`, deserialised from `[[stage.room]]`;
  - `pub enum CountSpec { Fixed(u32), Range(u32, u32) }`, read from `count = n` or `count = [a, b]`;
  - `pub struct MissionState { pub graph: RoomGraph, pub lattice: Lattice, pub program: ProgramQueue, pub next_key: KeyId }`, where `ProgramQueue::take(role) -> Room` takes from the program in order or makes a generic `path`/`leaf` room;
  - `pub struct Match { pub cells: Vec<Cell>, pub rooms: Vec<RoomId>, pub weight: u32 }`;
  - the trait `Rule` as spec §5.6 (with `rng: &mut StageRng` in `apply`), plus `fn makes_keys(&self, p: &dyn RuleParams, count_max: u32) -> u32` for the 16-key bound;
  - `pub fn rules() -> Vec<Box<dyn Rule>>`, one line per rule;
  - `pub struct RuleEntry { pub rule: String, pub count: CountSpec, pub required: bool, pub params: toml::Table }`;
  - `pub fn run_mission(state: &mut MissionState, entries: &[RuleEntry], rng: &mut StageRng) -> Result<MissionReport, MissionError>`;
  - `pub struct MissionReport { pub applied: u32, pub short: u32, pub per_entry: Vec<EntryReport> }`, with `EntryReport { rule, asked, applied, undone }`;
  - `pub fn key_budget(entries: &[RuleEntry]) -> Result<u32, ParamError>`, which refuses more than 16.
- Order of draws, exactly (§5.6): per entry its count, then per application `choose(matches)` (weighted by `weight`, matches in the rule's total order), then the rule's own draws.
- The guard: snapshot the graph and the program queue (`Clone`) before `apply`. If `solve::solvable` fails afterwards, restore the snapshot, drop that match and try the next draw from the remaining matches. Count `undone`.

- [ ] **Step 1: Write failing tests using two test-only rules in `#[cfg(test)]`:**
  - `add_leaf` attaches a leaf to the lowest free neighbour;
  - `broken_lock` locks a connection and puts its key behind it.
  - `counts_and_shortfalls`: an entry `count = 5` with 3 possible matches gives applied 3 and short 2 when required, and short 0 when `required = false`;
  - `guard_undoes_an_unsolvable_application`: `broken_lock` is applied, undone, counted in `undone`, and the graph is unchanged;
  - `draw_order_is_fixed`: the same seed gives the same `canonical_json`, and seeds 1 and 2 differ;
  - `key_budget_refuses_seventeen`.
- [ ] **Step 2:** Run `cargo nextest run -p theos-maps stages::interior::mission_core`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the same command. Expected: PASS.
- [ ] **Step 5:** Commit `feat(generators): the mission rule engine with its guard (P34)`.

### Task 6: Rules `main_cycle`, `leaf`, `locked_leaf`

**Files:**
- Create: `stages/interior/rules/{main_cycle,leaf,locked_leaf}.rs`
- Modify: `stages/interior/rules/mod.rs` (three lines)

**Interfaces:**
- Consumes: Task 5.
- Produces the rule ids `main_cycle`, `leaf` and `locked_leaf`, with these parameters:
  - `main_cycle { side: "south"|"north"|"east"|"west", cycle: "lock_key"|"two_routes"|"risk_reward", min_arc: u32 (default 2), risk_type: Option<String> }`;
  - `leaf { from_roles: Vec<Role> (default all but locked) }`;
  - `locked_leaf {}`.
- Behaviour exactly as the spec §5.6 table. The loop between start and goal is found as two disjoint lattice paths, by a BFS over free enabled cells with neighbours in (N, E, S, W) order. The first arc is the shortest path of at least `min_arc` interior cells; the second is the shortest path avoiding the first. Choices among equal options use the stream, matches in (row, col) order.

- [ ] **Step 1: Write failing tests on a fixed 6×6 lattice:**
  - `lock_key_cycle_is_solvable_and_shaped`: the goal's two main connections are one lock (key on arc B) and one drop from the goal; both arcs have ≥ `min_arc` rooms;
  - `two_routes_has_no_lock`;
  - `risk_reward_short_arc_has_the_risk_type`;
  - `main_cycle_reports_short_without_rim_cell` (Review Focus 1): an envelope disabling the whole south row gives `short = 1` and no panic;
  - `leaf_attaches_to_non_locked_rooms_only`;
  - `locked_leaf_key_is_reachable_before_its_lock`.
- [ ] **Step 2:** Run `cargo nextest run -p theos-maps stages::interior::rules`. Expected: FAIL.
- [ ] **Step 3:** Implement the three modules.
- [ ] **Step 4:** Run the same command. Expected: PASS.
- [ ] **Step 5:** Commit `feat(generators): main cycle and leaf rules (P34)`.

### Task 7: Rules `insert_cycle`, `extend_arc`, `close_loop`

**Files:**
- Create: `stages/interior/rules/{insert_cycle,extend_arc,close_loop}.rs`
- Modify: `rules/mod.rs` (three lines)

**Interfaces:**
- Produces:
  - `insert_cycle { lock: bool (default false) }`. Detours of 2×2, 2×3 and 3×3 are tried in that order around an open connection (§5.6). With `lock`, the detour's far connection is locked and the key sits on the detour's near side;
  - `extend_arc {}`;
  - `close_loop { min_dist: u32 (default 3), max_dist: u32 (default 7) }`. It matches only pairs whose rooms are adjacent **and** have a shared segment (`Lattice::shared_segment`) of at least `door + 2` raster cells.

- [ ] **Step 1: Write failing tests:**
  - `insert_cycle_adds_one_cycle`;
  - `insert_cycle_with_lock_stays_solvable`;
  - `insert_cycle_on_a_locked_connection_does_not_match` (§8 item 2);
  - `extend_arc_keeps_the_far_kind`;
  - `close_loop_respects_distance_and_count`: `count = 2` adds exactly 2 cycles;
  - `close_loop_needs_shared_segment` (Review Focus 3).
- [ ] **Step 2:** Run `cargo nextest run -p theos-maps stages::interior::rules`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the same command. Expected: PASS.
- [ ] **Step 5:** Commit `feat(generators): cycle and arc rules (P34)`.

### Task 8: Rules `shortcut`, `merge_hall`

**Files:**
- Create: `stages/interior/rules/{shortcut,merge_hall}.rs`
- Modify: `rules/mod.rs` (two lines)

**Interfaces:**
- Produces:
  - `shortcut { min_route: u32 (default 4) }`: the drop goes from the later room to the earlier, by `solve::later_order`;
  - `merge_hall { mode: "grow"|"merge" (default "grow") }`. `grow` extends a room whose program footprint exceeds its cells over free cells; `merge` joins two adjacent open `path` rooms. A merge never joins rooms of different roles, nor rooms holding keys of different locks.

- [ ] **Step 1: Write failing tests:**
  - `shortcut_drops_from_later_to_earlier`;
  - `shortcut_between_rooms_behind_different_locks_stays_safe` (§8 item 2);
  - `merge_hall_grows_to_the_footprint`;
  - `merge_hall_keeps_roles_and_keys` (Review Focus 2): growing next to the start, the goal or a key room never absorbs them, and the merged room keeps its role and keys.
- [ ] **Step 2:** Run `cargo nextest run -p theos-maps stages::interior::rules`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the same command. Expected: PASS.
- [ ] **Step 5:** Commit `feat(generators): shortcut and hall rules (P34)`.

### Task 9: The space core

**Files:**
- Create: `stages/interior/space_core.rs`, `stages/interior/shapes.rs`

**Interfaces:**
- Consumes: Tasks 1–2, `crate::kernels::noise::spectral_noise`, `crate::kernels::labels`.
- Produces:
  - `pub struct SpaceParams { pub wall: u32, pub door: u32 /* 1|2 */, pub passage: u32, pub ragged_t: f64 }`;
  - `pub struct Door { pub connection: usize, pub cells: Vec<(u32,u32)>, pub rooms: (RoomId, RoomId), pub kind: Passage, pub centre: (u32, u32) }`;
  - `impl ValueType for Doors { NAME = "doors" }` with `pub struct Doors(pub Vec<Door>)`;
  - `pub struct SpaceOut { pub rooms: Grid<u32> /* number+1 */, pub walkable: Grid<bool>, pub walls: Grid<bool>, pub corridors: Grid<bool>, pub doors: Doors, pub centres: Vec<(u32,u32)> }`;
  - `pub fn build_space(g: &RoomGraph, lat: &Lattice, envelope: &Grid<bool>, p: &SpaceParams, ragged_noise: &Grid<f64>, doors_rng: &mut StageRng) -> Result<SpaceOut, SpaceError>`;
  - `SpaceError { NoSharedSegment { connection }, EmptyRoom { room }, RoomsTouch { a, b }, DoorMismatch { door }, OutsideEnvelope { cell } }`.
- Rules exactly as spec §5.7: walls of `wall` cells on shared lines; door gaps placed by `doors_rng.below(len - door - 1) + 1` per connection in `(a, b)` order; room centre by integer squared distance to the cell mean, ties lowest (row, col); shapes per §5.7, with a straight carved line from each door to the centre.

- [ ] **Step 1: Write failing tests:**
  - `rect_rooms_share_walls_with_doors`: on a 3×3 hand graph, every door touches exactly its two rooms and unconnected neighbours have a solid wall;
  - `halls_have_no_inner_walls`;
  - `space_refuses_corner_only_connection` (Review Focus 3);
  - `round_and_ragged_rooms_keep_door_paths` (Review Focus 4): for seeds 1–10, a round and a ragged room in 1×1 blocks at pitch 6 keep every door connected to the centre (`labels`);
  - `door_positions_depend_only_on_the_doors_stream`: changing the noise does not move doors;
  - `nothing_outside_the_envelope`.
- [ ] **Step 2:** Run `cargo nextest run -p theos-maps stages::interior::space_core`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the same command. Expected: PASS.
- [ ] **Step 5:** Commit `feat(generators): rooms, walls and doors from the lattice (P34)`.

### Task 10: The cover core

**Files:**
- Create: `stages/interior/cover_core.rs`

**Interfaces:**
- Produces:
  - `pub struct CoverParams { pub max_sightline_m: f64, pub max_items: u32 /* 120 */, pub pillar_ring_m: f64 }`;
  - `pub fn place_cover(space: &SpaceOut, g: &RoomGraph, cell_m: f64, p: &CoverParams) -> (Grid<bool>, u32 /* items */)`;
  - `pub fn longest_sightline_cells(free: &Grid<bool>) -> u32`, the longest straight run along rows, columns and both diagonals.
- Behaviour as §5.8. Keep-out zones: a 5×5 square around each door centre and a 3×3 square around each room centre. Candidate offsets from the run's middle: 0, ±2, ±4, ±6, ±8. Every placement is checked with `labels` (4-connected) on `walkable & !cover`.

- [ ] **Step 1: Write failing tests:**
  - `long_hall_gets_pillars_until_the_limit`;
  - `cover_never_disconnects`;
  - `pillar_ring_dropped_when_it_cuts_a_door` (§8 item 5);
  - `rubble_only_interior_stops_at_max_items_without_disconnecting`;
  - `diagonal_sightline_counts`.
- [ ] **Step 2:** Run `cargo nextest run -p theos-maps stages::interior::cover_core`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the same command. Expected: PASS.
- [ ] **Step 5:** Commit `feat(generators): sightline cover (P34)`.

### Task 11: The playability core

**Files:**
- Create: `stages/interior/playability_core.rs`

**Interfaces:**
- Consumes: Tasks 1, 3, 9, 10; `kernels::paths::shortest_paths` (with `Directions::Eight` and a cost closure returning `None` for diagonal steps and `cell_m` for orthogonal ones).
- Produces:
  - `pub struct Analysis` with the §5.9 metrics as fields, plus `route: Vec<RoomId>`, `route_points: Vec<(u32,u32)>` (start centre, door centres, goal centre) and `findings: Vec<(Code, String)>`;
  - `pub enum Code { UncarvedRoom, Disconnected, UnreachableRoom, UnsafeDrop, DoorMismatch }`, written in snake case;
  - `pub fn analyse(g: &RoomGraph, s: &SpaceOut, cover: &Grid<bool>, envelope: &Grid<bool>, cell_m: f64, frame_cells: (usize, usize)) -> Result<Analysis, KernelError>`;
  - `impl ValueType for Analysis { NAME = "analysis" }`.
- A finding's message names its rooms (`room-<k>`), keys (`key-<n>`) and doors.

- [ ] **Step 1: Write failing tests:**
  - `a_clean_interior_has_no_findings_and_route_metres_are_whole_cells`;
  - one test per code, built by damaging a clean hand-made case (blank a room; wall off a door; key behind its own lock gives `unreachable_room` naming `key-1`; an unsafe drop; a door moved off its rooms);
  - `metrics_match_hand_counts` on the probe's choir-hall shape, rebuilt by hand: rooms, connections, cycles, locks, one_way, chokepoints, dead_end_pct, footprint and hall share.
- [ ] **Step 2:** Run `cargo nextest run -p theos-maps stages::interior::playability_core`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the same command. Expected: PASS.
- [ ] **Step 5:** Commit `feat(generators): the interior playability analysis (P34)` with `Refs: CE-80.2`.

**After Task 11:** run `cargo xtask gate`, then push once (CLAUDE.md "CI minutes"). Then merge in P29's plumbing (see "The dependency").

---

## Part B — stages, outputs, recipes (base: Part A plus P29's plumbing)

### Task 12: The interior stages, and CE-80.3

**Files:**
- Create: `stages/interior/{envelope,mission,space,cover,playability}.rs`, `stages/interior/hand_graph.rs` (`#[cfg(test)]`), `crates/theos-maps/recipes/fixture-interior.toml`
- Modify: `crates/theos-maps/src/stages/mod.rs` (five registry lines); the recipe loader (`recipe/load` or the validation step P29 adds for kinds): the checks below
- Test: `crates/theos-maps/tests/maps/interiors.rs` (new), registered in `tests/maps/main.rs`

**Interfaces:**
- Consumes: Part A; `MapKind::Interior` (P29); `TypedStage`.
- Produces these stages (each `KINDS = &[MapKind::Interior]`), with the reads and writes of §5.5 exactly:
  - `envelope { shape: "rect"|"circle"|"ragged_circle", margin_m, pitch_m, jitter_m (default 0), stagger (default "rows"), ragged_t }` writes `envelope` (Mask) and `lattice` (Value). The outline noise comes from `sub_rng("outline")`, β 2.4 (0.3.0's envelope), kmin 1;
  - `mission { [[room]], [[rule]] }` writes `room_graph`. Metrics: `applied`, `short`;
  - `space { wall_m, door_m, passage_m, ragged_t }` writes `rooms` (U32), `walkable`, `walls`, `corridors`, `doors`. Noise from `sub_rng("ragged")` (β 2.2, kmin 1); doors from `sub_rng("doors")`;
  - `cover { max_sightline_m, max_items, pillar_ring_m }` writes `cover`. Metric: `items`;
  - `playability {}` writes `analysis`. Metrics: §5.9's names. Its `checks` turn each analysis finding into `Finding::error` with the code at the start of the message.
- Loader checks:
  - room types must match `^[a-z][a-z0-9-]*$`;
  - `key_budget` ≤ 16;
  - `pitch_m` and `wall_m`/`door_m`/`passage_m` must be whole raster cells;
  - the cell size must be a multiple of 1/16 m;
  - a lattice of at least 1×1 (refused with the stage and the parameter).

- [ ] **Step 1: Write failing tests in `tests/maps/interiors.rs`:**
  - `fixture_interior_runs_and_passes` (40², 6 rooms);
  - `hand_graph_interior_passes_playability` (`// AC: CE-80.3`): a test registry from `Registry::builtin()` plus `hand_graph` replaces `mission` on a 6×6 rect recipe without offsets. `hand_graph` reads `lattice` and writes the seven-room graph of spec §5.16. The run passes;
  - `hand_graph_with_key_behind_its_lock_fails` (`// AC: CE-80.3`): the error is a `FindingFailed` whose message starts with `unreachable_room` and contains `key-1`;
  - `loader_refusals`: an underscore type, 17 keys, pitch 15.5 m at 1 m, cell size 0.1 m and an unknown rule each fail before any stage runs, naming the stage and the parameter.
- [ ] **Step 2:** Run `cargo nextest run -p theos-maps --test maps interiors`. Expected: FAIL.
- [ ] **Step 3:** Implement the five stages as thin wrappers over the cores, the registry lines and the loader checks.
- [ ] **Step 4:** Run the same command. Expected: PASS.
- [ ] **Step 5:** Commit `feat(generators): the interior stages (P34)` with `Refs: CE-80.3`.

### Task 13: Interior features and package layers

**Files:**
- Create: `crates/theos-maps/src/features/interior.rs`
- Modify: the per-kind tables P29 adds (package layers, feature types, the id rule), to add the interior entries
- Test: `tests/maps/interiors.rs`

**Interfaces:**
- Consumes: `Geometry::None` (P29); `kernels::outline`; `Analysis`, `Doors`, `RoomGraph`.
- Produces `pub fn interior_features(world: &World) -> Result<FeatureSet, FeatureError>`, with features as in spec §5.10:
  - ids `<kind>-<n>`;
  - room `number = k-1`; polygons from `outline` of each room's cells, in metres (vertex × `cell_m`);
  - door Points at door centres;
  - connections with null geometry, `from`/`to` (for `oneway`, `from` is the drop's source), `door`;
  - one `route`: a LineString of route points with z = 0, plus `rooms` and `length_m`.
- Package layers for `interior`: `walkable` (u8), `rooms` (u16 from the U32 world layer; refuse values > 65535 with `PackageError`), `corridors`, `cover`, `walls` (u8).
- Known feature types for constraints: the static types (`open`, `locked`, `drop`, `oneway`, `critical`) plus the recipe's program room types.

- [ ] **Step 1: Write failing tests** (`// AC: CE-80.1`, on `fixture-interior` seed 1):
  - `room_polygons_enclose_their_cells`: each room's shoelace area equals its cell count × cell area;
  - `doors_and_connections_follow_the_format`: every door has two room ids that exist; every connection has `"geometry": null` in the GeoJSON bytes, plus `from` and `to`;
  - `one_critical_route_from_start_to_goal`;
  - `ids_are_unique_per_kind`;
  - `package_layers_and_dtypes`: `rooms` is u16, and `walls` is present;
  - `room_type_constraint_counts`: `"features.entrance" = [1, 1]` passes; `"features.no-such" = [0, 0]` is refused before the run.
- [ ] **Step 2:** Run `cargo nextest run -p theos-maps --test maps interiors`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the same command. Expected: PASS.
- [ ] **Step 5:** Commit `feat(generators): interior features and package layers (P34)` with `Refs: CE-80.1`.

### Task 14: Interior renders

**Files:**
- Create: `crates/theos-maps/src/render/interior/{mod,base,blueprint,graph}.rs`
- Modify: the per-kind render table (P29): interior standard renders `base`, `blueprint`, `graph`; extra `blueprint-ruin`

**Interfaces:**
- Produces three render functions with P08's `RenderFn` signature, following spec §5.11. Each declares its `needs`:
  - `base`: `walkable`, `walls`, `cover`, `envelope`, at 2 pixels per cell;
  - `blueprint`: adds the features and the `analysis` value;
  - `graph`: `room_graph`, `lattice`, `analysis`.
- Reuse P08's canvas, fonts, legend and scale bar.

- [ ] **Step 1: Write failing tests:**
  - `base_is_frame_exact` (`// AC: CE-80.1`): width = 2 × cols and height = 2 × rows, and a wall cell's pixel has the wall colour;
  - `blueprint_and_graph_render`: both decode as PNG with non-zero size, and identical bytes on two runs;
  - `blueprint_ruin_is_extra_only`: present only when the recipe lists it.
- [ ] **Step 2:** Run `cargo nextest run -p theos-maps render::interior`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the same command. Expected: PASS.
- [ ] **Step 5:** Commit `feat(generators): interior renders (P34)` with `Refs: CE-80.1`.

### Task 15: Built-in recipes, tuning, CE-80.1 and CE-80.2

**Files:**
- Create: `crates/theos-maps/recipes/{choir-hall-ruin,works-halls}.toml`
- Modify: the built-in recipe list (`recipe/builtin.rs`); `examples/run.rs` if it needs an interior contact sheet
- Test: `tests/maps/interiors.rs`

**Interfaces:**
- Recipes as in spec §5.12 (frames, programs, rules), with constraints:
  - room count, `stages.mission.short = [0, 0]`, `critical_path_m`, `dead_end_pct`, `cycles`;
  - `choir-hall-ruin` also `longest_sightline_m = [10, 45]` and `chokepoints = [1, 6]`;
  - every room type in hyphen form.
- Tuning: run `cargo run -p theos-maps --example run --release -- <recipe> 1..8`. Review the `blueprint` contact sheet of seeds 1–8 and adjust the parameters until every built-in passes its constraints on all eight seeds (spec ruling 10).
  - Record the final values and one line on the look in the ledger.
  - If the 150-room `works-halls` still reads as uniform tiles, add hall types or raise `merge_hall` counts (ruling 6). Do not change the method.

- [ ] **Step 1: Write failing tests:**
  - `builtins_produce_all_outputs` (`// AC: CE-80.1`): for each built-in (the child via Task 16's helper once it exists, the standalone ones now), the run returns `base`, `blueprint`, `graph`, `features` and `package`; the manifest kind is `interior`;
  - `slow_builtins_pass_playability_on_seeds_1_to_8` (`// AC: CE-80.2`): every built-in, seeds 1–8, the run passes. The analysis reports no finding, every room reached, and the key fixed point reaching the goal. It is slow (name segment `slow_`) only if it takes over 10 s locally in a release build; otherwise it is a plain test.
- [ ] **Step 2:** Run the tests. Expected: FAIL (no recipes).
- [ ] **Step 3:** Write the recipes and tune them.
- [ ] **Step 4:** Run `cargo nextest run -p theos-maps --test maps interiors` and, if slow, `cargo xtask slow-tests`. Expected: PASS.
- [ ] **Step 5:** Commit `feat(generators): built-in interior recipes (P34)` with `Refs: CE-80.1, CE-80.2`.

### Task 16: The ruin's child map, CE-80.4

**Files:**
- Create: `crates/theos-maps/recipes/ruin-undercroft.toml`
- Test: `tests/maps/interiors.rs`

**Interfaces:**
- Consumes: P29's `from`, `[inputs]` with `parent_types`, `child_frame` (interior rule: centred on the point, size from `size_km`/`grid`, origin snapped to whole cells).
- `ruin-undercroft` declares `[inputs] feature = { type = "feature", parent_types = ["ruin"] }`, on an 80² frame at 1 m (§5.12).
- Test helper `fn ruin_parent(dir) -> (PathBuf, String)`: runs `fixture-small` seed 1 and returns its package path and its one ruin's id.

- [ ] **Step 1: Write failing tests:**
  - `ruin_child_frame_and_parent` (`// AC: CE-80.4`): `frame.parent.package_sha256` equals the SHA-256 of the parent file's bytes; `frame.parent.feature` is the ruin id; `frame.origin_m` = ruin point − 40 m, snapped to whole metres;
  - `ruin_child_repeats` (`// AC: CE-80.4`): two runs give the same package bytes;
  - `ruin_child_refusals` (`// AC: CE-80.4`): a settlement id, an unknown id, and no `from` are each refused before any stage runs;
  - `ruin_child_at_parent_corner_runs` (Review Focus 5): a synthetic parent package with a ruin 10 m from its north-west corner runs, and `origin_m` is negative.
- [ ] **Step 2:** Run them. Expected: FAIL.
- [ ] **Step 3:** Write the recipe; fix only what the tests show, in P34's own code. If the shared plumbing needs a change, tell the P29 session and change it in agreement.
- [ ] **Step 4:** Run them. Expected: PASS.
- [ ] **Step 5:** Commit `feat(generators): the ruin undercroft child map (P34)` with `Refs: CE-80.4`.

### Task 17: Golden runs and the generator version, CE-80.5

**Files:**
- Modify: `crates/theos-maps/src/golden.rs` (`GOLDEN_RUNS` gains the two runs, in P29's run-descriptor form), `crates/theos-maps/src/api.rs` (`VERSION`)
- Create: `crates/theos-maps/tests/fixtures/golden/<VERSION>.toml` (recorded by `cargo run -p theos-maps --example bless`)

**Interfaces:**
- Golden runs added:
  - `choir-hall-ruin` seed 1;
  - `ruin-undercroft` seed 1, `from` = (`fixture-small`, 1), `inputs.feature` = its ruin's id.
- `VERSION` follows spec §5.2 item 9: 1.1.0 if `main` holds no 1.1.0 golden file when P34 merges, otherwise 1.2.0. Check `main` right before recording. Merge `main` only when the PR is next in line (CLAUDE.md), then re-bless if P29 merged first.

- [ ] **Step 1:** Add the runs and bump `VERSION`. Run the golden test. Expected: FAIL (no file for the new version).
- [ ] **Step 2:** Run `cargo run -p theos-maps --example bless`. Expected: it writes `<VERSION>.toml` and refuses to overwrite it on a second call.
- [ ] **Step 3:** Run `cargo nextest run -p theos-maps --test maps golden` (`// AC: CE-80.5` on the test that lists the interior runs). Expected: PASS. Run `cargo xtask gate`. Expected: every check passes.
- [ ] **Step 4:** Commit `feat(generators): golden hashes for interiors (P34)` with `Refs: CE-80.5`.
- [ ] **Step 5:** On the build PR, add the labels `windows` and `macos` (CE-77.1's platforms, as P08 did) and link the runs.

---

## After the build

- Final review loop (CLAUDE.md), then `cargo xtask gate` on Windows, then `cargo xtask ac-trace --phase P34`: every CE-80 AC has a passing test.
- `docs/status.md`: P34 → `pr-open` and the build PR number. The PR description lists every ledger ruling.
