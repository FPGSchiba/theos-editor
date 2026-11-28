# P34 · Interiors — phase spec

Status: approved (#55) · built in #69 · as built (this erratum, 28 Nov 2026) · repo of the build:
`theos-editor`
Source: `docs/specs/00-umbrella.md` §3 P34, decision 0004
(`docs/decisions/0004-map-generation-research.md`, decisions 5b and 5c) and the P08 spec
(`docs/specs/phases/P08-map-format-and-terrain-generator.md`), whose Stage contract, runner,
determinism rules and golden hashes this phase builds on. Scope and acceptance criteria come from
the umbrella; this spec decides how.

## 1. Goal

After P34, `theos-maps` generates **interior maps on one level**, the buried halls, vaults and
complexes a party walks through, at the state of the art:

- the **structure** of an interior is a cyclic mission graph, grown by rewriting rules on a coarse
  lattice in the manner of *Unexplored* (Dormans 2017): a main loop between entrance and goal,
  then locks with their keys, one-way drops, minor loops, halls and dead ends;
- the graph is a typed world value, **`room_graph`**, which every later kind that has rooms can
  write (decision 0004, 5b);
- the **space** comes from the lattice: rooms fill their lattice cells, halls span several, and
  rooms share walls with doors in them;
- a **playability analysis** proves every interior: every room carved and reachable, lock and key
  solvable, one-way drops leaving no dead end;
- a **ruin's child map** runs from a terrain package and its ruin feature, through the child-map
  plumbing shared with P29 (§5.2).

0.3.0's interior generator (`theos_maps/interior.py`) is the baseline, not the target. It builds
one main cycle with one lock and one drop at any size, places rooms by spring forces and joins
them with Dijkstra corridors. Probe 4 of the map generation research measured it at 10–20 % of the
floor used, about half the rooms dead ends, and 44 s for 155 rooms (`research/map-generation/05-probes.md`).
P34's probe (§4.3) reaches 58–68 % of the floor, 8–43 % dead ends and 17 ms for 150 rooms, every
run solvable and bit-identical when repeated.

What later phases build on:

- P30 adds stacked levels to the same stages: the lattice and `room_graph` get a level coordinate,
  links connect levels, the analysis becomes hierarchical (§6);
- P23's blockout generator reads interior packages;
- the research track's ruin pass (P37), cavern kind (P40) and ring layouts write or read
  `room_graph`;
- P16 shows the interior renders; P22 and P25 read its `base`, frame and features.

## 2. Acceptance criteria (verbatim from the umbrella)

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

CE-78.2, which CE-80.4 refers to: "A child map runs from a parent package (`from`) and one of its
settlement features (an input of type `feature`); its frame carries `origin_m` in the parent's
frame and `parent` with the parent package's hash and the feature id; the same parent, feature,
seed and inputs give the same child."

## 3. Scope

**In P34:**

- The interior kind in `theos-maps`: its stages (§5.5), the mission rules (§5.6), space (§5.7),
  cover (§5.8), the playability analysis (§5.9), its package layers and features (§5.10) and its
  renders (§5.11).
- The `room_graph` world value (§5.4), documented as an interface.
- Built-in interior recipes: `choir-hall-ruin`, `works-halls` and the child recipe
  `ruin-undercroft` (§5.12).
- The shared child-map plumbing (§5.2), if P34's build starts before P29's; otherwise P34 reuses
  P29's.
- The interior golden runs and the generator version bump (§5.15).
- Format 1.1 rows for interiors in `docs/map-package.md` (§5.17).

**Not in P34:**

| Item | Where |
|---|---|
| Stacked levels, stairs, lifts, drops between levels, the analysis across levels | P30 |
| Places (`chime`, `gate-up`, `gate-down`) inside interiors | Research track (`09` P41's floor places; decision 0004) |
| Ring layouts around a void (band 10), a polar lattice | Research track (ring layouts) |
| Cavern interiors (cave processes, water levels) | Research track (P40) |
| Damage on intact interiors (rubble, broken walls) | Research track (P37, the ruin pass) |
| Room fill by tile solving (machines, catwalks) | Not now; the research recommends it only as an optional fill (`06c` §5) |
| A layout step for graphs without lattice positions (configuration spaces, Ma et al. 2014) | Later, when a kind writes rooms without a lattice (§9, ruling 4) |
| Settlement maps and their child maps | P29 |
| Showing interiors in the Maps screen and the Atlas | P16, P25 |

## 4. Decisions

### 4.1 Decided by Jann (27 Nov 2026)

| Topic | Jann's choice | Where it lands |
|---|---|---|
| The method | "Cyclic graph on a lattice": the mission graph grows by rewriting on a coarse integer node grid; nodes are never moved, so the graph is already laid out; rooms come from lattice cells, halls from merged cells | §5.5–§5.7 |
| The rules | "Rust modules, recipe picks": each rule or cycle type is one Rust module behind one trait plus one registry line; the recipe chooses which rules apply, with counts and weights | §5.6 |
| The child-map plumbing | "Shared spec, first build wins": P34's spec carries the same interface as P29's; whichever build starts first builds it, the other reuses it | §5.2 |

The other options Jann had: for the method, the same mission graph laid out by configuration
spaces and simulated annealing (Ma et al. 2014; Nepožitek & Gemrot 2018), or by a BSP partition
whose leaves the graph's nodes are assigned to; for the rules, a generic graph-rewriting engine
with the rules as TOML data (Dormans 2010); for the plumbing, P29 owning it and P34 waiting.

### 4.2 Research (27 Nov 2026)

What was read for this spec, and what each source gives. Marks: **[opened]** the body was read;
**[partial]** parts could not be extracted; nothing below is cited from memory.

| Source | State | What it gives P34 |
|---|---|---|
| Boris the Brave 2021, "Dungeon Generation in Unexplored"; and his "Lock and Key Dungeons" and "Graph Rewriting" (2021) | [opened] | A level starts on a 5×5 (or similar) grid of nodes. Nodes are annotated, never moved or deleted, so the graph is always laid out in 2D. A start and an end, then a loop between them; the cycle type decides what the two arcs mean. Later rules insert cycles into the existing one, lengthen arcs and add dead ends. Space: the grid is doubled so corridor and door cells sit between nodes, then scaled into tiles, and each block is shaped by rules. Locks and keys are abstract pairs with an edge from key to lock. About 50 grammar modules and 5,000 rules in the game |
| Dormans 2017, "Unexplored's Secret: Cyclic Dungeon Generation" | [opened] | Named patterns: a two-route cycle, a lock-and-key cycle (arc A ends in a lock, arc B holds the key), risk and reward, a hub, a lake. New cycles are injected into existing paths ("a second lock and key cycle injected into one of the existing paths"). One-way valves force an order. Main lock-and-key cycles are safe by construction; unsafe locks go off the main path. "The average map in the game only contains a couple of embedded cycles" |
| Dormans 2010, "Adventures in Level Design" (PCG workshop) | [partial] (figures unreadable) | Mission graph first, space second; rewriting with numbered left and right sides; tight coupling puts a key behind its test. No rule count, runtime or proof |
| Ma, Vining, Lefebvre, Sheffer 2014, "Game Level Layout from Design Specification" (author's preprint) | [partial] (equations unreadable) | Layout of a planar graph by configuration spaces, chain decomposition and simulated annealing; graphs up to 40 nodes, 1–80 s per layout, no completeness guarantee |
| Nepožitek & Gemrot 2018, "Fast Configurable Tile-Based Dungeon Level Generator"; Edgar-DotNet (MIT) | [opened] | Ma et al. with integer rectilinear rooms: 0.00–0.62 s for up to about 41 nodes. Edgar's README: "not suitable for large levels — we recommend less than 30 rooms" |
| `research/map-generation/06c-built-environments.md` §5 | [opened] | The three-layer recommendation (mission graph, partition, optional fill); keys that are never lost make the check a fixed point; drops need a no-dead-end check; correct by construction first, checked second |
| 0.3.0 `theos_maps/interior.py`, `render_interior.py`, `recipes/choir-hall-ruin.yaml` | read | The baseline: its room program, its renders (`blueprint` with skins, `graph`), its analysis and its constraints |

Conclusions:

- **Unexplored's lattice fits G20 best.** Nodes on integer cells, matches enumerated in a total
  order, one seeded stream per stage: no search, no floats, linear in the number of rewrites.
- **Configuration-space layout does not scale to P34's sizes** (40 nodes in the papers, under 30
  recommended by Edgar; P34's `works-halls` asks for 150) and gives no guarantee. Its ideas (rooms
  of authored shapes fitted by configuration spaces) stay for a later layout step (§9, ruling 4).
- **Rules as Rust modules, few of them.** Unexplored's 5,000 rules are the authoring cost the
  research warned about. P34 starts with eight rules (§5.6), each a module with its own tests.

### 4.3 Probes (27 Nov 2026, rustc 1.99.1, Windows 11)

A throwaway Rust prototype of the method, kept as text in `research/P34-interiors/` (about 1,400
lines; `rand_pcg`, `png`, `sha2`). It ran three recipes on seeds 1–8 with two space steps, each run
twice:

- `sparse`: rooms inside their lattice cells, joined by corridors (round 1);
- `dense`: rooms fill their cells minus a one-cell wall, doors in the shared walls, multi-cell
  halls (round 2).

| Recipe (target rooms) | Space | Floor | Rooms reached | Floor used % | Dead ends % | Cycles | Hall share % | Runtime |
|---|---|---|---|---|---|---|---|---|
| choir hall (14) | sparse | 200 m | 8/8 | 20.0–21.7 | 36–50 | 1–2 | 40–46 | 0.2 ms |
| choir hall (14) | dense | 80 m | 8/8 | 58.3–68.0 | 14–43 | 2–4 | 22–44 | 0.2 ms |
| 150 rooms | sparse | 992 m | 8/8 | 8.1–8.5 | 10–17 | 23–37 | 7–11 | 13 ms |
| 150 rooms | dense | 240 m | 8/8 | 58.4–65.6 | 8–15 | 39–67 | 22–29 | 17 ms |
| cave circle (22) | sparse | 350 m | 6/8 | 15.7–20.2 | 27–36 | 1–4 | ≈40 | 0.8 ms |
| cave circle (22) | dense | 112 m | 4/8 | 60.0–66.7 | 12–40 | 4–7 | 25–46 | 0.7 ms |

0.3.0 on comparable recipes: choir hall 20.5 % used, 50 % dead ends, 2 cycles; 155 rooms 9.8 % used,
about 50 % dead ends, 17 cycles, 44 s.

- **Every run passed every check:** every room carved, the walkable raster one component, the key
  fixed point reaching every room, no dead end after a drop, and in dense runs no two rooms
  touching without a wall and every door touching exactly its two rooms.
- **Determinism:** each of the 48 (recipe, seed, space) runs gave the same `rooms` raster hash and
  graph hash twice in one process and across two processes.
- **Solvable by construction:** the guard that undoes a rule leaving the graph unsolvable never
  fired. A guard that never fires proves nothing, so §5.6 keeps it and §5.16 tests it with a rule
  built to break solvability.
- **What the pictures showed:**
  - Sparse rooms are small islands on long one-cell corridors across a mostly empty floor, as in
    0.3.0.
  - The dense choir hall reads as a compact floor plan, with one round hall dominating, rooms
    sharing walls, round vaults and a ragged collapsed room.
  - The dense 150-room interior still reads as a grid of near-identical tiles: one pitch, one
    wall thickness, a door in the middle of each wall.
  - The dense cave has organic ragged rooms, but the grid shows through.
- **What failed:**
  - The loop-closing rule made far too many cycles (39–67 at 150 rooms).
  - The small ragged circle ran out of free cells on 4 of 8 seeds (17–22 of 22 rooms).
  - The critical route through the dense interiors is short: 96–144 m for the choir hall, against
    0.3.0's 246 m.

What the spec takes from it:

- the dense space step (§5.7), with the tile look broken by staggered walls and hall types;
- a cap on loop closing (§5.6);
- room targets as constraints, so a short run fails visibly instead of silently (§5.12);
- the guard test (§5.16).

The probe is not the build: its code has no tests, `unwrap`s, and an axis-only sightline.

### 4.4 Approaches considered

| Approach | For | Against | Verdict |
|---|---|---|---|
| Port 0.3.0 (topology, springs, corridors) | Known numbers | 10–20 % floor use, one cycle at any size, float spring layout; the research rejects it | No |
| Cyclic graph on a lattice, dense space | Integer, linear, solvable by construction, 58–68 % floor use, 17 ms for 150 rooms | A lattice look that must be broken deliberately | **Chosen** (Jann) |
| Cyclic graph + configuration-space layout | Organic placement of authored room shapes | Stochastic search, 40 rooms in the papers, no guarantee, floats in annealing | Later layout step, not now |
| Cyclic graph + BSP partition | Dense packing | The graph's edges rarely match neighbouring leaves; needs repair | No |
| Rules as TOML data with a rewriting engine | Designers add rules without code | A pattern matcher with total match order; solvability only checked, not built in | No (Jann) |

## 5. Design

### 5.1 Parts, and the tool or crate each one uses

| Part | Tool or crate | Why |
|---|---|---|
| Recipes, rule parameters | `toml` + `serde`, P08's recipe loader | P08 §5.5 |
| Random streams | P08's `rng` (`Pcg64` per stage key) | G20; P08 §5.7 |
| The room graph and the lattice | own code, `BTreeMap`/`Vec` with sorted ids | No crate is needed for a small typed graph; `petgraph`'s Dijkstra returns a `HashMap` (`06c` §5) |
| Rewriting rules | own code, one module per rule (§5.6) | Jann's choice; no maintained Rust crate does graph rewriting on a lattice |
| Room rasters, walls, doors | own code on P08's `Grid` | Integer rectangles and masks |
| Round and ragged rooms | own code; ragged uses P08's `kernels::noise` thresholded in integers | Determinism |
| Connectivity | P08's `kernels::labels` | Exists |
| Distances on the raster (critical route in metres) | P08's `kernels::paths` grid search | Exists |
| Room outlines | own `kernels::outline`: the cell-edge boundary of a mask, as integer vertices | P08's `kernels::contour::rings` is marching squares through cell centres, which cuts corners and changes the area; a room's polygon must enclose exactly its cells |
| Articulation points, cycle count | own code (Tarjan's articulation points; cycles = E − V + C) | 40 lines; no graph crate (above) |
| Features, package, GeoJSON | P08's `features`, `package`, `geojson` | P08 §5.10 |
| Renders | `tiny-skia`, P08's canvas, fonts and legend | P08 §5.11 |
| Errors, logging | `thiserror`, `tracing` | G16 |

### 5.2 Shared child-map plumbing (identical in the P29 and P34 specs)

P29 (settlements) and P34 (interiors) both run maps of a new kind from a parent package. Jann
ruled (27 Nov 2026) that this plumbing is shared: whichever build starts first builds it, and the
other merges `main` and reuses it. This section is word for word the same in both specs; a change
to it is made in both, in agreement between the two phases.

1. **Kinds.** `MapKind` gets `Settlement` and `Interior` next to `Terrain`; a recipe's `kind` takes
   `settlement` and `interior`. A stage lists the kinds it applies to, as in P08.
2. **Inputs.** The recipe's `[inputs]` declares inputs in the form of `docs/generators.md` §3, and
   `describe` reports them. Planning accepts declared inputs and refuses undeclared ones and wrong
   types before the run. A child kind's recipe declares exactly one input of type `feature`, with
   `parent_types`, the parent feature types it accepts (`["city", "town", "village", "hamlet"]`
   for settlements, `["ruin"]` for interiors). A feature of another type, or an id the parent does
   not have, is refused before the run.
3. **`from`.** `RunRequest.from` is the path of the parent package (`.theosmap`). The run reads it
   with `theos_map_format::read` (hashes and `requires` are checked) and takes the SHA-256 of the
   file's bytes as `frame.parent.package_sha256`. A recipe that declares an input of type
   `feature` is a child recipe: without `from` it is refused before the run, and a recipe without
   such an input given `from` is refused too.
4. **Child frame.** One function per kind, `child_frame(kind, recipe, parent, feature) ->
   ChildFrame { origin_m, rows, cols, cell_m }`, places the child in the parent's frame:
   axis-aligned (`docs/map-package.md` §2), centred on the feature's point (a Polygon's
   bounding-box centre), with `origin_m` snapped to whole child cells. The size comes from the
   kind's rule: a settlement's from its class table (P29), an interior's from the recipe's
   `size_km` and `grid`, which stay required for interiors (P34). The window may reach past the
   parent's edge.
5. **Parent layers in the world.** Before the first stage the runner puts into the world only the
   parent values that some stage of the plan `reads`:
   - `parent.<layer>`: a parent package layer resampled into the child frame by a new kernel,
     `kernels::resample` (bicubic, Catmull-Rom, every sample point clamped to the parent's grid,
     so cells past the parent's edge read the nearest edge cell);
   - `parent.features`: the parent's features that touch the window, in the child's frame;
   - `parent.feature`: the chosen feature (id, kind, type, geometry, properties, in the child's
     frame) with the parent's frame (`size_m`, `cell_m`, `grid`) and `package_sha256`.
   Planning treats them like layers written before the first stage, so a stage reading a parent
   value the kind does not offer is refused before the run. The memory estimate counts them.
6. **Seed and repeatability.** The child's seed is the request's seed (`docs/generators.md` §3);
   the stage RNG streams are P08's. The parent's package hash and the feature id are inputs of
   the run, so the same parent, feature, seed and inputs give the same package bytes.
7. **Package and manifest.** The package writes `kind`, `frame.origin_m` and
   `frame.parent {package_sha256, feature}`; the run's manifest `frame` adds `origin_m`. The
   package layer table, the standard renders, the feature types (the constraint paths
   `features.<type>`) and the `needs` check are looked up per kind: one table per kind, chosen by
   `MapKind`, with terrain's unchanged. `base` is standard for every kind and lies on the child's
   frame. Settlement: P29 §5.14–§5.15; interior: P34 §5.10–§5.11.
8. **Features without geometry.** `Geometry` gets `None`, written as GeoJSON `"geometry": null`
   (`docs/map-package.md` §5, `connection`).
9. **Golden runs.** A golden run is `{recipe, seed, inputs, from: Option<(recipe, seed)>}`; the
   parent is generated in the same test from its own golden run, so no package fixture is kept.
   Both phases use `fixture-small` seed 1 as the parent (it places one ruin and its settlements).
   Each phase adds its runs under a new `VERSION`: the first to merge raises it to 1.1.0 and
   records `1.1.0.toml`; the second merges `main`, raises it to 1.2.0 and records `1.2.0.toml`.
10. **Integer lattice.** Geometry that later code compares or splits is snapped to a lattice of
    1/16 m and compared with i64 predicates, in `kernels::geometry`. Whoever needs it first
    builds it.
11. **Format 1.1.** Format 1.1 holds the settlement and interior additions;
    `theos-map-format` writes 1.1 from the first build that adds them.

### 5.3 Modules

```text
crates/theos-maps/src/
  rooms/                 the room graph and the lattice, shared by every kind with rooms
    graph.rs             RoomGraph, Room, Connection, Passage, KeyId (§5.4); canonical JSON
    lattice.rs           Lattice, Cell, enabled cells, neighbours, line offsets (§5.7)
    solve.rs             the key fixed point, drop safety, the critical route (§5.9)
    measure.rs           cycles, articulation points, dead ends
  stages/interior/       one module per stage (§5.5)
    envelope.rs  mission.rs  space.rs  cover.rs  playability.rs
  stages/interior/rules/ one module per mission rule (§5.6), mod.rs holds the trait and the list
  kernels/outline.rs     cell-edge outlines of a mask (§5.1)
  features/interior.rs   room, door, connection, route features (§5.10)
  render/interior/       base, blueprint, graph (§5.11)
recipes/                 choir-hall-ruin.toml, works-halls.toml, ruin-undercroft.toml,
                         fixture-interior.toml (tests only)
```

`rooms/` sits outside `stages/` because P30 and the research track's kinds use it too (§6).

### 5.4 The room graph

`room_graph` is a world value (P08 §5.4, a `WorldValue` with a canonical JSON form). Decision
0004 (5b) makes it the shared form of rooms and connections:

```rust
pub struct RoomGraph {
    pub rooms: Vec<Room>,              // index = room number, 0-based, dense
    pub connections: Vec<Connection>,  // sorted by (a, b)
    pub lattice: LatticeRef,           // rows, cols of the lattice the cells refer to
}
pub struct Room {
    pub ty: String,                    // room type from the recipe's program, e.g. "choir-hall"
    pub role: Role,                    // start | goal | path | leaf | exit
    pub cells: Vec<Cell>,              // lattice cells, sorted; one for a room, several for a hall
    pub shape: Shape,                  // rect | round | ragged | passage
    pub keys: Vec<KeyId>,              // keys found in this room
    pub locked: bool,                  // the room lies behind a lock: RoomGraph::mark_locked
}
pub struct Connection {
    pub a: RoomId, pub b: RoomId,      // a < b
    pub kind: Passage,                 // open | locked(KeyId) | drop { from: RoomId }
    pub main: bool,                    // on the main cycle
}
```

- **Positions are lattice cells**, never metres: the space stage turns them into metres. A
  hand-made graph (CE-80.3) gives cells too.
- **Room types** are lower case with hyphens (`choir-hall`), as P08 §5.10 requires of every
  feature type; the recipe loader refuses others.
- **Keys:** `KeyId` is a `u8`, numbered from 1 in the order the mission creates locks, and written
  `key-<n>` in features and renders. A recipe may create at most 16 keys: the loader sums the
  maximum counts of the key-making rules (`main_cycle` with `lock_key`: 1; `insert_cycle` with
  `lock = true`: its count; `locked_leaf`: its count) and refuses more before the run.
- **Rules of the value**, checked by `RoomGraph::validate` after every stage that writes it:
  - room cells lie on enabled lattice cells and belong to one room each;
  - a connection joins two rooms with orthogonally adjacent cells;
  - exactly one `start` and one `goal`;
  - every key a lock names lies in some room.
- **Keys are never lost** and a key opens every lock that names it. This is what makes the check
  a fixed point (§5.9), and it matches Unexplored's safe main locks.
- **`locked`** is set by `RoomGraph::mark_locked`, which every writer of `room_graph` runs before
  writing it: a room is locked when a keyless flood from the start through open passages and
  drops does not reach it (as built). While the mission runs, the rules read it as the program's
  flag (`locked_leaf`'s rooms).
- **A drop** is a one-way connection: walkable from `from` to the other room only.
- **Role `exit`** is 0.3.0's `extraction` role in the room program: a room where a party can
  leave. It is not a place feature: `chime` and the gates inside interiors wait in the research
  track (§3).
- **For P30:** `Cell` becomes `(level, row, col)` and `Passage` gains the links between levels;
  P34 writes `Cell { row, col }` with no level, which is P30's level 0 (§6).

### 5.5 Stages of the interior kind

A built-in interior recipe runs, in order:

| Stage | Reads | Writes | Job |
|---|---|---|---|
| `envelope` | — | `envelope` (mask), `lattice` | The buildable outline (rect, circle, ragged circle) and the lattice over it: pitch, size, offsets, enabled cells (§5.7) |
| `mission` | `lattice` | `room_graph` | Runs the recipe's rules in order (§5.6) |
| `space` | `room_graph`, `lattice`, `envelope` | `rooms` (u16), `walkable`, `walls`, `doors` (value), `corridors` | Rooms fill their cells, doors in shared walls (§5.7) |
| `cover` | `walkable`, `rooms`, `doors` | `cover` | Pillars and rubble that break long sightlines without disconnecting (§5.8) |
| `playability` | `room_graph`, `rooms`, `walkable`, `cover`, `doors` | `analysis` (value) | The analysis; an unplayable interior fails the run (§5.9) |

- Every stage is a P08 `Stage` with `kinds() = [Interior]`; the registry gains one line each.
- **CE-80.3's test** keeps `envelope` (a test recipe with a rect outline and no offsets) and
  replaces only `mission` by a test-only stage `hand_graph`. It reads `lattice` and writes a
  hand-made `room_graph` on fixed cells; `space`, `cover` and `playability` run unchanged.
- **Child recipes** read no parent value in P34: the frame already sits on the ruin (§5.2 item 4,
  §5.13), and no `parent.<layer>` or `parent.feature` is loaded (§5.2 item 5).

### 5.6 Mission rules

```rust
/// One rewriting rule of the mission graph (Dormans 2017; Boris the Brave 2021).
pub trait Rule: Send + Sync {
    fn id(&self) -> &'static str;
    fn params(&self, table: &toml::Table) -> Result<Box<dyn RuleParams>, ParamError>;
    /// Every place the rule can apply, in a total order (sorted by lattice cell, then room id).
    fn matches(&self, g: &MissionState, p: &dyn RuleParams) -> Vec<Match>;
    /// Applies the rule at `m`; the stage chose `m` with its stream.
    fn apply(&self, g: &mut MissionState, m: &Match, p: &dyn RuleParams, rng: &mut StageRng);
}
```

- **The `mission` stage** reads the recipe's `[[stage.rule]]` list in order. Each entry names a
  rule and its parameters, and `count = n` or `count = [min, max]` (drawn once). For each
  application the stage enumerates the matches, picks one with the stage's stream (uniform, or
  weighted by the rule's integer `weight` per match), applies it, and validates.
- **The guard:** after each application the stage runs the solver (§5.9, graph part). If the graph
  is no longer solvable, the application is undone and the match is dropped. The guard is a safety
  net: every rule below is solvable by construction.
- **Shortfalls.** A rule whose matches run out stops early. Each `[[stage.rule]]` entry has
  `required` (default `true`). The stage reports two metrics with fixed names, so P08's
  `metric_names` contract (`&'static str`) holds:
  - `stages.mission.applied`: applications in total;
  - `stages.mission.short`: the applications that required entries missed.

  The summary also lists, per entry index, the rule id, the count asked, the count applied and
  the guard's undos, as data only (not constraint paths). A built-in asks
  `"stages.mission.short" = [0, 0]`.
- **Streams and order.** All draws of `mission` come from its stage stream, in this order: each
  entry's count, then per application the choice among the matches (in their total order), then
  the rule's own draws (an arc's route, a hall's placement among its footprint positions, a key's
  room). Nothing in the mission depends on iteration over a hash map.
- **The room program** (`[[stage.room]]`, as 0.3.0's `program`) lists room types with role, count,
  lattice footprint (`[1, 1]`, `[2, 2]`, …), shape and `locked`. Rules draw rooms from it in
  program order; a rule that needs a room of a role the program has used up creates a generic one
  of that role (`path`, `leaf`).

The eight rules of P34. Each is one module, unit-tested on a small fixed lattice:

| Rule | What it does | Solvable by construction because |
|---|---|---|
| `main_cycle` | Places `start` on an enabled cell of the rim side the recipe names (`side`), chosen among those cells by the stream, and `goal` (with its program footprint) on the enabled cells whose integer squared distance to the lattice centre is least, ties by the stream in (row, col) order. Then draws a loop through free cells between them, each arc at least `min_arc` rooms long. Parameter `cycle`: `lock_key` (arc A's last connection into the goal is locked; its key lies on arc B; arc B's last connection is a drop from the goal into arc B), `two_routes` (both arcs open), `risk_reward` (arc A at most `min_arc` rooms with a recipe-named room type; arc B longer and open) | `lock_key`: arc B is open from the start, so the key is reached without any key; the lock then opens arc A into the goal; the drop only leads out of the goal |
| `insert_cycle` | Replaces an open connection u–v by a detour loop of 2×2, 2×3 or 3×3 cells through free cells; optional `lock = true` locks the detour's far end and puts its key on the detour's near side | The key lies on the entrance side of its lock |
| `close_loop` | Adds an open connection between two adjacent rooms whose graph distance is in `[min_dist, max_dist]`; capped by `count` (the probe's uncapped version made 39–67 cycles at 150 rooms) | Adding an open connection never removes a route |
| `extend_arc` | Inserts a new `path` room into a connection along free cells | The new room is open on both sides and inherits the connection's kind on its far side |
| `leaf` | Attaches a dead-end room of the program to a room the program does not mark `locked` (`locked_leaf`'s rooms); with `role = "exit"` it places the program's exit rooms | Open |
| `locked_leaf` | Attaches a `locked` room behind a lock, and puts its key in a room the solver reaches before the lock | The key's room is chosen from the rooms reachable without the new lock |
| `shortcut` | Connects an adjacent, unconnected pair of rooms whose shortest route between them is at least `min_route` rooms by a one-way drop from the later room to the earlier. "Later" is the order of §5.9's fixed point with every key held: first the round in which a room is reached, then its graph distance from the start, then its room id | A drop toward an earlier room only returns the party to ground it has walked; §5.9's drop check confirms the goal stays reachable |
| `merge_hall` | Grows a room over free neighbour cells to its program footprint (1×2, 2×2, 2×3, 3×3), or merges two adjacent open `path` rooms into one hall | Merging open rooms keeps every route |

New rules are new modules plus one line in `rules/mod.rs` (engineering directive 1).

### 5.7 Space

The lattice is Unexplored's grid made concrete:

- **Pitch and size.** `pitch_m` (a parameter of `envelope`) is the size of a one-cell room plus
  its wall. It must be a whole number of raster cells, `p`, of at least 6; the loader refuses
  others. The recipe's `size_km` and `grid` set the frame. The lattice has
  `rows = floor(grid_rows / p)` and `cols = floor(grid_cols / p)` cells. It is centred, and the
  odd leftover raster cell goes to the south and east.
- **Staggered walls** (brick bond). The frame is cut into lattice rows by straight horizontal
  lines at the nominal positions. Within each lattice row, every vertical wall line moves by its
  own seeded integer offset in `[-j, +j]` raster cells, where `j` is `jitter_m` in raster cells,
  at most `floor(p / 4)`:
  - rooms differ in width;
  - vertical walls do not line up from one lattice row to the next.

  The axis is a recipe choice (`stagger = "columns"` swaps the roles). The invariants hold by
  construction:
  - within a lattice row, the blocks partition the row, because neighbouring blocks share their
    vertical line;
  - two vertically neighbouring blocks share a horizontal line along a segment of at least
    `p − 2j ≥ p/2` raster cells, the overlap of their column ranges.

  Doors between vertical neighbours lie on that shared segment. The offsets are drawn from
  `envelope`'s stream, lattice row by row, west to east. This breaks the probe's tile look at
  150 rooms (§9, ruling 6).
- **Enabled cells.** A lattice cell is enabled when its block (after the offsets) lies entirely
  inside `envelope`, so a ragged or round outline disables edge cells instead of clipping rooms.
- **Rooms fill their block:** a room covers the raster cells of its lattice cells minus a wall of
  `wall_m` (one raster cell by default) on the shared lines. A hall covers its merged cells
  without inner walls.
- **Shapes inside the block:**
  - `rect` fills it;
  - `round` is an integer ellipse in its bounding box;
  - `ragged` erodes the outer two rings of the block where the noise exceeds `ragged_t`, and
    keeps the component that holds the block's centre. The noise comes from one call of P08's
    `spectral_noise` over the whole grid per run of `space`, on `sub_rng("ragged")`, with
    β = 2.2 (0.3.0's) and kmin = 1. The comparison is a float comparison on values P08's
    golden hashes already pin, so it is deterministic under G20;
  - `passage` fills only a strip of `passage_m` across the block, between its doors.

  Every shape keeps its door cells and their path to the room's centre carved (a straight
  integer line inside the block).
- **Doors:** for every connection, a gap of `door_m` (1–2 raster cells) in the shared wall, plus
  a stub carved into both rooms where a shape left the wall's foot open. The gap lies at a seeded
  position along the shared segment, at least one cell from its ends. The positions are drawn
  from `sub_rng("doors")`, connections in their `(a, b)` order. Rooms not connected keep a solid
  wall.
- **Room centre:** the room's cell nearest to the mean of its cells (integer squared distance,
  ties to the lowest (row, col)). Door stubs, cover and the critical route use it.
- **Layers:**
  - `rooms` (u16): room number + 1 per cell, 0 for none;
  - `walkable`: room and door cells (cover is a separate layer);
  - `walls`: built wall cells;
  - `corridors`: door cells and `passage` rooms;
  - `doors`: a value, one door per connection with its cells, its two rooms, its kind and its
    centre in metres.
- **Checks** (errors fail the run, naming the stage):
  - every room has at least one cell;
  - no two rooms touch without a wall between them;
  - each door touches exactly its two rooms;
  - no walkable cell lies outside `envelope`.

### 5.8 Cover

0.3.0's sightline breaking, in integers: while the longest straight walkable run (rows, columns
and the two diagonals, in raster cells) exceeds `max_sightline_m`, place a 2×2 pillar in a room or
a rubble cell in a passage at the middle of that run, skipping cells near doors and room centres.
A placement that splits the walkable cells (P08's `labels`) is undone. The loop stops after
`max_items` (default 120). Round halls of at least `pillar_ring_m` across get a ring of pillars
first, as in 0.3.0, kept only if the ring disconnects nothing. Cover never makes an interior
unplayable: the analysis reads `walkable` minus `cover`.

### 5.9 Playability analysis

The stage `playability` reads the graph and the rasters, and writes `analysis`. It fails the run
with an error-level finding whose code names the check that failed and whose message names the
rooms, keys or doors involved. The codes are `uncarved_room`, `disconnected`, `unreachable_room`,
`unsafe_drop` and `door_mismatch`. CE-80.2's checks:

1. **Every room carved:** every room of `room_graph` has cells in `rooms`.
2. **Reachable on the raster:** `walkable` minus `cover` is one component (`labels`, 4-connected),
   and it holds every room's cells.
3. **Lock and key solvable (graph):** the fixed point:
   - start with no keys and the set {start};
   - flood the graph through open connections, locked ones whose key is held, and drops in their
     direction;
   - add every key in the reached rooms;
   - repeat until nothing changes.

   At most K + 1 floods for K keys. Every room and the goal must be reached.
4. **Drops leave no dead end:** for every drop, from its target room, with the keys *surely
   held* there, the goal must stay reachable. Surely held means the keys of the start room and of
   the room the drop leaves from: a party that falls has entered both, and keys are never lost.
   The flood from the target picks up the keys it finds. This is conservative: it refuses some
   drops a party could survive, never one it could not. Unsafe cases are refused, never
   guessed. (The keys reachable with every drop removed are not a subset of what a falling party
   holds: with start–A open, A holding key-1, start–B open, a drop B→C and C–goal locked by
   key-1, a party taking start→B→drop never holds key-1.)
5. **Doors agree with the graph:** every connection has its door, and a door's two rooms are its
   connection's.

**The critical route.** The shortest walk from start to goal that respects locks and drops:
- **Search:** a Dijkstra over states (room, keys held as a 16-bit mask), ordered by (length,
  room id, mask), so ties are resolved totally. At most 16 keys (§5.4) bound the states.
- **Edge lengths:** the length of a connection a–b is the raster path from a's centre through
  the door to b's centre, on `walkable` minus `cover`.
- **Raster paths:** P08's `kernels::paths::shortest_paths` with a cost closure that refuses
  diagonal steps and costs every orthogonal step one cell size. A length is therefore a whole
  number of cell sizes, which is exact in f64 at P34's sizes.
- **Output:** the route is written as the `route` feature (§5.10).

**Metrics** (paths `stages.playability.<metric>`, as P08's summary addresses stage metrics;
0.3.0's names where they exist):

- counts: `rooms`, `connections`, `cycles`, `locks`, `one_way`, `chokepoints` (articulation points);
- shares: `dead_end_pct`, `footprint_used_pct` (walkable over envelope), `hall_share_pct`;
- lengths: `critical_path_m`, `longest_sightline_m`, `start_to_exit_min_m`, `goal_to_exit_min_m`
  (the nearest exit room's centre; 0.3.0 reported lists, which a constraint cannot name).

Each can be named in `[constraints]`.

### 5.10 Features and package layers (interior)

Package layers of the interior kind (`docs/map-package.md` §4.4): `walkable`, `rooms` (u16),
`corridors`, `cover`, and `walls` (1.1, §5.17).

Features (`docs/map-package.md` §5):

- **Ids are per kind.** P08's terrain rule, `<type>-<n>`, would give doors and connections the
  same ids, since both have the types `open` and `locked`. Interior features are therefore
  numbered `<kind>-<n>` (`room-1`, `door-1`, `connection-1`, `route-1`), from 1 per kind, in the
  order the stages add them. The id rule is part of the interior kind's table (§5.2 item 7);
  terrain keeps its rule.
- **Room numbers:** room `room-<k>` has `number` = k − 1, its index in `room_graph`. The `rooms`
  layer holds `number + 1` (`docs/map-package.md` §4.4).

| Kind | Geometry | Properties |
|---|---|---|
| `room` (type = the room type) | Polygon: the cell-edge outline of its cells (`kernels::outline`), holes included | `number` (as in the `rooms` layer), `role`, `keys`, `locked`, `shape` (1.1), `area_m2` (1.1) |
| `door` (`open`, `locked`, `drop`) | Point: the door's centre | `rooms` (the two room ids), `key`; for a drop, `from` (1.1), the room it can be passed from |
| `connection` (`open`, `locked`, `oneway`) | null (§5.2 item 8) | `from`, `to` (room ids; for `oneway` the direction), `key`, `door` (1.1, the door's id) |
| `route` (`critical`) | LineString from the start room's centre through the door centres to the goal room's centre; the third coordinate is 0 (the floor lies at the base level on a one-level interior) | `rooms` (ids in order), `length_m` (1.1) |

- The manifest's summary holds the analysis metrics.
- **Constraints on feature counts.** `features.<type>` counts features by type. The interior
  kind's known types are its static ones (the door, connection and route types) plus the room
  types of the recipe's program. Planning builds that list from the recipe, so a recipe can ask
  for `"features.choir-hall" = [1, 1]`, and a type outside the list is refused before the run.

### 5.11 Renders (interior)

| Render | Content |
|---|---|
| `base` | Frame-exact, 2 pixels per raster cell: floor, walls, cover, and blank outside the envelope. No symbols, no text. It lies under the pins on the Atlas |
| `blueprint` | `base` in blueprint colours, plus: room labels by type; doors (open, locked in red with the key's name, drops as arrows); key rooms marked; exits; the critical route; a scale bar; a legend; and a title with the recipe, the seed and the critical path |
| `graph` | The room graph drawn at its lattice positions: nodes by role, halls sized by footprint, locked connections red with the key, drops as arrows, the critical route highlighted |

0.3.0's extra blueprint skins (`ruin`, `future`) are not standard renders; a recipe may ask for
them through `renders` as `blueprint-ruin` (§9, ruling 9). Drawing follows P08 §5.11.

### 5.12 Recipes

| Recipe | Frame | Program | Rules | Use |
|---|---|---|---|---|
| `choir-hall-ruin` | 84² at 1 m, square outline, 6×6 lattice at 14 m | 0.3.0's: entrance (start), choir hall (goal, 3×3, round), 2–3 tuning galleries, 1–2 junctions, operator room, echo vault (locked, round), 1–2 collapsed (ragged), shrine crypt, 1–2 stores, 2 exits | `main_cycle` (`lock_key`), `merge_hall`, `locked_leaf`, `insert_cycle` ×1, `leaf` | 0.3.0's intent, CE-80.1 |
| `works-halls` | 300² at 1 m (300 m) | about 150 rooms: machine halls (2×3, 3×3), galleries (passages), stores, 2–4 exits | `main_cycle` (`two_routes`), `extend_arc`, `insert_cycle` with locks, `close_loop` ×≤12, `merge_hall`, `locked_leaf`, `shortcut`, `leaf` | A large interior (bands 4 and 9) |
| `ruin-undercroft` | 80² at 1 m, square outline, 4×4 lattice at 20 m | entrance (start), crypt (goal, 2×1), 8–9 rooms with one exit | `main_cycle` (`lock_key`), `leaf`, `merge_hall` | Child of a terrain ruin (CE-80.4) |
| `fixture-interior` | 40² | 6 rooms | `main_cycle`, `leaf` | Tests only, not in `describe` |

- **As built.** The tuning moved three values from this table's first version: the choir hall's
  frame (100 m circle → 84 m square, so the rooms use more than half the floor; a circle holds
  only whole lattice blocks), its hall (2×2 → 3×3, which keeps 0.3.0's 40–52 m hall at the 14 m
  pitch), and the undercroft's crypt (2×1, since a 2×2 goal leaves no room for the loop on 4×4).
  Every built-in also carries a `stages.playability.footprint_used_pct` range.
- **Cell size.** 1 m raster cells, against 0.3.0's 2 m: a one-cell wall and a one-cell door read as
  walls and doors at 1 m, while at 2 m a door is four metres wide. The frame sizes follow from the
  probe's dense floor sizes with staggered walls.
- **Constraints** of every built-in:
  - its room count (`stages.playability.rooms` in a range);
  - its rules' shortfalls (`stages.mission.short = [0, 0]`, §5.6);
  - `stages.playability.critical_path_m`, `dead_end_pct` and `cycles` in ranges the tuning sets
    (§10);
  - `choir-hall-ruin` keeps 0.3.0's `longest_sightline_m` = [10, 45] and `chokepoints` = [1, 6].
- **Tuning target** (§10): every built-in passes its constraints on seeds 1–8. That is stronger
  than CE-80.2, which asks only for the playability analysis.
- **In-world texts:** none. Room type names are the recipe's ids, not canon (as P08 §9).

### 5.13 Ruin child maps

`ruin-undercroft` declares `[inputs] feature = { type = "feature", parent_types = ["ruin"] }`
(§5.2 item 2).

- **Frame:** a run with `from` = a terrain package and `feature` = one of its ruins centres the
  80 m frame on the ruin's point (§5.2 item 4), snapped to whole 1 m cells.
- **The entrance** lies on the rim side the recipe names; the ruin's own position decides nothing
  else in P34.
- **Repeatability:** the parent's hash and the feature id are inputs (§5.2 item 6), so the same
  parent, feature, seed and inputs give the same package bytes. CE-80.4's test runs it twice and
  compares.
- **Frame contents:** `frame.origin_m` and `frame.parent { package_sha256, feature }`, as CE-78.2
  says.

### 5.14 Determinism (G20)

P08 §5.7 applies unchanged.

- The mission and space stages use integers only: lattice cells, raster cells, offsets, counts.
- Matches are enumerated in a total order (§5.6), and every collection the stages iterate is a
  `Vec` or a `BTreeMap`.
- Distances in metres are integer cell steps times the cell size, which is exact in f64 for
  P34's sizes. 0.3.0 used BFS steps the same way.
- Room outlines have integer vertices in raster cells, multiplied by the cell size, so they lie on
  §5.2 item 10's lattice when the cell size is a multiple of 1/16 m (the recipe loader checks it).
- `ragged` shapes and the ragged outline compare P08's spectral noise with a recipe threshold:
  a float comparison on values that P08's kernel computes bit-identically on every platform
  (its golden hashes pin them). Each noise field is one call per stage run on a named sub-stream
  (`envelope`: `sub_rng("outline")`; `space`: `sub_rng("ragged")`), so adding a room does not
  shift another room's noise.

### 5.15 Golden hashes and the generator version

- **Golden runs added** (§5.2 item 9):
  - `choir-hall-ruin` seed 1;
  - `ruin-undercroft` seed 1 from `fixture-small` seed 1 and its one ruin.

  Every pipeline stage's layers, `features` and the package bytes are hashed, as in P08.
- **The version** rises under §5.2 item 9's rule: 1.1.0 if P34 merges first, else 1.2.0. The old
  golden file stays as it is (CE-77.2).
- **Platforms.** The build PR gets the labels `windows` and `macos` for CE-77.1's four platforms,
  as P08's did.

### 5.16 How each AC is proven

| AC | Test |
|---|---|
| CE-80.1 | For each built-in interior recipe: the run returns `base`, `blueprint`, `graph`, `features` and the package. The package's manifest has kind `interior`, and its layers are §5.10's with the format's dtypes. Every room feature is a Polygon whose area equals its cell count times the cell area. Every door is a Point with two room ids. Every connection has null geometry with `from` and `to`. There is one `route` of type `critical` whose rooms run from the start room to the goal room. `base`'s size equals the frame at 2 pixels per cell |
| CE-80.2 | A test over seeds 1–8 of every built-in interior recipe: the playability stage passes, and its analysis reports every room carved, one component, the key fixed point reaching every room and the goal, and every drop safe |
| CE-80.3 | A test registry adds `hand_graph`, a stage that reads `lattice` (from `envelope` on a 6×6 rect test recipe without offsets) and writes a hand-made `room_graph` on fixed cells: seven rooms, a main cycle, one lock with its key on the other arc, a drop from the goal, and a hall of 2×2 cells. `space`, `cover` and `playability` run on it, and the run passes. A second hand-made graph, whose key sits behind its own lock, fails `playability` with the finding code `unreachable_room`, naming the goal and `key-1`. So the analysis is not vacuous |
| CE-80.4 | `fixture-small` seed 1 is run into a temporary folder, then `ruin-undercroft` with that package as `from` and its ruin as `feature`. The child's `frame.parent` holds the package's SHA-256 and the ruin's id. `frame.origin_m` is the ruin's point minus half the frame, snapped. Running it twice gives the same package bytes. A run with a settlement's id, with an unknown id, or without `from` is refused before any stage runs |
| CE-80.5 | The golden test covers `choir-hall-ruin` seed 1 and the ruin child (§5.15): on Linux in the PR; on Windows and macOS through the labels |

Further tests (no AC of their own):

- each rule on a fixed small lattice (matches in order, the result, its solvability);
- the guard, with a test-only rule that locks a connection whose key lies behind it: it is undone
  and counted as `short`;
- `kernels::outline` against hand-drawn masks (holes, diagonal touches);
- the critical route on a graph with two keys, where the route must fetch both;
- the drop check on a graph whose drop leads past a lock without its key (refused);
- the recipe loader refusing a lattice that does not fit the frame, more than 16 keys, a cell size
  off the 1/16 m lattice, and an unknown rule.

### 5.17 Document changes in this PR

- **`docs/map-package.md`**, format 1.1 (§5.2 item 11), each addition marked "(1.1)":
  - §4.4: the interior layer `walls` (u8 mask, built wall cells);
  - §5: `room` gains `shape` and `area_m2`; `door` gains `from` for drops; `connection` gains
    `door`; `route` gains `length_m`;
  - the 1.1 title, `format_version` row and introductory sentence are P29's text, so the two PRs
    merge as a union of rows.
- **`docs/generators.md` §3:** unchanged by P34. The `parent_types` sentence is P29's.
- **`docs/status.md`:** P34 → `spec-pr`.
- **`research/P34-interiors/`:** the probe (§4.3).

## 6. Interfaces other phases use

| Interface | Used by |
|---|---|
| `rooms::{RoomGraph, Room, Connection, Passage, Role, Cell, Lattice}` and the `room_graph` value | P30 (levels: `Cell` gains a level, `Passage` gains links between levels), the research track's ruin pass (P37), cavern kind (P40) and ring layouts |
| `rooms::solve` (fixed point, drop check, critical route) | P30 makes it hierarchical over levels (CE-79.2) |
| The `Rule` trait and the rule registry | P30 (rules that place stairs, lifts and shafts), the research track |
| The interior stages `envelope`, `mission`, `space`, `cover`, `playability` | P30 extends them per level |
| The interior features, layers and renders | P16 (Maps screen), P22 and P25 (`base`, frame, features), P23 (the blockout reads interior packages) |
| The shared child-map plumbing (§5.2) | P29; later child kinds |

## 7. Risks

| Risk | Mitigation |
|---|---|
| The lattice look persists at large sizes: the probe's 150-room map read as tiles | Staggered walls (§5.7), hall footprints in the program, `passage` rooms; the tuning reviews `blueprint` sheets of seeds 1–8 (§10) |
| Too many or too few cycles | `close_loop` is capped by `count`; `cycles` and `dead_end_pct` are constraints of every built-in |
| Rules run out of free cells on small or ragged envelopes (the probe's cave reached its target on 4 of 8 seeds) | Rooms counted by a constraint, shortfalls reported per rule; the built-ins are tuned to pass on seeds 1–8 |
| Critical routes much shorter than 0.3.0's (dense floors are compact) | `critical_path_m` is a constraint; `main_cycle` takes a minimum arc length; `extend_arc` lengthens arcs |
| The guard hides a wrong rule by undoing it silently | Shortfalls are metrics, a test forces the guard, and each rule's unit test asserts it applies fully on its fixture |
| The (room, keys) Dijkstra grows with keys | At most 16 distinct keys per recipe; the states actually reached are few (the probe: 11 locks in under 20 ms) |
| P29 and P34 build the shared plumbing twice | §5.2: the first build to start builds it; the second merges `main` first |
| Both phases bump the generator version | §5.2 item 9 fixes the order |

## 8. Review Focus — inputs most likely to break it

1. **Lattices:**
   - a lattice of one row;
   - an envelope that disables the start's rim side;
   - a ragged circle leaving islands of enabled cells;
   - `jitter_m` at its maximum on a two-cell-wide room;
   - a pitch not a whole number of raster cells.
2. **Rules:**
   - a rule with no match on its first application;
   - `count = [0, 0]`;
   - `insert_cycle` on the main cycle's locked connection;
   - `merge_hall` swallowing a key room or the start;
   - `shortcut` between rooms behind different locks;
   - `locked_leaf` when every free cell borders only locked rooms.
3. **Space:**
   - two halls meeting at a corner;
   - a round room in a 1×1 block smaller than its door stubs;
   - a ragged room whose eroded edge cuts its own door;
   - a door between vertical neighbours whose shared segment is at its minimum, `p − 2j`.
4. **Analysis:**
   - a key in the start room;
   - a lock whose key lies behind another lock;
   - a drop into the goal;
   - a drop from the start;
   - two drops in a row;
   - a graph whose critical route must revisit a room;
   - 16 keys.
5. **Cover:**
   - a hall whose pillar ring would cut a door;
   - a corridor-only interior where every rubble cell disconnects.
6. **Child runs:**
   - a ruin at the parent's corner (the frame reaches past the parent);
   - a parent package with format 1.0 read by a 1.1 build;
   - a parent with `requires` the build does not know;
   - a renamed parent file (the hash, not the path, is recorded).
7. **Determinism:**
   - match enumeration order;
   - ties in the critical route;
   - a recipe whose cell size is not a multiple of 1/16 m.

## 9. Rulings (Claude's calls)

1. Ruling: interiors use 1 m raster cells, not 0.3.0's 2 m — doors and walls one cell wide read as
   doors and walls at 1 m; at 2 m a door is 4 m wide — cost if wrong: retuned frame sizes.
2. Ruling: the `exit` role replaces 0.3.0's `extraction` in the room program and places no
   feature of kind `poi` — places inside interiors wait in the research track (decision 0004) —
   cost if wrong: a later rename to `chime` with a new feature.
3. Ruling: keys are never lost and open every lock that names them — this makes solvability a
   fixed point, and Unexplored's safe main locks are of this kind (Dormans 2017) — cost if wrong:
   consumable keys need a search over key sets later.
4. Ruling: `room_graph` positions are lattice cells, so P34 has no layout step for graphs without
   positions — every writer in sight (P34's rules, P30, the research track's ring and cavern
   kinds) works on a lattice; a configuration-space step (Ma et al. 2014) can be added when a kind
   needs free shapes — cost if wrong: that step later, as one more stage.
5. Ruling: the space is dense (rooms fill their cells, doors in shared walls); the sparse
   corridor layout of the probe's round 1 is not built — dense reaches 58–68 % floor use against
   8–22 %, and `passage` rooms give corridors where a recipe wants them — cost if wrong: a
   corridor stage later.
6. Ruling: staggered walls (`jitter_m`, offsets per vertical wall within each lattice row) are
   designed without a third probe round — they keep every invariant by construction (§5.7) and
   only change room widths and where walls meet — cost if wrong: the tuning finds the 150-room
   look still too regular and adds hall types.
7. Ruling: the drop check holds only the start room's and the drop room's keys at the target
   (as built, §5.9 item 4) — a true lower bound and cheap; a dominator computation (`06c` §5) is
   more permissive but needs more code — cost if wrong: a recipe's drop refused that would have
   been safe.
8. Ruling: sightlines are measured along rows, columns and both diagonals in raster cells — 0.3.0
   used rows and columns only; line of sight at any angle is P33's (CE-83.1) — cost if wrong:
   long oblique sightlines pass the constraint.
9. Ruling: 0.3.0's blueprint skins `ruin` and `future` become an optional render `blueprint-ruin`
   only, and `future` is dropped — CE-80.1 names three renders; the future skin has no use in the
   project — cost if wrong: one more style table.
10. Ruling: the built-in interiors are tuned to pass their own constraints on seeds 1–8, not only
    the playability analysis — a short run must be visible — cost if wrong: tuning time.
11. Ruling: `choir-hall-ruin`'s critical route range is retuned to the dense floor, not 0.3.0's
    [90, 260] m — the dense plan is about a third of 0.3.0's floor, and the probe measured 96–144 m
    — cost if wrong: none for the ACs; a recipe value.
12. Ruling: P34 reads no parent layer — a ruin's undercroft takes its frame from the ruin's point
    and nothing from the terrain above; terrain-aware entrances can come with P30 or the ruin
    pass — cost if wrong: one `parent.height` read later.

## 10. Left to the builder

- The parameter values of the built-in recipes (pitch, jitter, wall, door, counts, constraint
  ranges) after tuning within §5.12's rules, with the `blueprint` sheets of seeds 1–8 reviewed.
- The palette and layout of the renders within §5.11.
- Module boundaries inside `rooms/` and `stages/interior/`, as long as each stage and each rule is
  one module.
- The order of tasks, provided that the shared plumbing (unless P29's build has it on `main`), the
  room graph and the solver come first, and every rule lands with its unit test.
