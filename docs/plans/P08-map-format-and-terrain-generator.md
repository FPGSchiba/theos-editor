# P08 · Map format and terrain generator — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (`CLAUDE.md` makes it the execution method) to implement this plan task by task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** two new library crates:
- `theos-map-format` reads and writes map packages (`docs/map-package.md`, format 1.0).
- `theos-maps` turns a terrain recipe and a seed into a package, a `features` output, a
  frame-exact `base` render and renders for people. It builds the package at the current state of
  the art, with the same bytes on every platform (G20).

**Architecture:**
- Every generation step is a `Stage`: it declares the layers it reads and writes, takes typed
  parameters, and is checked after it runs. The registry is one list.
- A run follows these steps:
  1. validate the recipe against the registry;
  2. estimate memory;
  3. run the stages over a `World` of named layers (each stage gets its own random stream);
  4. hash every written layer;
  5. evaluate the constraints;
  6. assign feature ids;
  7. draw the renders;
  8. write the package through `theos-map-format`.
- `landscape_evolution` runs its own list of per-step processes under the same contract.
- Shared mathematics live in `kernels/` as pure functions.

**Tech stack:**
- Rust 1.99.1 (edition 2024). Crates already in the workspace: `serde`, `serde_json`, `toml` 1.1,
  `sha2` 0.11, `thiserror`, `tracing`, `cargo_metadata`, `tempfile`, `anyhow`, `clap`.
- New crates, versions from the spec's probes (§4.3):
  - **Package:**
    - `zip` 9.5 without default features, with `deflate-flate2`;
    - `flate2` 1.1 without default features, with `rust_backend`;
    - `geojson` 1.0 without default features.
  - **Generator:**
    - `libm` 0.2.16;
    - `rand_pcg` 0.10 and `rand_core` 0.10 (trait `rand_core::Rng`);
    - `spade` 2.15 without default features;
    - `serde_ignored` 0.1.
  - **Renders:**
    - `tiny-skia` 0.12 without default features, with `no-std-float`;
    - `png` 0.18 without default features;
    - `ab_glyph` 0.2.32 without default features, with `libm`;
    - `wuff` 0.2.10 without default features, with `brotli`.

**Spec:** `docs/specs/phases/P08-map-format-and-terrain-generator.md` (approved in #27, amended by
decision 0004 in #31). Read it with this plan. Where this plan is silent, the spec decides; where
they differ, the spec wins. The section numbers (§5.9 …) below are the spec's.

**References for implementers:**
- The Python toolset `theos_maps` 0.3.0 (Jann's export, in no repository) is at
  `D:\Projects\theos_maps-export`:
  - `theos_maps/terrain.py`: grid, tectonics, geology, climate, evolution, hydrology, ecology,
    elements;
  - `theos_maps/features.py`: settlements, resources, pois, roads, sites, travel;
  - `map-stages/omega_detuning.py`: ω;
  - `theos_maps/render_terrain.py`: renders;
  - `theos_maps/common.py`: noise, curves, contact sheet;
  - `recipes/*.yaml`: the recipes.

  It runs with
  `uv run --no-project --python 3.12 --with fastscapelib==0.3.0 --with numpy==2.5.4 --with scipy==1.18.1 …`.
- fastscapelib and goSPL are GPL-3.0: never open their code. Their documentation, the papers and
  Landlab (MIT) may be read.

## Global Constraints

The umbrella's global constraints, verbatim. `CLAUDE.md` asks for G1–G16; G17–G20 are copied too,
as in the P02–P04 plans.

- **G1** Repositories: `theos-editor` (this repo) holds the editor and the map generator
  (`theos-map-format`, `theos-maps`). `tower-of-theos` holds the content project, its map recipes
  and the blockout generator. `Theos-Engine` is never a dependency, except behind the cargo
  feature `engine`, which no phase here builds; the engine may depend on the map crates.
- **G2** Platforms: Windows 11 x64 (primary), Linux x64, and macOS 13 or later on Apple silicon
  and Intel (universal builds). Every PR is checked on Linux; Windows and macOS are tested
  weekly, on demand (a label on the PR) and before every release (`docs/gate.md` §1).
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

**CE-76 Map package format** (`theos-map-format`, `docs/map-package.md`)
- CE-76.1 A package written by `theos-map-format` and read back gives the same manifest, layers
  and features, and its bytes follow `docs/map-package.md` §1–§5 (checked against a fixture
  package).
- CE-76.2 A package whose `format_version` has an unknown major is refused with a message naming
  both versions; fields, entries, layers and feature kinds the reader does not know are ignored.
- CE-76.3 A synthetic package with two stacked levels round-trips with every layer and feature on
  its level; on a map with levels, a layer or feature without a level, or with a level the frame
  does not list, is rejected.
- CE-76.4 `theos-map-format` depends on no other crate of this workspace.
- CE-76.5 A package whose `requires` names a capability the reader does not implement is refused
  with a message naming it.

**CE-28 theos-maps terrain generator**
- CE-28.1 `describe` gives the name `theos-maps`, the generator's own version, `produces: ["map"]`
  and the recipes with their inputs: the built-in terrain recipes converted from `theos_maps`
  0.3.0 (`hearth-valley`, `hearth-valley-omega`, `fixture-small`) and the recipes of a given
  folder, where a recipe of the folder wins over a built-in one with the same id.
- CE-28.2 `run` reports a progress event per stage and ends with a manifest carrying the frame,
  the outputs `base`, `truth`, `inworld`, `geology`, `resources`, `features` and `package` (plus
  the extra renders a recipe asks for), and results with each constraint under the keys of
  `docs/generators.md` §3 and `pass`.
- CE-28.3 A recipe whose stage order leaves a stage without an input fails before the first stage
  runs, with a message naming the stage and the missing layer; an unknown stage or recipe input
  fails the same way.
- CE-28.4 Cancelling a run stops it at its next cancel point, within one second at 384², and
  leaves no files behind; a stage that panics ends the run as failed with the panic's message.
- CE-28.5 For each built-in recipe over seeds 1–8, the constraints pass at least as often as with
  `theos_maps` 0.3.0, and the medians of relief, lake share, river share, vegetation cover and
  median precipitation stay within the tolerances the phase spec sets around the 0.3.0 reference,
  recorded as fixtures.
- CE-28.6 Every feature has an `id`, `properties.kind` and `properties.type` as in
  `docs/map-package.md` §5; lakes carry `surface_m`, and rivers are lines with width, discharge
  and bed height; the `base` render lies exactly on the frame (a feature at a known position lands
  on the expected pixel).
- CE-28.7 The omega detuning stage of 0.3.0 is a stage of the generator; `hearth-valley-omega`
  writes the `omega` layer.

**CE-77 Determinism** (G20)
- CE-77.1 For fixed recipes and seeds, the hashes of every layer after every stage, of the
  `features` output and of the package bytes equal the golden hashes on Linux x64, Windows x64,
  and macOS on Apple silicon and Intel.
- CE-77.2 Golden hashes are kept per generator version: outputs that differ from the golden hashes
  of the current version fail a test, and the tool that records golden hashes refuses to overwrite
  those of an existing version.

**Jann's decisions that bind the build** (spec §4.1):
- P08 builds a terrain generator at the current state of the art, not a port of 0.3.0. The
  physics tests prove correctness, and the 0.3.0 numbers are a sanity range (CE-28.5).
- **Every generation step is a `Stage`, now and later** (spec §5.3, `docs/architecture.md` §11).
- Renders: "a simpler version with the same data richness (or more)".
- Decision 0004 (12 Nov 2026), all binding:
  - the fluvial law as in Yuan et al. 2019, with their tolerance and physics test 14;
  - the place types `chime`, `gate-up` and `gate-down`;
  - `river_q` and `big_river_q` are required in every recipe;
  - the mill rule is relative to `river_q`, and the edge margins are a share of the frame;
  - channel discharge follows the single steepest receiver;
  - `requires` is written as `[]`;
  - a memory budget, 3 GiB by default, replaces a fixed grid limit;
  - `route_potential`, `market`, the extra metrics and the snapshot export are not in P08.

## Build rules (every task, every subagent)

Put this section into every implementer and reviewer brief.

- **Engineering directives** (`CLAUDE.md`):
  - Small modules with one job. A new stage is one module plus one line in
    `stages::register`; a new per-step process is one module plus one line in
    `stages::evolution::PROCESSES`; a new kernel is one module in `kernels/`.
  - Use the crate before custom code, as named in this plan (§5.1 of the spec). Never write a
    TOML, JSON or GeoJSON parser; ZIP, deflate, PNG and font decoding are crates.
  - If reviews keep finding new variants of one problem, stop and name the class (directive 3).
- **Determinism (G20, spec §5.7)**:
  - Math functions only through `libm`. From the gate PR (Task 1) on, clippy rejects the std
    methods.
  - Exact operations (`+ − × ÷`, `sqrt`, `abs`, `floor`, `round`, `ceil`, `trunc`,
    `rem_euclid`) are allowed.
  - Maxima and minima of floats go through `kernels::num::{max, min}`, never `f64::max`/`min`.
    Sort and queue keys go through `kernels::num::key` (adds `+0.0`).
  - Sorts and priority queues use `total_cmp` and break ties by cell index, then by feature
    handle.
  - No `HashMap`/`HashSet` anywhere in `theos-maps` or `theos-map-format`. Use `BTreeMap`/`Vec`.
  - No threads inside a run.
  - Seed and hash arithmetic uses `wrapping_*`.
- **Numeric casts:** `as` between integers and floats only inside `kernels::num` (Task 6), which
  carries the only `#[expect(clippy::cast_…, reason = "…")]` attributes. Everything else calls
  its helpers.
- **Lints** (workspace, `-D warnings`):
  - No `#[allow]` or `#[warn]`. `#[expect]` only with `reason = "…"`. Prefer renaming over
    `#[expect(clippy::many_single_char_names)]`; when a paper's notation is clearer, an expect
    with the reason "the notation of <paper>" is fine.
  - Every public item is documented, and every public fn returning `Result` has `# Errors`.
  - No `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `dbg!`, `print!` or `exit`
    outside tests. Examples write with `writeln!` to a locked stdout.
  - Pedantic is on: `float_cmp` means tests compare floats with a tolerance or by `to_bits()`.
  - Coverage counts `src/**`: test error paths too. Both crates need 80 %.
- **Dead code between tasks:** a module that a later task wires in carries
  `#[cfg_attr(not(test), expect(dead_code, reason = "wired in by Task N"))]` on its `mod` line.
  The wiring task removes it.
- **Tests:**
  - Units go in `#[cfg(test)] mod tests` next to the code. Physics tests are unit tests of
    their process modules (ruling L15).
  - Behaviour tests go in one integration crate per library: `crates/theos-map-format/tests/format/`
    (`main.rs` plus modules) and `crates/theos-maps/tests/maps/` (likewise). They use only the
    public API.
  - A test proving an AC is a plain `#[test]` fn with `// AC: CE-xx.y` on the line directly above
    `#[test]`.
  - Tests may return `Result<(), Box<dyn std::error::Error>>`.
  - Test grids are 64² unless the test needs more.
  - Reference values from numpy or scipy are computed with
    `uv run --no-project --python 3.12 --with numpy==2.5.4 --with scipy==1.18.1 python -c "…"`
    and pasted with a comment naming the call.
- **Fixtures:**
  - Recipes under `tests/fixtures/` start with `# synthetic fixture (P08), not canon`.
  - No lore text anywhere. Generated settlement names are marked "generated, not canon" (spec
    §5.10). Map texts come only from `[render.inworld]`.
- **Dependencies:** pin in the root `[workspace.dependencies]` (members use `dep.workspace = true`).
  The commit that adds one runs one unlocked `cargo build -p <crate>` to update `Cargo.lock`, then
  `cargo deny --locked check`.
- **Files with backslashes** (Rust string escapes, JSON) are written with the Write or Edit tool,
  never with a shell heredoc.
- **Git:**
  - One git command per Bash call in the worktree (no `&&` chains). Never `--no-verify`.
  - Commit message: a Conventional Commit with scope `generators` for both crates (`build` for a
    dependency-only commit). A body says why. The footers come as one block, with no line over
    100 characters (split long `Refs:` lines). Leave `Refs:` out when a task proves no AC.

  ```text
  Refs: CE-28.3
  Phase: P08
  Claude-Session: 78b6ed41-6252-4013-8697-8f7d7ad1f219
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  ```
- **Per-task checks** (run from the worktree root, read the output):
  - `cargo fmt --all --check`;
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`;
  - `cargo nextest run -p theos-map-format -p theos-maps --locked`;
  - a task that adds a dependency also runs `cargo deny --locked check`.

## Review Focus

These are the five inputs the spec implies, that no AC test covers and that are most likely to
bite. Each has its test in the owning task.

1. **A recipe with no drained edge, or with all four drained.** Expected:
   - the run finishes;
   - every layer is finite;
   - with no outlet, every depression is a terminal lake and discharge never leaves the map;
   - with four outlets, every cell drains to an edge.

   Tests: Task 22 (`closed_and_fully_open_edges_route_every_cell`), Task 28
   (`a_map_without_outlets_keeps_its_water`).
2. **A 64² or non-power-of-two grid whose metre-sized kernels are larger than the map.**
   Expected: the filters, the FFT and the noise give finite fields of the grid's shape, with
   reflect mode applied more than once where σ exceeds the map. Tests: Task 13
   (`noise_on_odd_shapes_is_finite`), Task 14
   (`gaussian_wider_than_the_map_reflects_and_stays_finite`).
3. **−0.0, NaN or a ±0 tie reaching a layer.** Expected:
   - a stored or hashed −0.0 is +0.0;
   - `num::max(-0.0, 0.0)` and `num::min(-0.0, 0.0)` are +0.0;
   - NaN or an infinity in a written layer fails the run with `RunError::NonFinite` naming the
     stage, the layer and the cell.

   Tests: Task 6 (`max_and_min_return_positive_zero_on_ties`), Task 7
   (`negative_zero_hashes_as_positive_zero`), Task 10 (`a_nan_in_a_written_layer_names_stage_layer_and_cell`).
4. **An output folder that already holds files, some with an output's name.** Expected:
   - unrelated files stay through success, failure and cancel;
   - a file with an output's name makes the run fail with `RunError::OutputExists` before
     anything is written.

   Test: Task 41 (`existing_files_stay_and_name_clashes_are_refused`).
5. **A broken folder recipe with a built-in's id.** Expected: `describe` lists it as a problem
   with its file, the built-in of that id is not offered, and `run` of that id fails with the
   recipe's error. Test: Task 42 (`a_broken_override_hides_the_built_in`).

## Rulings made while planning

These go into the ledger's "Rulings (build)" and the PR, in this form.

- **Process**
  - L1. Ruling: the phase branch merges `gate/map-crates` and builds while the gate PR waits. The
    implementation PR opens only after the gate PR is on `main` and `main` is merged in — the
    crates must exist for the workspace test and the coverage floors; the PR diff then shows no
    protected path — cost if wrong: a merge if Jann changes the gate PR.
  - L24. Ruling: the contact sheet for Jann's review is not committed (G10). The PR gives the
    command and the local file, and attaches the PNG in a PR comment when the session can upload
    images — cost if wrong: Jann runs one command.
  - L25. Ruling: the build records `cargo tree -e features -i serde_json` (Task 2) and
    `-i num-traits` (Task 16, when `spade` joins) in the ledger. `num-traits/std`
    comes in through spade's own dependency, but our code never calls `num_traits::Float`, and
    spade's paths we use are exact arithmetic. The golden hashes on four platforms are the proof —
    cost if wrong: a vendored predicate.
- **The Stage contract**
  - L2. Ruling: stages implement a typed trait `TypedStage` with an associated `Params` type, and
    one blanket `impl<T: TypedStage> Stage for T` does the parsing and the downcasts — no stage
    writes a downcast, and the object-safe `Stage` keeps every capability of spec §5.3 — cost if
    wrong: none.
  - L3. Ruling: unknown parameter keys are found with `serde_ignored` around the TOML
    deserialiser. If `toml::Value` cannot serve as its deserialiser, every `Params` struct gets
    `#[serde(deny_unknown_fields)]` instead, and a registry test proves it — one central check
    cannot be forgotten by a new stage — cost if wrong: one attribute per struct.
  - L4. Ruling: non-raster values are boxed `WorldValue` trait objects with a type name, not
    variants of a central enum — P34's `room_graph` is then one new module — cost if wrong: none.
  - L5. Ruling: `climate` is a value holding the parameters and the current fields
    (precipitation, PET, AET, runoff). `climate_model` runs before any height exists, so it writes
    the parameters only; `climate_update` fills the fields. The package's `precipitation` layer is
    taken from those fields. `elements` writes the five `element_*` rasters under their package
    names (spec §5.4) — one name throughout — cost if wrong: renames inside the crate.
  - L6. Ruling: `check_every` is a parameter of `landscape_evolution`, default 10 — it is the
    stage that runs the processes — cost if wrong: none.
  - L27. Ruling: `every` is an option of every process table, handled by `landscape_evolution`
    (default 1). A process with `every = n` runs at step 0, at every multiple of n and on the last
    step. So `climate`'s fields always exist during erosion and match the final surface for
    `hydrology` and `ecology`; 0.3.0 skipped step 0 and let hydrology recompute — cost if wrong:
    one more climate run per evolution.
- **Running**
  - L7. Ruling: the memory estimate counts a non-raster value as 8 bytes per cell — roads, flow
    graphs and features stay well below that — cost if wrong: the estimate is conservative.
  - L10. Ruling: outputs are named `s<seed>-<role>.png`, `s<seed>-features.geojson` and
    `s<seed>.theosmap` (`docs/generators.md` §3 example). If a file with one of these names already
    exists, the run fails before writing — "files that were there before stay" (spec §5.6) —
    cost if wrong: none.
  - L11. Ruling: the library returns `RunManifest` and writes no `manifest.json`; the developer
    example writes one, and P09's adapter decides for the protocol — cost if wrong: one write.
  - L12. Ruling: a stage's random stream is
    `Pcg64::from_seed(sm.next ×4 as LE bytes)`, where `sm` is
    `SplitMix64::new(run_seed ^ fnv1a64("<stage id>#<occurrence>"))`. Per-step processes use the
    key `landscape_evolution#<occ>/<process id>#<occ>`, made once per run. The formula is part of
    `VERSION` (spec §5.7) — cost if wrong: a new version.
  - L22. Ruling: the format crate offers `write_cancellable(&Package, W, &dyn Fn() -> bool)`;
    `write` calls it with `|| false` — the runner checks cancel between package entries (spec
    §5.6) without the format crate depending on ours — cost if wrong: none.
- **Recipes and stages**
  - L8. Ruling: lake outlines use our own marching squares, without probing `contour` (spec
    ruling 18 allows it) — about 80 lines with known semantics — cost if wrong: none.
  - L9. Ruling: render options are `[render.<id>]`. §5.11's `[renders.inworld]` would clash with
    the `renders` array in TOML, and §5.5's example uses `[render.inworld]` — cost if wrong: none.
  - L13. Ruling: `tectonics` turns a basin's or rift's `depth_m` into a `sag_rate` over
    `sag_duration_yr` (a tectonics parameter, default 6.0e6 yr = 0.3.0's 240 × 25 kyr) — a source
    states the depth, as in 0.3.0, and the rate follows (spec ruling 5) — cost if wrong: one
    parameter.
  - L16. Ruling: a broken folder recipe whose `id` can be read hides the built-in of that id and
    is listed as a problem — the user meant to override it — cost if wrong: none.
  - L17. Ruling: folders are read in the order given, and a later folder's recipe wins over an
    earlier one's — the same rule as folder over built-in — cost if wrong: none.
  - L18. Ruling: a constraint in P08 recipes has both `min` and `max`. A value that the run does
    not report gives `<name>_ok = false` without `<name>`, as 0.3.0 did — cost if wrong: optional
    bounds later.
  - L20. Ruling: `pois.gates` is 0, 1 (only `gate-down`) or 2 (`gate-down` and `gate-up`), as
    0.3.0's `portals`. The parameters are renamed `chimes`, `min_chime_sep_km` and
    `chime_min_km_from_settlement` — decision 0004's names — cost if wrong: renames before P23.
  - L21. Ruling: `fixture-small` gets the placement constraints `features.ruin = [1, 1]` and
    `features.chime = [2, 2]` — "the converted recipes state them as constraints" (spec §5.5). 0.3.0
    had no constraints for it, so CE-28.5 asks nothing of its pass count — cost if wrong: none.
- **Outputs**
  - L14. Ruling: `rivers_pct` is 0.3.0's measure: cells with discharge above `river_q` outside
    lakes, over all cells, × 100 — spec §5.5 says "as 0.3.0 defined them" — cost if wrong: a
    parity metric.
  - L19. Ruling: `water` classes are 0 dry, 1 stream (`river_q` < Q ≤ `big_river_q`), 2 river
    (Q > `big_river_q`) and 3 lake. `flow_dir` is 0 for an outlet or terminal-lake bottom, else
    1–8 for N, NE, E, SE, S, SW, W, NW. Both are written in the layer's legend — cost if wrong: a
    legend change before P22.
  - L23. Ruling: the golden file is TOML: `generator`, then a `[[run]]` per golden run with
    `recipe`, `seed`, `features`, `package` and a `[[run.stage]]` per pipeline stage, in order,
    with `key` and a `layers` table mapping layer → SHA-256 — readable diffs, and the first
    difference in pipeline order is easy to name — cost if wrong: none.
  - L26. Ruling: commits use scope `generators` for both crates — CONTRIBUTING has no crate scope
    for them — cost if wrong: none.
- **Tests**
  - L15. Ruling: physics tests and the CE-28.4 (c) panic test are unit tests inside the crate.
    The other AC tests go through the public API — the processes and a test registry are
    internal — cost if wrong: none.
  - L28. Ruling: "runs in two processes give identical package bytes" (spec §5.14) is proven by
    `bless` recording the hashes in one process and the golden test comparing them in another.
    No test spawns a second process — the golden test already compares across processes, and
    across platforms — cost if wrong: one spawning test.

## Tasks at a glance

| # | Task | ACs |
|---|---|---|
| 1 | Gate PR `gate/map-crates` | – |
| 2 | Format: manifest types and canonical JSON | – |
| 3 | Format: layer data | – |
| 4 | Format: validation and the writer | CE-76.1 |
| 5 | Format: the reader | CE-76.1, 76.2, 76.3, 76.4, 76.5 |
| 6 | Maps: crate setup, `Grid`, `num`, `rng` | – |
| 7 | Maps: the world and its layers | – |
| 8 | Maps: the Stage contract and the registry | – |
| 9 | Maps: recipes, loading and validation | – |
| 10 | Maps: the runner | – |
| 11 | Maps: constraints and results | – |
| 12 | Maps: library API types | – |
| 13 | Kernels: FFT, spectral noise, warp | – |
| 14 | Kernels: filters and statistics | – |
| 15 | Kernels: distance, labels, morphology | – |
| 16 | Kernels: curves, Voronoi, marching squares | – |
| 17 | Kernels: shortest paths, MST, priority-flood | – |
| 18 | Stages: `grid`, `tectonics` | – |
| 19 | Stage: `geology` | – |
| 20 | Stage: `climate_model` | – |
| 21 | Stage: `landscape_evolution` frame and `uplift` | – |
| 22 | Processes: `route_steepest`, `resolve_sinks` | – |
| 23 | Process: `partition_flow` | – |
| 24 | Process: `fluvial` without deposition | – |
| 25 | Process: `fluvial` with deposition | – |
| 26 | Processes: `hillslope`, `slope_limit`, `climate_update` | – |
| 27 | Budget: one 384² run, the dev profile | – |
| 28 | Stage: `hydrology` — lakes, discharge, rasters | – |
| 29 | Stage: `hydrology` — river lines and lake outlines | – |
| 30 | Stages: `ecology`, `elements` | – |
| 31 | Features model and `settlements` | – |
| 32 | Stages: `pois`, `resources` | – |
| 33 | Stage: `roads` | – |
| 34 | Stages: `sites`, `travel` | – |
| 35 | Stage: `detuning` | CE-28.7 |
| 36 | Features output | CE-28.6 |
| 37 | Metrics module and summary | – |
| 38 | Package building | – |
| 39 | Renders: infrastructure and `base` | CE-28.6 |
| 40 | Renders: the others and the contact sheet | – |
| 41 | API: `run`, outputs, cancel | CE-28.2, CE-28.4 |
| 42 | Built-in recipes and `describe` | CE-28.1, CE-28.3, CE-28.7 |
| 43 | Developer example and resolution independence | – |
| 44 | Tuning and parity | CE-28.5 |
| 45 | Golden hashes and determinism | CE-77.1, CE-77.2 |
| 46 | Finish: docs, contact sheet, PR material | – |

Order rules (spec §10): the format crate and the Stage contract come first, and each process lands
with its physics test.

---

## Part A — the gate PR

### Task 1: Gate PR `gate/map-crates` (spec §5.16)

This task runs before the build, on its own branch and PR, which wait for Jann (protected paths).
The phase branch then merges it (L1).

**Files** (worktree `.worktrees/gate-map-crates`, branch `gate/map-crates` from `origin/main`):
- Create:
  - `crates/theos-map-format/{Cargo.toml, README.md, src/lib.rs}`;
  - `crates/theos-maps/{Cargo.toml, README.md, src/lib.rs}`.

  Each `Cargo.toml` is shaped like `crates/theos-studio/Cargo.toml`, with no dependencies. Each
  `lib.rs` has crate docs that end "This crate is empty until the build of phase P08".
- Modify:
  - `xtask/tests/workspace.rs`: `workspace_members_are_the_g4_crates`, add
    `theos-map-format@crates/theos-map-format` and `theos-maps@crates/theos-maps`;
  - `gate/thresholds.toml`: `theos-map-format = "library"`, `theos-maps = "library"`;
  - `gate/clippy.toml` and its copy `clippy.toml` (kept identical by `cargo xtask lints`):
    `disallowed-methods`;
  - `deny.toml` `[bans]`: `deny`;
  - `.github/workflows/platforms.yml`: the Intel macOS job.

- [ ] **Step 1: Add the two crates and run the workspace test.**
  Run `cargo nextest run -p xtask workspace`. Expected: FAIL in `workspace_members_are_the_g4_crates`,
  naming the two new members.
- [ ] **Step 2: Update `xtask/tests/workspace.rs` and `gate/thresholds.toml`.** Run the same test,
  then `cargo xtask coverage`. Expected: both pass. The coverage check names no unlisted crate.
- [ ] **Step 3: Add the float ban** to `gate/clippy.toml` and `clippy.toml`. That is 66 entries:
  `f32::` and `f64::` × `sin cos tan asin acos atan atan2 sinh cosh tanh asinh acosh atanh exp exp2
  exp_m1 ln log log2 log10 ln_1p powf powi hypot cbrt sin_cos mul_add algebraic_add algebraic_sub
  algebraic_mul algebraic_div algebraic_rem`. Each entry has the same reason:

  ```toml
  disallowed-methods = [
    { path = "f64::sin", reason = "not the same bits on every platform; use libm (G20)" },
    # … one line per method and type
  ]
  ```
- [ ] **Step 4: Probe the ban** in a scratch copy of the repo, outside the worktree. Add
  `pub fn probe(x: f64) -> f64 { x.sin() + f64::cos(x) + x.mul_add(2.0, 1.0) }` to
  `theos-maps/src/lib.rs` there and run `cargo clippy -p theos-maps -- -D warnings`. Expected:
  three `disallowed_methods` errors. Paste the output into the PR description, then delete the
  copy.
- [ ] **Step 5: Add the deflate ban** to `deny.toml`:

  ```toml
  # Only miniz_oxide may deflate map packages: another backend changes their bytes (P08 §4.3).
  deny = [
    { crate = "zlib-rs", reason = "map packages must deflate with miniz_oxide (G20)" },
    { crate = "libz-sys", reason = "map packages must deflate with miniz_oxide (G20)" },
    { crate = "libz-ng-sys", reason = "map packages must deflate with miniz_oxide (G20)" },
    { crate = "cloudflare-zlib-sys", reason = "map packages must deflate with miniz_oxide (G20)" },
  ]
  ```

  Use the entry syntax of the pinned cargo-deny version (`gate/tools.toml`). If it takes `name`
  instead of `crate`, use `name`.
- [ ] **Step 6: Probe the deflate ban** in a scratch copy. Add
  `flate2 = { version = "1.1", default-features = false, features = ["zlib-rs"] }` to
  `theos-map-format` there and run `cargo deny check bans`. Expected: an error naming `zlib-rs`.
  Paste the output into the PR description (spec ruling 17).
- [ ] **Step 7: Add the Intel macOS job** to `platforms.yml`: `test-macos-intel`, a copy of
  `test-macos` with `runs-on: macos-15-intel` and the same `if:` on the `macos` label. If GitHub no
  longer offers that runner, run on `macos-latest` instead: install the `x86_64-apple-darwin`
  target and run the tests with `--target x86_64-apple-darwin` (Rosetta). Validate the YAML with
  `cargo xtask lints` (it pins the actions).
- [ ] **Step 8: Run `cargo xtask gate`** on Windows. Expected: every check passes.
- [ ] **Step 9: Commit and open the PR.**
  - Commits, one per concern: `build(gate): add the empty map crates`,
    `ci(gate): ban the std float methods outside libm`,
    `ci(gate): ban deflate backends other than miniz_oxide` and
    `ci(gate): run the tests on Intel macOS`. Footers: `Phase: P08`, the Claude trailers.
  - Push and open the PR titled `ci(gate): map crates and the float ban before P08 (gate)`. Fill
    the template: both probe outputs and the `cargo xtask gate` table.
  - No auto-merge and no `approved` label.
- [ ] **Step 10: Merge the gate branch into the phase branch** (L1). In
  `.worktrees/P08-map-format-and-terrain-generator`, run `git fetch origin`, then
  `git merge origin/gate/map-crates`. Record L1 in the ledger.

---

## Part B — `theos-map-format`

### Task 2: Manifest types and canonical JSON

**Files:**
- Modify: `crates/theos-map-format/Cargo.toml` (`serde`, `serde_json`, `thiserror`), `src/lib.rs`
- Create: `src/manifest.rs`, `src/json.rs`, `src/error.rs`

**Interfaces — Produces:**

```rust
pub const FORMAT: &str = "theos-map";
pub const FORMAT_VERSION: &str = "1.0";
pub struct Manifest {                       // field names exactly as docs/map-package.md §3
    pub format: String, pub format_version: String, pub generator: String,
    pub requires: Vec<String>, pub kind: String, pub recipe_id: String,
    pub recipe: serde_json::Value, pub seed: u64,
    pub inputs: serde_json::Map<String, serde_json::Value>, pub frame: Frame,
    pub layers: Vec<LayerEntry>, pub features: String,
    pub results: serde_json::Map<String, serde_json::Value>, pub summary: serde_json::Value,
}
pub struct Frame { pub size_m: [f64; 2], pub cell_m: f64, pub grid: [u32; 2],
                   pub origin_m: Option<[f64; 2]>, pub parent: Option<Parent>,
                   pub levels: Option<Vec<Level>> }
pub struct Parent { pub package_sha256: String, pub feature: String }
pub struct Level { pub id: String, pub elevation_m: f64, pub name: Option<String> }
pub struct LayerEntry { pub name: String, pub file: String, pub dtype: DType, pub shape: [u32; 2],
                        pub sha256: String, pub description: String, pub units: Option<String>,
                        pub scale: Option<f64>, pub offset: Option<f64>,
                        pub legend: Option<BTreeMap<String, String>>, pub min: Option<f64>,
                        pub max: Option<f64>, pub level: Option<String> }
pub enum DType { F32, U8, U16 }             // serde: "f32", "u8", "u16"
pub mod json {
    pub fn canonicalise(value: &serde_json::Value) -> serde_json::Value;   // keys sorted, recursively
    pub fn to_canonical_vec(value: &serde_json::Value) -> Result<Vec<u8>, FormatError>; // pretty + final LF
}
pub enum FormatError { … }                  // grows in Tasks 3–5; thiserror
```

All derive `Debug, Clone, PartialEq, Serialize, Deserialize`. `Option` fields are skipped when
`None`. Unknown fields are ignored on reading: no `deny_unknown_fields` anywhere in this crate.

- [ ] **Step 1: Write the failing tests** (`#[cfg(test)]` in `json.rs` and `manifest.rs`):
  - `canonical_json_is_serde_pretty_with_sorted_keys`: for
    `json!({"b": 1, "a": {"d": [1, 2], "c": 0.5}})`,
    `to_canonical_vec(&v)? == format!("{}\n", serde_json::to_string_pretty(&canonicalise(&v))?).into_bytes()`.
    The text starts with `"{\n  \"a\": {\n    \"c\": 0.5,"`.
  - `numbers_print_in_the_shortest_round_trip_form`: `24000.0` gives `24000.0`, `62.5` gives
    `62.5`, `0.1` gives `0.1`, `1e-7` gives `1e-7`.
  - `a_manifest_without_options_writes_sorted_keys_and_requires`: a terrain manifest with `None`
    options has no `origin_m`, `parent` or `levels` key. It has `"requires": []`, and its
    top-level keys are in the order of `docs/map-package.md` §3's example.
  - `unknown_manifest_fields_are_ignored_on_reading`: a JSON with `"future": 1` deserialises.
- [ ] **Step 2: Run** `cargo nextest run -p theos-map-format`. Expected: FAIL (items missing).
- [ ] **Step 3: Implement.**
  - `canonicalise` rebuilds every object with its keys sorted: `Vec<(String, Value)>` sorted,
    then inserted. It does not rely on `serde_json::Map` being a `BTreeMap` (spec §5.7).
  - `to_canonical_vec` is `serde_json::to_writer_pretty` of the canonical value plus `b'\n'`.
- [ ] **Step 4: Run the tests.** Expected: PASS. Run `cargo tree -e features -i serde_json` and
  record the result in the ledger (L25). `preserve_order` must not appear; if it does, the
  canonicaliser still sorts.
- [ ] **Step 5: Commit** `feat(generators): map manifest types and canonical JSON`.

### Task 3: Layer data

**Files:** Create `crates/theos-map-format/src/layer.rs`; modify `Cargo.toml` (`sha2`) and `src/error.rs`.

**Interfaces — Produces:**

```rust
pub enum LayerData { F32(Vec<f32>), U8(Vec<u8>), U16(Vec<u16>) }
impl LayerData {
    pub fn dtype(&self) -> DType;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn to_bytes(&self) -> Result<Vec<u8>, FormatError>;   // LE; −0.0 → +0.0; NaN/inf → NonFinite{index}
    pub fn from_bytes(dtype: DType, bytes: &[u8], cells: usize) -> Result<Self, FormatError>; // LayerSize
    pub fn decoded_range(&self, scale: Option<f64>, offset: Option<f64>) -> Option<(f64, f64)>;
}
pub fn sha256_hex(bytes: &[u8]) -> String;      // lower-case hex
#[derive(PartialOrd, Ord, …)] pub struct LayerKey { pub level: Option<String>, pub name: String }
```

- [ ] **Step 1: Write the failing tests:**
  - `f32_bytes_are_little_endian_and_zero_is_positive`:
    `LayerData::F32(vec![1.0, -0.0]).to_bytes()? == [0, 0, 128, 63, 0, 0, 0, 0]`.
  - `u16_bytes_are_little_endian`: `[258]` gives `[2, 1]`.
  - `nan_and_infinity_are_refused_with_their_index`: `[0.0, f32::NAN]` gives `NonFinite { index: 1 }`.
  - `from_bytes_checks_the_size`: 7 bytes for 2 `f32` cells gives `LayerSize { expected: 8, found: 7 }`.
  - `sha256_of_abc`: `sha256_hex(b"abc") == "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"`.
  - `decoded_range_applies_scale_and_offset`: `U8([0, 255])` with scale `1/255` gives `(0.0, 1.0)`.
- [ ] **Step 2: Run** them. Expected: FAIL.
- [ ] **Step 3: Implement** with `f32::to_le_bytes` and `u16::from_le_bytes`, and
  `sha2::Sha256::digest`.
- [ ] **Step 4: Run** them. Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): map layer encoding and hashes`.

### Task 4: Validation and the writer

**Files:**
- Modify: `crates/theos-map-format/Cargo.toml`:
  - `zip` (no default features, `deflate-flate2`);
  - `flate2` (no default features, `rust_backend`);
  - `geojson` (no default features);
  - dev: `zip`.
- Create: `src/package.rs`, `src/write.rs`, `tests/format/main.rs`, `tests/format/zip_bytes.rs`
  (a small parser of local headers, central directory and EOCD, for tests only),
  `tests/format/writer.rs`, `tests/fixtures/package/{manifest.json, features.geojson, layers.toml}`.

**Interfaces — Produces:**

```rust
pub struct Package { pub manifest: Manifest, pub features: geojson::FeatureCollection,
                     pub layers: BTreeMap<LayerKey, LayerData> }
impl Package { pub fn validate(&self) -> Result<(), FormatError>; }
pub fn write<W: Write + Seek>(package: &Package, out: W) -> Result<W, FormatError>;
pub fn write_cancellable<W: Write + Seek>(package: &Package, out: W,
                                          cancelled: &dyn Fn() -> bool) -> Result<W, FormatError>;
```

**What the writer fills in:**
- **Manifest:** `format`, `format_version` and `features` (`"features.geojson"`); `requires`
  sorted and deduplicated.
- **Each entry:** `file` (`layers/<name>.<dtype>`, or `layers/<level>/<name>.<dtype>`), `dtype`,
  `shape` (the frame's grid), `sha256`, and `min`/`max` (decoded).
- **The caller** gives `name`, `description`, `units`, `scale`, `offset`, `legend` and `level`.

**Validation errors** (each a `FormatError` variant that names what it found):
- an entry without data, or data without an entry;
- a length other than rows × columns;
- a duplicate name within a level;
- non-finite data;
- with `frame.levels`, a layer or feature (`properties.level`) whose level is missing or not
  listed;
- without `frame.levels`, a layer or feature that carries a level.

**ZIP options per entry** (spec §5.13):

```rust
SimpleFileOptions::default()
    .compression_method(CompressionMethod::Deflated).compression_level(Some(6))
    .last_modified_time(DateTime::DEFAULT).system(System::Unix).unix_permissions(0o644)
    .large_file(len > u64::from(u32::MAX))
```

Entries are written in this order: `manifest.json`, `features.geojson`, then the layers in index
order. `write_cancellable` checks `cancelled()` before each entry and returns
`FormatError::Cancelled`.

- [ ] **Step 1: Write the fixture.** It is synthetic: grid `[3, 4]`, `cell_m` 10.
  - **Layers:** `height` (f32, `m`), `water` (u8, legend `{"0": "dry", "3": "lake"}`).
  - **Features:** a lake `Polygon` with `surface_m`, a river `LineString` of three `[x, y, z]`
    points with per-vertex `width_m` and `discharge_m3s`, and a `chime` `Point`.
  - **`layers.toml`:** `[height] values = [12 floats]` and `[water] values = [12 ints]`.
  - **Marking:** every file says it is synthetic (JSON in a `"note"` key that readers ignore).
- [ ] **Step 2: Write the failing tests** in `tests/format/writer.rs`:

  ```rust
  // AC: CE-76.1
  #[test]
  fn written_bytes_follow_the_container_rules() -> TestResult {
      let bytes = write(&fixture()?, Cursor::new(Vec::new()))?.into_inner();
      let zip = zip_bytes::parse(&bytes)?;
      assert_eq!(zip.names(), ["manifest.json", "features.geojson", "layers/height.f32", "layers/water.u8"]);
      for e in zip.entries() {
          assert_eq!((e.method, e.mod_time, e.mod_date), (8, 0, 0x0021)); // deflate, 1980-01-01 00:00
          assert_eq!((e.local_extra_len, e.central_extra_len, e.comment_len), (0, 0, 0));
          assert_eq!(e.version_made_by >> 8, 3);                         // Unix
          assert_eq!(e.external_attrs >> 16, 0o100_644);
          assert!(!e.name.ends_with('/'));
      }
      assert_eq!(zip.eocd_comment_len, 0);
      assert_eq!(zip.entry("manifest.json")?, json::to_canonical_vec(&serde_json::to_value(&written_manifest)?)?);
      Ok(())
  }
  ```

  Add `zip64_is_used_only_above_4_gib` (on `pub(crate) fn needs_zip64(len: u64) -> bool`: false
  at `u32::MAX`, true at `u32::MAX + 1`; spec §8), `two_writes_are_byte_identical`,
  `requires_is_written_sorted_and_deduplicated`
  (`["b", "a", "a"]` gives `["a", "b"]`), `write_cancellable_stops_between_entries` (the closure
  turns true after its first call, so the result is `Cancelled`), and one unit test per validation
  error in `package.rs`.
- [ ] **Step 3: Run** `cargo nextest run -p theos-map-format`. Expected: FAIL.
- [ ] **Step 4: Implement** `Package::validate`, `write` and `write_cancellable`. Features are
  written with `json::to_canonical_vec(&serde_json::to_value(&features)?)`.
- [ ] **Step 5: Run.** Expected: PASS. Then write the fixture's package SHA-256 to
  `tests/fixtures/package/package.sha256` (one line, lower-case hex). Task 5's round trip checks
  it.
- [ ] **Step 6: Commit** `feat(generators): write map packages with fixed ZIP headers`
  (`Refs: CE-76.1`).

### Task 5: The reader

**Files:** Create `crates/theos-map-format/src/read.rs`, `tests/format/reader.rs`,
`tests/format/levels.rs` and `tests/format/deps.rs`; modify `Cargo.toml` (dev: `cargo_metadata`).

**Interfaces — Produces:**

```rust
pub fn read<R: Read + Seek>(input: R) -> Result<Package, FormatError>;
pub const IMPLEMENTED_CAPABILITIES: &[&str] = &[];   // none in format 1.0
```

**Reading rules** (spec §5.13):
- **Refusals:**
  - `format` other than `theos-map`: `NotAPackage`;
  - a major other than 1: `UnknownMajor { found, supported: "1.x" }`, with a message such as
    "format_version 2.0 is not supported; this reader reads 1.x";
  - each `requires` name outside `IMPLEMENTED_CAPABILITIES`:
    `UnsupportedCapability { capability }`, with the message "the package requires `slabs`,
    which this reader does not implement";
  - each listed layer's size and SHA-256 are checked: `LayerSize` and `LayerHash { name }`;
  - the levels rules are the writer's.
- **Ignored:**
  - entries not in the index;
  - layer entries whose `dtype` is unknown (`DType` reads unknown strings as an internal
    `Unknown`, and such layers are skipped, not stored);
  - unknown manifest fields;
  - unknown feature kinds, which are carried unchanged.

- [ ] **Step 1: Write the failing tests.** A helper `assemble(&[(&str, Vec<u8>)]) -> Vec<u8>`
  builds packages with the `zip` crate, bypassing the writer:
  - `// AC: CE-76.1` `fixture_package_round_trips`: write the fixture, and the SHA-256 of the bytes
    equals `package.sha256`. Read it back: the manifest equals the written manifest, the features
    and the layers are equal.
  - `// AC: CE-76.2` `an_unknown_major_is_refused_naming_both_versions`: `"format_version": "2.0"`
    gives an error whose text contains `2.0` and `1.`.
  - `// AC: CE-76.2` `unknown_fields_entries_layers_and_kinds_are_ignored`: the package has a
    manifest field `"future": {"x": 1}`, an entry `extras/notes.txt`, a listed layer `strata`
    (u8, a name the format does not define), a listed layer with `"dtype": "f64"` and a feature of
    kind `monolith`. It reads: `strata` is present, the f64 layer is absent and the monolith is
    present.
  - `// AC: CE-76.3` `a_two_level_package_round_trips`: levels `ground` (0 m) and `gallery`
    (6 m); `walkable` on both levels (`layers/ground/walkable.u8`, `layers/gallery/walkable.u8`);
    one room feature per level. The round trip is equal.
  - `// AC: CE-76.3` `levels_rules_reject_missing_and_unknown_levels`: four cases (a layer without
    a level, a layer with an unknown level, a feature without a level, a feature with an unknown
    level). Each is rejected by `write`, and by `read` on a package from `assemble`.
  - `// AC: CE-76.5` `a_required_capability_the_reader_lacks_is_refused`: `"requires": ["slabs"]`
    gives an error whose text contains `slabs`. `"requires": []` reads.
  - `// AC: CE-76.4` `the_format_crate_depends_on_no_workspace_member`: using `cargo_metadata`,
    no normal, build or dev dependency of `theos-map-format` is a workspace member.
  - `a_wrong_layer_hash_is_refused` and `a_short_layer_is_refused`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement `read`.**
- [ ] **Step 4: Run.** Expected: PASS. Run `cargo deny --locked check`. Expected: pass.
  `cargo tree -i miniz_oxide` shows it under `flate2`.
- [ ] **Step 5: Commit** `feat(generators): read map packages leniently with version and capability checks`
  (`Refs: CE-76.1, CE-76.2, CE-76.3, CE-76.4, CE-76.5`).

---

## Part C — `theos-maps` core

### Task 6: Crate setup, `Grid`, `kernels::num`, `rng`

**Files:**
- Modify: root `Cargo.toml`:
  - add `[profile.dev.package.theos-maps] opt-level = 2`;
  - the comment above `[profile.dev.package."*"]` gains: "except `theos-maps`, whose physics,
    parity and golden runs would take the better part of an hour unoptimised (P08 §5.16)".
- Modify: `crates/theos-maps/Cargo.toml` (`theos-map-format = { path = "../theos-map-format" }`,
  `libm`, `rand_pcg`, `rand_core`, `thiserror`, `tracing`, `serde`, `serde_json`, `toml`, `sha2`).
- Create: `src/grid.rs`, `src/kernels/mod.rs`, `src/kernels/num.rs`, `src/rng.rs`.

**Interfaces — Produces:**

```rust
pub struct Grid<T> { /* rows, cols, data: Vec<T> */ }
impl<T: Copy> Grid<T> {
    pub fn filled(rows: usize, cols: usize, value: T) -> Self;
    pub fn from_vec(rows: usize, cols: usize, data: Vec<T>) -> Result<Self, GridError>;
    pub fn rows(&self) -> usize; pub fn cols(&self) -> usize; pub fn len(&self) -> usize;
    pub fn index(&self, r: usize, c: usize) -> usize; pub fn cell(&self, i: usize) -> (usize, usize);
    pub fn get(&self, r: usize, c: usize) -> T; pub fn set(&mut self, r: usize, c: usize, v: T);
    pub fn as_slice(&self) -> &[T]; pub fn as_mut_slice(&mut self) -> &mut [T];
    pub fn map<U: Copy>(&self, f: impl Fn(T) -> U) -> Grid<U>;
    pub fn neighbour(&self, i: usize, d: (isize, isize)) -> Option<usize>;
}
/// (d_row, d_col) in order N, NE, E, SE, S, SW, W, NW; flow_dir codes 1–8 follow this order (L19).
pub const D8: [(isize, isize); 8];
/// D8, then the knight moves NNE (−2,+1), ENE (−1,+2), ESE (+1,+2), SSE (+2,+1),
/// SSW (+2,−1), WSW (+1,−2), WNW (−1,−2), NNW (−2,−1).
pub const D16: [(isize, isize); 16];
pub mod num {
    pub fn max(a: f64, b: f64) -> f64;            // +0.0 on ±0 ties
    pub fn min(a: f64, b: f64) -> f64;
    pub fn key(x: f64) -> f64;                    // x + 0.0, for sort and queue keys
    pub fn canonical(x: f64) -> f64;              // −0.0 → +0.0
    pub fn to_f64(i: usize) -> f64;               // the cast helpers (only expects here)
    pub fn to_index(x: f64, len: usize) -> usize; // floor, clamped to 0..len
    pub fn to_f32(x: f64) -> f32;
    pub fn to_u8_unit(x: f64) -> u8;              // round(clamp(x, 0, 1) × 255)
    pub fn smoothstep(e0: f64, e1: f64, x: f64) -> f64; // works for e0 > e1 as 0.3.0's
}
pub struct StageRng { /* Pcg64 */ }
impl StageRng {
    pub fn for_key(run_seed: u64, key: &str) -> Self;   // L12
    pub fn next_u64(&mut self) -> u64;
    pub fn uniform(&mut self) -> f64;                   // (next_u64 >> 11) × 2⁻⁵³, in [0, 1)
    pub fn below(&mut self, n: u64) -> u64;             // Lemire, n > 0
    pub fn range(&mut self, lo: f64, hi: f64) -> f64;
    pub fn normal(&mut self) -> f64;                    // Box–Muller with libm; one draw per call
}
pub fn fnv1a64(bytes: &[u8]) -> u64;
pub struct SplitMix64(u64); // new(seed), next() -> u64
```

- [ ] **Step 1: Write the failing tests:**
  - `max_and_min_return_positive_zero_on_ties` (Review Focus 3): for `(a, b)` in
    `[(-0.0, 0.0), (0.0, -0.0), (-0.0, -0.0)]`, `max(a, b).to_bits() == 0` and
    `min(a, b).to_bits() == 0`. Also `key(-0.0).to_bits() == 0`.
  - `smoothstep_matches_0_3_0`: `smoothstep(0.0, 1.0, 0.5) == 0.5`;
    `smoothstep(-1.5, -5.0, -6.0) == 1.0`.
  - `to_index_clamps`: `to_index(-0.5, 10) == 0`, `to_index(9.7, 10) == 9`,
    `to_index(3.2, 10) == 3`.
  - `fnv1a64_matches_reference_vectors`: `""` gives `0xcbf2_9ce4_8422_2325`; `"a"` gives
    `0xaf63_dc4c_8601_ec8c`.
  - `splitmix64_matches_reference`: `SplitMix64::new(0).next() == 0xe220_a839_7b1d_cdaf`.
  - `stage_streams_do_not_depend_on_other_streams`: four values of
    `for_key(1, "tectonics#0")` are equal whether or not `for_key(1, "grid#0")` was drawn first,
    and they differ from `for_key(1, "tectonics#1")`'s and `for_key(2, "tectonics#0")`'s.
  - `below_stays_below`: for `n` in `[1, 7, 1 << 40]`, 1000 draws are all `< n`.
  - `normal_has_unit_spread`: 20 000 draws have a mean within ±0.03 and a variance within
    1 ± 0.05.
  - `grid_index_and_cell_round_trip`, `from_vec_checks_the_length`, and
    `knight_moves_follow_the_d16_table`.
- [ ] **Step 2: Run** `cargo nextest run -p theos-maps`. Expected: FAIL.
- [ ] **Step 3: Implement.** `for_key` follows L12:
  `SplitMix64::new(run_seed ^ fnv1a64(key.as_bytes()))`; four `next()` give the 32 seed bytes
  (little-endian, in order) for `rand_pcg::Pcg64::from_seed`. All of it uses `wrapping_*`
  arithmetic.
- [ ] **Step 4: Run.** Expected: PASS. Run `cargo deny --locked check`.
- [ ] **Step 5: Commit** `feat(generators): grid, float helpers and per-stage random streams`.

### Task 7: The world and its layers

**Files:** Create `crates/theos-maps/src/world/{mod.rs, layer.rs, value.rs, hash.rs}`.

**Interfaces:**
- **Consumes:** `Grid`, `num`, `theos_map_format::{json, sha256_hex}`.
- **Produces:**

```rust
pub struct MapFrame { pub rows: usize, pub cols: usize, pub cell_m: f64 }
impl MapFrame {
    pub fn size_m(&self) -> [f64; 2];                   // [cols·cell, rows·cell]
    pub fn cells(&self) -> usize;
    pub fn cell_area_m2(&self) -> f64;
    pub fn centre_m(&self, r: usize, c: usize) -> [f64; 2];   // ((c+0.5)·cell, (r+0.5)·cell)
    pub fn cell_at_m(&self, x: f64, y: f64) -> (usize, usize);
    pub fn cell_at_fraction(&self, fx: f64, fy: f64) -> (usize, usize);  // G19: [x, y]
}
pub enum LayerKind { F64, U8, Mask, U32, Value(&'static str) }
pub enum Layer { F64(Grid<f64>), U8(Grid<u8>), Mask(Grid<bool>), U32(Grid<u32>),
                 Value(Box<dyn WorldValue>) }
pub trait WorldValue: Any + Debug + Send + Sync {
    fn type_name(&self) -> &'static str;
    fn canonical_json(&self) -> serde_json::Value;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
pub trait ValueType: WorldValue + Sized { const NAME: &'static str; }
pub struct World { /* frame, layers: BTreeMap<String, Layer> */ }
impl World {
    pub fn new(frame: MapFrame) -> Self;
    pub fn frame(&self) -> &MapFrame;
    pub fn get(&self, name: &str) -> Option<&Layer>;
    pub fn insert(&mut self, name: &str, layer: Layer) -> Result<(), WorldError>; // raster shape = frame
    pub fn remove(&mut self, name: &str) -> Option<Layer>;
    pub fn f64(&self, name: &str) -> Option<&Grid<f64>>;   // likewise u8, mask, u32
    pub fn value<T: ValueType>(&self, name: &str) -> Option<&T>;
    pub fn names(&self) -> impl Iterator<Item = &str>;
}
impl Layer { pub fn kind(&self) -> LayerKind; }
pub fn bytes_per_cell(kind: LayerKind) -> u64;      // F64 8, U8 1, Mask 1, U32 4, Value 8 (L7)
pub fn canonical_bytes(layer: &Layer) -> Result<Vec<u8>, NonFinite>;  // NonFinite { cell }
pub fn layer_sha256(layer: &Layer) -> Result<String, NonFinite>;
```

Canonical bytes, per kind:
- F64: little-endian of `num::canonical(x)`;
- U8 raw;
- Mask 0/1;
- U32 little-endian;
- Value: the canonical JSON bytes of `canonical_json()`.

- [ ] **Step 1: Write the failing tests:**
  - `negative_zero_hashes_as_positive_zero` (Review Focus 3): two grids `[-0.0, 1.0]` and
    `[0.0, 1.0]` have the same `layer_sha256`.
  - `a_non_finite_cell_is_reported`: `[1.0, f64::INFINITY]` gives `NonFinite { cell: 1 }`.
  - `values_hash_by_their_canonical_json`: a test value whose JSON keys are written out of order
    hashes like the sorted form.
  - `insert_checks_the_raster_shape` and `typed_access_returns_none_for_another_kind`.
  - `frame_geometry_follows_the_format`: for 4 × 8 cells of 10 m, `size_m` is `[80, 40]`;
    `centre_m(0, 0)` is `[5, 5]`; `cell_at_fraction(0.999, 0.0)` is `(0, 7)`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): the world of named layers`.

### Task 8: The Stage contract and the registry

**Files:**
- Modify: `Cargo.toml` (`serde_ignored`).
- Create:
  - `crates/theos-maps/src/stage/{mod.rs, typed.rs, context.rs, report.rs, error.rs, registry.rs}`;
  - `src/progress.rs` (`Progress`, `CancelToken`);
  - `src/stages/mod.rs` (`pub fn register(r: &mut Registry)`, empty for now).

**Interfaces — Produces** (spec §5.3; the capabilities are binding, these signatures are L2's):

```rust
pub enum MapKind { Terrain }                          // serde "terrain"
pub struct LayerSpec { pub name: &'static str, pub kind: LayerKind, pub optional: bool }
impl LayerSpec { pub const fn f64(n: &'static str) -> Self; /* u8, mask, u32, value(n, type) */
                 pub const fn optional(self) -> Self; }
pub trait StageParams: Any + Debug + Send + Sync { fn as_any(&self) -> &dyn Any; }
pub trait Stage: Send + Sync {
    fn id(&self) -> &'static str;
    fn summary(&self) -> &'static str;
    fn kinds(&self) -> &'static [MapKind];
    fn params(&self, table: &toml::Table) -> Result<Box<dyn StageParams>, ParamError>;
    fn reads(&self, p: &dyn StageParams) -> Vec<LayerSpec>;
    fn writes(&self, p: &dyn StageParams) -> Vec<LayerSpec>;
    fn metric_names(&self, p: &dyn StageParams) -> Vec<&'static str>;
    /// Checks the order of nested steps against the layers available before this stage.
    fn check_order(&self, p: &dyn StageParams, available: &BTreeSet<&str>) -> Result<(), ParamError>;
    fn run(&self, ctx: &mut StageContext<'_>, p: &dyn StageParams) -> Result<StageReport, StageError>;
    fn checks(&self, w: &World, p: &dyn StageParams, r: &StageReport) -> Vec<Finding>;
    fn metrics(&self, w: &World, p: &dyn StageParams, r: &StageReport) -> Metrics;
}
pub trait TypedStage: Send + Sync + 'static {
    type Params: DeserializeOwned + Debug + Send + Sync + 'static;
    const ID: &'static str; const SUMMARY: &'static str; const KINDS: &'static [MapKind];
    fn reads(&self, p: &Self::Params) -> Vec<LayerSpec>;
    fn writes(&self, p: &Self::Params) -> Vec<LayerSpec>;
    fn metric_names(&self, _p: &Self::Params) -> Vec<&'static str> { Vec::new() }
    fn validate(&self, _p: &Self::Params) -> Result<(), String> { Ok(()) }
    fn check_order(&self, _p: &Self::Params, _a: &BTreeSet<&str>) -> Result<(), ParamError> { Ok(()) }
    fn run(&self, ctx: &mut StageContext<'_>, p: &Self::Params) -> Result<StageReport, StageError>;
    fn checks(&self, _w: &World, _p: &Self::Params, _r: &StageReport) -> Vec<Finding> { Vec::new() }
    fn metrics(&self, _w: &World, _p: &Self::Params, _r: &StageReport) -> Metrics { Metrics::default() }
}
impl<T: TypedStage> Stage for T { /* strips `id`, serde_ignored (L3), validate, downcasts */ }
pub struct StepInfo { pub step: u32, pub steps: u32, pub dt_yr: f64 }
pub struct StageContext<'w> { /* world, stage key, declared reads/writes, written set, rng,
                                 run seed, cancel, progress, step */ }
impl<'w> StageContext<'w> {
    pub fn frame(&self) -> &MapFrame;
    pub fn f64(&self, name: &str) -> Result<&Grid<f64>, StageError>;   // also u8, mask, u32
    pub fn f64_opt(&self, name: &str) -> Result<Option<&Grid<f64>>, StageError>;
    pub fn value<T: ValueType>(&self, name: &str) -> Result<&T, StageError>;
    pub fn value_opt<T: ValueType>(&self, name: &str) -> Result<Option<&T>, StageError>;
    pub fn take_f64(&mut self, name: &str) -> Result<Grid<f64>, StageError>; // read+write declared
    pub fn take_value<T: ValueType>(&mut self, name: &str) -> Result<Box<T>, StageError>;
    pub fn write(&mut self, name: &str, layer: Layer) -> Result<(), StageError>;
    pub fn rng(&mut self) -> &mut StageRng;
    pub fn sub_rng(&self, purpose: &str) -> StageRng;      // for_key(seed, "<key>/<purpose>")
    pub fn check_cancel(&self) -> Result<(), StageError>; // Err(Cancelled)
    pub fn is_cancelled(&self) -> bool;
    pub fn progress(&mut self, done: u32, total: u32);
    pub fn step(&self) -> Option<StepInfo>;
    pub fn child(&mut self, process: &dyn Stage, p: &dyn StageParams, key: &str,
                 step: StepInfo) -> StageContext<'_>;
    pub fn written(&self) -> &BTreeSet<String>;
    pub fn finish(&self) -> Result<(), StageError>;        // every non-optional write present
}
pub struct StageReport { pub values: BTreeMap<String, f64>, pub findings: Vec<Finding> }
pub struct Finding { pub severity: Severity, pub message: String, pub layer: Option<String> }
pub enum Severity { Warning, Error }
pub struct Metrics(pub BTreeMap<String, f64>);
pub enum StageError { Undeclared { stage, layer, access }, Missing { stage, layer },
                      WrongKind { stage, layer, expected, found }, NotWritten { stage, layer },
                      Cancelled, Failed { stage, message } }
pub struct ParamError { pub stage: String, pub parameter: Option<String>, pub message: String }
pub struct Registry { /* Vec<Box<dyn Stage>> */ }
impl Registry { pub fn builtin() -> Self; pub fn empty() -> Self;
                pub fn add(&mut self, s: Box<dyn Stage>); pub fn get(&self, id: &str) -> Option<&dyn Stage>;
                pub fn ids(&self) -> impl Iterator<Item = &'static str>; }
#[derive(Clone, Default)] pub struct CancelToken(Arc<AtomicBool>);   // cancel(), is_cancelled()
#[derive(Serialize, …)] pub struct Progress { pub stage: String, pub done: u32, pub total: u32 }
```

A stage may read a layer it also declares as a write (in-place update through `take_*` and
`write`).

- [ ] **Step 1: Write the failing tests** (test stages defined in the test module):
  - `params_reject_unknown_keys_naming_them`: `{id = "t", bogus = 1}` gives a `ParamError` whose
    text has `bogus` and `t`.
  - `params_fill_defaults_and_run_validate`.
  - `reading_an_undeclared_layer_names_stage_and_layer`.
  - `writing_the_wrong_kind_is_an_error`.
  - `finish_names_a_declared_write_that_was_not_written`.
  - `optional_reads_return_none_when_absent`.
  - `a_child_context_sees_only_its_process_layers`.
  - `sub_streams_are_named_and_stable`.
  - `the_builtin_registry_has_unique_snake_case_ids`: every id matches `^[a-z][a-z0-9_]*$` and is
    unique (trivially true now; it grows).
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
  - **Parameters (L3):** `serde_ignored::deserialize(toml::Value::Table(t), |p| unknown.push(p.to_string()))`.
    If `toml::Value` cannot serve as that deserialiser on toml 1.1, switch to
    `#[serde(deny_unknown_fields)]` and record it in the ledger.
  - **Errors:** a `Display` that names the stage and the layer, or the parameter.
- [ ] **Step 4: Run.** Expected: PASS. Run `cargo deny --locked check`.
- [ ] **Step 5: Commit** `feat(generators): the Stage contract, its context and the registry`.

### Task 9: Recipes: schema, loading, validation

**Files:** Create `crates/theos-maps/src/recipe/{mod.rs, schema.rs, load.rs, plan.rs, builtin.rs}`.
`builtin.rs` holds `pub(crate) const BUILTIN: &[(&str, &str)] = &[];` until Task 42.

**Interfaces — Produces:**

```rust
#[serde(deny_unknown_fields)]
pub struct Recipe { pub id: String, pub title: String, pub kind: MapKind, pub grid: [usize; 2],
                    pub size_km: [f64; 2], #[serde(default)] pub renders: Vec<String>,
                    #[serde(default)] pub inputs: toml::Table,
                    #[serde(default, rename = "stage")] pub stages: Vec<toml::Table>,
                    #[serde(default)] pub constraints: BTreeMap<String, [f64; 2]>,
                    #[serde(default)] pub render: BTreeMap<String, toml::Table> }
pub struct LoadedRecipe { pub recipe: Recipe, pub raw: toml::Table, pub origin: RecipeOrigin }
pub enum RecipeOrigin { Builtin, File(PathBuf) }
pub struct Catalog { pub recipes: BTreeMap<String, LoadedRecipe>, pub problems: Vec<RecipeProblem> }
pub struct RecipeProblem { pub file: PathBuf, pub id: Option<String>, pub message: String }
pub fn parse(text: &str, origin: RecipeOrigin) -> Result<LoadedRecipe, RecipeError>;
pub fn catalog(dirs: &[PathBuf]) -> Result<Catalog, RecipeError>;      // only I/O fails
pub(crate) fn catalog_with(builtins: &[(&str, &str)], dirs: &[PathBuf]) -> Result<Catalog, RecipeError>;
pub struct Plan { pub recipe_id: String, pub kind: MapKind, pub frame: MapFrame,
                  pub stages: Vec<PlannedStage>, pub constraints: BTreeMap<String, [f64; 2]>,
                  pub renders: Vec<String>, pub render_options: BTreeMap<String, toml::Table> }
pub struct PlannedStage { pub id: &'static str, pub key: String,      // "<id>#<occurrence>"
                          pub params: Box<dyn StageParams>, pub reads: Vec<LayerSpec>,
                          pub writes: Vec<LayerSpec>, pub metric_names: Vec<&'static str> }
pub fn plan(recipe: &LoadedRecipe, registry: &Registry,
            inputs: &serde_json::Map<String, serde_json::Value>) -> Result<Plan, RecipeError>;
pub enum RecipeError { Parse { origin: String, message: String },
                       Invalid { recipe: String, stage: Option<String>, layer: Option<String>,
                                 parameter: Option<String>, message: String },
                       UnknownRecipe(String), Io { path: PathBuf, source: std::io::Error } }
pub const EXTRA_RENDERS: &[&str] = &["omega"];
```

**Catalog rules:**
- the built-ins come first; then each folder in order, `*.toml` files directly inside, in sorted
  file-name order;
- a later recipe with the same id replaces an earlier one (CE-28.1, L17);
- a file that fails to parse is a `RecipeProblem`, and other files still load;
- if the broken file's raw TOML has a string `id`, the recipe of that id is removed (L16).

**`plan` checks, in this order** (spec §5.5). Each failure is `Invalid` naming what it can:
1. `grid` ≥ `[2, 2]`, positive `size_km`, and square cells (`size_km[0]/cols` equals
   `size_km[1]/rows` within 1e-9 relative). `cell_m = size_km[0] · 1000 / cols`.
2. Every name in `renders` is in `EXTRA_RENDERS`, and every `[render.<id>]` names a standard or
   extra render.
3. Inputs: any key of `[inputs]` or of the request's `inputs` gives `"unknown recipe input `<k>`"`
   with `parameter: Some(k)` (terrain recipes take none in P08).
4. Per stage: known id ("unknown stage"), its kind, its params, then
   `check_order(&available)`, then each non-optional read in `available` (otherwise
   `stage: Some(id), layer: Some(name)`, message "stage `<id>` reads `<layer>`, which no earlier
   stage writes"), then its writes added to `available`.
5. Constraint paths: Task 11.

- [ ] **Step 1: Write the failing tests** (in-module, with a test registry of stages `a` (writes
  `x`), `b` (reads `x`, writes `y`) and `c` (reads optional `z`)):
  - `a_stage_before_its_input_fails_naming_stage_and_layer`;
  - `an_unknown_stage_fails_naming_it`;
  - `an_unknown_parameter_fails_naming_it`;
  - `a_recipe_input_is_unknown` and `a_request_input_is_unknown`;
  - `an_optional_read_may_be_missing`;
  - `non_square_cells_are_refused`;
  - `an_unknown_extra_render_is_refused`;
  - catalog: `folder_recipes_win_over_builtins_and_later_folders_win`,
    `a_broken_file_is_a_problem_not_an_error`, `a_broken_override_hides_the_built_in` (L16),
    `other_files_and_subfolders_are_ignored`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): recipes, folders and validation before a run`.

### Task 10: The runner

**Files:**
- Create: `crates/theos-maps/src/runner/{mod.rs, memory.rs}`, `src/error.rs` (`RunError`),
  `src/golden.rs` (only `GoldenRun` for now).
- Modify: `src/lib.rs`.

**Interfaces — Produces:**

```rust
pub struct RunOptions { pub memory_budget: Option<u64> }        // serde, Default
pub const DEFAULT_MEMORY_BUDGET: u64 = 3 * 1024 * 1024 * 1024;
pub struct MemoryEstimate { pub bytes: u64, pub largest: Vec<(String, u64)> } // top 3 by size
pub fn estimate(plan: &Plan) -> MemoryEstimate;   // distinct layer names of all reads/writes, × 3 / 2
pub struct GoldenRun { pub recipe: String, pub seed: u64, pub features: String,
                       pub package: String, #[serde(rename = "stage")] pub stages: Vec<GoldenStage> }
pub struct GoldenStage { pub key: String, pub layers: BTreeMap<String, String> }   // L23
pub(crate) struct Execution { pub world: World, pub stage_metrics: Vec<(String, Metrics)>,
                              pub findings: Vec<StageFinding>, pub reports: BTreeMap<String, StageReport>,
                              pub golden: GoldenRun, pub timing_s: BTreeMap<String, f64> }
pub struct StageFinding { pub stage: String, pub message: String }   // warnings, serde
pub(crate) fn execute(plan: &Plan, seed: u64, registry: &Registry, options: &RunOptions,
                      progress: &mut dyn FnMut(&Progress), cancel: &CancelToken)
                      -> Result<Execution, RunError>;
pub enum RunError {
    Recipe(RecipeError), OverBudget { needed: u64, budget: u64, largest: String }, Cancelled,
    StageFailed { stage: String, source: StageError }, StagePanicked { stage: String, message: String },
    FindingFailed { stage: String, message: String },
    NonFinite { stage: String, layer: String, cell: usize },
    OutputExists { path: PathBuf }, Io { path: PathBuf, source: std::io::Error },
    Format(theos_map_format::FormatError), Render { render: String, message: String },
}
```

**Per stage** (spec §5.6, in this order):
1. Emit `Progress { stage: id, done: index, total: len }`.
2. Check cancel (`RunError::Cancelled`).
3. Build the context with `StageRng::for_key(seed, key)`.
4. Run the stage under `std::panic::catch_unwind(AssertUnwindSafe(…))`. The panic payload,
   `&str` or `String`, becomes `StagePanicked`. `StageError::Cancelled` becomes `Cancelled`.
5. Call `ctx.finish()`.
6. Run `checks`. An `Error` becomes `FindingFailed`; warnings go into `findings`.
7. Record the metrics under the stage's id (`<id>#<n>` for later occurrences).
8. Hash every written layer into `golden.stages`. A `NonFinite` becomes `RunError::NonFinite`.
9. Store the wall time in `timing_s`.

Before any stage, `estimate(plan)` against `options.memory_budget` (or the default) may fail with
`OverBudget`.

- [ ] **Step 1: Write the failing tests** (in-module, test registry):
  - `progress_reports_every_stage_in_order`;
  - `cancel_between_stages_returns_cancelled`;
  - `a_panicking_stage_fails_the_run_with_its_message`: the stage panics with `"boom at cell 7"`,
    giving `StagePanicked { stage: "boom", message }` with the text in `message`;
  - `an_error_finding_fails_naming_the_stage` and `warnings_go_into_findings`;
  - `a_nan_in_a_written_layer_names_stage_layer_and_cell` (Review Focus 3);
  - `the_memory_budget_refuses_before_any_stage_runs`: with a budget of 1 KiB and a 64² plan, the
    result is `OverBudget`, the largest layers are named, and a counter stage never ran;
  - `estimate_counts_each_layer_once`: two stages write `x`, which counts once;
    `64·64·8·2·1.5` for two F64 layers;
  - `an_estimate_exactly_at_the_budget_runs`: a budget equal to the estimate runs, one byte less
    refuses (spec §8);
  - `layer_hashes_are_recorded_per_stage_in_pipeline_order`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): the runner with checks, hashes, budget, cancel and panics`.

### Task 11: Constraints, results and the known summary paths

**Files:** Create `crates/theos-maps/src/constraints.rs`, `src/metrics.rs` (the path list only)
and `src/features/{mod.rs, types.rs}`; modify `src/recipe/plan.rs` (check 5).

**Interfaces — Produces:**

```rust
pub fn result_key(path: &str) -> String;   // lower case; every char outside [a-z0-9] → '_'
pub fn evaluate(constraints: &BTreeMap<String, [f64; 2]>, summary: &serde_json::Value)
    -> serde_json::Map<String, serde_json::Value>;   // <key>, <key>_min, <key>_max, <key>_ok, pass
pub const PATHS: &[&str] = &["elevation_m.relief", "lake_pct", "rivers_pct",
    "vegetation_cover_pct", "precip_mm_yr.median", "travel.chime_to_chime_min"];  // metrics.rs
pub const TYPES: &[(&str, &str)];   // (kind, type) for terrain: settlement city town village hamlet;
    // poi ruin chime gate-up gate-down; site mill mine quarry lumber-camp shrine watchtower inn
    // cave camp bridge ford; resource iron-ore copper-ore silver-ore stone clay timber peat-reeds
    // salt resonance-crystal herbs; road trail track road main; water river lake
pub(crate) fn is_known_path(path: &str, stages: &[PlannedStage]) -> bool;
```

**Known paths:**
- every entry of `PATHS`. `travel.chime_to_chime_min` counts only when a `travel` stage is
  planned;
- `features.<type>` for a type in `TYPES`;
- `stages.<id>.<metric>` for a planned stage whose `metric_names` holds `<metric>`.

The summary is read by walking the dotted path. A value that is absent or not a number gives
`<key>_ok = false`, no `<key>`, and the bounds (L18).

- [ ] **Step 1: Write the failing tests:**
  - `result_keys_follow_the_protocol`: `elevation_m.relief` gives `elevation_m_relief`;
    `features.gate-up` gives `features_gate_up`; `Travel.X` gives `travel_x`.
  - `evaluate_reports_value_bounds_ok_and_pass`.
  - `a_missing_value_fails_without_a_value_key`.
  - `no_constraints_pass`.
  - `an_unknown_constraint_path_fails_naming_it`: `plan` gives `Invalid` with the path in
    `parameter`.
  - `the_chime_walk_needs_a_travel_stage`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): constraint results with the protocol's keys`.

### Task 12: Library API types

**Files:** Create `crates/theos-maps/src/api.rs`; modify `src/lib.rs` (re-exports, crate docs that
tell engine callers not to set flush-to-zero, spec §5.7).

**Interfaces — Produces** (spec §5.12; `run` and `describe` are filled in Tasks 41–42):

```rust
pub const GENERATOR: &str = "theos-maps";
pub const VERSION: &str = "1.0.0";
pub struct RunRequest { pub recipe: String, pub seed: u64,
                        pub inputs: serde_json::Map<String, serde_json::Value>,
                        pub from: Option<PathBuf>, pub notes: Vec<Note>, pub options: RunOptions }
pub struct Note { pub id: String, pub about: String, pub text: String }
pub struct RunManifest { pub generator: String, pub recipe: String, pub seed: u64,
                         pub inputs: serde_json::Map<String, serde_json::Value>,
                         pub frame: ManifestFrame, pub outputs: BTreeMap<String, String>,
                         pub results: serde_json::Map<String, serde_json::Value>,
                         pub timing: BTreeMap<String, f64> }
pub struct ManifestFrame { pub size_m: [f64; 2] }
pub struct Description { pub name: String, pub version: String, pub produces: Vec<String>,
                         pub accepts_notes: bool, pub recipes: Vec<RecipeInfo>,
                         pub problems: Vec<RecipeProblem> }
pub struct RecipeInfo { pub id: String, pub title: String,
                        pub inputs: serde_json::Map<String, serde_json::Value>,
                        pub seeded: bool, pub stages: Vec<StageInfo> }
pub struct StageInfo { pub id: String, pub summary: String }
```

All derive `Debug, Clone, Serialize, Deserialize`.

- [ ] **Step 1: Write the failing tests:**
  - `a_description_serialises_with_the_protocol_keys`: its JSON has `name`, `version`,
    `produces`, `accepts_notes` and `recipes[].{id, title, inputs, seeded}`
    (`docs/generators.md` §3).
  - `a_run_manifest_serialises_with_the_protocol_keys`: `generator`, `recipe`, `seed`, `inputs`,
    `frame.size_m`, `outputs`, `results`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): the map generator's library types`.

---

## Part D — kernels (spec §5.8)

Kernels are pure functions. Each comes with tests against known values; scipy or numpy values come
from the `uv` call named in the build rules. A kernel's error type is `KernelError` (thiserror) in
`kernels/mod.rs`.

### Task 13: FFT, spectral noise, warp

**Files:** Create `crates/theos-maps/src/kernels/{complex.rs, fft.rs, noise.rs}`.

**Interfaces — Produces:**

```rust
pub struct Complex { pub re: f64, pub im: f64 }   // add, sub, mul, div, scale, conj, from_polar (libm)
pub fn next_pow2(n: usize) -> usize;
pub fn fft(data: &mut [Complex], inverse: bool) -> Result<(), KernelError>;  // iterative radix-2;
                                            // inverse scales by 1/n; len not a power of two → error
pub fn fft2(data: &mut [Complex], rows: usize, cols: usize, inverse: bool) -> Result<(), KernelError>;
pub fn spectral_noise(rows: usize, cols: usize, rng: &mut StageRng, beta: f64, kmin: f64,
                      kmax: Option<f64>) -> Grid<f64>;
pub fn warped_coords(rows: usize, cols: usize, rng_x: &mut StageRng, rng_y: &mut StageRng,
                     strength: f64, beta: f64) -> (Grid<f64>, Grid<f64>);   // x, y fractions
```

`Complex` is our own type: `num-complex` would bring `num-traits` with std float methods (spec
§4.3). The twiddles are `libm::cos`/`libm::sin` of `−2πk/n`, computed once per call.

**Spectral noise** (spec §5.7, §5.8; a lattice sum since Task 43b, so a seed gives the same
landscape at every grid size):
- The wavenumbers form a lattice that the map fixes, not the grid: step δ = 3/4 cycle per map
  along each axis. Point `(a, b)` of ℤ² is `kx = δ·a` cycles per map width and `ky = δ·b` per map
  height, and `k = sqrt(kx² + ky²)`. A grid of `rows × cols` cells holds the points up to its
  Nyquist wavenumber, `|a| ≤ ⌊2·cols/3⌋` and `|b| ≤ ⌊2·rows/3⌋`.
- The amplitude is 0 at `(0, 0)`, where `k < kmin` or `k > kmax`, and `libm::pow(k, −β/2)`
  elsewhere.
- A call takes one draw of its stream, `key = rng.next_u64()`. The coefficient of `(a, b)` is
  counter-based: a SplitMix64 seeded with `key XOR FNV-1a-64(a, b)` (little-endian `i64`) gives
  two outputs, their top 53 bits give `u₁ ∈ (0, 1]` and `u₂ ∈ [0, 1)`, and Box–Muller on both
  gives `√(−2 ln u₁)·(cos 2πu₂ + i sin 2πu₂)`.
- The field at the cell centre `(x, y) = ((c + ½)/cols, (r + ½)/rows)` is the real part of the
  sum over the band of amplitude × coefficient × `e^(2πi (kx·x + ky·y))`, evaluated along each
  axis by chirp-z transforms (`kernels::czt`, Bluestein on `fft`). Then subtract the mean and
  divide by the population standard deviation (all zeros when it is 0).

`warped_coords` follows 0.3.0's `warped_grid` (`common.py`): the normalised `[x, y]` of each cell
plus `strength` × noise, with x from columns and y from rows (G19). It uses cell centres,
`((c + 0.5)/cols, (r + 0.5)/rows)`, where 0.3.0 used corners: the format places a cell's value
at its centre (`docs/map-package.md` §2).

- [ ] **Step 1: Write the failing tests:**
  - `fft_matches_the_direct_dft`: 8 values from `StageRng::for_key(1, "t")`, against an O(n²) DFT
    with `libm`, within 1e-12.
  - `inverse_fft_restores_the_input`: 1e-12, at length 64 and in 2D at 16 × 32.
  - `fft_of_an_impulse_is_flat`.
  - `a_length_that_is_not_a_power_of_two_is_an_error`.
  - `noise_is_zero_mean_unit_variance`: |mean| < 1e-12 and |var − 1| < 1e-9.
  - `noise_is_repeatable_for_a_key`.
  - `noise_on_odd_shapes_is_finite` (Review Focus 2): the lattice sum on shapes `37×64`, `64×5`
    and `1×1` (all zeros) is finite and has the right shape.
  - `higher_beta_is_smoother`: the mean |Laplacian| at β = 4 is below that at β = 2.
  - `fft_sizes_the_recipes_use_round_trip`: 2D at 64², 512² and 1024², within 1e-10 (spec §8:
    the padded sizes of 64², 384² and 1.5 × 384²).
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): radix-2 FFT and spectral noise with libm twiddles`.

### Task 14: Filters and statistics

**Files:** Create `crates/theos-maps/src/kernels/{reflect.rs, filters.rs, diff.rs, stats.rs}`.

**Interfaces — Produces:**

```rust
pub fn reflect_index(i: isize, n: usize) -> usize;  // scipy 'reflect' (d c b a | a b c d | d c b a), any i
pub fn gaussian(g: &Grid<f64>, sigma_m: f64, cell_m: f64) -> Grid<f64>;   // separable, truncate 4σ
pub fn uniform(g: &Grid<f64>, size: usize) -> Grid<f64>;     // scipy uniform_filter, reflect, origin 0
pub fn maximum(g: &Grid<f64>, size: usize) -> Grid<f64>;     // scipy maximum_filter, same window
pub fn laplace(g: &Grid<f64>, cell_m: f64) -> Grid<f64>;     // 5-point, reflect, ÷ cell²
pub fn gradient(g: &Grid<f64>, cell_m: f64) -> (Grid<f64>, Grid<f64>); // (d/drow, d/dcol), numpy.gradient
pub fn slope(g: &Grid<f64>, cell_m: f64) -> Grid<f64>;       // sqrt(gr² + gc²)
pub fn percentile(values: &[f64], q: f64) -> f64;            // numpy 'linear'; sorts a copy by total_cmp
pub fn median(values: &[f64]) -> f64;
pub fn mean(values: &[f64]) -> f64;
pub fn metres_to_cells(m: f64, cell_m: f64) -> usize;        // max(1, round(m / cell))
```

**Details** (scipy's semantics):
- **Gaussian kernel:** radius `r = round(4σ_cells)`, weights `exp(−0.5·x²/σ²)` from `libm::exp`,
  normalised to sum 1.
- **Window of size `s`:** offsets `−(s/2) … s−1−(s/2)`.

- [ ] **Step 1: Write the failing tests**, each against values pasted from scipy or numpy:
  - `gaussian_matches_scipy`: 5 × 7 grid with values `0..35`, σ = 1 cell;
  - `gaussian_wider_than_the_map_reflects_and_stays_finite` (Review Focus 2): 4 × 4, σ = 10 cells;
  - `uniform_matches_scipy_for_odd_and_even_sizes`: sizes 3 and 4;
  - `maximum_matches_scipy`;
  - `laplace_matches_scipy`;
  - `gradient_matches_numpy`;
  - `percentiles_match_numpy`: q 5, 50 and 95 on 10 values, and an even-length median;
  - `reflect_index_wraps_more_than_one_period`: `reflect_index(-9, 4)` and `reflect_index(12, 4)`
    against scipy's padding.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): raster filters with scipy's semantics`.

### Task 15: Distance transform, labels, morphology

**Files:** Create `crates/theos-maps/src/kernels/{distance.rs, labels.rs, morphology.rs}`.

**Interfaces — Produces:**

```rust
pub fn distance(mask: &Grid<bool>, cell_m: f64) -> Result<(Grid<f64>, Grid<u32>), KernelError>;
    // exact Euclidean distance in metres to the nearest `true` cell and its index
    // (Felzenszwalb & Huttenlocher 2012); ties → smallest index; no `true` cell → EmptyMask
pub enum Connectivity { Four, Eight }
pub fn labels(mask: &Grid<bool>, c: Connectivity) -> (Grid<u32>, u32);  // 0 = background;
    // 1..=n numbered by the scan order of each component's first cell (scipy.ndimage.label)
pub enum Structure { Cross, Disk { radius_m: f64 } }
pub fn dilate(mask: &Grid<bool>, s: Structure, cell_m: f64, iterations: usize) -> Grid<bool>;
pub fn erode(mask: &Grid<bool>, s: Structure, cell_m: f64, iterations: usize) -> Grid<bool>;
pub fn open(mask: &Grid<bool>, s: Structure, cell_m: f64) -> Grid<bool>;
    // scipy's border_value = 0: cells outside the grid are false
```

- [ ] **Step 1: Write the failing tests:**
  - `distance_matches_brute_force`: a 20 × 17 random mask from a fixed key, exact to 1e-12, with
    nearest indices equal to brute force's first minimum;
  - `distance_of_an_empty_mask_is_an_error`;
  - `labels_number_components_in_scan_order` and `eight_connectivity_joins_diagonals`;
  - `dilation_and_erosion_match_scipy` (cross and a 1.5-cell disk);
  - `erosion_clears_cells_at_the_border`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): distance transform, labels and morphology`.

### Task 16: Curves, Voronoi, Delaunay edges, marching squares

**Files:**
- Modify: `Cargo.toml` (`spade`, no default features).
- Create: `crates/theos-maps/src/kernels/{curves.rs, voronoi.rs, contour.rs}`.

**Interfaces — Produces:**

```rust
pub fn dense_curve(points: &[[f64; 2]], samples: usize) -> Vec<[f64; 2]>;  // 0.3.0 dense_curve
pub fn signed_distance(curve: &[[f64; 2]], x: &Grid<f64>, y: &Grid<f64>)
    -> (Grid<f64>, Grid<f64>);   // (signed, unsigned), as 0.3.0 signed_distance_to_curve
pub fn nearest_seed(seeds: &[[f64; 2]], x: &Grid<f64>, y: &Grid<f64>) -> Grid<u32>;
pub fn delaunay_edges(points: &[[f64; 2]]) -> Vec<(usize, usize)>;   // a < b, sorted
pub fn rings(mask: &Grid<bool>, cell_m: f64) -> Vec<Vec<[f64; 2]>>;
    // marching squares at 0.5 between cell centres, on the mask padded with false; closed rings
    // in metres; exterior rings have positive shoelace area in (x, y), holes negative;
    // saddles resolved as separated (the 4-connected reading)
```

**`spade` use:**
- `DelaunayTriangulation<Point2<f64>>`; insert the points in index order. A vertex handle maps to
  the lowest index of its point (duplicates).
- Nearest neighbours come from spade's `nearest_neighbor`.
- `dense_curve` and `signed_distance` port `common.py`. `dense_curve` is its Catmull–Rom spline:
  a straight line for two points, otherwise phantom end points and `max(2, n / segments)` samples
  per segment.
- **The sign** at the nearest dense point `i` (clamped to `1..len−2`) is
  `sign(t_x·r_y − t_y·r_x)`, with `t = p[i+1] − p[i−1]` and `r = q − p[i]`. That is 0.3.0's
  formula with named components, so it is unchanged by the `[y, x]` → `[x, y]` swap.

- [ ] **Step 1: Write the failing tests:**
  - `dense_curve_passes_through_its_control_points`;
  - `signed_distance_flips_sign_across_a_straight_line`;
  - `nearest_seed_matches_brute_force`;
  - `delaunay_of_a_square_and_its_centre_has_eight_edges`;
  - `a_three_by_three_lake_ring_has_area_8_5_cells`: the shoelace area equals `8.5 · cell²`
    (corners cut by 0.125 cell² each);
  - `a_lake_with_an_island_gives_a_hole`;
  - `rings_close_at_the_map_edge`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS. Run `cargo tree -e features -i num-traits` and record the
  result in the ledger (L25). Run `cargo deny --locked check`.
- [ ] **Step 5: Commit** `feat(generators): curves, Voronoi, Delaunay edges and lake rings`.

### Task 17: Shortest paths, MST, priority-flood

**Files:** Create `crates/theos-maps/src/kernels/{paths.rs, mst.rs, flood.rs}`.

**Interfaces — Produces:**

```rust
pub enum Directions { Eight, Sixteen }
pub struct PathTree { pub dist: Vec<f64>, pub prev: Vec<u32> }  // unreachable: dist f64::INFINITY,
                                                                // prev u32::MAX (internal only)
pub fn shortest_paths(rows: usize, cols: usize, sources: &[usize], dirs: Directions,
                      cost: &mut dyn FnMut(usize, usize) -> Option<f64>,
                      extra: &mut dyn FnMut(usize) -> Vec<(usize, f64)>,
                      cancel: &dyn Fn() -> bool) -> Result<PathTree, KernelError>; // Cancelled
pub fn path_to(tree: &PathTree, target: usize) -> Option<Vec<usize>>;   // source … target
pub struct UnionFind;                                     // new(n), find, union (by index)
pub fn kruskal(n: usize, edges: &[(usize, usize, f64)]) -> Vec<usize>;  // indices of tree edges
pub struct Flooded { pub surface: Grid<f64>, pub order: Vec<u32> }
pub fn priority_flood(height: &Grid<f64>, seeds: &[usize], epsilon: f64) -> Flooded;
```

**Details:**
- **`shortest_paths`:** a `BinaryHeap` of `Reverse((OrdF64(key(dist)), index))` with `total_cmp`.
  `cancel()` is checked every 4,096 pops. `cost(from, to)` returns `None` for a forbidden step.
  `extra` adds bridge-like edges.
- **`kruskal`:** sorts by `(key(w), a, b)` with `total_cmp`.
- **`priority_flood`** follows Barnes et al. 2014's "+ε":
  - seeds start the queue;
  - each popped cell's unvisited neighbours (D8) get `max(own, popped + ε)`;
  - ties go by cell index;
  - `order` is the pop order.

- [ ] **Step 1: Write the failing tests:**
  - `dijkstra_matches_bellman_ford` (a 7 × 9 random cost field, both direction sets);
  - `knight_steps_cost_their_own_length`;
  - `equal_paths_break_ties_by_index` (two runs give the same `prev`);
  - `cancel_is_checked_every_4096_pops` (a counter in the closure);
  - `mst_of_a_known_graph`;
  - `equal_weights_tie_by_index`;
  - `the_flooded_surface_has_no_interior_pit`: on 5 random 32 × 32 surfaces, every non-seed cell
    has a neighbour at least ε lower on the flooded surface, so it drains to a seed (physics test
    7's property);
  - `flood_order_starts_at_the_seeds`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): grid shortest paths, Kruskal and priority-flood`.

---

## Part E — terrain stages before evolution

### Task 18: Stages `grid` and `tectonics`

**Files:** Create `crates/theos-maps/src/stages/terrain/{mod.rs, grid.rs, tectonics.rs}`; modify
`src/stages/mod.rs` (two registry lines).

**Interfaces:**
- **Consumes:** `TypedStage`, `StageContext`, kernels `noise`, `curves`.
- **Produces** (world values, each a `ValueType`):

```rust
pub struct FrameValue { pub drained: Edges }            // NAME "frame"
pub struct Edges { pub north: bool, pub east: bool, pub south: bool, pub west: bool }
pub struct Belts { pub first_orogen: Option<Grid<f64>> } // NAME "belts", signed distance in fractions
pub struct Warp { pub x: Grid<f64>, pub y: Grid<f64> }   // NAME "warp"
```

**`grid`:**
- params `edges: Vec<"north"|"east"|"south"|"west">`, default `["south"]`;
- writes `frame` (value) and `fixed` (Mask: the outermost row or column on each drained side).

**`tectonics`** (port `terrain.py::tectonics`):
- **Reads and writes:** reads `frame`, `fixed`; writes `uplift` (m/yr), `sag_rate` (m/yr),
  `belts`, `warp`.
- **Params, defaults as 0.3.0:** `background_uplift` 1.5e-4, `background_variation` 0.4,
  `background_beta` 3.2, `warp` 0.035, `warp_beta` 3.6, `sag_duration_yr` 6.0e6 (L13), and
  `source`, an array of tables tagged by `type`:
  - `orogen` {`path`, `width`, `rate`, `asymmetry` 0.5, `jitter` 0.06};
  - `fault` {`path`, `rate`, `reach` 0.25, `jitter` 0.06};
  - `rift` {`path`, `width` 0.06, `rate`, `depth_m` 150, `jitter` 0.06};
  - `dome` and `volcano` {`at`, `r`, `rate`, `jitter` 0.06};
  - `basin` {`at`, `r`, `rate` 0, `depth_m` 80, `jitter` 0.06}.
- **Positions** are `[x, y]` fractions (G19). The code reads `x` with columns and `y` with rows.
- **Random draws** in 0.3.0's order: the rate jitter, then the path or centre jitter, then the
  radius jitter. Each of 0.3.0's `subseed` names becomes `ctx.sub_rng("<same name>")`.
- **Sag:** 0.3.0's `sag` (negative metres) becomes `sag_rate = −sag / sag_duration_yr`.
- **Clamping:** `uplift` is clipped at 0. `uplift` and `sag_rate` are 0 on `fixed` cells.

- [ ] **Step 1: Write the failing tests:**
  - `drained_edges_are_marked_on_their_sides`: `edges = ["north"]` marks exactly row 0.
  - `closed_and_open_grids_are_valid`: `[]` marks no cell; all four mark the whole rim (Review
    Focus 1, the grid side).
  - `a_source_at_a_known_fraction_appears_at_the_expected_cell` (physics test 11): 64², a dome at
    `[0.25, 0.75]` with `r` 0.05, `jitter` 0, `background_variation` 0 and `warp` 0. The maximum of
    `uplift` lies at row 48 ± 1 and column 16 ± 1.
  - `basins_become_a_sag_rate`: `depth_m` 60 over the default duration gives a peak `sag_rate` of
    1.0e-5.
  - `uplift_is_never_negative_and_zero_on_fixed_cells`.
  - `unknown_source_types_are_parameter_errors`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement**, with one module per source type if `tectonics.rs` grows past about
  300 lines.
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): grid and tectonics stages`.

### Task 19: Stage `geology`

**Files:** Create `crates/theos-maps/src/stages/terrain/{geology.rs, rocks.rs}`.

**Interfaces — Produces:**

```rust
pub enum Rock { Granite, Quartzite, Basalt, Limestone, Sandstone, Shale, Alluvium } // u8 = order
pub struct RockProps { pub k: f64, pub kd: f64, pub critical_slope: f64, pub colour: [u8; 3] }
pub fn props(rock: Rock) -> RockProps;    // k, kd and colour from 0.3.0's ROCKS table
```

**`critical_slope`** (tan of the angle) is the builder's choice within Montgomery & Brandon 2002's
ranges, with the source in the doc comment. The order is binding: granite = quartzite ≥ basalt ≥
limestone ≥ sandstone ≥ shale ≥ alluvium (spec §5.9).

**`geology`** (port `terrain.py::geology`):
- **Reads and writes:** reads `frame`, `uplift`, `sag_rate`, `belts`, `warp`; writes `rock` (U8),
  `erodibility`, `diffusivity`, `critical_slope`.
- **Params:** `provinces` 6, `rock_mix` (table rock → weight), `fold_belt` (optional {`width`
  0.12, `strength` 0.6, `period` 0.035, `contrast` 2.5}), `plutons` `[0, 2]`.
- **Weights** are taken in `Rock` order (independent of TOML key order).
- **Alluvium** goes where `sag_rate > 0.3 × max(sag_rate)` (0.3.0's basin-floor rule, mirrored).
- Every `sin`, `exp` and `log` is from `libm`.

- [ ] **Step 1: Write the failing tests:**
  - `a_single_rock_mix_fills_the_map`: `{granite = 1.0}` with no basin gives granite everywhere;
  - `alluvium_fills_sinking_basins`;
  - `erodibility_stays_in_the_table_range` (×0.5 … ×1.5 of the table's extremes, as 0.3.0);
  - `critical_slopes_follow_the_rock_order`;
  - `unknown_rocks_are_parameter_errors`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): geology with rock-dependent erodibility and threshold slopes`.

### Task 20: Stage `climate_model` and the climate functions

**Files:** Create `crates/theos-maps/src/stages/terrain/climate/{mod.rs, orographic.rs, water.rs}`.

**Interfaces — Produces:**

```rust
pub struct ClimateParams { pub wind_from_deg: f64, pub wind_speed: f64, pub base_precip_mm: f64,
    pub orographic_mm: f64, pub min_precip_mm: f64, pub t_base_c: f64, pub lapse_c_per_km: f64,
    pub nm: f64, pub hw: f64, pub cw: f64, pub tau_c: f64, pub tau_f: f64, pub budyko_w: f64 }
pub struct ClimateFields { pub precipitation: Grid<f64>, pub pet: Grid<f64>, pub aet: Grid<f64>,
                           pub runoff: Grid<f64> }        // all mm/yr
pub struct Climate { pub params: ClimateParams, pub fields: Option<ClimateFields> }  // NAME "climate"
pub fn orographic(height: &Grid<f64>, frame: &MapFrame, p: &ClimateParams) -> Grid<f64>; // mm/yr
pub fn temperature(height: f64, p: &ClimateParams) -> f64;   // t_base − lapse·max(h, 0)/1000
pub fn pet(t_c: f64) -> f64;                                  // clamp(300 + 45·T, 200, 1800)
pub fn fu_aet(p: f64, pet: f64, w: f64) -> f64;               // P·(1 + φ − (1 + φ^w)^(1/w)), φ = PET/P
pub fn fields(height: &Grid<f64>, frame: &MapFrame, p: &ClimateParams) -> ClimateFields;
```

**Defaults** are 0.3.0's: wind_from_deg 270, wind_speed 10, base_precip_mm 600, orographic_mm 1600,
min_precip_mm 120, t_base_c 13.0, lapse_c_per_km 6.5, nm 0.005, hw 3000, cw 0.004, tau_c 1000,
tau_f 1000. `budyko_w` 2.6 is the builder's choice within Fu's and Zhang's range; record it.
0.3.0's recipe key `T_base_c` becomes `t_base_c`.

**`orographic`** follows Smith & Barstad 2004 as in `terrain.py::orographic_precip`:
- pad to `P = next_pow2(ceil(1.5·rows))` × `Q = next_pow2(ceil(1.5·cols))`, centred, with a
  linear ramp to 0 (numpy's `linear_ramp`), on `height − min(height)`;
- the wavenumbers are `2π·fftfreq(n, cell)`;
- the complex `m` follows 0.3.0's branch rule;
- the result is the real part, cropped, times 3.156e7.

**`fields`** builds the precipitation, then PET, AET and runoff from it:
- `precipitation = max(base + scale·oro, min)`, where `scale = orographic_mm / p95(oro > 0)`, or 1
  when no cell is positive;
- `pet(temperature(h))`;
- `fu_aet`;
- `runoff = P − AET`.

**`climate_model`:** reads `frame`; writes `climate` with `fields: None` (no height exists yet).

- [ ] **Step 1: Write the failing tests:**
  - `smith_barstad_wets_the_windward_side` (physics test 9): a 64² Gaussian hill and wind from 270°
    (west). The mean orographic part west of the crest is > 0, and east of it < 0.
  - `fu_curve_limits`: φ → 0 gives AET → 0; φ → ∞ gives AET → P; at φ = 1 and w = 2.6 the value is
    `P·(2 − 2^(1/2.6))`.
  - `pet_is_clamped`: −20 °C gives 200, and 40 °C gives 1800.
  - `a_flat_surface_gets_only_base_precipitation`.
  - `precipitation_respects_the_minimum`.
  - `runoff_is_precipitation_minus_aet_and_never_negative`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): climate model with orographic precipitation and a Budyko water balance`.

---

## Part F — landscape evolution (spec §5.9, each process with its physics test)

All processes live in `crates/theos-maps/src/stages/evolution/`. Each is a `TypedStage` whose
`KINDS` is `&[MapKind::Terrain]`, registered in `evolution::PROCESSES` (one line each), not in
`stages::register`. Physics tests are unit tests in the process's module (L15). They drive a
`World` through a test helper `evolve(world, processes, steps, dt)` defined in Task 21.

### Task 21: `landscape_evolution` and `uplift`

**Files:** Create `crates/theos-maps/src/stages/evolution/{mod.rs, processes.rs, initial.rs,
uplift.rs, test_support.rs}`.

**Interfaces — Produces:**

```rust
pub struct EvolutionParams { pub steps: u32, pub dt_yr: f64, pub check_every: u32,
                             pub initial_relief_m: f64, pub process: Vec<toml::Table> }
    // defaults 240, 2.5e4, 10 (L6), 40; `process` parsed into PlannedProcess in validate
pub(crate) struct PlannedProcess { pub id: &'static str, pub key: String,
                                   pub params: Box<dyn StageParams>, pub every: Option<u32> }
pub(crate) const PROCESSES: &[fn() -> Box<dyn Stage>];      // one line per process
pub(crate) fn evolve(world: &mut World, processes: &[PlannedProcess], steps: u32, dt_yr: f64,
                     seed: u64) -> Result<(), StageError>;  // test_support, cfg(test)
```

**How `landscape_evolution` works:**
- **Parsing:** its typed `Params` hold the raw process tables. A custom `params` step (in
  `validate`) turns each table into a `PlannedProcess` through `PROCESSES` and fails naming
  `process[<i>]` and its id.
- **Its own writes:** `height` (the initial surface), `erosion` and `deposition` (zeros). Its
  `writes` are these plus every process's writes.
- **Its `reads`:** for each process in order, the reads not written by the initial surface or by
  an earlier process.
- **`check_order(available)`:** each process's reads must be in `available`, in the initial
  writes or in an earlier process's writes. A failure names `landscape_evolution`, the process and
  the layer.
- **The order of a time step** (spec §5.3, ruling 58; Task 43d): the recipes list
  `uplift → hillslope → route_steepest → resolve_sinks → partition_flow → fluvial → slope_limit →
  climate_update`, so each step ends on the surface `fluvial` cut. `check_order` also refuses a
  `hillslope` that comes after `fluvial` or after a routing process (`route_steepest`,
  `resolve_sinks`, `partition_flow`), at `process[<i>]` of the `hillslope`, naming `hillslope`,
  the process before it and the rule. It lives in one module, `hillslope_order.rs`. Lists without
  `hillslope` stay valid.
- **Initial surface** (port 0.3.0): `relief · noise(β = 3.0) + relief + uniform`, with the noise
  and uniform from `sub_rng("z0")`, and `fixed` cells at 0.
- **Loop**, for `step in 0..steps`:
  1. `check_cancel`;
  2. every 10 steps, `progress(step, steps)`;
  3. each process runs in a child context keyed `"<evolution key>/<id>#<occ>"` with
     `StepInfo { step, steps, dt_yr }`;
  4. `every` is taken out of each process table before the process parses it (default 1, L27).
     A process with `every = n` runs when `step % n == 0` or on the last step;
  5. its `checks` run when `step % check_every == 0` or on the last step. An `Error` ends the
     stage with `StageError::Failed { stage: "landscape_evolution/<id>", … }`. The first warning
     per process is kept with a count of repeats in the stage report.

**`uplift`:**
- reads `height`, `uplift`, `sag_rate`, `fixed`; writes `height`;
- `h += (uplift − sag_rate)·dt` on non-fixed cells; fixed cells stay at 0 (the base level).

- [ ] **Step 1: Write the failing tests** (with test processes where real ones do not exist
  yet):
  - `evolution_reads_and_writes_follow_its_processes`;
  - `a_process_before_its_input_fails_naming_the_process`;
  - `processes_parse_their_parameters_and_unknown_ones_fail`;
  - `uplift_adds_rate_times_dt_and_keeps_fixed_cells_at_base_level`;
  - `progress_is_reported_every_ten_steps`;
  - `cancel_is_checked_every_step`;
  - `a_process_with_every_runs_at_step_zero_multiples_and_the_last_step`;
  - `checks_run_every_check_every_steps_and_on_the_last`;
  - `a_panicking_process_fails_the_run_as_a_stage_panic` (spec §8): through `execute` with a
    registry whose evolution stage was built by `Evolution::with_processes` with a panicking test
    process. The result is `StagePanicked { stage: "landscape_evolution", message }` with the
    process's text.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement**; register `landscape_evolution` in `stages::register`.
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): landscape evolution loop and uplift`.

### Task 22: `route_steepest` and `resolve_sinks`

**Files:** Create `crates/theos-maps/src/stages/evolution/{route_steepest.rs, resolve_sinks.rs,
flow.rs}`.

**Interfaces — Produces:**

```rust
pub struct Depressions { pub label: Grid<u32>, pub spill: Vec<SpillPoint> }   // NAME "depressions"
pub struct SpillPoint { pub cell: u32, pub to: u32, pub level: f64 }          // label n ↔ spill[n-1]
pub fn d8_distance(cell_m: f64, k: usize) -> f64;            // cell or cell·√2 (flow.rs)
```

**`route_steepest`:**
- reads `height`, `fixed`; writes `receivers` (U32);
- the receiver is the D8 neighbour with the steepest drop ÷ distance, with ties to the lower
  neighbour index;
- a cell with no lower neighbour, or a `fixed` cell, receives itself.

**`resolve_sinks`** (Cordonnier, Bovy & Braun 2019 + Barnes 2014). It reads `height`, `receivers`
and `fixed`, and writes `receivers`, `routing_surface` and `depressions`:
1. **Outlets** are the `fixed` cells. With none, the lowest cell (ties by index) is the map's
   sink (Review Focus 1).
2. **Basins:** each cell's basin is the outlet or pit its receiver chain ends in. All outlet
   basins form one root.
3. **Basin graph:** for each D8 pair of cells in different basins, the pass height is
   `max(h_a, h_b)`. Keep the lowest pass per basin pair, with ties by the smaller `(min cell,
   max cell)`.
4. **Tree:** run `kruskal` over the basins, then orient the tree to the root. For each non-root
   basin `B`, the pass to its parent is `(a ∈ B, b ∈ parent)` at level `L`.
5. **Depressions** (a depression is one lake, nested pits included; Task 43e):
   - `B`'s cells with `h < L` that are connected (D8) to its pit form depression `n`, with
     `SpillPoint { cell: a, to: b, level: L }`;
   - nested-pit rule: when `B`'s flood starts above its pass (the lake `B` spills into stands
     above `L`, so `B` lies in a lifted lake), `B`'s raised cells take the label of that lake and
     add no spill point. A depression has one spill point, its outlet pass, at whose level its
     routing surface stands, and `fluvial` fills it as one body up to that level (a per-basin
     trap at each nested pass let a lake pass sediment on while it had room);
   - `routing_surface` lifts those cells by `priority_flood` restricted to the depression, seeded
     at `a` with `ε = 1e-6`. So it equals `L + k·ε` along the flood order, and `height` is
     unchanged;
   - all other cells keep `height`.
6. **Receivers again:** steepest descent on `routing_surface`, and each `a` gets `receiver = b`.

- [ ] **Step 1: Write the failing tests:**
  - `closed_and_fully_open_edges_route_every_cell` (Review Focus 1): on 64² noise with
    `edges = []`, every receiver chain ends at the sink; with all four edges, at a fixed cell; no
    chain cycles.
  - `a_bowl_drains_through_its_notch`: a bowl with one notch. The depression covers the cells
    below the notch level, and every bowl cell's chain leaves through the notch.
  - `a_pit_nested_in_a_lake_is_part_of_the_lake` (Task 43e; it replaced
    `nested_depressions_spill_in_order`): one spill point (the lake's notch), the shallow pit
    labelled with the deep one, its routing level the lake's, and every chain from the shallow
    pit stays in the lake until the notch. A closed-basin mass-balance test in `fluvial`
    (`a_pit_nested_in_a_lake_fills_with_the_lake_and_keeps_its_sediment`) keeps the sediment of
    such a lake.
  - `routing_surface_lifts_only_depressions_and_height_is_unchanged`.
  - `flat_terrain_routes_deterministically`: two runs are identical, and every cell reaches an
    outlet.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): steepest receivers and the MST sink resolver`.

### Task 23: `partition_flow` and accumulation

**Files:** Create `crates/theos-maps/src/stages/evolution/partition_flow.rs`; extend `flow.rs`.

**Interfaces — Produces:**

```rust
pub struct FlowWeights { pub start: Vec<u32>, pub to: Vec<u32>, pub weight: Vec<f64>,
                         pub dist: Vec<f64> }   // CSR per cell; NAME "weights"
pub fn accumulate(stack: &[u32], w: &FlowWeights, input: &Grid<f64>) -> Grid<f64>;
    // reverse stack order: out[i] = input[i] + Σ donors out[d]·w(d→i)
pub fn outflow(stack: &[u32], w: &FlowWeights, input: &Grid<f64>, outlets: &Grid<bool>) -> f64;
```

**`partition_flow`:**
- **Reads and writes:** reads `routing_surface`, `receivers`; writes `weights`, `stack` (U32
  grid holding the cell order).
- **Params:** `exponent = "steepest" | "slope_adaptive"` (no default: the recipe states it).
- **Weights:**
  - `steepest` sends everything to the receiver;
  - `slope_adaptive` spreads over the strictly lower D8 neighbours on `routing_surface`, in
    proportion to `slope_j^p` with `p = 0.5 + 0.6·S`, where `S` is the steepest of those slopes;
  - a cell without a lower neighbour sends everything to its receiver (the pass links of Task 22);
  - each slope is `drop / d8_distance`, and `dist` holds that distance.
- **Stack:** Kahn's algorithm from the outlets (cells whose receiver is themselves), in
  cell-index order, receivers before donors. A cycle is a `StageError::Failed`.

- [ ] **Step 1: Write the failing tests:**
  - `weights_sum_to_one`;
  - `steepest_sends_everything_to_the_receiver`;
  - `slope_adaptive_spreads_more_on_gentle_ground` (p is lower at small S);
  - `the_stack_puts_receivers_before_donors_and_covers_every_cell`;
  - `discharge_routing_loses_nothing` (physics test 6): on a 64² surface without depressions, with
    all four edges drained and random runoff, `outflow` equals the total input to 1e-9 relative,
    for both exponents.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): multiple-flow partition with a fresh stack`.

### Task 24: `fluvial` without deposition (G = 0)

**Files:** Create `crates/theos-maps/src/stages/evolution/fluvial/{mod.rs, solve.rs}`.

**Interfaces — Produces:**
- `fluvial` params: `m` 0.45, `g` (no default: the recipe states it), `max_iterations` 50; n is 1
  (spec ruling 3).
- Reads `height`, `erodibility`, `receivers`, `weights`, `stack`, `depressions`, `fixed`,
  `climate`, `erosion`, `deposition`. Writes `height`, `erosion`, `deposition`, `drainage_area`.
- `pub fn runoff_weight(climate: &Climate, frame) -> Grid<f64>` is `p̂ = runoff / mean(runoff)`,
  or all 1 while `fields` is `None`. With a mean runoff of 0 (no precipitation), `p̂` is 0
  everywhere, never NaN.
- Where `Q = 0`, a cell has no fluvial term: `F = 0`, and no `G·Qs/Q` (the cell gets no
  sediment, so `Qs_in` is 0 there too).

**The law with G = 0** (spec §5.9). Let `h*` be the height entering the process (uplift already
applied).
- `Q = accumulate(stack, weights, p̂ × cell area)`.
- Then, in stack order (receivers first), with fixed cells unchanged:

  ```text
  F_ir = K_i · Q_i^m · w_ir · Δt / d_ir                       (one term per weighted receiver r)
  h_i  = (h*_i + Σ_r F_ir · h_r) / (1 + Σ_r F_ir)
  ```
- `Q^m` is `libm::pow`.
- `erosion += max(h* − h, 0)` and `deposition += max(h − h*, 0)`.
- `drainage_area = accumulate(stack, weights, cell area)` (not runoff-weighted, spec §5.10).

- [ ] **Step 1: Write the failing tests:**
  - `steady_state_slopes_follow_the_stream_power_law` (physics test 1): 64², uniform U = 1e-3 m/yr,
    K = 1e-5, m = 0.5, p̂ = 1, all four edges drained, processes `uplift`, `route_steepest`,
    `resolve_sinks`, `partition_flow`, `fluvial(g = 0)` until the max |Δh| per step < 1e-6 m.
    Then, for every cell with A > 10 cells, the flow-weighted slope `Σ_r w_ir·S_ir` equals
    `(U/K)·Q^(−m)` within 2 %. Run with `steepest` (the spec's case) and with `slope_adaptive`
    (the risk table's).
  - `stability_at_ten_times_the_time_step` (physics test 2, G = 0): Δt = 2.5e5. After every step,
    no non-fixed cell lies below its receiver. No cell's height change flips sign over three
    consecutive steps by more than 1e-9 m.
  - `fixed_cells_never_change`.
  - `erosion_is_booked_and_non_negative`.
  - `zero_precipitation_erodes_nothing_and_stays_finite` (spec §8).
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): implicit stream-power incision over multiple flow`.

### Task 25: `fluvial` with deposition (G > 0) and depression filling

**Files:** Modify `crates/theos-maps/src/stages/evolution/fluvial/{mod.rs, solve.rs}`; create
`fluvial/fill.rs`.

**Gauss–Seidel iterations** (Yuan et al. 2019, eq. 15, 19–21, generalised to the weights; spec
§5.9). Start with `h⁰ = h*`. For each iteration `k`:
1. **Sediment:** per cell, `e_j = (h*_j − h^k_j)·area / Δt`. In reverse stack order,
   `Qs_in(i) = Σ_donors w(d→i)·(Qs_in(d) + e_d)` (strictly upstream, as the paper's `ups(i)`).
2. **Solve** in stack order:
   `h^{k+1}_i = (h*_i + Σ_r F_ir·h^{k+1}_r + Δt·G·Qs_in(i)/Q_i) / (1 + Σ_r F_ir)`.
3. **Stop** when `max|h^{k+1} − h^k|` < `tol` = `1e-3 × max(U)·Δt` (`1e-6` m when `max(U) = 0`),
   or after `max_iterations`. Non-convergence is a `Warning` finding with the residual.

With `uplift` absent from the world (tests), `max(U)` is 0.

**Depression filling** (`fill.rs`, only when G > 0). It runs once per step after the iterations,
on the converged heights:
- Walk the stack in reverse order, carrying `Qs`. When the walk enters a cell of depression `n`
  from outside, add `Qs·Δt` to that depression's inflow volume.
- After the walk, for each depression in reverse stack order of its spill cell:
  - fill its cells in height order (ties by index) up to `min(spill level, the level its volume
    reaches)`;
  - give the filled cells an ε = 1e-6 slope toward `SpillPoint.cell`, along the depression's
    flood order;
  - pass the excess volume on from `SpillPoint.to` into the walk of the depression downstream.
- Book the deposited thickness in `deposition`.
- Keep the outflow at drained edges (Qs leaving the map) in the stage report as
  `sediment_out_m3`.

- [ ] **Step 1: Write the failing tests:**
  - `deposition_steepens_steady_state_by_one_plus_g` (physics test 14): uniform U, K and runoff,
    m = 0.5, no hillslope. Each partition runs with G = 1 from seed 1's landscape and is compared
    with (1 + G) times the G = 0 law on its own final flow weights (ruling 59: a G = 1 run need
    not keep a G = 0 run's network): the flow-weighted slope of every channel cell (A > 10
    cells) and the relief are (1 + G) times the law's within 2 %. Both exponents. `steepest`
    runs to a full steady state; `slope_adaptive` runs 1 500 steps and also requires that
    channels over 22 cells' area move less than 1e-3·U·Δt over the last 50 steps (its wobble
    check fails on seed 7, so the leg rests on seed 1).
  - `mass_balance_holds_with_deposition` (physics test 4): over 50 steps,
    `Σ(h_end − h_start)·area = Σ uplift·dt·area − Σ sediment_out_m3` to 1e-9 relative. Both
    exponents.
  - `sediment_fills_a_closed_depression_and_passes_the_excess` (physics test 5): a bowl fed by an
    uplifted catchment. After enough steps the bowl's cells reach the spill level (within ε per
    cell), and later steps pass sediment out through the spill. Both exponents.
  - `deposition_converges_at_ten_times_the_time_step` (physics test 2, G > 0): G = 1, Δt = 2.5e5.
    It stays within the iteration limit (no warning) and every height is finite.
  - `g_zero_switches_off_deposition_and_filling` (spec ruling 29): `deposition` stays 0.
  - `strong_deposition_stays_finite_at_the_iteration_limit` (spec §8, G/p̂ > 1): G = 4. Either it
    converges, or it stops at 50 iterations with one warning carrying the residual; every height
    is finite, and two runs are identical.
  - `uplift_free_runs_use_the_fallback_tolerance` (spec §8): no uplift, so the tolerance is
    1e-6 m.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): erosion-deposition after Yuan et al. 2019 with depression filling`.

### Task 26: `hillslope`, `slope_limit`, `climate_update`

**Files:** Create `crates/theos-maps/src/stages/evolution/{hillslope.rs, slope_limit.rs,
climate_update.rs}`; create `src/kernels/tridiagonal.rs` (Thomas algorithm).

**`hillslope`:**
- reads `height`, `diffusivity`, `fixed`, `erosion`, `deposition`; writes `height`, `erosion`,
  `deposition`;
- linear diffusion `∂h/∂t = ∇·(kd∇h)` by Peaceman–Rachford ADI: a half step implicit along rows,
  then along columns, each a tridiagonal solve;
- interface diffusivities are the arithmetic mean of neighbours;
- fixed cells are Dirichlet; closed edges have no flux;
- per cell, a net loss books as erosion and a net gain as deposition;
- it runs after `uplift` and before the routing (`route_steepest`, `resolve_sinks`,
  `partition_flow`) and `fluvial` of its step; the planner refuses it after any of them
  (Task 21's `check_order`, ruling 58).

**`slope_limit`** (spec §5.9 "Slope limit"):
- reads `height`, `critical_slope`, `fixed`, `erosion`, `deposition`; writes the same three;
- passes highest to lowest (ties by index), half the excess moves to the steepest neighbour;
- it stops when no slope exceeds critical + 1e-6, or after 100 passes (then a `Warning`);
- material onto drained edges leaves the map, reported as `removed_m3`.

**`climate_update`:**
- reads `height`, `climate`; writes `climate`;
- no params of its own (how often it runs is the evolution's `every`, L27);
- sets `fields = Some(climate::fields(height, …))`.

- [ ] **Step 1: Write the failing tests:**
  - `adi_matches_the_heat_kernel` (physics test 3): 128², constant kd = 0.01 m²/yr, a Gaussian
    bump with σ₀ = 8 cells and amplitude 10 m, closed edges, 20 steps of 1e4 yr. The result matches
    `A·σ₀²/(σ₀² + 2kd·t)·exp(−r²/(2(σ₀² + 2kd·t)))` within 1 % of the peak.
  - `hillslope_books_loss_and_gain`.
  - `slope_limit_leaves_no_slope_above_critical` (physics test 10): random steep terrain. After
    the step, no D8 slope exceeds critical + 1e-6, and the volume change equals −`removed_m3` to
    1e-9 relative.
  - `climate_update_fills_the_fields_from_the_current_surface`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): hillslope diffusion, slope limit and climate update`.

### Task 27: Budget — one 384² run with the full process list (spec §5.14 "Budget")

**Files:** Create `crates/theos-maps/tests/fixtures/recipes/evolution-384.toml` (synthetic, 384²;
0.3.0's hearth-valley `grid`, `tectonics`, `geology`, `climate_model` and the full process list of
the spec's example recipe; Task 41's cancel test reuses it). The measuring test is temporary and
not committed.

- [ ] **Step 1: Measure.**
  - Add a temporary in-crate test that plans `evolution-384.toml` and calls `execute` with seed 1.
  - Run `cargo nextest run -p theos-maps budget --no-capture` (the test profile, `opt-level = 2`).
  - Read the time per stage from `timing_s`. Record it in the ledger.
  - Delete the test.
- [ ] **Step 2: Extrapolate** the suite's map runs:
  - parity: 16 runs at 384², 8 at 64²;
  - resolution test 13: 4 at 384², 4 at 192²;
  - golden: 1 at 384², 2 at 64²;
  - CE-28.7: 1 at 384²;
  - CE-28.4 (a): a part of one.

  Record the total.
- [ ] **Step 3: Decide**, by the spec's rule.
  - **Within the budget** (24 parity runs in under five minutes): note it in the ledger and go on.
  - **Over it:**
    1. Record it.
    2. Name every map-scale test `slow_…`.
    3. Open a second gate PR `gate/slow-map-tests`, waiting for Jann. It adds a nextest filter
       that keeps `slow_` tests out of the default `test` job and a CI job `slow-tests` that runs
       only them on every PR and in `platforms`. That keeps CE-77.1 on Linux in every PR.
    4. Merge that branch into the phase branch like Task 1 (L1).
  - **Either way:** if one 384² run takes over 60 s, profile the hottest stage before going on
    (fluvial iterations, `resolve_sinks`), and record what changed.
- [ ] **Step 4: Commit** the fixture recipe and any `slow_` renames:
  `test(generators): measure a full 384² run`.

---

## Part G — terrain stages after evolution

### Task 28: `hydrology` — lakes, channel discharge, rasters

**Files:** Create `crates/theos-maps/src/stages/terrain/hydrology/{mod.rs, lakes.rs, discharge.rs,
rasters.rs}`.

**Interfaces — Produces:**

```rust
pub struct WaterThresholds { pub river_q: f64, pub big_river_q: f64 }   // NAME "water_thresholds"
```

**Params:** `river_q` and `big_river_q` (m³/s, required, no default; decision 0004),
`min_lake_depth_m` 2.0, `min_lake_area_m2` (optional; 4 cells when absent).

**Reads and writes:**
- reads `height`, `climate` (with fields), `fixed`;
- writes `height`, `lake` (Mask), `lake_id` (U32, 0 = none), `lake_level` (the level on lake
  cells, else 0), `river`, `big_river` (Masks), `discharge` (m³/s), `flow_dir` (U8, L19),
  `water_surface`, `water_thresholds`.

**Algorithm** (spec §5.9 "Hydrology"):
1. **Fill:** `priority_flood(height, outlets, 1e-6)`. The outlets are the `fixed` cells, or the
   lowest cell when there are none.
2. **Depressions:** 8-connected cells where the flooded surface lies more than 1e-3 m above
   `height`. A depression's spill level is its highest flooded value.
3. **Shallow pits:** a depression shallower than `min_lake_depth_m` (spill − min height) or
   smaller than the minimum area is set to the flooded surface in `height`.
4. **Inflow:**
   - route `runoff` (mm/yr → m³/yr × cell area) over single steepest receivers on the flooded
     surface;
   - process the remaining depressions upstream to downstream (by the flood order of their spill
     cells, last first);
   - a depression's `Q_land` is the inflow from outside its cells;
   - a terminal depression passes nothing on.
5. **Lake balance:**
   - `E` is the mean PET (m/yr) over the depression and `P` the mean precipitation (m/yr);
   - when `E > P`, the equilibrium area is `A = Q_land / (E − P)`. If `A` is at least the
     depression's area, the lake fills to spill. Otherwise it is terminal: fill the cells in
     height order until the area reaches `A`, at the level of the last cell filled;
   - when `E ≤ P`, it fills to spill.
6. **Routing:**
   - flood again with seeds = outlets ∪ each terminal lake's lowest cell;
   - receivers are the steepest descent on that surface;
   - `flow_dir` is the D8 code (0 at the seeds);
   - `discharge` is runoff (m³/s) accumulated over those single receivers, with a terminal lake's
     bottom absorbing its inflow.
7. **Rasters:**
   - `river` = discharge > `river_q` and not lake, `big_river` likewise with `big_river_q`;
   - `water_surface` = the lake level on lake cells, else `height`.

- [ ] **Step 1: Write the failing tests:**
  - `a_map_without_outlets_keeps_its_water` (Review Focus 1): 64², `fixed` empty. The lowest
    cell's depression holds a lake or the sink, no discharge leaves the map, and `flow_dir` is 0
    only at the seeds.
  - `the_routing_surface_drains_to_an_outlet_over_non_increasing_heights` (physics test 7).
  - `a_terminal_lake_balances_evaporation_and_inflow` (physics test 8): a closed bowl in an arid
    climate (PET > P). `lake area × (E − P)` equals `Q_land` within one cell's
    `(E − P)·cell area`.
  - `wet_basins_fill_to_spill_and_route_over_their_outlet`.
  - `shallow_pits_are_filled_in_height`.
  - `channel_discharge_follows_single_receivers`: on a synthetic V-valley, the discharge at the
    mouth equals the total runoff, and side cells carry only their own.
  - `rivers_are_above_river_q_outside_lakes`.
  - `missing_thresholds_are_parameter_errors`: without `river_q` the `ParamError` names `river_q`
    (spec §8).
  - spec §8 lake inputs:
    - `a_lake_touching_a_drained_edge_drains_away`;
    - `a_pit_exactly_at_the_depth_limit_is_kept`: depth == `min_lake_depth_m` keeps the lake;
      only shallower ones are filled;
    - `a_terminal_lake_below_one_cell_is_no_lake`;
    - `a_lake_covering_most_of_the_map_stays_finite`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement**; register `hydrology`.
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): hydrology with a lake water balance and single-flow channels`.

### Task 29: `hydrology` — river lines and lake outlines

**Files:** Create `crates/theos-maps/src/stages/terrain/hydrology/{lines.rs, outlines.rs}`.

**Interfaces — Produces:**

```rust
pub struct RiverLine { pub points: Vec<[f64; 3]>, pub width_m: Vec<f64>, pub depth_m: Vec<f64>,
                       pub discharge_m3s: Vec<f64>, pub order: u32 }
pub struct LakeShape { pub id: u32, pub rings: Vec<Vec<[f64; 2]>>, pub surface_m: f64,
                       pub terminal: bool, pub outlet: Option<[f64; 2]>, pub depth_max_m: f64 }
pub struct WaterFeatures { pub rivers: Vec<RiverLine>, pub lakes: Vec<LakeShape> } // NAME "water_features"
```

`hydrology` now also writes `water_features`. New params: `width_a` and `depth_c`, the hydraulic
geometry constants, chosen by the builder within the cited ranges and recorded.

**River lines:**
- **Sources** are river cells without a river donor, taken in index order.
- **Tracing** follows Task 28's receivers. A line ends at a confluence cell (≥ 2 river donors), at
  a lake cell (the shore) or at a seed. The next line starts at the confluence, or at a lake's
  outlet.
- **Points** are cell centres in metres. Two Chaikin passes keep the end points.
- **Per-vertex values:** discharge is interpolated like the points; `width = a·Q^0.5` and
  `depth = c·Q^0.4` (`libm::pow`); the bed height is the bilinear `height` at the vertex
  (spec ruling 34).
- **Strahler order:** sources 1; at a confluence, the maximum, plus 1 when two inflows share it.

**Lakes:**
- `rings` from `kernels::contour::rings` on the lake's cells;
- `surface_m` is the level, and `depth_max_m` is level − min height;
- `outlet` is the spill cell's centre, `None` for a terminal lake.

- [ ] **Step 1: Write the failing tests:**
  - `lines_end_at_confluences_and_lake_shores`;
  - `strahler_orders_follow_their_rule`;
  - `per_vertex_arrays_match_the_points`;
  - `bed_height_is_the_terrain_height_at_the_vertex`;
  - `lake_rings_enclose_their_cells`;
  - `terminal_lakes_have_no_outlet`;
  - `river_lines_run_over_river_cells`: before smoothing, every vertex is a `river` cell, so the
    lines, the raster and `discharge` agree (spec §8, single-flow channels);
  - `single_cell_rivers_make_no_line`: a line needs two points (spec §8).
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): river lines and lake outlines`.

### Task 30: `ecology` and `elements`

**Files:** Create `crates/theos-maps/src/stages/terrain/{ecology.rs, elements.rs}`.

**`ecology`:**
- **Reads and writes:** reads `height`, `climate`, `drainage_area`; writes `temperature`,
  `slope`, `vegetation`, `snow`, `wetness`.
- **Temperature** comes from `climate::temperature`, and **slope** from `kernels::slope`.
- **Vegetation:**
  - `NPP = 3000·(1 − exp(−0.0009695·(AET − 20)))`, at least 0;
  - density = `min(NPP/2000, 1) × (1 − smoothstep(0.45, 0.9, slope)) × (1 + bonus)`, capped at 1;
  - the bonus is `wet_bonus × clamp((TWI − twi_ref)/twi_span, 0, 1)`, with
    `TWI = ln(a / max(tan β, 1e-3))` and `a = drainage_area / cell_m`;
  - params `wet_bonus` 0.25, `twi_ref` 6, `twi_span` 6 (the builder may tune them; record).
- **Snow** is 0.3.0's: `smoothstep(-1.5, -5, T)·(1 − smoothstep(0.6, 1.1, slope))`.

**`elements`** (port `terrain.py::elements`):
- **Reads and writes:** reads `river`, `lake`, `climate`, `vegetation`, `slope`, `rock`,
  `temperature`; writes the rasters `element_water`, `element_wood`, `element_earth`,
  `element_metal`, `element_fire` (L5).
- **Params:** `wet_sigma_m` 250 (0.3.0's 4 cells at 62.5 m).
- **Metrics** `water`, `wood`, `earth`, `metal`, `fire`: each element's share of the mix.

- [ ] **Step 1: Write the failing tests:**
  - `npp_follows_the_thornthwaite_memorial_formula`: AET 500 gives `3000·(1 − e^(−0.46536))`;
    AET 10 gives 0;
  - `steep_slopes_thin_vegetation`;
  - `wet_hollows_get_the_bonus_capped_at_one`;
  - `snow_needs_cold_and_gentle_ground`;
  - `elements_stay_in_the_unit_range` and `basalt_feeds_fire`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement**; register both.
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): ecology from the water balance and the element mix`.

### Task 31: The feature model and `settlements`

**Files:** Create `crates/theos-maps/src/features/{model.rs, pick.rs, names.rs}` and
`src/stages/terrain/settlements.rs`.

**Interfaces — Produces:**

```rust
pub struct FeatureHandle(pub u32);
pub enum Geometry { Point([f64; 2]), Line(Vec<[f64; 3]>), Polygon(Vec<Vec<[f64; 2]>>) } // metres
pub enum Prop { Num(f64), Int(i64), Text(String), Bool(bool), Nums(Vec<f64>), At([f64; 2]),
                Ref(FeatureHandle), Refs(Vec<FeatureHandle>) }
pub struct Feature { pub kind: &'static str, pub ty: String, pub geometry: Geometry,
                     pub props: BTreeMap<String, Prop> }
pub struct FeatureSet { /* Vec<Feature> */ }                   // NAME "features"
impl FeatureSet { pub fn add(&mut self, f: Feature) -> FeatureHandle;
                  pub fn get(&self, h: FeatureHandle) -> Option<&Feature>;
                  pub fn iter(&self) -> impl Iterator<Item = (FeatureHandle, &Feature)>;
                  pub fn of_kind(&self, kind: &str) -> impl Iterator<Item = (FeatureHandle, &Feature)>;
                  pub fn count_type(&self, ty: &str) -> usize;
                  pub fn set_type(&mut self, h: FeatureHandle, ty: &str) -> bool; }
pub fn pick(score: &Grid<f64>, n: usize, min_sep_m: f64, taken: &[[f64; 2]], taken_sep_m: f64,
            frame: &MapFrame) -> Vec<(usize, usize)>;          // port features.py::_pick
pub fn interior(frame: &MapFrame, edge_margin: f64) -> Grid<bool>; // margin = share of shorter side
pub fn generated_name(rng: &mut StageRng, used: &mut BTreeSet<String>, list: Option<&[String]>) -> String;
```

**`pick`:**
- greedy, the highest score first (ties by index);
- a cell is skipped when it lies within `min_sep_m` of a pick of this call, or within
  `taken_sep_m` of `taken`;
- cells with score ≤ 0 are never picked.

**`generated_name`:**
- without a list, it uses 0.3.0's syllable lists from `features.py::_name`;
- with a recipe list, it draws from the list without repeats.

**`settlements`** (port `features.py::settlements`):
- **Reads and writes:** reads `height`, `slope`, `temperature`, `river`, `lake`, `big_river`,
  `discharge`, `rock`, optional `features`; writes `features`, `site_quality`.
- **Params:**
  - `counts` (city, town, village, hamlet);
  - `spacing_km`, defaults 7, 4.5, 2.5, 1.4;
  - `edge_margin` 0.025 (spec ruling 46);
  - `names` (optional list).
- **Filter sizes in metres:** slope smoothing σ 187.5 m, confluence window 1.2 km, shore dilation
  0.5 km, hinterland window 3 km.
- **Properties:** `name` plus `generated_name: true`, `population`, `water`, `elevation_m`.

- [ ] **Step 1: Write the failing tests:**
  - `pick_keeps_its_spacing_and_ignores_non_positive_scores`;
  - `interior_excludes_the_margin_share`: on 64², a margin of 0.025 excludes 2 cells on each side
    (`round(0.025·64)`);
  - `settlements_avoid_lakes_big_rivers_and_the_margin`;
  - `cities_take_the_best_sites_first`;
  - `names_are_unique_and_come_from_the_recipe_list_when_given`;
  - `handles_follow_insertion_order_and_types_can_change`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement**; register `settlements`.
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): feature model and settlement siting`.

### Task 32: `pois` and `resources`

**Files:** Create `crates/theos-maps/src/stages/terrain/{pois.rs, resources.rs}`.

**`pois`** (port `features.py::pois`, L20):
- **Reads and writes:** reads `height`, `slope`, `temperature`, `rock`, `river`, `lake`, optional
  `features`; writes `features`.
- **Params:** `ruins` 2, `chimes` 2, `gates` 0 (0, 1 or 2), `min_chime_sep_km` 8,
  `chime_min_km_from_settlement` 3, `edge_margin` 0.025.
- **Types:**
  - `ruin`;
  - `chime` (0.3.0's extraction);
  - `gate-down` (0.3.0's `portal_in`, the arrival);
  - `gate-up` (0.3.0's `portal_out`, the ascent).
- **Properties:** none beyond kind and type (format §5). 0.3.0's `culture` and `note` texts are
  dropped: no lore in code.
- **Without settlements**, no distance to settlements applies.

**`resources`** (port `features.py::resources`):
- **Reads and writes:** reads 0.3.0's needs (with `element_*` instead of `elements`) and optional
  `omega`; writes `features`, `resource_fields`.
- **Types** are 0.3.0's with hyphens (`iron-ore`, `peat-reeds`, `resonance-crystal`, …).
- **Params:** `counts`, `spacing_km`, `edge_margin` 0.0125.
- **Properties:** `richness`, `tier`, `element`, `harvest`, `radius_m`.
- `resource_fields` is a value `ResourceFields { fields: BTreeMap<String, Grid<f64>> }` for the
  `resources` render.

- [ ] **Step 1: Write the failing tests:**
  - `ruins_sit_on_prominent_hard_ridges`;
  - `chimes_keep_their_separation_and_distance_from_settlements`;
  - `gates_are_placed_as_asked`: 0 gives none, 1 gives `gate-down`, 2 gives both;
  - `without_settlements_no_distance_applies`;
  - `deposits_carry_the_format_properties`;
  - `places_stay_off_the_margin`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement**; register both.
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): ruins, Chimes, gates and resource deposits`.

### Task 33: `roads`

**Files:** Create `crates/theos-maps/src/stages/terrain/roads/{mod.rs, cost.rs, bridges.rs,
network.rs}`.

**Interfaces — Produces:**

```rust
pub enum RoadClass { Trail, Track, Road, Main }               // u8 1–4 in the `roads` raster
pub struct Road { pub class: RoadClass, pub cells: Vec<u32>, pub from: FeatureHandle,
                  pub to: FeatureHandle, pub bridges: Vec<(u32, u32)> }
pub struct RoadNetwork { pub roads: Vec<Road> }                // NAME "road_network"
pub struct StepCost<'a> { /* layers + params */ }              // reused by `sites` for spurs
impl StepCost<'_> { pub fn step(&self, from: usize, to: usize, class: RoadClass) -> Option<f64>;
                    pub fn bridges(&self, from: usize) -> Vec<(usize, f64)>; }
```

**Reads and writes:** reads `height`, `vegetation`, `river`, `big_river`, `lake`, `flow_dir`,
`features`; writes `road_network`, `roads` (U8), `features` (bridge and ford sites).

**Params** (the builder sets the defaults and records them):
- `comfort_grade`;
- `max_grade` {main 0.08, road 0.10, track 0.15, trail 0.25} (spec ruling 35);
- `forest_cost`, `ford_cost`, `bridge_cost`, `angle_penalty`, `deck_cost`;
- `max_bridge_span_m` 400, `clearance_m` 3;
- `loop_detour`, `merge_discount`;
- `width_m` per class.

**Step cost** (spec §5.9 "Roads"; 16 directions):
- `L·(1 + (grade/comfort)²·6)` while grade ≤ the class's `max_grade`, with `L` the step length and
  grade `|Δh|/L`;
- plus `forest_cost·vegetation_to·L`;
- plus `ford_cost` per stream cell crossed;
- `None` when any cell the step passes through (Bresenham over the step's cells, end points
  included) is lake or big river.

**Bridge edges:**
- from a land cell, a straight line along each of the 16 directions across lake or big-river
  cells to the first land cell, up to `max_bridge_span_m`;
- cost: `span·bridge_cost·(1 + angle_penalty·|cos θ|)` + `deck_cost·deck_m`, with θ between the
  crossing and the river's flow direction (`flow_dir` of the first water cell; lakes: θ = 90°);
- `deck_m` is the higher bank + `clearance_m`.

**Network** (own design, spec §5.9):
- candidates are the `delaunay_edges` over settlements and gates;
- each candidate's cost is routed with the class from its end points (0.3.0's class rule,
  `features.py::roads`);
- `kruskal` gives the tree;
- a non-tree candidate between settlements of class village or higher is added when its cost
  through the current network exceeds `loop_detour` × its direct cost;
- links are routed in order of class (main first), then cost. Cells that already carry a road
  cost × `merge_discount`;
- trails run to each ruin from the nearest road cell or settlement;
- progress per routed link, and cancel through the kernel.

**Features:** a `bridge` site at each bridge edge (`span_m`, `deck_m`) and a `ford` site at each
stream crossing, with `road` → the road's handle.

- [ ] **Step 1: Write the failing tests:**
  - `no_road_step_exceeds_its_class_grade` (physics test 12): on a 64² synthetic ridge with a big
    river, every step of every road is within `max_grade`, except bridge edges;
  - `a_bridge_spans_the_water_in_one_edge`;
  - `knight_steps_check_every_cell_they_cross`;
  - `the_network_connects_every_settlement_and_gate`;
  - `a_loop_joins_when_the_detour_is_long`;
  - `trunks_merge_on_existing_roads`;
  - `fords_mark_stream_crossings`;
  - `fewer_than_two_places_build_no_network` (spec §8): no roads and no error;
  - `links_blocked_by_water_are_skipped_with_a_warning` (spec §8): a settlement on an island
    beyond `max_bridge_span_m` gets no road, and the report has one warning naming it.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement**; register `roads`.
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): roads by grade in 16 directions with bridges and fords`.

### Task 34: `sites` and `travel`

**Files:** Create `crates/theos-maps/src/stages/terrain/{sites.rs, travel.rs}`.

**`sites`** (port `features.py::sites`):
- **Reads and writes:** reads terrain layers, `road_network`, `features`, `discharge`,
  `water_thresholds`; writes `features`, `road_network`, `roads`.
- **Params:** `counts` (watchtower, cave, camp, …), `mill_q_factor` 2.7 (spec ruling 45),
  `edge_margin` 0.025.
- **Which sites:**
  - mills where discharge ≥ `mill_q_factor` × `river_q`;
  - mines and quarries at deposits (`deposit` → the handle);
  - lumber camps;
  - shrines;
  - watchtowers;
  - inns at the midpoint of main and road links longer than 0.3.0's threshold, away from
    settlements;
  - caves;
  - camps.
- **Spur tracks** are routed with `roads::StepCost` and added to the network and the raster.
- **Properties** link by handle: `road`, `serves`, `deposit`.

**`travel`:**
- **Reads and writes:** reads `height`, optional `road_network`, `lake`, `big_river`, `river`,
  `features`; writes `reach_min`.
- **Speed:** Tobler, `6·exp(−3.5·|s + 0.05|)` km/h with `libm::exp`. On roads, ×1.3 and at least
  1.2 m/s; off roads, × the off-path factor 0.6 (spec §5.9).
- **Penalties:** ford 120 s and lake 900 s, as 0.3.0.
- **Paths:** 16 directions, multi-source from every `chime`. `reach_min` is in minutes, and −1
  where unreachable or when there is no Chime (spec ruling 36).
- **Metrics:** `chime_to_chime_min` (the first Chime by handle to the second, only with ≥ 2) and
  `floor_pct_within_15_min`.

- [ ] **Step 1: Write the failing tests:**
  - `mills_need_enough_discharge`: a river at 2.6 × `river_q` gets no mill, at 2.8 × it may;
  - `inns_sit_midway_on_long_links`;
  - `spurs_join_the_network`;
  - `tobler_speed_peaks_on_a_gentle_downhill`: the fastest slope is −0.05;
  - `roads_are_faster_than_open_ground`;
  - `reach_is_minus_one_without_chimes`;
  - `the_chime_walk_is_reported_with_two_chimes`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement**; register both.
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): sites with spur tracks and walking times from the Chimes`.

### Task 35: `detuning`

**Files:** Create `crates/theos-maps/src/stages/terrain/detuning.rs`.

**`detuning`** (port `map-stages/omega_detuning.py`):
- reads `height`, `features`; writes `omega` (F64 in [0, 1]);
- params `baseline` 0.12, `ruin_radius_km` 2.5, and the rest of 0.3.0's parameters with its
  defaults;
- the drift noise comes from `sub_rng("drift")`.

- [ ] **Step 1: Write the failing tests:**

  ```rust
  // AC: CE-28.7
  #[test]
  fn detuning_is_a_registered_stage_that_writes_omega() -> TestResult {
      assert!(Registry::builtin().get("detuning").is_some());
      // 64² world with one ruin feature at the centre, run the stage through a context
      let omega = run_detuning_on_test_world()?;
      assert!(omega.as_slice().iter().all(|v| (0.0..=1.0).contains(v)));
      assert!(omega.get(32, 32) > omega.get(2, 2));          // hot spot around the ruin
      Ok(())
  }
  ```

  Add `the_baseline_holds_far_from_ruins`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement**; register `detuning`.
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): the omega detuning stage` (`Refs: CE-28.7`).

---

## Part H — outputs

### Task 36: The features output

**Files:** Create `crates/theos-maps/src/features/output.rs`,
`tests/fixtures/recipes/water-small.toml` (synthetic, 64², the full terrain pipeline, tuned so
seed 1 has a lake and a river).

**Interfaces — Produces:**

```rust
pub(crate) fn build(world: &World) -> Result<(geojson::FeatureCollection, BTreeMap<String, usize>),
                                             FeaturesError>;   // the collection and counts per type
```

**Building the features:**
- **Order:** the `FeatureSet` in handle order, then roads in network order, then rivers, then
  lakes.
- **Ids:** `<type>-<n>`, numbered per final type from 1 in that order (spec §5.10). Every `Ref` or
  `Refs` becomes ids (`from`, `to`, `road`, `serves`, `deposit`).
- **Roads:** kind `road`, type = class, a `LineString` of cell centres, with `from`, `to`,
  `length_m`, `width_m` and `grade_max`.
- **Rivers:** kind `water`, type `river`, `[x, y, bed]`, the per-vertex arrays and `order`.
- **Lakes:** kind `water`, type `lake`, a `Polygon` with `surface_m`, `terminal`, `outlet` (when
  present) and `depth_max_m`.
- **Every feature** gets `properties.kind` and `properties.type`. Coordinates are metres.

- [ ] **Step 1: Write the failing tests:**
  - `ids_are_numbered_per_type_in_order`;
  - `references_resolve_to_final_ids`;
  - `a_type_change_before_ids_renames_no_reference`;
  - the AC test: it plans `water-small.toml`, executes seed 1 and builds the features.

    ```rust
    // AC: CE-28.6
    #[test]
    fn features_carry_ids_kinds_types_and_water_properties() -> TestResult {
        let fc = run_and_build("water-small", 1)?;
        let lakes = of(&fc, "water", "lake"); let rivers = of(&fc, "water", "river");
        assert!(!lakes.is_empty() && !rivers.is_empty());           // the test's precondition
        for f in &fc.features { assert!(f.id.is_some()); assert!(prop(f, "kind").is_some() && prop(f, "type").is_some()); }
        for l in lakes { assert!(prop(l, "surface_m").is_some()); }
        for r in rivers { let n = coords(r).len();
            for key in ["width_m", "discharge_m3s"] { assert_eq!(array_len(r, key), n); }
            assert!(coords(r).iter().all(|c| c.len() == 3)); }   // bed height
        Ok(())
    }
    ```
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): the features output with stable ids` (`Refs: CE-28.6`).

### Task 37: The metrics module and the summary

**Files:** Modify `crates/theos-maps/src/metrics.rs`; create `src/summary.rs`.

**Interfaces — Produces:**

```rust
pub(crate) fn compute(world: &World, travel: Option<&Metrics>) -> serde_json::Value;
pub(crate) fn summary(metrics: serde_json::Value, feature_counts: &BTreeMap<String, usize>,
                      stage_metrics: &[(String, Metrics)], findings: &[StageFinding])
                      -> serde_json::Value;
```

**Metrics** (spec §5.14, 0.3.0's definitions and rounding):
- `elevation_m.relief` = max − min of `height`, rounded to whole metres;
- `lake_pct` = lake cells / all cells × 100, 2 decimals;
- `rivers_pct` (L14), 2 decimals;
- `vegetation_cover_pct` = the share of `vegetation` > 0.5 × 100, 1 decimal;
- `precip_mm_yr.median` = the median of `climate.fields.precipitation`, whole mm;
- `travel.chime_to_chime_min` = `travel`'s metric when present, 1 decimal.

Rounding uses `f64::round` (exact). The summary object holds these paths, then `features` (counts
per type), `stages` (`<id>` → metrics) and `findings`.

- [ ] **Step 1: Write the failing tests:**
  - `metrics_follow_0_3_0s_definitions` (a synthetic world with known values for each);
  - `river_share_counts_rivers_outside_lakes_over_all_cells`;
  - `the_chime_walk_comes_from_travel`;
  - `the_summary_nests_dotted_paths`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): parity metrics and the run summary`.

### Task 38: Building the package

**Files:** Create `crates/theos-maps/src/package.rs`.

**Interfaces — Produces:**

```rust
pub(crate) struct PackageLayer { pub name: &'static str, pub dtype: DType,
                                 pub units: Option<&'static str>, pub scale: Option<f64>,
                                 pub legend: Option<&'static [(&'static str, &'static str)]>,
                                 pub description: &'static str, pub source: Source }
pub(crate) const TERRAIN_LAYERS: &[PackageLayer];
pub(crate) fn build(world: &World, plan: &Plan, raw_recipe: &toml::Table, seed: u64,
                    inputs: &serde_json::Map<String, serde_json::Value>,
                    results: serde_json::Map<String, serde_json::Value>, summary: serde_json::Value,
                    features: geojson::FeatureCollection) -> Result<Package, PackageError>;
```

**`TERRAIN_LAYERS`, in this order** (spec §5.10, format §4.4):

| Layer | Type and encoding |
|---|---|
| `height`, `water_surface` | f32 m |
| `flow_dir` | u8 legend L19 |
| `drainage_area` | f32 m2 |
| `erosion`, `deposition` | f32 m |
| `discharge` | f32 m3/s |
| `precipitation` | f32 mm/yr, from `climate.fields` |
| `temperature` | f32 degC |
| `water` | u8 legend L19, from `river`, `big_river`, `lake` |
| `rock` | u8, the legend of rock names |
| `vegetation`, `snow` | u8, scale 1/255 |
| `roads` | u8 legend `0 none, 1 trail, 2 track, 3 road, 4 main` |
| the five `element_*` | u8, scale 1/255 |
| `omega` | u8, scale 1/255, only when present |

u8 values come from `num::to_u8_unit`, and f32 values from `num::to_f32`.

**Manifest:**
- `generator` = `"theos-maps@1.0.0"`, `requires` = `[]` (spec ruling 47), `kind` = `"terrain"`;
- `recipe_id`;
- `recipe` = `serde_json::to_value(raw_recipe)`;
- `seed`, `inputs`;
- `frame` {`size_m`, `cell_m`, `grid`};
- `results`, `summary`.

- [ ] **Step 1: Write the failing tests:**
  - `the_package_holds_the_standard_terrain_layers_in_order`;
  - `quantised_layers_round_trip_within_half_a_step`;
  - `the_manifest_records_recipe_seed_frame_and_empty_requires`;
  - `omega_is_written_only_when_detuning_ran`;
  - `the_package_reads_back_with_the_format_crate`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): terrain packages with the standard layers`.

### Task 39: Renders — infrastructure and `base`

**Files:**
- Modify: `Cargo.toml` (`tiny-skia`, `png`, `ab_glyph`, `wuff`).
- Create: `crates/theos-maps/src/render/{mod.rs, canvas.rs, fonts.rs, palette.rs, shade.rs,
  base.rs}`.

**Interfaces — Produces:**

```rust
pub(crate) struct RenderInput<'a> { pub world: &'a World, pub features: &'a FeatureSet,
                                    pub options: Option<&'a toml::Table>, pub cancel: &'a CancelToken,
                                    pub title: String }
pub(crate) type RenderFn = fn(&RenderInput<'_>) -> Result<tiny_skia::Pixmap, RenderError>;
pub(crate) const RENDERS: &[(&str, RenderFn)];   // base, truth, inworld, geology, resources, omega
pub(crate) fn encode_png(p: &tiny_skia::Pixmap) -> Result<Vec<u8>, RenderError>; // png, Compression::Fast
pub(crate) fn fonts() -> Result<&'static Fonts, RenderError>;  // OnceLock; Inter, Spectral via wuff
pub fn font_licences() -> &'static [(&'static str, &'static str)];  // names + OFL texts
```

**Fonts:**
- `include_bytes!("../../../../design/theos-ui/fonts/InterVariable.woff2")` and
  `Spectral-Regular.woff2`, decoded with `wuff` once;
- the licence texts are `include_str!` from `design/theos-ui/licenses/{Inter-OFL.txt,
  Spectral-OFL.txt}` (spec §5.11).

**`base`** is frame-exact at 2 pixels per cell, with no text:
- a three-light hillshade;
- rock and soil colours from `rocks::props`;
- vegetation, farmland around settlements and snow;
- lakes filled from `lake`;
- rivers from `water_features` as lines, width by `width_m`.

Rows of the raster pass check cancel.

- [ ] **Step 1: Write the failing tests:**
  - `base_is_frame_exact`: width = cols·2, height = rows·2;
  - the AC test:

    ```rust
    // AC: CE-28.6
    #[test]
    fn a_lake_at_a_known_position_lands_on_its_pixel() -> TestResult {
        // 64² flat world, cell 62.5 m; a lake of 5×5 cells centred on (1000 m, 1500 m)
        let png = render_base_of_test_world_with_lake([1000.0, 1500.0])?;
        let (px, py) = (1000.0 / 62.5 * 2.0, 1500.0 / 62.5 * 2.0);   // 32, 48
        assert_eq!(pixel(&png, px as u32, py as u32), palette::LAKE);
        Ok(())
    }
    ```
  - `fonts_decode_and_licences_are_exposed`;
  - `a_render_stops_on_cancel`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS. Run `cargo deny --locked check`.
- [ ] **Step 5: Commit** `feat(generators): frame-exact base render` (`Refs: CE-28.6`).

### Task 40: Renders — `truth`, `geology`, `resources`, `inworld`, `omega`, contact sheet

**Files:** Create `crates/theos-maps/src/render/{truth.rs, geology.rs, resources.rs, inworld.rs,
omega.rs, legend.rs, contact_sheet.rs}`.

**Interfaces — Produces:**

```rust
pub fn contact_sheet(tiles: &[(String, PathBuf)], columns: usize, out: &Path) -> Result<(), RenderError>;
```

`contact_sheet` takes a label and a PNG per tile and writes one PNG. Each render follows spec
§5.11's table:
- `truth` has every feature kind as a plain shape per type, names, a legend, a scale bar and a
  title (recipe, seed, relief, lakes, Chime walk);
- `inworld` draws texts only from `[render.inworld]` (`title`, `labels = [{text, at = [x, y]}]`).

- [ ] **Step 1: Write the failing tests:**
  - `each_render_has_its_size_and_is_not_blank`;
  - `inworld_draws_no_text_without_options`: two renders, without options and with
    `title = ""`, `labels = []`, are identical;
  - `inworld_draws_the_recipe_labels`: a label changes the pixels inside the frame;
  - `the_contact_sheet_tiles_in_rows`: 5 tiles in 2 columns give 3 rows;
  - `omega_marks_ruins`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): renders for people and the contact sheet`.

---

## Part I — the API, recipes, parity and determinism

### Task 41: `run`, outputs and cancel

**Files:**
- Create: `crates/theos-maps/src/api/{run.rs, outputs.rs}`, `tests/maps/{main.rs, support.rs,
  api.rs}`.
- Create the fixture `tests/fixtures/recipes/api-small.toml` (64², full pipeline with `detuning`,
  `renders = ["omega"]`, constraints including `features.chime`). The cancel test uses Task 27's
  `evolution-384.toml`.

**Interfaces — Produces:**

```rust
pub fn run(request: &RunRequest, recipe_dirs: &[PathBuf], out_dir: &Path,
           progress: &mut dyn FnMut(&Progress), cancel: &CancelToken) -> Result<RunManifest, RunError>;
pub(crate) fn run_inner(registry: &Registry, …same…) -> Result<(RunManifest, GoldenRun), RunError>;
    // `run` = run_inner(&Registry::builtin(), …).0; GoldenRun gets the stage hashes from
    // `execute`, `features` = SHA-256 of the features file's bytes, `package` = of the .theosmap
```

**Order of `run`** (spec §5.6):
1. `catalog` → `UnknownRecipe` if the id is not there.
2. `from` must be `None` (`Recipe(Invalid { parameter: "from" })`).
3. `plan` with `request.inputs`.
4. If any output name exists in `out_dir`, `OutputExists` (L10), before anything is written.
5. `execute`.
6. Progress `constraints`: `metrics::compute`, `summary`, `constraints::evaluate`.
7. Progress `features`: `features::build`.
8. Progress `renders`: the standard renders `base`, `truth`, `inworld`, `geology`, `resources`,
   plus the recipe's extras.
9. Progress `package`: `package::build` and `write_cancellable` with the token.

**Outputs:**
- each output is written as `.<name>.part`, and all are renamed at the end;
- on any error or cancel, the run deletes the `.part` files and renamed outputs it created, and
  nothing else;
- the roles are `base`, `truth`, `inworld`, `geology`, `resources`, `omega` (when asked),
  `features` and `package`;
- `RunManifest.frame.size_m` comes from the plan, and `timing` from `execute` plus the
  post-pipeline steps.

- [ ] **Step 1: Write the failing tests:**
  - `// AC: CE-28.2` `run_reports_progress_and_returns_outputs_and_results`: `api-small` seed 1.
    - The first event of each pipeline stage arrives in pipeline order, then `constraints`,
      `features`, `renders` and `package`.
    - The output keys are exactly `{base, truth, inworld, geology, resources, omega, features,
      package}`, and each file exists.
    - The results hold `features_chime`, `features_chime_min`, `features_chime_max`,
      `features_chime_ok` and `pass`. `frame.size_m` is the recipe's.
  - `// AC: CE-28.4` `cancelling_from_an_evolution_event_returns_within_one_second`:
    `evolution-384` seed 1. The progress closure cancels on the first `landscape_evolution` event
    with `done ≥ 10` and stores `Instant::now()`. The result is `Cancelled`, the elapsed time
    from the cancel to the return is < 1 s, and the folder is empty.
  - `// AC: CE-28.4` `cancelling_at_the_package_event_leaves_only_the_files_that_were_there`: the
    folder holds `notes.txt`; cancel on the `package` event. The result is `Cancelled`, and only
    `notes.txt` remains, with the same bytes.
  - `// AC: CE-28.4` `a_panicking_stage_ends_the_run_with_its_message_and_no_files` (in-crate,
    `run_inner` with a test registry whose second stage panics): `StagePanicked` with the message,
    and the folder is empty.
  - `existing_files_stay_and_name_clashes_are_refused` (Review Focus 4): with `s1-base.png`
    already there, the result is `OutputExists`, and every file keeps its bytes.
  - `from_and_inputs_are_refused`, `an_unknown_recipe_is_an_error`, and
    `the_memory_budget_comes_from_run_options` (a budget of 1 MiB at 384² gives `OverBudget` and
    no files).
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): run with outputs, cleanup and cancel` (`Refs: CE-28.2, CE-28.4`).

### Task 42: The built-in recipes and `describe`

**Files:** Create `crates/theos-maps/recipes/{hearth-valley, hearth-valley-omega,
fixture-small}.toml`, `src/api/describe.rs` and `tests/maps/recipes.rs`; modify
`src/recipe/builtin.rs`.

**Converting the recipes** from `D:\Projects\theos_maps-export\recipes\*.yaml` (spec §5.5):
- **Shape:**
  - `name` becomes `id`; add `title`;
  - every `[y, x]` pair becomes `[x, y]` (orogen paths, dome and basin centres; spec ruling 21);
  - `renders` holds only the extras: `[]`, `["omega"]`, `[]`;
  - the `constraints` stage becomes `[constraints]`.
- **Stages:**
  - `hydrology` gets `river_q` and `big_river_q`: 0.15 and 2.0, and 0.05 and 0.5 for
    `fixture-small`;
  - `landscape_evolution` gets the spec's process list (§5.5 example), with
    `climate_update every = 30` (25 in `fixture-small`, 0.3.0's default), `fluvial m = 0.45`,
    `g = 1.0` and `partition_flow exponent = "slope_adaptive"`. Task 44 tunes them;
  - `pois` renames (L20): `extractions` → `chimes`, `portals` → `gates` (hearth-valley 2, the
    others 0), `min_extraction_sep_km` → `min_chime_sep_km`,
    `extraction_min_km_from_settlement` → `chime_min_km_from_settlement`. Drop `settlements` on
    omega's `pois` (spec §10);
  - `resources` types get hyphens.
- **Constraints:**
  - `hearth-valley`: `"elevation_m.relief" = [800, 2200]`, `lake_pct = [0, 8]`,
    `vegetation_cover_pct = [25, 75]`, `"features.ruin" = [2, 2]`, `"features.chime" = [2, 2]`;
  - `hearth-valley-omega`: relief and vegetation as above, `"travel.chime_to_chime_min" = [60, 240]`,
    ruin and chime counts;
  - `fixture-small`: `"features.ruin" = [1, 1]`, `"features.chime" = [2, 2]` (L21).
- **Omega's `detuning`:** `baseline = 0.12`, `ruin_radius_km = 2.5`.
- **Filter sizes** given in cells in 0.3.0 become metres at 62.5 m.

**`describe`:**

```rust
pub fn describe(recipe_dirs: &[PathBuf]) -> Result<Description, RecipeError>;
```

It builds the catalog, plans each recipe with empty inputs, and lists the recipe as `RecipeInfo`
(`inputs: {}`, `seeded: true`, stages with their summaries) or as a problem.

- [ ] **Step 1: Write the failing tests** (`tests/maps/recipes.rs`):
  - `// AC: CE-28.1` `describe_lists_the_three_built_in_recipes`: name `theos-maps`, version
    `1.0.0`, `produces == ["map"]`, `accepts_notes == false`; the three ids are present with
    `inputs == {}` and `seeded`.
  - `// AC: CE-28.1` `a_folder_recipe_wins_over_the_built_in`: a temp folder with
    `fixture-small.toml`, title "Folder fixture". `describe` shows that title, once.
  - `// AC: CE-28.3` `a_stage_before_its_input_fails_before_any_stage_runs`: a folder recipe with
    `tectonics` before `grid`. `run` returns `Recipe(Invalid)` naming `tectonics` and `fixed`, and
    the progress closure was never called.
  - `// AC: CE-28.3` `an_unknown_stage_fails_before_any_stage_runs`.
  - `// AC: CE-28.3` `an_unknown_parameter_fails_before_any_stage_runs`.
  - `// AC: CE-28.3` `a_recipe_input_fails_as_unknown`: a recipe with `[inputs]`, and a request
    with `inputs = {"floor": 1}`.
  - `// AC: CE-28.7` `hearth_valley_omega_writes_the_omega_layer`: run seed 1, read the package
    with `theos_map_format::read`, and find layer `omega` with `scale` 1/255.
  - `a_broken_override_hides_the_built_in` (Review Focus 5): a broken `fixture-small.toml` in the
    folder. `describe` has a problem naming the file, `fixture-small` is not offered, and `run` of
    it fails with the recipe's parse error.
  - `every_built_in_recipe_plans`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): built-in terrain recipes and describe`
  (`Refs: CE-28.1, CE-28.3, CE-28.7`).

### Task 43: The developer example and resolution independence

**Files:**
- Modify: `crates/theos-maps/Cargo.toml` (dev: `clap`, `anyhow`).
- Create: `examples/run.rs`, `tests/maps/{resolution.rs, fixture_pairs.rs}`,
  `tests/fixtures/recipes/{hearth-192.toml, hearth-384.toml, basin-192.toml, basin-384.toml}`
  (synthetic: the hearth pair are copies of hearth-valley at both sizes, the basin pair is a
  closed basin at both sizes; same `size_km` in each pair).

**`examples/run.rs`:**

```text
cargo run -p theos-maps --release --example run -- <recipe>… --seeds 1..8 --out <dir> [--recipes <dir>]
```

- each seed's outputs go to `<out>/<recipe>/s<seed>/`;
- its `manifest.json` is written with `theos_map_format::json`;
- a table of the parity metrics and the pass count is written to stdout (`writeln!`);
- `<out>/<recipe>/contact-sheet.png` comes from the `truth` renders;
- errors use `anyhow`.

- [ ] **Step 1: Write the failing test** `slow_resolution_does_not_change_the_landscape` (physics
  test 13). Two pairs on seeds 1–4, each recipe at 192² cells of 125 m and 384² cells of 62.5 m
  over the same 24 km: `hearth-192/384` (hearth-valley) and `basin-192/384` (a closed basin under
  a dry climate; the basin sinks faster than sediment can fill it, so its lake is terminal and the
  water balance sets its area). The metrics (O2) count what both grids resolve:
  - relief (`elevation_m.relief`), tolerance 15 %;
  - lake share over lake bodies of at least 0.25 km² (8-connected `water` = lake cells; 16 cells
    at 125 m, 64 at 62.5 m), tolerance 15 %;
  - river length per km² through lakes: every cell whose `discharge` exceeds `river_q`, lake cells
    included, each with its D8 step to its receiver, tolerance 25 %.

  For every pair and metric, take the ratio 192² / 384² per seed (two zeros agree) and require the
  **median of the per-seed ratios** within the tolerance. The noise draws its coefficients per
  wavenumber of the map, so a seed gives one landscape at both sizes. Every basin run must also
  hold a lake body of at least 0.25 km². The hearth pair has no lakes at either size, so the basin
  pair carries the lake check. Fast tests in `fixture_pairs.rs` keep the hearth pair equal to
  `recipes/hearth-valley.toml` and the basin pair equal to each other, apart from `id`, `title`
  and `grid`.
- [ ] **Step 2: Run.** Expected: FAIL, or PASS if the stages are already right. A pass here
  without a code change is fine; this task's code is the example.
- [ ] **Step 3: Implement the example.** Run it on `fixture-small` with `--seeds 1..2`.
  Expected: two seed folders and a contact sheet.
- [ ] **Step 4: Fix** any stage whose parameters are still in cells (spec §5.5 "Units") until
  test 13 passes.
- [ ] **Step 5: Commit** `feat(generators): developer example and the resolution check`.

### Task 44: Tuning and parity (CE-28.5)

**Files:**
- Create: `crates/theos-maps/tests/fixtures/parity/theos_maps-0.3.0.toml` and
  `tests/maps/parity.rs`.
- Modify: `recipes/*.toml` (process parameters only).

**The fixture** copies spec Appendix A:

```toml
# 0.3.0 reference for CE-28.5 — copied from the P08 spec, Appendix A
[provenance]
toolset = "theos_maps 0.3.0"
python = "3.12"
fastscapelib = "0.3.0"
numpy = "2.5.4"
scipy = "1.18.1"
command = "uv run --no-project --python 3.12 --with fastscapelib==0.3.0 … (do_export=False)"
date = "2026-11-10"
reference = "Jann's export, in no repository"

[[seed]]
recipe = "hearth-valley"
seed = 1
relief_m = 1281
lake_pct = 3.63
rivers_pct = 0.93
vegetation_cover_pct = 55.1
precip_median_mm = 895
passes = true
# … all 24 rows of Appendix A; fixture-small rows have passes = "no constraints"
```

**The tolerance** (spec §5.14), per recipe and metric:
- the allowed range is `[min(ref min, wanted min), max(ref max, wanted max)]`, where "wanted" is
  the recipe's constraint on that metric if any;
- lake share's lower bound is always 0.

Seeds passing all constraints must be at least 0.3.0's: 2 for hearth-valley, 3 for omega, and
none asked for fixture-small.

- [ ] **Step 1: Write the test.**

  ```rust
  // AC: CE-28.5
  #[test]
  fn built_in_recipes_stay_within_the_0_3_0_range() -> TestResult {
      let reference = parity::load("tests/fixtures/parity/theos_maps-0.3.0.toml")?;
      for recipe in ["hearth-valley", "hearth-valley-omega", "fixture-small"] {
          let runs = (1..=8).map(|s| run_metrics(recipe, s)).collect::<Result<Vec<_>, _>>()?;
          for metric in Metric::ALL {
              let median = median(runs.iter().map(|r| r.get(metric)));
              let range = reference.allowed(recipe, metric);   // §5.14 rule
              assert!(range.contains(median), "{recipe} {metric}: median {median} outside {range:?}");
          }
          assert!(runs.iter().filter(|r| r.pass).count() >= reference.passes(recipe));
      }
      Ok(())
  }
  ```
- [ ] **Step 2: Run it.** Expected: likely FAIL before tuning. Record the medians and the pass
  counts in the ledger.
- [ ] **Step 3: Tune** with `examples/run` and its contact sheets, within spec §5.5:
  - keep the sources, rock mixes, climate, feature counts and constraints;
  - change only process parameters (K scale, uplift scale, `g`, `m`, kd, lake parameters);
  - record each round's medians in the ledger.

  If the tolerance cannot be reached without breaking §5.5, stop and tell Jann that CE-28.5 needs
  his decision (CLAUDE.md "When to stop").
- [ ] **Step 4: Run** the parity test. Expected: PASS. Rerun the whole crate's tests, since
  recipe changes affect the earlier fixtures.
- [ ] **Step 5: Commit** `feat(generators): tune the built-in recipes to 0.3.0's range` (`Refs: CE-28.5`).

### Task 45: Golden hashes and determinism

**Files:**
- Modify: `crates/theos-maps/src/golden.rs`.
- Create: `examples/bless.rs`, `tests/maps/{golden.rs, determinism.rs}`, and
  `tests/fixtures/golden/1.0.0.toml`, which `bless` writes (L23).

**Interfaces — Produces:**

```rust
pub const GOLDEN_RUNS: &[(&str, u64)] = &[("fixture-small", 1), ("fixture-small", 2), ("hearth-valley", 1)];
pub struct GoldenFile { pub generator: String, pub run: Vec<GoldenRun> }      // serde, TOML
pub fn compute(recipe: &str, seed: u64) -> Result<GoldenRun, RunError>;       // runs into a temp dir
pub fn compare(expected: &GoldenRun, actual: &GoldenRun) -> Result<(), GoldenMismatch>;
    // the first difference in pipeline order: stage key, layer (or "features"/"package"), both hashes
pub fn record(path: &Path, file: &GoldenFile) -> Result<(), GoldenError>;     // Exists when present
pub fn golden_path(version: &str) -> PathBuf;   // <CARGO_MANIFEST_DIR>/tests/fixtures/golden/<v>.toml
```

**Hashes:**
- `features` is the SHA-256 of the `features` output's bytes;
- `package` is the SHA-256 of the `.theosmap` bytes;
- the stage hashes are `Execution.golden`'s.

`compute` calls `run_inner(&Registry::builtin(), …)` into a temporary folder and returns its
`GoldenRun`. `examples/bless.rs` calls `compute` for `GOLDEN_RUNS` and
`record(golden_path(VERSION))`, and refuses with a message to raise `VERSION` when the file exists.

- [ ] **Step 1: Write the failing tests:**
  - `// AC: CE-77.1` `outputs_match_the_golden_hashes_of_this_version`: for every golden run,
    `compare(file, compute(..))`. A missing file fails with "run `cargo run -p theos-maps
    --example bless`".
  - `// AC: CE-77.2` `a_changed_output_fails_the_golden_comparison`: alter one stage hash of a
    computed run. `compare` names that stage and layer.
  - `// AC: CE-77.2` `bless_refuses_to_overwrite_an_existing_version`: `record` into a temp path
    twice; the second gives `Exists`, and the file is unchanged.
  - `two_runs_in_one_process_give_identical_packages` (spec §5.14). Running in two processes is
    covered by `bless` recording in one process and the golden test comparing in another (L28).
- [ ] **Step 2: Run.** Expected: FAIL (no golden file).
- [ ] **Step 3: Record** with `cargo run -p theos-maps --example bless`. This is the last build
  task that changes outputs (spec §5.14). A later review fix that changes outputs deletes the file
  and records it again (spec ruling 41).
- [ ] **Step 4: Run** the crate's tests. Expected: PASS.
- [ ] **Step 5: Commit** `feat(generators): golden hashes per generator version`
  (`Refs: CE-77.1, CE-77.2`).
- [ ] **Step 6: Platforms.** After the PR is open (Task 46), add the labels `windows` and `macos`.
  Link the `platforms` runs (Windows, macOS Apple silicon, macOS Intel) in the PR.

### Task 46: Finish

**Files:** Modify `crates/theos-map-format/README.md`, `crates/theos-maps/README.md` and the crate
docs.

- [ ] **Step 1: Update the READMEs.** Say what each crate does, how to run the example and
  `bless`, the determinism rules for contributors (libm, `num::max`, no `HashMap`), and how to add
  a stage (one module + one registry line).
- [ ] **Step 2: Make the contact sheet.** Run
  `cargo run -p theos-maps --release --example run -- hearth-valley hearth-valley-omega fixture-small --seeds 1..8 --out <scratch>`.
  Keep the contact sheets for the PR (L24).
- [ ] **Step 3: Verify:**
  - `cargo xtask gate` on Windows;
  - `cargo xtask ac-trace --phase P08`, where every CE-76.x, CE-28.x and CE-77.x has a passing
    test;
  - `cargo tree -e features -i num-traits`, `-i serde_json` and `-i flate2`, to confirm L25 and
    the deflate backend.

  Read each output.
- [ ] **Step 4: Commit** `docs(generators): map crate guides`.

---

## After the build

The theos-phase skill continues:
1. **Final review loop:** fresh whole-branch reviewers on the most capable model, with this plan,
   the spec, G1–G20, the ACs and every earlier finding, until one is clean (cap 5).
2. **Verification.**
3. **The PR.** It is opened only once both gate PRs are on `main` and `main` is merged in (L1):
   - title `feat(generators): map format and terrain generator (P08)`;
   - the template filled, with every ruling of spec §9 and this plan (L1–L28);
   - the `windows` and `macos` labels;
   - the contact sheet;
   - auto-merge only if the PR touches no protected path.
