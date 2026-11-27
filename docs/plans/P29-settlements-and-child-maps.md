# P29 · Settlements and child maps — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (`CLAUDE.md` makes it the execution method) to implement this plan task by task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** `theos-maps` makes settlement maps (hamlet to planned grid town) with streets, blocks,
plots, buildings, landmarks and an optional wall with gates, as a child map of a region package
or standalone, and gains the child-map plumbing that P29 and P34 share.

**Architecture:**
- Part A builds the shared plumbing of spec §5.2: map kinds, one table per kind (layers, renders,
  feature types, id scheme, metrics, features, frame rule), declared recipe inputs, runs from a
  parent package (`from`), the child frame, parent values in the world, `Geometry::None`, the
  integer lattice, format 1.1 and golden runs with a parent.
- Part B adds the lattice geometry of §5.4: a thin `i_overlay` wrapper plus own hull,
  minimum-area box, bisector cut and snap-rounding noding.
- Parts C–E add the settlement stages in pipeline order (§5.5–§5.13), each one module under
  `stages/settlement/` plus one line in `stages::register`, then the features, renders, package
  layers and the three built-in recipes.
- Part F records the CE-78.3 reference from 0.3.0, proves parity, records the golden hashes under
  a new `VERSION` and verifies the whole phase.

**Tech stack:** Rust 1.99.1 (edition 2024) and the crates P08 added. One new dependency group:
`i_overlay` 9.2.0 without default features (licence `MIT OR Apache-2.0`; its dependencies
`i_float` 5, `i_shape` 5, `i_tree` 0.20, `i_key_sort` 0.12 are `MIT`, and `libm`, already in the
tree). Both licences are on the `deny.toml` allowlist.

**Spec:** `docs/specs/phases/P29-settlements-and-child-maps.md` (approved in #54). Read it with
this plan. Where this plan is silent, the spec decides; where they differ, the spec wins. Section
numbers (§5.9 …) below are the spec's. §5.2 is word for word P34's §5.2
(`docs/specs/phases/P34-interiors.md`).

**References for implementers:**
- P08's plan `docs/plans/P08-map-format-and-terrain-generator.md` ("Build rules", Tasks 8–12,
  33, 36–42, 45) shows how the crates are built; its code is the reference for every existing
  signature.
- The Python toolset `theos_maps` 0.3.0 is at `D:\Projects\theos_maps-export` (in no repository):
  `theos_maps/settlement.py` (class table, landmarks, fields), `theos_maps/details.py`,
  `theos_maps/run.py`. Only Task 13 runs it.
- `research/P29-settlements/` holds the 0.3.0 baseline driver and its README.

## Before the build

- **No gate PR is needed.** `i_overlay` and its `i_*` dependencies are `MIT` or
  `MIT OR Apache-2.0`, both on `deny.toml`'s allowlist; `i_overlay`'s only thread code
  (`rayon`) sits behind its feature `allow_multithreading`, which stays off; no `xtask` or `gate/`
  file lists the crates of `theos-maps`. Nothing on this branch touches a protected path except
  `docs/plans/` and `docs/status.md`.
- **Optional hardening, not needed for the build:** a `deny.toml` `[bans] deny` entry for
  `rayon` ("no threads inside a run, G20") would keep a later feature flip from pulling threads
  into a map run. It is a gate change and would be its own PR for Jann.
- **P34.** P29 builds Part A first; P34's build merges `main` and reuses it. A change to
  anything Part A builds is agreed with the P34 session first (spec §5.2, risk "P29 and P34 change
  the same files").

## Global Constraints

The umbrella's global constraints, verbatim (`CLAUDE.md` asks for G1–G16):

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

G20 binds every task too: "Map generation is deterministic: the same generator version, recipe,
seed and inputs give the same package bytes and layers on every platform of G2. Canon maps are
baked into packages (`docs/map-package.md`) and shipped, never regenerated in the game (decision
0003)."

Phase ACs, verbatim:

**CE-78 theos-maps settlements and child maps**
- CE-78.1 Settlement maps run from a built-in settlement recipe, with the renders `base` and
  `plan`, and packages whose layers and features follow `docs/map-package.md` §4.4 and §5
  (buildings, streets, walls, gates, landmarks).
- CE-78.2 A child map runs from a parent package (`from`) and one of its settlement features (an
  input of type `feature`); its frame carries `origin_m` in the parent's frame and `parent` with
  the parent package's hash and the feature id; the same parent, feature, seed and inputs give the
  same child.
- CE-78.3 For seeds 1–8, the built-in settlement recipe passes its constraints at least as often as
  0.3.0's settlement recipe did on the same seeds, recorded as a fixture.
- CE-78.4 The golden hashes of CE-77.1 cover a settlement and a settlement's child map.

Every P29 stage also keeps these (spec §2), verbatim from the umbrella's P08 entry:
- CE-77.1 For fixed recipes and seeds, the hashes of every layer after every stage, of the
  `features` output and of the package bytes equal the golden hashes on Linux x64, Windows x64,
  and macOS on Apple silicon and Intel.
- CE-77.2 Golden hashes are kept per generator version: outputs that differ from the golden hashes
  of the current version fail a test, and the tool that records golden hashes refuses to overwrite
  those of an existing version.
- CE-28.3 A recipe whose stage order leaves a stage without an input fails before the first stage
  runs, with a message naming the stage and the missing layer; an unknown stage or recipe input
  fails the same way.

Jann's decisions that bind the build (spec §4.1): approach B (one pipeline, two street styles);
the plumbing is shared with P34; a run without buildings fails; the first towns are early starter
towns for floors 1–10; the range goes "from simple settlements to more medieval towns to more
modern city layouts".

**Engineering directives** (`CLAUDE.md`, in brief; put them in every brief):
1. Maintainability first: small modules with one job; a new stage is one module plus one line in
   `stages::register`; a new kind is one entry in `kind::KINDS`.
2. Use what exists: P08's `Stage` contract, runner, `Router`/`StepCost`, kernels, renders and
   format crate; `i_overlay` for polygon booleans, offsets and slices. Own code only where the
   spec says no crate fits (hull, box, bisector cut, noding, organic growth, tensor field, wall).
3. When review fixes keep leaking, name the class and change the approach.
4. The gate catches everyday shortcuts; it is not a fortress.

## Build rules (every task, every subagent)

Put this section into every implementer and reviewer brief. P08's "Build rules" apply unchanged;
the points below repeat the ones implementers break most and add P29's.

- **Determinism (G20, P08 §5.7):** math functions only through `libm`; no std float methods
  (clippy rejects them), no `mul_add`; float `max`/`min` only through `kernels::num::{max, min}`;
  sort and queue keys through `kernels::num::key` and `total_cmp`, ties broken by lattice
  position (y, then x), then by index; no `HashMap`/`HashSet` (use `BTreeMap`, `Vec`); no
  threads; one RNG stream per stage (`ctx.rng()`, `ctx.sub_rng(purpose)`); seed arithmetic
  `wrapping_*`.
- **Lattice (spec §5.4):** every street vertex, polygon vertex and point that later code compares
  or splits is an `i32` lattice point (1/16 m) from the moment it is made. Orientation and
  intersection tests in `i64`, areas and box comparisons in `i128`. No float enters a geometric
  decision. Floats only in the terrain, the interest field and the tensor field.
- **Casts:** `as` only inside `kernels::num` (and its new lattice helpers in
  `kernels::geometry::lattice`), each with an `#[expect(clippy::cast_…, reason = "…")]`.
- **Lints:** no `#[allow]`; `#[expect]` only with a reason; every public item documented; every
  public fn returning `Result` has `# Errors`; no `unwrap`, `expect`, `panic!`, `todo!`, `dbg!`,
  `print!` outside tests.
- **Errors and logs:** `thiserror` in the library; a stage fails with `ctx.fail("…")` naming what
  is missing (spec §5.3: "no entry reaches the centre", "no block is left", "no building
  fits"), never by returning an empty town. Log with `tracing` (`debug!` per stage step,
  `warn!` with every warning finding).
- **Files:** one job per file, under about 400 lines; tests in `#[cfg(test)] mod tests` or a
  sibling `tests.rs` as P08 does; behaviour tests in `crates/theos-maps/tests/maps/`; fixtures in
  `crates/theos-maps/tests/fixtures/`. Fixture recipes start with
  `# synthetic fixture (P29), not canon`. Generated names are "generated, not canon".
- **Dead code between tasks:** a module a later task wires in carries
  `#[cfg_attr(not(test), expect(dead_code, reason = "wired in by Task N"))]` on its `mod` line;
  the wiring task removes it.
- **AC tests:** a plain `#[test]` with `// AC: CE-78.x` on the line directly above `#[test]`.
  Map-scale tests (a parent plus a town, or more) are named `slow_…` so the default nextest
  filter skips them (`.config/nextest.toml`).
- **Dependencies:** pinned in the root `[workspace.dependencies]`; the adding commit runs one
  unlocked `cargo build -p theos-maps`, then `cargo deny --locked check`.
- **Git:** one git command per Bash call in the worktree; never `--no-verify`; never push.
  Conventional Commits, scope `generators` (`build` for a dependency-only commit), a body that
  says why, and one footer block (no line over 100 characters; leave `Refs:` out when the task
  proves no AC):

  ```text
  Refs: CE-78.2
  Phase: P29
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  ```
- **Per-task checks** (from the worktree root; read the output): `cargo fmt --all --check`;
  `cargo clippy --workspace --all-targets --locked -- -D warnings`;
  `cargo nextest run -p theos-map-format -p theos-maps --locked`; with a new dependency also
  `cargo deny --locked check`. Until Task 33 the 1.0.0 golden test
  (`cargo nextest run -p theos-maps -E 'test(/slow_outputs_match/)'`) must keep passing after
  every Part A task: no task before Task 33 changes a terrain output.

## Review Focus

The inputs the spec implies (§8) that no AC test covers and that are most likely to bite. Each
line names its test and the task that owns it.

1. **A settlement at the parent's edge or corner, and a parent without roads.** Expected: the
   window reaches past the edge, cells past it read the nearest edge cell, `origin_m` may be
   negative; with no entry the stage fails with "no road enters the window of `<feature>`"
   instead of an empty town. Tests: Task 5 (`resample_clamps_samples_past_the_parent_edge`),
   Task 4 (`a_window_past_the_parent_corner_keeps_its_size_and_a_negative_origin`), Task 15
   (`a_parent_without_roads_fails_naming_the_feature`).
2. **A wrong feature input.** Expected: an unknown id, a ruin given to a settlement recipe, a
   `.theosmap` whose hashes fail, and a parent from another major format are refused before any
   stage with a message naming the id, the type and the accepted types, or the file; a Polygon
   feature is centred on its bounding box. Tests: Task 4
   (`a_feature_id_the_parent_lacks_is_refused`, `a_ruin_is_refused_by_a_settlement_recipe`,
   `a_polygon_feature_is_centred_on_its_bounding_box`,
   `a_parent_with_a_broken_hash_is_refused_naming_the_file`).
3. **Degenerate street input.** Expected: two entries at one cell give one node and two main
   streets that merge; a street ending exactly on another street's vertex makes one node; two
   collinear overlapping streets become one edge of the higher class; streamlines exactly
   `d_sep` apart are kept or stopped the same way on every run; noding creates no new crossing.
   Tests: Task 14 (`collinear_overlaps_merge_keeping_the_higher_class`,
   `an_end_on_a_vertex_makes_one_node`, `snap_rounding_creates_no_new_crossing`), Task 17
   (`two_entries_at_one_cell_share_a_node`), Task 20
   (`lines_exactly_d_sep_apart_are_decided_by_lattice_order`).
4. **A wall zone a lake blocks, or a river or main street crossing the ring twice.** Expected:
   with no closed ring the stage fails naming the zone; each river crossing gets its own
   `water-gate`; a main street crossing twice fails the check "every main street crosses it
   exactly once"; a ring zone narrower than one cell fails naming the zone. Tests: Task 21
   (`a_lake_across_the_ring_zone_fails_naming_the_zone`,
   `each_river_crossing_gets_a_water_gate`, `a_main_street_crossing_twice_fails_the_check`,
   `a_ring_zone_narrower_than_a_cell_fails_naming_it`).
5. **Awkward blocks and plots.** Expected: a sliver or a triangle becomes open ground or one
   parcel, never a parcel without area; a block with a hole keeps the hole; a corner near 180°
   gets no bisector cut; frontage bounds wider than the block give one parcel; a landmark that
   cannot grow to its area is reported in `landmarks_absent`. Tests: Task 22
   (`a_sliver_and_a_triangle_become_open_ground`, `a_block_with_a_hole_keeps_it`), Task 23
   (`a_straight_corner_gets_no_cut`, `frontage_wider_than_the_strip_gives_one_parcel`), Task 24
   (`a_landmark_that_cannot_grow_is_absent_with_its_reason`).
6. **Lattice limits and determinism.** Expected: `snap` rounds half away from zero, coordinates on
   the window edge snap inside the i32 range, windows up to 134 km fit; switching a stage off
   does not shift other stages' RNG streams; `-0.0` in the tensor field gives the same streets
   as `+0.0`. Tests: Task 8 (`snap_rounds_half_away_from_zero`,
   `a_134_km_window_fits_and_a_larger_one_is_refused`), Task 6
   (`switching_a_stage_off_keeps_the_other_streams`), Task 19
   (`negative_zero_in_the_field_traces_like_positive_zero`).

## Rulings made while planning

These go into the ledger's "Rulings (build)" and the PR. Spec gaps and contradictions behind
them are listed at the end of the plan ("Spec gaps found while planning").

- **Plumbing**
  - L1. Ruling: everything that differs per kind lives in one `KindTable` per kind in
    `src/kind/` (package layers, renders, feature types, id scheme, summary paths, metrics,
    features builder, title line, frame rule, seeded values), looked up by `MapKind` — §5.2
    item 7 asks for "one table per kind, chosen by `MapKind`" — cost if wrong: none.
  - L2. Ruling: `MapKind::Interior` and its table entry come with Part A, with an empty layer,
    render and type list and the interior frame rule of §5.2 item 4; P34 fills it — one enum and
    one table, built once — cost if wrong: P34 edits one entry.
  - L3. Ruling: the id scheme is part of the kind table: `IdScheme::ByType` (`<type>-<n>`,
    terrain and settlement) or `IdScheme::ByKind` (`<kind>-<n>`, interior, P34 §5.10) — agreed
    with the P34 session — cost if wrong: none.
  - L4. Ruling: a stage says whether it runs in a child run, a standalone run or both through
    `TypedStage::RUNS` (`Runs::Always` by default, `Runs::Child` for `local_terrain`,
    `Runs::Standalone` for `site_terrain`). Planning parses every stage's table, then leaves out
    the stages that do not run in this mode; their RNG keys are never drawn — spec ruling 4 needs
    one recipe file for both modes, and a recipe key could be misused — cost if wrong: a recipe
    key later.
  - L5. Ruling: a child recipe without `from` is refused, unless its plan has a
    `Runs::Standalone` stage; then it runs standalone and the `feature` input must be absent — the
    reconciliation of §5.2 item 3 with §5.16 and ruling 2 (see the gaps) — cost if wrong: the
    standalone golden run moves to a separate recipe.
  - L6. Ruling: recipe inputs take the three declarable types of `docs/generators.md` §3 that a
    recipe can check on its own: `feature` (with `parent_types`), `number` (`default`, `min`,
    `max`, `integer`) and `enum` (`values`, `default`). `cameras` is refused at loading as "not an
    input type of theos-maps" — §5.2 item 2 says "in the form of §3" — cost if wrong: one more
    variant.
  - L7. Ruling: `grid` and `size_km` become optional in the recipe schema; terrain and interior
    recipes must give them, settlement recipes must not (their frame comes from the class table,
    §5.5) and are refused naming the key — cost if wrong: one check.
  - L8. Ruling: the class table is the recipe's top-level `[child.class.<class>]` (`window_m`,
    `cell_m`, `r_built_m`), defaults from spec §5.5. Standalone, the class is `site_terrain`'s
    `class` parameter, read by the settlement frame rule from the raw stage table — spec §5.5
    names both, a stage cannot read recipe-level tables — cost if wrong: a top-level `class` key.
  - L9. Ruling: a window has `round_half_even(window_m / cell_m)` cells per side (village:
    333 cells of 3 m, 999 m) and its origin is the feature point minus half the size, snapped to
    `round_half_even(o / cell_m) · cell_m` in the parent's frame — §5.2 item 4 "snapped to whole
    child cells" — cost if wrong: a new `VERSION`.
  - L10. Ruling: besides `parent.*`, the runner seeds the world value `class_frame` (the resolved
    class entry) for settlement runs in both modes, declared like a parent value — stages need
    `r_built` and there is no parent standalone — cost if wrong: none.
  - L11. Ruling: `parent.<layer>` of a class layer (one with a legend, such as `water`) is
    resampled by nearest cell, a measured layer by Catmull-Rom — bicubic class codes are not
    classes (see the gaps) — cost if wrong: lake outlines change.
  - L12. Ruling: the golden record gets a pseudo stage `parent#0` with the hash of every seeded
    value, so a platform difference in `kernels::resample` is named before the first stage —
    cost if wrong: none.
  - L13. Ruling: features may reference items of other world values by list and index
    (`Prop::Item { list, index }`), resolved to ids by the kind's features builder, as P08's
    `Prop::Road` does for roads; P34 can use it for rooms — cost if wrong: none.
  - L14. Ruling: `VERSION` becomes 1.1.0 with `1.1.0.toml` holding every golden run, terrain
    included (spec ruling 16: format 1.1 changes terrain package bytes) — §5.2 item 9 — cost if
    wrong: none. If P34 merges first, P29 merges `main` and takes 1.2.0.
- **Settlements**
  - L15. Ruling: `local_terrain` and `site_terrain` also write `rivers` (lattice polylines with
    `width_m` and `discharge_m3s` per vertex: the parent's `river` features clipped to the window,
    or the standalone river with a `discharge_m3s` parameter); `buildings` reads it optionally for
    the mill rule's `mill_q` — the spec's mill rule reads a discharge no listed value holds —
    cost if wrong: one more value.
  - L16. Ruling: organic streets connect each candidate with an early-exit least-cost search
    (`kernels::paths::path_to_nearest`, stops at the first cell of the target set) — one full
    tree per candidate breaks the 10 s budget at 400² — cost if wrong: none.
  - L17. Ruling: lattice polygons are wound with a positive shoelace area in (x east, y south),
    the convention of `kernels::contour::rings`; holes negative — the spec's "counter-clockwise"
    depends on the y axis — cost if wrong: none.
  - L18. Ruling: settlement metrics are stage metrics of `settlement_metrics` only; constraint
    paths are `stages.settlement_metrics.<name>`, and the settlement kind has no top-level summary
    paths — §5.13 "reports under `summary.stages.settlement_metrics`" — cost if wrong: none.
  - L19. Ruling: `grid_index` is computed without trigonometry: per 40 m chord `(dx, dy)` of
    length L, `c2 = (dx² − dy²)/L²`, `s2 = 2·dx·dy/L²`, `c4 = c2² − s2²`, `s4 = 2·s2·c2`;
    the index is `sqrt(mean c4² + mean s4²)` over all chords (the remainder shorter than 40 m
    dropped) — exact operations only — cost if wrong: none.
  - L20. Ruling: the `buildings` stage writes `features` with the building and landmark
    features (refs to parcels as `Prop::Item`); streets, parcels, wall, towers and gates are
    built after the pipeline from their values — §5.3 lists `features` as a write of `buildings`
    and §5.14 builds the rest from the values — cost if wrong: none.

## Tasks at a glance

| # | Task | ACs |
|---|---|---|
| **A** | **Shared child-map plumbing (shared with P34)** | |
| 1 | Kinds, stage modes and the kind table | – |
| 2 | Features: `Geometry::None`, `Prop::Item`, id schemes | – |
| 3 | Recipe inputs and the optional frame keys | CE-28.3 |
| 4 | Parent package, feature input and child frame | – |
| 5 | Kernel `resample` | – |
| 6 | Parent values in the world, planning modes | CE-28.3 |
| 7 | Runs from `from`: package and manifest | – |
| 8 | Kernel `geometry::lattice` | – |
| 9 | Golden runs with a parent | CE-77.2 |
| **B** | **Geometry** | |
| 10 | `i_overlay` and its wrapper | – |
| 11 | Hull, minimum-area box, bisector cut | – |
| **C** | **Settlement stages to the main streets** | |
| 12 | Settlement values and the class table | – |
| 13 | The 0.3.0 reference (CE-78.3 fixture) | CE-78.3 (fixture) |
| 14 | `street_graph`: noding and the `streets` raster | – |
| 15 | `local_terrain` | – |
| 16 | `site_terrain` | – |
| 17 | `site_plan` and `main_streets` | – |
| **D** | **Streets, wall, blocks, plots, buildings** | |
| 18 | `organic_streets` | – |
| 19 | Tensor field and streamline tracer | – |
| 20 | `planned_streets` | – |
| 21 | `wall` and gates | – |
| 22 | `blocks` | – |
| 23 | `parcels` | – |
| 24 | `buildings` and landmarks | – |
| 25 | Budget: one 1,600 m town at 4 m | – |
| 26 | `fields` | – |
| 27 | `settlement_metrics` | – |
| **E** | **Outputs and recipes** | |
| 28 | Settlement features (format 1.1) | – |
| 29 | Settlement package layers | – |
| 30 | Renders `base` and `plan` | – |
| 31 | Built-in recipes and the AC runs | CE-78.1, CE-78.2 |
| **F** | **Parity, golden, finish** | |
| 32 | Parity with 0.3.0 | CE-78.3 |
| 33 | Golden runs, format 1.1 and `VERSION` 1.1.0 | CE-78.4, CE-77.1, CE-77.2 |
| 34 | Finish and verification | – |

Order: Part A before everything; Task 13 needs only the spec and may run any time before Task 32;
Tasks 18–21 depend on Task 17; Task 25 runs as soon as Task 24 is in, before tuning starts.

> **The interface blocks are planning-time snapshots.** Signatures and names below show the build
> as planned; the ledger's rulings and the code supersede them, and the code is the reference.

---

## Part A — shared child-map plumbing (shared with P34)

> P29 builds this part first (agreed with the P34 session); P34's build merges `main` and reuses
> it. Before starting, `git fetch` and check `origin/main` once for an already merged
> implementation. A change to an interface below is agreed with the P34 session.

### Task 1: Kinds, stage modes and the kind table (§5.2 items 1, 7)

**Files:**
- Modify: `crates/theos-maps/src/stage/mod.rs` (`MapKind`), `stage/typed.rs`, `stage/mod.rs`
  (`Stage::runs`), `recipe/plan.rs`, `api/needs.rs`, `api/steps.rs`, `api/describe.rs`,
  `constraints.rs`, `summary.rs`, `render/mod.rs`, `package/mod.rs`, `package/table.rs`,
  `features/types.rs`, `lib.rs`.
- Create: `crates/theos-maps/src/kind/{mod.rs, terrain.rs, interior.rs, settlement.rs}`.

**Interfaces — Produces:**

```rust
// stage/mod.rs
pub enum MapKind { Terrain, Settlement, Interior }       // serde snake_case; Display "settlement" …
pub enum Runs { Always, Child, Standalone }              // which runs a stage takes part in (L4)
// Stage gains:      fn runs(&self) -> Runs;
// TypedStage gains: const RUNS: Runs = Runs::Always;

// kind/mod.rs
pub enum IdScheme { ByType, ByKind }                     // L3
pub(crate) struct KindTable {
    pub kind: MapKind,
    pub layers: &'static [PackageLayer],                 // package layers in order (Task 29 fills settlement)
    pub optional_layers: &'static [PackageLayer],        // written when the world holds them (terrain: omega)
    pub renders: &'static [Render],                      // standard and extra renders of the kind
    pub types: &'static [(&'static str, &'static str)],  // (kind, type); `features.<type>` paths
    pub ids: IdScheme,
    pub summary_paths: &'static [&'static str],          // terrain: metrics::PATHS; others: []
    pub metrics: MetricsFn,                              // top-level summary metrics
    pub features: FeaturesFn,                            // builds the features output
    pub title: TitleFn,                                  // title line of the people renders
    pub frame: FrameRule,                                // Task 4
    pub seeded: SeededFn,                                // Task 6: values the runner seeds
}
pub(crate) type MetricsFn = fn(&World, &[(String, Metrics)]) -> Result<Value, FormatError>;
pub(crate) type FeaturesFn = fn(&World, IdScheme) -> Result<(FeatureCollection, BTreeMap<String, usize>), FeaturesError>;
pub(crate) type TitleFn = fn(recipe: &str, seed: u64, summary: &Value) -> String;
pub(crate) const KINDS: &[KindTable];                    // one entry per MapKind
pub(crate) fn table(kind: MapKind) -> &'static KindTable;
```

`FrameRule` and `SeededFn` are placeholders here (`FrameRule::Recipe`, `seeded` returning
nothing) that Tasks 4 and 6 complete. The terrain entry takes today's `TERRAIN_LAYERS`,
`OMEGA_LAYER`, `RENDERS`, `TYPES`, `metrics::PATHS`, `metrics::compute`,
`features::output::build`, `render::title_line`. Settlement and interior entries have empty
lists; their `metrics` return `{}`, their `features` the set alone.

Every user switches to the table of `plan.kind`: `needs::render_names`/`needs`, `steps::draw`
(render lookup), `steps::constraints` (metrics, features, title), `package::build` (layers and
`manifest.kind = plan.kind.to_string()`), `constraints::is_known_path` (paths and types of the
kind), `summary::summary` (feature counts of the kind's types), `recipe::plan::check_renders`.
`render::names(role)` and `render::needs(name)` take the kind.

- [ ] **Step 1: Write the failing tests:**
  - `map_kinds_are_snake_case_in_recipes`: extend the existing test with `"settlement"` and
    `"interior"` both ways.
  - `every_kind_has_exactly_one_table`: for each of the three kinds, `table(kind).kind == kind`,
    and `KINDS.len() == 3`.
  - `the_terrain_table_is_p08s`: its layers' names equal today's `TERRAIN_LAYERS` names, its
    standard renders are `["base", "truth", "geology", "resources", "inworld"]`, extras
    `["omega"]`, its ids `ByType`.
  - `a_feature_type_path_is_known_only_for_its_kind`: `features.ruin` is a known path in a
    terrain plan and unknown in a settlement plan.
  - `stages_default_to_running_always`: a test stage without `RUNS` reports `Runs::Always`.
- [ ] **Step 2: Run.** Expected: FAIL (no `Settlement`, no `kind` module).
- [ ] **Step 3: Implement** as above. Terrain behaviour and bytes do not change.
- [ ] **Step 4: Run** the crate's tests and the 1.0.0 golden test
  (`cargo nextest run -p theos-maps -E 'test(/slow_outputs_match/)'`). Expected: PASS.
- [ ] **Step 5: Commit** `refactor(generators): one table per map kind`.

### Task 2: Features: `Geometry::None`, `Prop::Item`, id schemes (§5.2 items 7, 8)

**Files:** Modify `crates/theos-maps/src/features/model.rs`, `features/output.rs` (+ its
`tests.rs`), `features/mod.rs`.

**Interfaces — Produces:**

```rust
pub enum Geometry { Point([f64; 2]), Line(Vec<[f64; 3]>), Polygon(Vec<Vec<[f64; 2]>>), None }
pub enum Prop { /* existing */ Item { list: &'static str, index: u32 } }   // L13
// features/output.rs
pub(crate) struct Assembly { pub features: Vec<Feature>, pub lists: BTreeMap<&'static str, Vec<FeatureHandle>> }
pub(crate) fn assemble(assembly: &Assembly, ids: IdScheme)
    -> Result<(FeatureCollection, BTreeMap<String, usize>), FeaturesError>;
pub(crate) fn id_of(scheme: IdScheme, kind: &str, ty: &str, counts: &mut BTreeMap<String, usize>) -> String;
// FeaturesError gains: UnknownItem { list: String, index: u32, by: String }
```

- `Geometry::None` writes `"geometry": null` (`geojson::Feature.geometry = None`).
- `assemble` numbers ids per scheme in feature order (`ByType`: `<type>-<n>` counted per type;
  `ByKind`: `<kind>-<n>` counted per kind), resolves `Prop::Ref` by handle and `Prop::Item` by
  `lists[list][index]`, and returns the counts per **type** in both schemes (they feed
  `features.<type>`).
- The terrain builder keeps its order and output; it may move onto `assemble` only if its bytes
  stay the same (the 1.0.0 golden test proves it).

- [ ] **Step 1: Write the failing tests** (in `features/output/tests.rs`):
  - `a_feature_without_geometry_is_written_as_null`: the JSON of a `connection`/`open` feature
    with `Geometry::None` has `"geometry": null` and keeps `id`, `kind`, `type`.
  - `ids_by_type_number_each_type_from_one`: features of types `street`, `lane`, `street` get
    `street-1`, `lane-1`, `street-2`.
  - `ids_by_kind_number_each_kind_from_one`: kinds `room`, `door`, `room` with types `hall`,
    `open`, `cell` get `room-1`, `door-1`, `room-2`; the counts are `{hall: 1, cell: 1, open: 1}`.
  - `an_item_reference_becomes_the_items_id`: `Prop::Item { list: "parcels", index: 1 }` →
    the id of the second parcel; index 5 of a 2-item list fails with `UnknownItem` naming the
    list and the referring feature.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the crate's tests and the 1.0.0 golden test. Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): features without geometry, item references and id schemes`.

### Task 3: Recipe inputs and the optional frame keys (§5.2 item 2)

**Files:** Create `crates/theos-maps/src/recipe/inputs.rs` (+ `inputs/tests.rs`). Modify
`recipe/schema.rs`, `recipe/plan.rs` (`check_inputs`, `check_frame`), `recipe/mod.rs`,
`api/describe.rs`.

**Interfaces — Produces:**

```rust
// recipe/inputs.rs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputDecl {
    Feature { parent_types: Vec<String> },
    Number { default: Option<f64>, min: Option<f64>, max: Option<f64>, #[serde(default)] integer: bool },
    Enum { values: Vec<String>, default: Option<String> },
}
pub struct Inputs(pub BTreeMap<String, InputDecl>);
impl Inputs {
    /// The one input of type `feature`, `None` when there is none.
    pub fn feature(&self) -> Option<(&str, &[String])>;
    /// Checks request values: declared names only, the declared type, range, integer, enum value.
    pub fn check(&self, given: &Map<String, Value>) -> Result<(), InputProblem>;
    /// The declarations as `describe` reports them (`{"feature": {"type": "feature", "parent_types": […]}}`).
    pub fn describe(&self) -> Map<String, Value>;
}
pub struct InputProblem { pub input: String, pub message: String }
// schema.rs: Recipe.inputs: BTreeMap<String, InputDecl>; Recipe.grid: Option<[usize; 2]>;
//            Recipe.size_km: Option<[f64; 2]>; Recipe.child: Option<toml::Table>  (L7, L8)
```

Loading refuses: two `feature` inputs; a `feature` input with empty `parent_types`; `min > max`;
an `enum` without values or with a default not in them; type `cameras` (L6). `check_frame` takes
the kind: terrain and interior need `grid` and `size_km`; settlement refuses both, naming the key
(L7) — its frame comes in Task 4. `describe` fills `RecipeInfo.inputs` from `Inputs::describe`.

- [ ] **Step 1: Write the failing tests:**
  - `// AC: CE-28.3` `an_undeclared_input_is_refused_before_the_run`: a recipe declaring no
    inputs, request `{"floor": 2}` → `RecipeError::Invalid` with `parameter == "floor"` and
    message `unknown recipe input `floor``.
  - `// AC: CE-28.3` `an_input_of_the_wrong_type_is_refused_before_the_run`: declared
    `floor = { type = "number", min = 1, max = 10, integer = true }`; values `"two"`, `11`, `2.5`
    each refused naming `floor`; `3` accepted.
  - `a_feature_input_is_reported_by_describe`: a fixture recipe with
    `feature = { type = "feature", parent_types = ["town", "city"] }` gives
    `{"feature": {"type": "feature", "parent_types": ["town", "city"]}}`.
  - `two_feature_inputs_are_refused`, `cameras_inputs_are_refused_as_unsupported`.
  - `a_settlement_recipe_with_grid_is_refused`, `a_terrain_recipe_without_grid_is_refused`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.** The feature input's value must be a string; whether the parent has it
  is Task 4's check.
- [ ] **Step 4: Run.** Expected: PASS; built-in terrain recipes unchanged.
- [ ] **Step 5: Commit** `feat(generators): declared recipe inputs` (`Refs: CE-28.3`).

### Task 4: Parent package, feature input and child frame (§5.2 items 3, 4)

**Files:** Create `crates/theos-maps/src/child/{mod.rs, parent.rs, frame.rs, tests.rs}`. Modify
`kind/*.rs` (frame rules), `lib.rs`, `error.rs`. Create
`crates/theos-maps/src/stages/settlement/class.rs` (the class table, used here by the frame rule;
`stages/settlement/mod.rs` gets only `pub mod class;` now).

**Interfaces — Produces:**

```rust
// child/parent.rs
pub struct ParentFeature {
    pub id: String, pub kind: String, pub ty: String,
    pub geometry: Geometry,                         // in the parent's frame, metres
    pub properties: serde_json::Map<String, Value>,
}
pub struct ParentContext {
    pub path: PathBuf,
    pub package: theos_map_format::Package,
    pub sha256: String,                             // SHA-256 of the file's bytes
    pub feature: ParentFeature,
    pub input: String,                              // the recipe's feature input name
}
impl ParentContext {
    /// Reads `from` with `theos_map_format::read`, hashes its bytes, finds the feature the
    /// request's input names and checks its type against `parent_types`.
    pub fn open(from: &Path, input: &str, feature_id: &str, parent_types: &[String]) -> Result<Self, ChildError>;
    /// The feature's anchor: a Point, or a Polygon's bounding-box centre.
    pub fn anchor_m(&self) -> Result<[f64; 2], ChildError>;
}
#[derive(Debug, thiserror::Error)]
pub enum ChildError {
    Read { path: PathBuf, source: theos_map_format::FormatError },   // hashes, requires, major
    Io { path: PathBuf, source: std::io::Error },
    UnknownFeature { id: String, path: PathBuf },
    WrongType { id: String, ty: String, accepted: Vec<String> },
    NoAnchor { id: String },                                           // a Line or no geometry
    NotAFeatureId { input: String },                                   // value is not a string
}

// child/frame.rs
pub struct ChildFrame { pub origin_m: [f64; 2], pub rows: usize, pub cols: usize, pub cell_m: f64 }
pub(crate) enum FrameRule { Recipe, SettlementClass, RecipeChild }
//   Recipe: terrain, from `grid`/`size_km`, no origin.
//   SettlementClass: from the class table (L8, L9).
//   RecipeChild: interior, size from `grid`/`size_km`, centred on the feature (P34).
pub fn child_frame(kind: MapKind, recipe: &Recipe, parent: &ParentContext) -> Result<ChildFrame, RecipeError>;
pub fn standalone_frame(kind: MapKind, recipe: &Recipe) -> Result<MapFrame, RecipeError>;

// stages/settlement/class.rs
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ClassFrame { pub window_m: f64, pub cell_m: f64, pub r_built_m: f64 }
pub const CLASSES: [(&str, ClassFrame); 4];      // hamlet 600/2/70, village 1000/3/170,
                                                  // town 1600/4/330, city 2400/5/560 (§5.5)
pub fn resolve(class: &str, child: Option<&toml::Table>) -> Result<ClassFrame, String>;
// `[child.class.<class>]` overrides any of the three fields; unknown keys refused.
```

Rules (L9): cells per side `round_half_even(window_m / cell_m)`; size `cells · cell_m`;
`origin_m = round_half_even((anchor − size/2) / cell_m) · cell_m` per axis; the frame passes
P08's `MIN_CELL_M`/`MAX_CELL_M` and `rows · cols` checks. The window may reach past the parent's
edge (origin negative or beyond).

- [ ] **Step 1: Write the failing tests** (`child/tests.rs`; parents are written in the test with
  `theos_map_format::write` from a small synthetic `Package` with point and polygon features):
  - `a_town_window_is_400_cells_of_4_m_centred_on_the_feature`: town at (2000, 1500) in a
    4,000 m parent → rows = cols = 400, cell 4.0, origin (1200.0, 700.0).
  - `a_village_window_has_333_cells_of_3_m`: size 999 m.
  - `a_window_past_the_parent_corner_keeps_its_size_and_a_negative_origin`: town at (100, 100)
    → origin (−700.0, −700.0), 400 × 400.
  - `the_origin_snaps_to_whole_child_cells`: anchor (2001.3, 1499.0), town → origin
    `(1200.0, 700.0)` (1201.3/4 → 300; 699/4 = 174.75 → 175 → 700.0).
  - `recipe_class_tables_override_the_defaults`: `[child.class.town] cell_m = 5.0` → 320 cells.
  - `a_feature_id_the_parent_lacks_is_refused`: `UnknownFeature` naming `town-9` and the file.
  - `a_ruin_is_refused_by_a_settlement_recipe`: `WrongType` naming `ruin-1`, `ruin` and
    `["city", "town", "village", "hamlet"]`.
  - `a_polygon_feature_is_centred_on_its_bounding_box`: a square ring (100,100)–(300,500) →
    anchor (200, 300).
  - `a_parent_with_a_broken_hash_is_refused_naming_the_file`: flip one byte of a layer entry
    before reading → `ChildError::Read` with the path in its message.
  - `the_package_hash_is_of_the_file_bytes`: `sha256` equals `sha256_hex(&std::fs::read(path))`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.** `RunError` gains `Child(#[from] ChildError)`.
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): parent packages and child frames`.

### Task 5: Kernel `resample` (§5.2 item 5)

**Files:** Create `crates/theos-maps/src/kernels/resample.rs`; modify `kernels/mod.rs`.

**Interfaces — Produces:**

```rust
pub enum Sampling { CatmullRom, Nearest }                       // L11
/// `parent` (rows × cols of `parent_cell_m`) sampled at the centres of a child grid of
/// `rows × cols` cells of `cell_m` whose north-west corner lies at `origin_m` in the parent's frame.
pub fn resample(parent: &Grid<f64>, parent_cell_m: f64, origin_m: [f64; 2],
                rows: usize, cols: usize, cell_m: f64, sampling: Sampling) -> Grid<f64>;
```

The sample point of child cell (r, c) in parent cell units is
`u = (origin_x + (c + 0.5)·cell_m) / parent_cell_m − 0.5`, likewise `v` for y. Catmull-Rom uses
the 4 × 4 neighbourhood with the same basis as `kernels::curves::spline`; every index is clamped
to the parent grid (spec: "cells past the parent's edge read the nearest edge cell"). `Nearest`
takes the cell containing the point, clamped. Exact operations only (`floor` is allowed).

- [ ] **Step 1: Write the failing tests:**
  - `resample_reproduces_a_linear_ramp_inside_the_grid`: parent `h = 3x + 2y` (metres) → child
    values equal `3x + 2y` at child centres within 1e-9, for cells whose 4 × 4 stencil lies
    inside.
  - `resample_hits_parent_values_at_parent_centres`: child cells on parent centres equal them
    exactly (`to_bits`).
  - `resample_clamps_samples_past_the_parent_edge`: origin (−100, −100) → the first child cells
    equal parent cell (0, 0) for `Nearest` and are finite for `CatmullRom`.
  - `nearest_keeps_class_codes`: a parent of codes {0, 3} gives only {0.0, 3.0}.
- [ ] **Step 2: Run.** Expected: FAIL. **Step 3: Implement.** **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): resample a parent layer into a child frame`.

### Task 6: Parent values in the world, planning modes (§5.2 items 5, 6)

**Files:** Modify `recipe/plan.rs` (+ `plan/tests.rs`), `runner/mod.rs`, `runner/memory.rs`,
`kind/*.rs` (`seeded`), `child/mod.rs`. Create `crates/theos-maps/src/child/values.rs`.

**Interfaces — Produces:**

```rust
// recipe/plan.rs
pub enum Planning<'a> { Standalone, Child(&'a ParentContext), Describe }
pub fn plan(recipe: &LoadedRecipe, registry: &Registry, inputs: &Map<String, Value>,
            planning: Planning<'_>) -> Result<Plan, RecipeError>;
// Plan gains: pub origin_m: Option<[f64; 2]>, pub seeded: Vec<LayerSpec>,
//             pub parent: Option<ParentRef>  (ParentRef { package_sha256: String, feature: String })

// child/values.rs — world values the runner seeds
pub struct ParentFeatureValue {            // NAME "parent_feature", world name "parent.feature"
    pub id: String, pub kind: String, pub ty: String,
    pub geometry: Geometry,                // in the child's frame
    pub properties: Map<String, Value>,
    pub parent_size_m: [f64; 2], pub parent_cell_m: f64, pub parent_grid: [u32; 2],
    pub package_sha256: String,
}
pub struct ParentFeatures { pub features: Vec<ParentFeatureValue> } // NAME "parent_features", "parent.features"
// settlement only (L10):
pub struct ClassFrameValue { pub class: String, pub frame: ClassFrame } // NAME "class_frame"
pub(crate) type SeededFn = fn(Option<&ParentContext>, &Plan) -> Vec<(String, Layer)>;
```

- Planning (`Planning::Child`) offers, before the first stage, `parent.<layer>` for every layer of
  the parent package (F64, resampled with `Sampling::Nearest` when its entry has a `legend`,
  else `CatmullRom`), `parent.features` and `parent.feature`, plus the kind's own seeded values
  (`class_frame` for settlements, both modes). Only those that some planned stage `reads` go
  into `Plan.seeded`; a read of a `parent.*` name the parent does not offer is refused like a
  missing layer ("stage `x` reads `parent.discharge`, which the parent does not have").
- `Standalone` leaves out `Runs::Child` stages, `Child` leaves out `Runs::Standalone` stages
  (after parsing their tables); `Describe` keeps both and treats every `parent.*` read as
  available, with the frame of the first `parent_types` entry for settlements.
- The memory estimate counts seeded values like any read.
- The runner inserts `Plan.seeded` into the world before the first stage (values computed by the
  kind's `seeded` fn) and records their hashes as the golden pseudo stage `parent#0` (L12).
- `parent.features` holds the parent's features whose geometry touches the window (bounding box
  overlap), translated by `−origin_m`, in the parent file's order.

- [ ] **Step 1: Write the failing tests** (`recipe/plan/tests.rs` and `runner/tests.rs`, with test
  stages reading `parent.height`):
  - `// AC: CE-28.3` `a_stage_reading_a_parent_value_the_parent_lacks_is_refused`: names the
    stage and `parent.discharge`.
  - `only_parent_values_a_stage_reads_are_seeded`: a plan reading `parent.height` seeds that one
    and not `parent.water`.
  - `child_only_stages_are_left_out_of_standalone_plans` and the converse.
  - `switching_a_stage_off_keeps_the_other_streams`: the RNG first draws of stage `b#0` are equal
    with and without a `Runs::Child` stage `a` before it.
  - `seeded_values_are_hashed_as_the_parent_pseudo_stage`: the golden record's first stage key is
    `parent#0` and lists `parent.height`.
  - `the_memory_estimate_counts_seeded_values`.
- [ ] **Step 2: Run.** Expected: FAIL. **Step 3: Implement.** **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): parent values in the child's world` (`Refs: CE-28.3`).

### Task 7: Runs from `from`: package and manifest (§5.2 items 3, 6, 7)

**Files:** Modify `api/run.rs` (+ `run/tests.rs`), `api.rs` (`ManifestFrame`), `api/describe.rs`,
`package/mod.rs`. (Format 1.1, item 11, is switched on in Task 33 with the new `VERSION`, so the
1.0.0 golden hashes stay valid until then.)

**Interfaces — Produces:**

```rust
pub struct ManifestFrame {
    pub size_m: [f64; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")] pub origin_m: Option<[f64; 2]>,
}
```

`run_inner` order: find the recipe; check inputs; decide the mode — `from` given and the recipe
has a feature input → `ParentContext::open` → `Planning::Child`; `from` given without a feature
input → refused ("recipe `x` starts from no other map, so `from` must be empty"); no `from` and a
feature input → `Planning::Standalone` if the plan has a `Runs::Standalone` stage and the feature
input is absent from the request, else refused ("recipe `x` is a child recipe and needs `from`")
(L5). The package writes `kind`, `frame.origin_m`, `frame.parent {package_sha256, feature}`; the
manifest frame adds `origin_m`. Terrain packages keep their bytes.

- [ ] **Step 1: Write the failing tests** (a synthetic child kind from test stages that copy
  `parent.height` to `height`; the parent is a fixture terrain run of `api-small` written in the
  test):
  - `a_child_run_writes_origin_and_parent_into_the_package`: `frame.origin_m` equals the child
    frame's, `frame.parent.package_sha256` the parent file's SHA-256, `feature` the input.
  - `a_child_recipe_without_from_is_refused_before_the_run` (no standalone stage).
  - `a_child_recipe_with_a_standalone_stage_runs_without_from` (L5), and the same recipe
    without `from` but with the feature input given is refused naming the input.
  - `a_recipe_without_a_feature_input_refuses_from`: the existing terrain refusal keeps its
    message.
- [ ] **Step 2: Run.** Expected: FAIL. **Step 3: Implement.** **Step 4: Run** the crate's tests
  and the 1.0.0 golden test. Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): child runs from a parent package`.

### Task 8: Kernel `geometry::lattice` (§5.2 item 10, §5.4)

**Files:** Create `crates/theos-maps/src/kernels/geometry/{mod.rs, lattice.rs, predicates.rs}`.

**Interfaces — Produces:**

```rust
pub const UNITS_PER_M: i32 = 16;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Point { pub y: i32, pub x: i32 }      // field order = lattice order (y, then x)
pub type Ring = Vec<Point>;                      // closed implicitly; first != last
pub struct Polygon { pub outer: Ring, pub holes: Vec<Ring> }   // outer area > 0, holes < 0 (L17)
pub fn snap(m: f64) -> Result<i32, LatticeError>;              // round half away from zero
pub fn to_m(n: i32) -> f64;                                    // n / 16, exact
pub fn point_m(p: Point) -> [f64; 2];
pub fn orient(a: Point, b: Point, c: Point) -> i64;            // sign of the cross product
pub fn twice_area(ring: &[Point]) -> i128;                     // shoelace in (x, y)
pub enum Crossing { None, Proper(PointQ), Touch(PointQ), Overlap }   // PointQ: rational i128 point
pub fn segments(a: Point, b: Point, c: Point, d: Point) -> Crossing;
pub fn contains(polygon: &Polygon, p: Point) -> Containment;   // Inside, Outside, Boundary
pub enum LatticeError { OutOfRange { metres: f64 } }           // |m·16| beyond i32, or not finite
```

- [ ] **Step 1: Write the failing tests:**
  - `snap_rounds_half_away_from_zero`: `snap(0.03125) == 1`, `snap(-0.03125) == -1`,
    `snap(0.09375) == 2`, `snap(2400.0) == 38_400`.
  - `a_134_km_window_fits_and_a_larger_one_is_refused`: `snap(134_000.0)` is `Ok(2_144_000)`;
    `snap(134_218_000.0)` is `OutOfRange`; `snap(f64::NAN)` is `OutOfRange`.
  - `to_m_is_exact`: `to_m(1) == 0.0625`, `to_m(38_400) == 2400.0` (`to_bits`).
  - `orientation_and_crossings_use_integers`: hand cases for proper, touching, collinear
    overlapping and parallel segments, including coordinates near `i32::MAX`.
  - `area_sign_follows_the_winding`: a square wound (0,0)→(16,0)→(16,16)→(0,16) has positive
    `twice_area` in (x east, y south).
- [ ] **Step 2: Run.** Expected: FAIL. **Step 3: Implement** (the casts with `#[expect]` live in
  `lattice.rs`). **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): the integer lattice for map geometry`.

### Task 9: Golden runs with a parent (§5.2 item 9)

**Files:** Modify `crates/theos-maps/src/golden.rs`, `golden/file.rs`, `examples/bless.rs`,
`tests/maps/golden.rs`, `tests/maps/support.rs`.

**Interfaces — Produces:**

```rust
pub struct GoldenSpec { pub recipe: &'static str, pub seed: u64,
                        pub inputs: &'static [(&'static str, &'static str)],
                        pub from: Option<(&'static str, u64)> }          // the parent's golden run
pub const GOLDEN_RUNS: &[GoldenSpec];                                    // the three terrain runs
// GoldenRun gains (serde default, skipped when empty/None, so 1.0.0.toml still parses):
//   pub inputs: BTreeMap<String, String>, pub from: Option<GoldenParent>
pub struct GoldenParent { pub recipe: String, pub seed: u64 }
pub fn compute(spec: &GoldenSpec, dir: &Path) -> Result<GoldenRun, RunError>;
//   with `from`, first runs the parent's recipe and seed into `dir/parent/`, then the child with
//   `from = dir/parent/s<seed>.theosmap`; no package fixture is kept.
```

- [ ] **Step 1: Write the failing tests:**
  - `a_golden_spec_with_a_parent_runs_the_parent_first` (unit, with a test child recipe from
    Task 7's fixtures): `compute` returns a run whose `from` names the parent and whose first
    stage is `parent#0`.
  - `// AC: CE-77.2` `bless_refuses_to_overwrite_an_existing_version`: keep passing with the new
    fields.
  - `golden_runs_round_trip_with_inputs_and_parent` (TOML).
- [ ] **Step 2: Run.** Expected: FAIL. **Step 3: Implement**; the golden test compares the recorded
  list with `GOLDEN_RUNS` by `(recipe, seed, inputs, from)`. **Step 4: Run.** Expected: PASS
  (the 1.0.0 package-hash failure of Task 7 stays until Task 33).
- [ ] **Step 5: Commit** `feat(generators): golden runs with a parent` (`Refs: CE-77.2`).

---

## Part B — geometry (§5.4)

### Task 10: `i_overlay` and its wrapper

**Files:** Modify root `Cargo.toml` (`i_overlay = { version = "9.2.0", default-features = false }`),
`crates/theos-maps/Cargo.toml`, `Cargo.lock`. Create
`crates/theos-maps/src/kernels/geometry/overlay.rs` (+ `overlay/tests.rs`).

**Interfaces — Produces** (the only module that names `i_overlay` types):

```rust
pub fn union(a: &[Polygon], b: &[Polygon]) -> Vec<Polygon>;
pub fn difference(a: &[Polygon], b: &[Polygon]) -> Vec<Polygon>;
pub fn intersection(a: &[Polygon], b: &[Polygon]) -> Vec<Polygon>;
pub fn offset(polygons: &[Polygon], by: i32) -> Vec<Polygon>;      // > 0 outwards, < 0 inset
pub fn slice(polygons: &[Polygon], paths: &[Vec<Point>]) -> Vec<Polygon>;  // faces
```

One place fixes the solver: `Solver::with_precision(Precision::HIGH)` (no multithreading; the
probe found every strategy gives the same bytes), passed through
`Overlay::with_shapes_options(subj, clip, options, solver)`; `FillRule::NonZero`; offsets through `IntOutlineOffset::outline` with
`IntOutlineStyle::new(by).math(MathMode::Integer)` and a miter join; slices through
`IntSlice::slice_by_paths`. Results are normalised: rings rotated to start at their smallest
point, outer positive and holes negative (L17), polygons sorted by their outer ring's first
point.

- [ ] **Step 1: Write the failing tests:**
  - `union_of_two_overlapping_squares_is_one_polygon` with the exact outline.
  - `an_inset_square_shrinks_by_the_offset`: 32 × 32 inset by 4 → 24 × 24.
  - `slicing_a_square_by_a_cross_gives_four_faces`.
  - `the_probe_polygons_hash_the_same`: 400 polygons from a fixed `Pcg64` seed through union,
    difference, intersection, inset, outset and slice; the SHA-256 of their canonical JSON is
    pasted into the test after the first run (risk table: "the probe's polygons become a unit
    test with a fixed hash"); the same value in debug and release.
- [ ] **Step 2: Run.** Expected: FAIL. **Step 3: Add the dependency** (one unlocked
  `cargo build -p theos-maps`, then `cargo deny --locked check`; record
  `cargo tree -e features -i i_overlay` in the ledger: no `allow_multithreading`). **Step 4:
  Implement.** **Step 5: Run** in debug and with `--release`. Expected: PASS, same hash.
- [ ] **Step 6: Commit** `feat(generators): polygon booleans, offsets and slices on the lattice`.

### Task 11: Hull, minimum-area box, bisector cut

**Files:** Create `crates/theos-maps/src/kernels/geometry/{hull.rs, min_box.rs, bisector.rs}`.

**Interfaces — Produces:**

```rust
pub fn convex_hull(points: &[Point]) -> Ring;                         // monotone chain, no collinear
pub struct OrientedBox { pub corners: [PointQ; 4], pub long_axis: (i64, i64), pub twice_area: i128 }
pub fn min_area_box(ring: &[Point]) -> OrientedBox;                   // rotating calipers, i128 compare
/// The cut line at the convex corner `ring[i]` along its inner bisector, as a lattice segment
/// reaching `reach` units into the polygon; `None` for a reflex corner or one within
/// `straight_tolerance` of 180°.
pub fn bisector_cut(ring: &[Point], i: usize, reach: i32) -> Option<(Point, Point)>;
```

The bisector direction is the sum of the two unit edge directions; normalising uses
`libm::sqrt` on the integer lengths and the end point is snapped once (the only float use); a
corner is straight when `|cross| · 1000 ≤ |a|·|b|` (about 0.06°), compared in i128.

- [ ] **Step 1: Write the failing tests:** `hull_of_a_square_with_inner_points`,
  `hull_drops_collinear_points`; `min_box_of_an_axis_rectangle_is_itself`,
  `min_box_of_a_rotated_rectangle_matches_its_corners` (a 3-4-5 rotated rectangle, exact
  corners); `bisector_of_a_right_corner_runs_at_45_degrees`,
  `a_straight_corner_gets_no_cut`, `a_reflex_corner_gets_no_cut`.
- [ ] **Step 2: Run.** Expected: FAIL. **Step 3: Implement.** **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): hull, minimum-area box and bisector cuts`.

---

## Part C — settlement stages to the main streets

All stages live under `crates/theos-maps/src/stages/settlement/`, list `KINDS = &[MapKind::Settlement]`
and are registered in `stages::register` in pipeline order. Each `Params` struct is
`#[serde(default)]` with the defaults named below; `validate` checks every value with
`stage::Check`.

### Task 12: Settlement values and the class table

**Files:** Create `stages/settlement/{mod.rs, values.rs, values/tests.rs}`; modify
`stages/mod.rs`. (`class.rs` exists from Task 4.)

**Interfaces — Produces** (each a `WorldValue` + `ValueType`, canonical JSON through
`to_checked_value`, lattice geometry):

```rust
pub enum SettlementClass { Hamlet, Village, Town, City }              // serde lower case
pub struct Site { pub at: Point, pub class: SettlementClass, pub name: String,
                  pub population: Option<i64>, pub r_built_m: f64, pub feature: Option<String> } // "site"
pub struct Entry { pub at: Point, pub class: String, pub road: Option<String>, pub inward: [i32; 2] }
pub struct Entries { pub entries: Vec<Entry> }                                                  // "entries"
pub struct RiverLine { pub line: Vec<Point>, pub width_m: Vec<f64>, pub discharge_m3s: Vec<f64> }
pub struct Rivers { pub rivers: Vec<RiverLine> }                                                // "rivers" (L15)
pub struct Centre { pub at: Point, pub square: Polygon }                                        // "centre"
pub struct Zone { pub name: String, pub inner_m: f64, pub outer_m: Option<f64> }                // None: the frame
pub struct Zones { pub zones: Vec<Zone> }                                                       // "zones"
pub enum ZoneSpec { Metres(f64), Share(f64) }    // a radius in m or a share of r_built (§5.6)
```

- [ ] **Step 1: Write the failing tests:** `values_hash_as_canonical_json` (a `Site` with a
  negative zero `r_built_m` hashes as `+0.0`); `zones_name_metres_and_shares`.
- [ ] **Step 2–4:** FAIL → implement → PASS.
- [ ] **Step 5: Commit** `feat(generators): settlement world values`.

### Task 13: The 0.3.0 reference (CE-78.3 fixture, §5.17)

This task runs Python only; it may run at any time before Task 32.

**Files:** Modify `research/P29-settlements/settle_baseline.py` and `README.md`. Create
`crates/theos-maps/tests/fixtures/parity/settlement-0.3.0.toml`.

- [ ] **Step 1: Change the driver** to the definitions of §5.13, computed on 0.3.0's own grid:
  - a footprint covers the cells whose centres lie inside its polygon (`skimage.draw.polygon`
    as today);
  - `street_access_pct`: share of buildings with a street cell (`street_class > 0`) within
    `access_m` = 8 m of a footprint cell (EDT of `street_class == 0` × cell), × 100;
  - `buildings_on_water`: a footprint cell on `river | lake`; `buildings_on_street`: on
    `street_class > 0`; `building_overlap_cells`: cells covered more than once;
  - `buildings_steep`: footprint mean slope above `max_slope` = 0.3 (slope as in the driver);
  - `street_components`: 8-connected parts of `street_class > 0`;
  - `entries_connected`: share of road entries on the window edge joined to the centre through
    the street mask;
  - `landmarks_missing_required`: of `market`, `shrine`, `inn`, `smithy`, `guardhouse`,
    `granary` (spec ruling 18), those 0.3.0 did not place;
  - `buildings`: the count.
  - Per seed: `passes` = every value within §5.17's ranges.
- [ ] **Step 2: Make the parent.** Run 0.3.0's `fixture-small` seed 1 with its own runner
  (`python -m theos_maps.run recipes/fixture-small.yaml <out> --seeds 1`, see `run.py`), with
  `PYTHONPATH` at a copy of `D:\Projects\theos_maps-export`, `MPLBACKEND=Agg`, through
  `uv run --no-project --python 3.12 --with fastscapelib==0.3.0 --with numpy==2.5.4
  --with scipy==1.18.1 --with scikit-image --with matplotlib --with networkx --with PyYAML
  --with pillow`. Record the resolved versions (`uv pip freeze` in that environment).
- [ ] **Step 3: Run the driver**:
  `python settle_baseline.py <parent engine folder> town-1 <scratch> --seeds 1 2 3 4 5 6 7 8`.
- [ ] **Step 4: Write the fixture** from its results JSON:

  ```toml
  # 0.3.0 reference for CE-78.3 (spec §5.17), recorded by research/P29-settlements/settle_baseline.py
  # at <git hash of the driver>; Python 3.12, fastscapelib 0.3.0, numpy 2.5.4, scipy 1.18.1,
  # scikit-image …, … (all resolved versions); parent fixture-small seed 1, feature town-1,
  # child seeds 1–8 set directly.
  [ranges]
  buildings = [100, 5000]
  street_access_pct = [100, 100]
  street_components = [1, 1]
  entries_connected = [1, 1]
  buildings_on_water = [0, 0]
  buildings_on_street = [0, 0]
  building_overlap_cells = [0, 0]
  buildings_steep = [0, 0]
  landmarks_missing_required = [0, 0]

  [[seed]]
  seed = 1
  buildings = …
  # … every metric of [ranges]
  passes = …
  ```

  The spec's estimate (0.3.0 passes 4 of 8) is not the bar; the recorded file is. If the count
  is 0, stop and tell Jann (CE-78.3 would be empty; `CLAUDE.md` "When to stop").
- [ ] **Step 5: Commit** `test(generators): the 0.3.0 settlement reference` (`Refs: CE-78.3`).

### Task 14: `street_graph`: noding and the `streets` raster (§5.6, §5.7 "Noding")

**Files:** Create `stages/settlement/graph/{mod.rs, noding.rs, raster.rs, tests.rs}`.

**Interfaces — Produces:**

```rust
pub enum StreetClass { Lane = 1, Street = 2, MainStreet = 3 }   // code in the `streets` raster
pub struct Node { pub id: u32, pub at: Point }
pub struct Edge { pub a: u32, pub b: u32, pub line: Vec<Point>, pub class: StreetClass,
                  pub width_m: f64, pub bridge: bool, pub stage: String, pub entry: Option<String> }
pub struct StreetGraph { pub nodes: Vec<Node>, pub edges: Vec<Edge> }          // "street_graph"
impl StreetGraph {
    /// Adds the polyline as edges, noded against the graph by snap rounding (Hobby 1999):
    /// every segment passing through a lattice hot pixel (a crossing or an end point) is routed
    /// through its centre; collinear overlaps merge, keeping the higher class.
    pub fn add_line(&mut self, line: &[Point], class: StreetClass, width_m: f64,
                    bridge: bool, stage: &str, entry: Option<&str>) -> Vec<u32>;  // new edge ids
    pub fn node_at(&self, p: Point) -> Option<u32>;
    pub fn nearest(&self, p: Point) -> Option<(u32, Point)>;   // nearest point on any edge
    pub fn length_m(&self, class: StreetClass) -> f64;
    /// Ids renumbered: nodes by (y, x), edges by (a, b, first vertex) after every change.
    fn canonicalise(&mut self);
}
/// Marks each cell a street covers (cells within width/2 of the centreline) with its class,
/// the higher class where streets overlap.
pub fn rasterise(graph: &StreetGraph, frame: &MapFrame) -> Grid<u8>;
```

Hot pixels are lattice cells of `hot_px` units (a constant 8 = 0.5 m); crossings are computed
exactly (`kernels::geometry::segments`) and rounded to their hot pixel; a segment within the hot
pixel's square is routed through its centre. Spec: "rounding never creates a new crossing".

- [ ] **Step 1: Write the failing tests:**
  - `a_crossing_splits_both_edges_at_one_node`.
  - `an_end_on_a_vertex_makes_one_node` (Review Focus 3).
  - `collinear_overlaps_merge_keeping_the_higher_class` (Review Focus 3): a `lane` laid on part
    of a `main-street` leaves one `main-street` edge there.
  - `snap_rounding_creates_no_new_crossing`: 200 random segments from a fixed seed, noded; no two
    edges cross except at shared nodes (checked with `segments`).
  - `node_and_edge_order_is_canonical`: adding the same lines in two orders gives equal graphs
    (Review Focus 7, "ordering of nodes and edges after noding").
  - `the_raster_marks_width_and_keeps_the_higher_class`.
- [ ] **Step 2–4:** FAIL → implement → PASS.
- [ ] **Step 5: Commit** `feat(generators): a noded street graph on the lattice`.

### Task 15: `local_terrain` (§5.5 child)

**Files:** Create `stages/settlement/local_terrain/{mod.rs, rivers.rs, lakes.rs, entries.rs, tests.rs}`.

**Reads → writes:** `parent.height`, `parent.water`, `parent.features`, `parent.feature`,
`class_frame` → `height` (F64), `water` (U8: 0 dry, 1 stream, 2 river, 3 lake), `entries`,
`site`, `rivers`. `RUNS = Runs::Child`.

**Params:** `relief_m` 1.2 (0.3.0's), `relief_length_m` (correlation length), `beta`,
`lake_smooth_m`.

- `height`: `parent.height` plus `relief_m ·` P08's `spectral_noise` (band from
  `relief_length_m`).
- `water`: rivers from the `parent.features` of kind `water`, type `river`, clipped to the window,
  drawn at their per-vertex `width_m`; a vertex is class 2 (river) when its `discharge_m3s`
  exceeds the parent recipe's `big_river_q` (the `hydrology` table of the parent manifest's
  `recipe`, carried in `parent.feature`), else class 1; without that value every river is
  class 2 (gap 6); lakes:
  `parent.water == 3` → marching squares (`kernels::contour::rings`) → Chaikin smoothing over
  `lake_smooth_m` with no randomness → filled (spec ruling 6).
- `entries`: every parent `road` LineString crossing the window edge gives one entry (crossing
  point snapped, the road's type as `class`, its id as `road`, `inward` the direction into the
  window); a road ending inside the window is followed to the edge the other way. A road entering
  and leaving twice gives two entries. No entry: fail "no road enters the window of
  `<feature id>`".
- `site`: `parent.feature`'s anchor in the child frame, class from its type, `name` and
  `population` from its properties (spec ruling 19), `r_built_m` from `class_frame`.

- [ ] **Step 1: Write the failing tests** (64² synthetic parents built in the test):
  - `height_is_the_resampled_parent_plus_relief` (relief 0 → equals `parent.height`).
  - `rivers_keep_their_parent_width`: a river of `width_m` 12 at 3 m cells marks a 4-cell band.
  - `lake_shores_do_not_change_with_the_seed`: seeds 1 and 2 give the same `water`.
  - `a_road_crossing_twice_gives_two_entries` (Review Focus 1).
  - `a_parent_without_roads_fails_naming_the_feature` (Review Focus 1).
  - `site_takes_name_and_population_from_the_parent_feature`.
- [ ] **Step 2–4:** FAIL → implement, register → PASS.
- [ ] **Step 5: Commit** `feat(generators): local terrain of a settlement from its parent`.

### Task 16: `site_terrain` (§5.5 standalone)

**Files:** Create `stages/settlement/site_terrain.rs` (+ tests).

**Reads → writes:** `class_frame` → `height`, `water`, `entries`, `site`, `rivers`.
`RUNS = Runs::Standalone`.

**Params:** `class` (required, one of the four; the frame rule reads it too, L8), `name`
(default "generated, not canon" name from `features::names`), `population` (optional),
`tilt` `[dx, dy]` m per m, `relief_m`, `relief_length_m`, `river` (optional:
`path` as `[[fx, fy], …]` frame fractions, `width_m`, `discharge_m3s`), `entries`
(`[{ at = [fx, fy], class = "main" }, …]`, each on the frame edge, at least one).

- [ ] **Step 1: Write the failing tests:** `an_entry_off_the_edge_is_refused` (planning error
  naming `entries[0]`), `the_tilted_plane_has_the_given_slope`,
  `the_river_is_drawn_at_its_width`, `site_lies_at_the_frame_centre`.
- [ ] **Step 2–4:** FAIL → implement, register → PASS.
- [ ] **Step 5: Commit** `feat(generators): standalone settlement terrain`.

### Task 17: `site_plan` and `main_streets` (§5.6)

**Files:** Create `stages/settlement/site_plan.rs`, `stages/settlement/main_streets.rs`,
`stages/settlement/route.rs` (+ tests). Modify `stages/terrain/roads/mod.rs` only to widen
visibility if needed (`Router`, `StepCost`, `Terrain`, `RoadsParams` are `pub(crate)` already).

**`site_plan`:** reads `height`, `water`, `entries`, `site` → writes `centre`, `interest`
(F64), `zones`.
- Params: `max_slope` 0.3 (hard limit of flatness), weights `w_flat`, `w_water`, `w_paths`,
  `w_site`, `square_radius_m` (town 14, city 24 per 0.3.0; the recipes set it), `zones`
  (`[{ name = "core", outer = { share = 0.5 } }, …]`).
- `interest`: flatness bell on slope (0 above `max_slope`), water cells −1, closeness to water,
  closeness to the entries' least-cost paths, closeness to the site point.
- `centre`: highest interest within `0.35 · r_built` of the site among cells a least-cost path
  reaches from at least one entry; ties by lattice order. None: fail naming the site.

**`main_streets`:** reads `height`, `water`, `entries`, `centre` → writes `street_graph`,
`streets` (U8). Params: `route` (`RoadsParams`, default P08's, `max_bridge_span_m` from
`bridge_max_m`), `width_m` 8.
- `route.rs`: `pub(crate) fn settlement_terrain(height, water) -> SettlementTerrain` owning the
  grids `Terrain` borrows: `vegetation` all 0, `river` = water 1|2, `big_river` = water 2,
  `lake` = water 3, `flow_dir` all 0. Routes use `Router::route` with `RoadClass::Main`.
- One `main-street` per entry to the centre, in entry order; each later route uses the roads
  laid so far (merge discount). Each line is snapped and added with `entry = road id`. The
  square becomes the centre's polygon (later the `plaza` parcel). An unreachable entry fails
  naming it.

- [ ] **Step 1: Write the failing tests:** `the_centre_lies_within_0_35_r_built`,
  `a_site_on_an_island_fails_naming_the_site` (Review Focus 3, "a centre on a river island"),
  `main_streets_merge_before_the_square`, `two_entries_at_one_cell_share_a_node`,
  `an_entry_on_a_river_bridges_or_fails_naming_it`, `every_entry_reaches_the_centre`.
- [ ] **Step 2–4:** FAIL → implement, register both → PASS.
- [ ] **Step 5: Measure** (early speed check, spec §7): a temporary test plans a 400 × 400
  synthetic `site_terrain` town with four entries and runs `site_plan` and `main_streets` in
  release (`cargo nextest run -p theos-maps --release budget_probe --no-capture`). Record the
  times in the ledger; if the two stages take over 2 s, profile before Task 18. Delete the test.
- [ ] **Step 6: Commit** `feat(generators): site plan and main streets`.

---

## Part D — streets, wall, blocks, plots, buildings

### Task 18: `organic_streets` (§5.7, after Emilien 2012)

**Files:** Create `stages/settlement/organic/{mod.rs, candidates.rs, connect.rs, loops.rs, tests.rs}`.
Modify `kernels/paths.rs`.

**Interfaces — Produces:**

```rust
// kernels/paths.rs (L16)
/// The least-cost path from `source` to the first cell of `is_target` reached, both included;
/// `None` when none is reachable. Same costs, ties and cancel points as `shortest_paths`.
pub fn path_to_nearest(rows: usize, cols: usize, source: usize, dirs: Directions,
    cost: &mut dyn FnMut(usize, usize) -> Option<f64>, extra: &mut dyn FnMut(usize) -> Vec<(usize, f64)>,
    is_target: &dyn Fn(usize) -> bool, cancel: &dyn Fn() -> bool) -> Result<Option<Vec<usize>>, KernelError>;
```

**Reads → writes:** `height`, `water`, `interest`, `zones`, `street_graph`, [`wall`] →
`street_graph`, `streets`.

**Params:** `zone`, `attempts`, `spacing_m`, `street_km`, `loop_angle` (degrees), `loop_reach_m`,
`loop_p` (0.3.0's: hamlet 0, village 0.4, town 0.75, city 0.9; the recipes set it), `snap_m`,
`width_m.street`, `width_m.lane`, `route` (`RoadsParams`).

Steps 1–5 of spec §5.7 in that order. Candidates: `attempts` cells drawn from `ctx.rng()` in a
fixed order, kept with probability `interest` (clamped to [0, 1]) when inside the zone and at
least `spacing_m` from every street (distance transform of the `streets` raster). Each kept point
connects to the nearest street cell by `path_to_nearest` with the reuse discount; class `street`
while the graph's street length is below `street_km`, then `lane`. Loops: from the new end, look
in the cone (`loop_angle`, `loop_reach_m`) for the nearest node not on its own path; connect with
probability `loop_p`. Ends within `snap_m` of a node or street extend to it (Parish & Müller).
A step that would cross the wall except at a gate, or a lake, is forbidden; rivers only as
bridges (the `StepCost` rules).

- [ ] **Step 1: Write the failing tests:** `path_to_nearest_stops_at_the_first_target`,
  `candidates_keep_their_spacing`, `the_class_switches_to_lane_after_street_km`,
  `no_loops_with_loop_p_zero`, `streets_never_cross_the_wall_except_at_gates`,
  `the_attempt_budget_bounds_the_work` (the RNG draws exactly `attempts` cells).
- [ ] **Step 2–4:** FAIL → implement, register → PASS.
- [ ] **Step 5: Measure** organic streets on the 400² probe town of Task 17 in release; record it;
  over 3 s, profile before going on.
- [ ] **Step 6: Commit** `feat(generators): organic street growth`.

### Task 19: Tensor field and streamline tracer (§5.7 planned, after Chen 2008)

**Files:** Create `stages/settlement/planned/{mod.rs, field.rs, trace.rs, tests.rs}`.

**Interfaces — Produces:**

```rust
pub enum Basis { Grid { angle: f64 }, GridAlongMain, Radial { at: [f64; 2] },
                 Height, Boundary }                        // decay: Gaussian of `decay_m`
pub struct TensorField { /* values on grid vertices, (a, b) of the symmetric traceless tensor */ }
impl TensorField {
    pub fn build(bases: &[(Basis, f64 /*weight*/, f64 /*decay_m*/)], height: &Grid<f64>,
                 water: &Grid<u8>, graph: &StreetGraph, frame: &MapFrame) -> Self;
    pub fn at(&self, x: f64, y: f64) -> (f64, f64);       // bilinear, canonical (+0.0)
    pub fn major(&self, x: f64, y: f64, prev: [f64; 2]) -> Option<[f64; 2]>;   // None: degenerate (< eps)
    pub fn minor(&self, x: f64, y: f64, prev: [f64; 2]) -> Option<[f64; 2]>;
}
/// One fixed-step RK4 streamline from `seed` (step `step_m`), the eigenvector's sign kept
/// continuous with `prev`; stops by `stop`. Every vertex is snapped to the lattice as made.
pub fn trace(field: &TensorField, seed: [f64; 2], major: bool, step_m: f64, max_len_m: f64,
             stop: &mut dyn FnMut(Point, Point) -> Stop) -> Vec<Point>;
pub enum Stop { Go, Halt, HaltAndJoin }
```

Eigenvectors from `(a, b)` with `libm::atan2`/`libm::cos`/`libm::sin`; every key through
`num::key`.

- [ ] **Step 1: Write the failing tests:** `a_grid_basis_traces_straight_lines_at_its_angle`,
  `a_radial_basis_traces_circles_around_its_centre` (radius error < 0.5 m over one turn),
  `the_sign_stays_continuous`, `degenerate_points_stop_the_line`,
  `negative_zero_in_the_field_traces_like_positive_zero` (Review Focus 6), and
  `a_trace_hashes_the_same_in_debug_and_release` (fixed hash pasted after the first run).
- [ ] **Step 2–4:** FAIL → implement → PASS (also with `--release`).
- [ ] **Step 5: Commit** `feat(generators): tensor field and streamline tracer`.

### Task 20: `planned_streets` (§5.7)

**Files:** Create `stages/settlement/planned/{seeds.rs, stage.rs}` (+ tests).

**Reads → writes:** `height`, `water`, `zones`, `street_graph`, [`wall`] → `street_graph`,
`streets`.

**Params:** `zone`, `bases` (`[{ type = "grid", align = "main-street" }, …]`), `step_m`,
`eps`, `d_sep_major`, `d_sep_minor`, `lookahead_m`, `max_len_m`, `bridge_max_m`,
`bridge_spacing_m` 120, `degenerate_clear_m` (default `d_sep_major`), `simplify_m` 0.5,
`stub_min_m`, `width_m.street`, `width_m.lane`.

Seeds: a priority queue keyed by (distance to the centre, then −distance from degenerate points
and water), ties by lattice position; the first major seeds lie on the main streets. Major lines
first (class `street`), then minor (class `lane`). Stop rules exactly as §5.7 (zone edge,
degenerate point, lake, river unless the crossing is under `bridge_max_m` and
`bridge_spacing_m` from every bridge, max length, closer than `d_sep` to the same family), then
look ahead `lookahead_m` to join a street. Douglas–Peucker at `simplify_m` on the lattice before
`add_line`; stubs under `stub_min_m` removed after noding. No seed within `degenerate_clear_m`
of a degenerate point; a line entering that disc stops.

- [ ] **Step 1: Write the failing tests:** `bridges_keep_their_spacing`,
  `no_seed_near_a_degenerate_point` (Review Focus 3, "a planned zone with a degenerate point
  inside"), `lines_exactly_d_sep_apart_are_decided_by_lattice_order` (Review Focus 3),
  `the_grid_hangs_off_the_main_streets`, `stubs_are_removed`, `simplified_lines_stay_within_0_5_m`.
- [ ] **Step 2–4:** FAIL → implement, register → PASS.
- [ ] **Step 5: Commit** `feat(generators): planned streets by streamlines`.

### Task 21: `wall` and gates (§5.8)

**Files:** Create `stages/settlement/wall/{mod.rs, ring.rs, gates.rs, checks.rs, tests.rs}`.

**Interfaces — Produces:**

```rust
pub enum GateKind { Gate, WaterGate }
pub struct Gate { pub at: Point, pub kind: GateKind, pub edge: Option<u32> }
pub struct Wall { pub ring: Ring, pub band: Polygon, pub thickness_m: f64, pub height_m: f64,
                  pub gates: Vec<Gate>, pub towers: Vec<Point> }                     // "wall"
```

**Reads → writes:** `height`, `water`, `street_graph`, `centre`, `zones` → `wall`,
`street_graph` (main streets split at their gates).

**Params:** `zone` (a ring zone), `step_cost`, `high_ground_w`, `slope_w`, `river_cost`,
`thickness_m`, `height_m`, `tower_spacing_m`. A ring zone narrower than one cell fails at the
start of the stage, naming the zone (parameters do not know the cell size).

Ring: cost per cell (constant per step, lower on high ground and gentle slope, lake impassable,
river `river_cost`, cheap across a main street when square to it); cut the ring zone with a seam
from the centre to the outer edge; for each seam cell, `shortest_paths` from its neighbour on
the other side with the seam closed; the cheapest closed loop wins (ties by lattice order). No
loop: fail naming the zone. Gates: each main-street crossing (`gate`, `edge` set), each river
crossing (`water-gate`); towers at the gates and every `tower_spacing_m` along the ring. Band:
ring offset by ±`thickness_m/2` (`overlay::offset`, difference).

**Checks** (`TypedStage::checks`, errors): the ring is closed and simple, encloses the centre,
crosses no lake cell, crosses a river only at a `water-gate`, every main street crosses it
exactly once, at a gate.

- [ ] **Step 1: Write the failing tests:** `the_ring_encloses_the_centre_and_is_simple`,
  `a_lake_across_the_ring_zone_fails_naming_the_zone`, `each_river_crossing_gets_a_water_gate`,
  `a_main_street_crossing_twice_fails_the_check`, `a_ring_zone_narrower_than_a_cell_fails_naming_it`,
  `towers_stand_at_gates_and_at_the_spacing`.
- [ ] **Step 2–4:** FAIL → implement, register → PASS.
- [ ] **Step 5: Commit** `feat(generators): a town wall with gates and towers`.

### Task 22: `blocks` (§5.9)

**Files:** Create `stages/settlement/blocks.rs` (+ tests).

**Interfaces — Produces:**

```rust
pub struct Block { pub polygon: Polygon, pub sides: Vec<Option<u32>>,   // street edge per outer side
                   pub open: bool }                                      // open ground
pub struct Blocks { pub blocks: Vec<Block> }                             // "blocks"
```

**Reads → writes:** `street_graph`, `water`, `zones`, [`wall`] → `blocks`. **Params:**
`zones` (the zones the town fills; default: the street stages' zones), `min_block_m2`.

The filled zone is sliced by every street centreline, the wall ring and the water outlines
(`overlay::slice`); each face is inset by half the width of the street on each side, or half the
wall's thickness on a wall side (per-side insets: offset the face by the smallest half width,
then subtract each wider street's band). Faces below `min_block_m2` or with no frontage are
`open`; water is cut out. Order: by the first lattice point of the outer ring.

- [ ] **Step 1: Write the failing tests:** `four_streets_around_a_square_give_one_inset_block`,
  `a_sliver_and_a_triangle_become_open_ground` (Review Focus 5),
  `a_block_with_a_hole_keeps_it` (a street loop inside),
  `a_block_on_both_sides_of_a_river_is_cut_by_it`, `each_side_knows_its_street`.
- [ ] **Step 2–4:** FAIL → implement, register → PASS.
- [ ] **Step 5: Commit** `feat(generators): blocks between streets`.

### Task 23: `parcels` (§5.10, after Vanegas 2012)

**Files:** Create `stages/settlement/parcels/{mod.rs, strips.rs, cuts.rs, core.rs, tests.rs}`.

**Interfaces — Produces:**

```rust
pub enum ParcelUse { House, Landmark, Yard, Garden, Plaza, Open }       // §5.14 types
pub struct Parcel { pub polygon: Polygon, pub use_: ParcelUse, pub street: Option<u32>, pub block: u32 }
pub struct Parcels { pub parcels: Vec<Parcel> }                          // "parcels"
```

**Reads → writes:** `blocks`, `street_graph`, `height` → `parcels`. **Params** per style
(`style = "organic" | "planned"`, set per zone): organic `frontage_m = [5, 9]`,
`depth_m = [25, 45]`; planned `frontage_m = [10, 18]`, `depth_m = [15, 25]`; `irregularity`;
`min_area_m2`.

1. Band = block − block inset by `depth_m` (drawn per block); cut at each convex corner with
   `bisector_cut`; at a corner the wider street keeps the piece.
2. Strips cut across, perpendicular to their street, at frontages from a normal distribution
   around the middle of the range (Box–Muller with `libm`), clipped to the range; remnants below
   the minimum join their neighbour along the longest shared edge.
3. The inner core: recursive split of `min_area_box`'s long side through its middle, shifted by
   `irregularity ·` short side; a piece that would lose street access is cut the other way;
   stop when area and frontage are in range; pieces without access are `yard` or `garden` of
   the plot they touch most.
4. A child RNG per block whose seed is drawn from `ctx.rng()` before any recursion.
The square from `centre` becomes the `plaza` parcel.

- [ ] **Step 1: Write the failing tests:** `every_house_parcel_faces_a_street`,
  `frontages_stay_within_the_range` (organic 5–9 m), `a_straight_corner_gets_no_cut`,
  `frontage_wider_than_the_strip_gives_one_parcel` (Review Focus 5),
  `a_change_in_one_block_leaves_the_others`, `the_square_is_the_plaza_parcel`.
- [ ] **Step 2–4:** FAIL → implement, register → PASS.
- [ ] **Step 5: Commit** `feat(generators): street-front parcels`.

### Task 24: `buildings` and landmarks (§5.11)

**Files:** Create `stages/settlement/buildings/{mod.rs, footprints.rs, landmarks.rs, tests.rs}`.

**Reads → writes:** `parcels`, `street_graph`, `height`, `water`, `centre`, `site`, [`wall`],
[`rivers`] → `buildings` (U8 mask), `parcels` (landmark merges and yards), `features`.

**Params:** `building_depth_m`, `snap_m`, `max_slope` 0.3, `storeys_max`, `storey_m`,
`roof_m`, `landmarks` (`[{ type = "market", required = true }, …]`), `landmark_area_m2` per
type, `mill_q`.

- Footprints: a rectangle from the frontage, aligned with the street, `building_depth_m` deep;
  sides within `snap_m` of the parcel's sides snap to them; footprint = rectangle ∩ parcel
  (`overlay::intersection`). Dropped on water or when mean slope exceeds `max_slope` (the parcel
  becomes `yard`).
- Storeys 1..=`storeys_max` by a density falling off with distance from the centre and the main
  streets; `height_m = storeys · storey_m + roof_m`; `use = "house"` unless a landmark.
- Landmarks by the rules of the spec's table, in the recipe's order; each merges its parcel with
  neighbours along the same street until `landmark_area_m2`. No place: `landmarks_absent` is a
  list in the stage report plus one `Finding::warning` per landmark naming its type and reason
  (gap 7); a missing required landmark counts in `landmarks_missing_required`.
- The `buildings` mask marks cells whose centres lie inside a footprint.
- Features (L20): `building`/`building` Polygons with `use`, `storeys`, `height_m`, `parcel`
  (`Prop::Item { list: "parcels", .. }`); `landmark`/<type> Points with `name`
  (`features::generated_name`, "generated, not canon") and `building` or `parcel`.
- No building at all: fail "no building fits" (spec §5.3 and §5.13, Jann's bar).

- [ ] **Step 1: Write the failing tests:** `no_building_leaves_its_parcel_or_touches_a_street`,
  `neighbours_snap_into_rows`, `a_footprint_on_water_leaves_a_yard`,
  `the_market_takes_the_square`, `a_mill_without_a_river_is_absent_with_its_reason`,
  `a_landmark_that_cannot_grow_is_absent_with_its_reason` (Review Focus 5),
  `a_town_without_buildings_fails`.
- [ ] **Step 2–4:** FAIL → implement, register → PASS.
- [ ] **Step 5: Commit** `feat(generators): buildings and landmarks`.

### Task 25: Budget: one 1,600 m town at 4 m (spec §7)

**Files:** Create `crates/theos-maps/tests/fixtures/recipes/settle-town-400.toml` (synthetic,
standalone `site_terrain` town, 400 × 400, a river, three entries; stages to `buildings`). The
measuring test is temporary.

- [ ] **Step 1: Measure** in release: a temporary in-crate test plans the fixture and calls
  `execute` with seed 1; `cargo nextest run -p theos-maps --release settle_budget --no-capture`;
  read `timing_s` per stage; record it in the ledger. Delete the test.
- [ ] **Step 2: Decide.** Under 10 s (spec §7 budget): note it and go on. Over: profile the
  hottest stage, fix it (fewer `shortest_paths` calls, bounded searches, coarser tracer step)
  and measure again before Task 26; never lower a test's coverage to fit.
- [ ] **Step 3: Extrapolate** the suite: parity 1 parent + 8 towns, golden 1 parent + 2 towns +
  1 village, CE-78.1/78.2 runs; record the total in the ledger. Name every test with a parent
  run `slow_…`.
- [ ] **Step 4: Commit** `test(generators): a 400² settlement fixture for the budget`.

### Task 26: `fields` (§5.12)

**Files:** Create `stages/settlement/fields.rs` (+ tests).

**Reads → writes:** `height`, `water`, `buildings`, `street_graph`, `centre`, `parcels` →
`fields` (U8). **Params:** `parcel_m` 55, `reach_m` 700, slope and distance thresholds of 0.3.0
(`settlement.py`).

0.3.0's method on `kernels::voronoi::nearest_seed`: seeds on a jittered grid of `parcel_m` out to
`reach_m` from the built area, typed `field`, `orchard` or `pasture` by slope and distance. Inside
the town: parcels give `garden` 4, `yard` 5, `open` 6, `plaza` 7. Legend: 0 none, 1 field,
2 orchard, 3 pasture, 4 garden, 5 yard, 6 open ground, 7 plaza.

- [ ] **Step 1: Write the failing tests:** `no_field_beyond_reach`, `water_and_streets_stay_0`,
  `parcels_set_garden_yard_open_and_plaza`.
- [ ] **Step 2–4:** FAIL → implement, register → PASS.
- [ ] **Step 5: Commit** `feat(generators): fields around a settlement`.

### Task 27: `settlement_metrics` (§5.13)

**Files:** Create `stages/settlement/metrics/{mod.rs, defects.rs, grid_index.rs, tests.rs}`.

**Reads:** `buildings`, `streets`, `height`, `water`, `street_graph`, `parcels`, `entries`,
`centre`, [`wall`], `features`. **Writes:** nothing. **Params:** `persons_per_house` 5,
`access_m` 8, `max_slope` 0.3, `required` (the required landmarks; the recipes copy their
`buildings` stage's list).

`metric_names` (all reported, L18): `buildings`, `houses`, `landmarks`, `population_est`,
`street_km_main`, `street_km_street`, `street_km_lane`, `street_components`, `entries_connected`,
`street_access_pct`, `buildings_on_water`, `buildings_on_street`, `building_overlap_cells`,
`buildings_steep`, `landmarks_missing_required`, `gates`, `water_gates`, `towers`,
`wall_lake_cells`, `built_area_ha`, `grid_index`.

Definitions exactly as §5.13, on the map's own grid (the same as Task 13's driver): a footprint
covers the cells whose centres lie inside it (`geometry::contains` per cell centre);
`street_components` 8-connected (`kernels::labels`); `entries_connected` through the `streets`
raster; slope from `kernels::diff::slope`; `grid_index` as L19.

- [ ] **Step 1: Write the failing tests** (hand-made 32² worlds): `a_footprint_on_a_street_cell_counts`,
  `overlap_counts_cells_not_buildings`, `access_uses_8_m`, `two_street_parts_give_two_components`,
  `grid_index_is_1_for_a_grid_and_near_0_for_a_radial_fan`
  (`(1.0 − grid_index).abs() < 1e-12` for an axis grid; `< 0.2` for 36 spokes),
  `the_metrics_match_the_driver_on_a_shared_case` (a 16² case whose values were computed with the
  driver's functions and pasted with the command in a comment).
- [ ] **Step 2–4:** FAIL → implement, register → PASS.
- [ ] **Step 5: Commit** `feat(generators): settlement metrics`.

---

## Part E — outputs and recipes

### Task 28: Settlement features (format 1.1, §5.14)

**Files:** Create `crates/theos-maps/src/features/settlement.rs` (+ tests); modify
`kind/settlement.rs` (`types`, `features`).

**Settlement types** (`kind/settlement.rs`, `(kind, type)`): `street`: `main-street`, `street`,
`lane`; `parcel`: `house`, `landmark`, `yard`, `garden`, `plaza`, `open`; `building`: `building`;
`landmark`: `market`, `well`, `shrine`, `temple`, `keep`, `inn`, `smithy`, `guardhouse`,
`granary`, `archive`, `mill`, `harbour`; `wall`: `wall`, `tower`; `gate`: `gate`, `water-gate`.
Ids `IdScheme::ByType`.

`build(world, ids)` assembles, in this order: street edges (`from`/`to` = `node-<id>` text,
`width_m`, `length_m`, `bridge`, `entry` on main streets), parcels (`street` as an item ref,
absent for none, `area_m2`), the `features` set (buildings, landmarks), the wall (Polygon band,
`height_m`, `thickness_m`), towers (`wall`), gates (`wall`, `street` for `gate`); lists
`street_graph`, `parcels`, `wall` for `Prop::Item`; then `assemble` (Task 2). Coordinates are
`to_m` of lattice points (exact).

- [ ] **Step 1: Write the failing tests:** `every_settlement_feature_has_its_properties` (one of
  each kind from a hand-made world, with the property names of §5.14),
  `references_resolve_to_ids` (`building.parcel` = a `house-<n>`/`landmark-<n>` id;
  `gate.street` = a `main-street-<n>` id), `no_block_feature_and_no_parent_settlement`.
- [ ] **Step 2–4:** FAIL → implement → PASS.
- [ ] **Step 5: Commit** `feat(generators): settlement features`.

### Task 29: Settlement package layers (§5.3, `docs/map-package.md` §4.4)

**Files:** Modify `package/table.rs`, `kind/settlement.rs` (`layers`).

Layers in order: `height` (f32, m), `water` (u8, legend `0 dry, 1 stream, 2 river, 3 lake`),
`streets` (u8, legend `0 none, 1 lane, 2 street, 3 main-street`), `buildings` (u8 mask, legend
`0 no, 1 yes`), `fields` (u8, the legend of §5.12). All `Source::Raster`.

- [ ] **Step 1: Write the failing test** `settlement_packages_hold_the_five_layers_with_legends`.
- [ ] **Step 2–4:** FAIL → implement → PASS.
- [ ] **Step 5: Commit** `feat(generators): settlement package layers`.

### Task 30: Renders `base` and `plan` (§5.15)

**Files:** Create `crates/theos-maps/src/render/settlement/{mod.rs, base.rs, plan.rs, tests.rs}`;
modify `kind/settlement.rs` (`renders`, `title`).

- `base` (standard): frame-exact, 2 pixels per cell; hillshade of `height` (P08's `shade`),
  water, fields by land use, street surfaces by width, roofs as fills, the wall band. No symbols,
  no text.
- `plan` (standard): `base` plus building and parcel outlines, streets by class, the wall with
  gates and towers, landmark symbols with their generated names, the town's name, a legend, a
  scale bar and a title line "`<recipe>` · seed `<n>` · `<buildings>` buildings · population
  `<population_est>`"; it may have margins.
- Terrain renders never run for settlements (the kind table).

- [ ] **Step 1: Write the failing tests:** `base_is_frame_exact` (a building at a known lattice
  position lands on the expected pixels, 800 × 800 for 400²), `base_has_no_text`
  (no `fonts` call; pixel equality with a run without names), `plan_has_a_title_and_legend`.
- [ ] **Step 2–4:** FAIL → implement → PASS.
- [ ] **Step 5: Commit** `feat(generators): settlement renders base and plan`.

### Task 31: Built-in recipes and the AC runs (§5.16, §5.19)

**Files:** Create `crates/theos-maps/recipes/{village.toml, market-town.toml, grid-town.toml}`,
`tests/maps/settlements.rs`, `tests/fixtures/recipes/settle-child-only.toml`; modify
`recipe/builtin.rs`, `tests/maps/main.rs`, `tests/maps/support.rs`, `examples/run.rs`
(`--from <package> --input <name>=<value>`).

Recipes (each declares `[inputs] feature = { type = "feature", parent_types = […] }`, has both
`local_terrain` and `site_terrain`, and carries the §5.17 constraint set under
`stages.settlement_metrics.<name>`):

| Recipe | `parent_types` | Stages after `site_plan`/`main_streets` | Landmarks (required) |
|---|---|---|---|
| `village` | `["village", "hamlet"]` | `organic_streets`, `blocks`, `parcels`, `buildings`, `fields`, `settlement_metrics` | `well`, `shrine`, `smithy`, `inn`, `mill` (none) |
| `market-town` | `["town", "city"]` | `wall` (zone `core`), `organic_streets` (core), `organic_streets` (outer), `blocks`, `parcels`, `buildings`, `fields`, `settlement_metrics` | `market`, `well`, `shrine`, `inn`, `smithy`, `guardhouse`, `granary`, `mill` (`market`, `shrine`, `inn`, `smithy`, `guardhouse`, `granary`) |
| `grid-town` | `["town", "city"]` | `organic_streets` (core), `planned_streets` (outer, `align = "main-street"`), `blocks`, `parcels`, `buildings`, `fields`, `settlement_metrics` | as `market-town` without the wall |

`settle-child-only.toml` is `village` without `site_terrain` (for the missing-`from` refusal).
Tune defaults with `examples/run` contact sheets of `plan` against the constraints; record each
round in the ledger.

- [ ] **Step 1: Write the failing tests** (`tests/maps/settlements.rs`; a helper
  `parent_package(dir) -> PathBuf` runs `fixture-small` seed 1 once into `dir`):
  - `// AC: CE-78.1` `slow_each_built_in_settlement_recipe_runs_standalone_and_as_a_child`:
    for `village` (feature `village-1`), `market-town` and `grid-town` (feature `town-1`),
    standalone seed 1 and child seed 1; each package reads back with `theos_map_format::read`,
    `kind == "settlement"`, holds `height`, `water`, `streets`, `buildings`, `fields`, and its
    features include kinds `street`, `parcel`, `building`, `landmark`, and for `market-town`
    `wall` and `gate`; outputs `base` and `plan` exist.
  - `// AC: CE-78.1` `slow_settlement_base_is_frame_exact`: the PNG of `base` is
    `2·cols × 2·rows` pixels and a building's centre pixel has the roof colour.
  - `// AC: CE-78.2` `slow_a_child_carries_origin_and_parent`: `market-town` from the parent,
    `feature = "town-1"`, seed 1: `frame.origin_m` equals the snapped window corner (computed in
    the test from the parent's `town-1` point: `round_half_even((p − 800) / 4) · 4`),
    `frame.parent.package_sha256` equals the SHA-256 of the parent file, `frame.parent.feature`
    is `town-1`; the run manifest's `frame.origin_m` is the same.
  - `// AC: CE-78.2` `slow_the_same_parent_feature_seed_and_inputs_give_the_same_child`: two runs
    give equal package bytes; seed 2 or feature `village-1` with `village` give other bytes.
  - `// AC: CE-78.2` `child_runs_are_refused_before_the_run`: `settle-child-only` without
    `from` → refused naming `from`; `market-town` with `feature = "ruin-1"` → refused naming the
    type `ruin` and the accepted types; `feature = "town-9"` → refused naming the id.
  - `describe_lists_the_settlement_recipes_with_their_feature_input`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement and tune.** If a recipe cannot meet its constraints on the standalone
  and child seed-1 runs, record why and tune further; Task 32 is the hard bar.
- [ ] **Step 4: Run** the crate's tests, `slow_` included. Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): built-in settlement recipes` (`Refs: CE-78.1, CE-78.2`).

---

## Part F — parity, golden, finish

### Task 32: Parity with 0.3.0 (CE-78.3, §5.17)

**Files:** Create `tests/maps/settlement_parity.rs`; modify `recipes/market-town.toml`
(parameters only).

- [ ] **Step 1: Write the test.**

  ```rust
  // AC: CE-78.3
  #[test]
  fn slow_market_town_passes_at_least_as_often_as_0_3_0() -> TestResult {
      let reference = SettlementReference::load("tests/fixtures/parity/settlement-0.3.0.toml")?;
      // market-town's constraints include every range of the fixture, with equal bounds
      reference.assert_constraints_of("market-town")?;
      let parent = parent_package(&dir)?;                       // Rust fixture-small seed 1
      let passed = (1..=8).filter(|&s| run_child("market-town", s, &parent, "town-1")?.pass).count();
      assert!(passed >= reference.passes(), "{passed} < {}", reference.passes());
      Ok(())
  }
  ```

  The reference's pass count is recomputed from its per-seed values and `[ranges]` and must
  equal the recorded `passes` flags.
- [ ] **Step 2: Run.** Expected: likely FAIL before tuning; record per-seed metrics in the ledger.
- [ ] **Step 3: Tune** `market-town`'s parameters (not its constraints, not the fixture) until it
  passes; if it cannot without breaking the spec, stop and tell Jann (`CLAUDE.md` "When to stop").
- [ ] **Step 4: Run** the crate's tests (recipe changes affect Task 31's runs). Expected: PASS.
- [ ] **Step 5: Commit** `test(generators): settlement parity with 0.3.0` (`Refs: CE-78.3`).

### Task 33: Golden runs and `VERSION` 1.1.0 (CE-78.4, §5.18, §5.2 item 9)

**Files:** Modify `api.rs` (`VERSION = "1.1.0"`), `golden.rs` (`GOLDEN_RUNS`), `tests/maps/golden.rs`,
`crates/theos-map-format/src/manifest.rs` (`FORMAT_VERSION = "1.1"`, §5.2 item 11; spec ruling
16: every kind, terrain included) and the format crate's tests that spell `1.0` as the written
version. Create `tests/fixtures/golden/1.1.0.toml` (written by `bless`).

`GOLDEN_RUNS`: the three terrain runs, then
`{ recipe: "village", seed: 1, inputs: &[], from: None }`,
`{ recipe: "market-town", seed: 1, inputs: &[("feature", "town-1")], from: Some(("fixture-small", 1)) }`,
`{ recipe: "grid-town", seed: 1, inputs: &[("feature", "town-1")], from: Some(("fixture-small", 1)) }`.
If P34 merged 1.1.0 first: merge `main`, take 1.2.0 (L14). This is the last task that changes
outputs; a later fix that changes outputs deletes `1.1.0.toml` and records it again (before the
PR merges only).

- [ ] **Step 1: Write the failing tests:**
  - `// AC: CE-78.4` `the_golden_runs_cover_a_settlement_and_a_child_map`: `GOLDEN_RUNS` holds a
    settlement run without `from` and one with `from`, both settlement recipes.
  - `// AC: CE-77.1` `slow_outputs_match_the_golden_hashes_of_this_version` (existing): now
    against 1.1.0 with the six runs.
  - `// AC: CE-77.2` (existing) `a_changed_output_fails_the_golden_comparison`: also alter a
    `parent#0` hash of the `market-town` run; `compare` names `parent#0`.
  - `packages_are_written_as_format_1_1` (format crate): `written_manifest(..).format_version ==
    "1.1"`; `a_1_0_package_still_reads`: a package whose manifest says `1.0` reads.
- [ ] **Step 2: Run.** Expected: FAIL (no 1.1.0 file, format 1.0).
- [ ] **Step 3: Record** with `cargo run -p theos-maps --example bless`.
- [ ] **Step 4: Run** the crate's tests, `slow_` included. Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): golden hashes 1.1.0 with settlements`
  (`Refs: CE-78.4, CE-77.1, CE-77.2`).
- [ ] **Step 6: Platforms** (after the PR is open): labels `windows` and `macos`; link the
  `platforms` runs (Windows, macOS Apple silicon, macOS Intel) in the PR.

### Task 34: Finish and verification

**Files:** Modify `crates/theos-maps/README.md`, `crates/theos-map-format/README.md`, crate docs;
`docs/status.md` only at PR time (the theos-phase skill).

- [ ] **Step 1: READMEs:** settlement maps, child runs (`from` and the `feature` input), the
  example's `--from`, how to add a street style (one stage module, one register line), the
  lattice rules for contributors.
- [ ] **Step 2: Contact sheet:** `cargo run -p theos-maps --release --example run -- market-town
  grid-town village --seeds 1..8 --from <fixture-small seed 1 package> --input feature=town-1
  --out <scratch>` (village with `feature=village-1`); keep the sheets for the PR (not committed,
  G10).
- [ ] **Step 3: Verify**, reading each output:
  - `cargo xtask gate` on Windows;
  - `cargo xtask ac-trace --phase P29`: CE-78.1–78.4 each with a passing test;
  - `cargo nextest run -p theos-maps -E 'test(/slow_/)' --locked` (the slow suite);
  - `cargo tree -e features -i i_overlay` (no `allow_multithreading`);
  - the release timing of one `market-town` child run at 400² (spec §7: under 10 s).
- [ ] **Step 4: Commit** `docs(generators): settlement and child map guides`.

---

## After the build

The theos-phase skill continues: the final review loop (fresh whole-branch reviewers on the most
capable model with this plan, the spec, G1–G16, the ACs, CE-77/CE-28.3 and every earlier
finding; cap 5), verification, then the PR `feat(generators): settlements and child maps (P29)`
with every ruling of spec §9 and this plan (L1–L20), the contact sheets, and the `windows` and
`macos` labels. The PR touches no protected path, so auto-merge is allowed once CI is green.

## Spec gaps found while planning

Recorded for the PR; the spec is not changed on this branch.

1. **§5.2 item 3 vs §5.16 and ruling 2:** item 3 refuses a recipe with a `feature` input when
   `from` is missing; §5.16 lets the same recipes run standalone. Resolved by L5.
2. **Settlement frame keys:** the shared item 4 keeps `grid`/`size_km` for interiors but the
   schema had them required for all; for settlements their place and the standalone class are
   not stated. Resolved by L7 and L8.
3. **Bicubic class layers:** §5.2 item 5 resamples every parent layer bicubically, but `water` is
   a class layer that `local_terrain` thresholds. Resolved by L11 (nearest for layers with a
   legend). This touches shared item 5: agree with P34.
4. **Mill discharge:** the mill rule reads "parent discharge" but no value in the §5.3 table
   carries it into `buildings`. Resolved by L15 (`rivers`).
5. **`buildings` writes `features`** (§5.3) while §5.14 builds features after the pipeline.
   Resolved by L20.
6. **River class in a child:** §5.5 draws rivers at their widths but does not say which `water`
   class (stream or river) a parent river line gets; Task 15 uses the parent recipe's
   `big_river_q` when the parent manifest's recipe has it, else class 2.
7. **`landmarks_absent`** is named in §5.11 but is not a number, so it is a report list and a
   warning, not a metric (Task 24).
